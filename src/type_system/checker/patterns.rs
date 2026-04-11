extern crate alloc;

use super::helpers::{ensure_boolean_type, literal_to_core_type};
use crate::ast::{AstNode, MatchArm, Pattern};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::vec::Vec;

impl TypeChecker {
    /// Type-check a match expression and infer a single compatible arm result type.
    pub(super) fn type_check_match_expr(
        &mut self,
        scrutinee: &crate::ast::Expr,
        arms: &[MatchArm],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let scrutinee_type = self.type_check_expr(scrutinee)?;
        let mut arm_result_type: Option<CoreType> = None;

        for (arm_index, arm) in arms.iter().enumerate() {
            self.symbol_table.enter_scope();
            self.type_check_pattern(&arm.pattern, &scrutinee_type)?;

            if let Some(ref guard_expr) = arm.guard {
                let guard_type = self.type_check_expr(guard_expr)?;
                ensure_boolean_type(&guard_type, guard_expr.span(), "match arm guard")?;
            }

            let current_arm_type = self.type_check_expr(&arm.body)?;
            self.symbol_table.exit_scope();

            if let Some(ref expected_arm_type) = arm_result_type {
                if !self.types_compatible(expected_arm_type, &current_arm_type) {
                    return Err(TypeError::MatchArmTypeMismatch {
                        arm_index,
                        expected: expected_arm_type.to_string(),
                        found: current_arm_type.to_string(),
                        span: TypeError::span_from_span(arm.span),
                    });
                }
            } else {
                arm_result_type = Some(current_arm_type);
            }
        }

        if !self.match_is_exhaustive(arms, &scrutinee_type) {
            let missing_variants = self.collect_missing_variants(arms, &scrutinee_type);
            return Err(TypeError::NonExhaustiveMatch {
                missing_variants,
                span: TypeError::span_from_span(span),
            });
        }

        arm_result_type.ok_or_else(|| TypeError::ConstraintSolvingFailed {
            reason: "match expression must include at least one arm".to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Validate a match pattern against the scrutinee type and register arm-local bindings.
    pub(super) fn type_check_pattern(
        &mut self,
        pattern: &Pattern,
        scrutinee_type: &CoreType,
    ) -> Result<(), TypeError> {
        match *pattern {
            Pattern::Wildcard { .. } => Ok(()),
            Pattern::Binding { ref name, span } => {
                self.symbol_table.register(SymbolInfo {
                    name: name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: scrutinee_type.clone(),
                    visibility: Visibility::Private,
                    source_location: span,
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                });
                Ok(())
            }
            Pattern::Literal {
                ref value, span, ..
            } => {
                let found = literal_to_core_type(value);
                if self.types_compatible(scrutinee_type, &found) {
                    Ok(())
                } else {
                    Err(TypeError::PatternTypeMismatch {
                        expected: scrutinee_type.to_string(),
                        found: found.to_string(),
                        span: TypeError::span_from_span(span),
                    })
                }
            }
            Pattern::Tuple {
                ref elements, span, ..
            } => {
                if let CoreType::Array(ref element_type) = *scrutinee_type {
                    for element_pattern in elements {
                        self.type_check_pattern(element_pattern, element_type.as_ref())?;
                    }
                    Ok(())
                } else {
                    Err(TypeError::PatternTypeMismatch {
                        expected: "tuple/array-compatible type".to_owned(),
                        found: scrutinee_type.to_string(),
                        span: TypeError::span_from_span(span),
                    })
                }
            }
            Pattern::Variant {
                ref type_name,
                ref variant_name,
                ref fields,
                span,
            } => {
                let variant_type_name = type_name
                    .clone()
                    .unwrap_or_else(|| Self::adt_name_for_type(scrutinee_type));
                let qualified_variant = format!("{variant_type_name}.{variant_name}");
                let Some(known_variants) = self.adt_variants.get(&variant_type_name) else {
                    return Err(TypeError::PatternTypeMismatch {
                        expected: variant_type_name,
                        found: variant_name.clone(),
                        span: TypeError::span_from_span(span),
                    });
                };

                if !known_variants.contains(&qualified_variant) {
                    return Err(TypeError::PatternTypeMismatch {
                        expected: Self::adt_name_for_type(scrutinee_type),
                        found: qualified_variant,
                        span: TypeError::span_from_span(span),
                    });
                }

                for field_pattern in fields {
                    self.type_check_pattern(&field_pattern.1, scrutinee_type)?;
                }

                Ok(())
            }
        }
    }

    /// Resolve the nominal ADT name used for variant lookup and exhaustiveness tracking.
    fn adt_name_for_type(core_type: &CoreType) -> String {
        match *core_type {
            CoreType::Generic { ref name, .. } => name.clone(),
            _ => core_type.to_string(),
        }
    }

    /// Determine whether a set of match arms exhaustively covers the scrutinee type.
    fn match_is_exhaustive(&self, arms: &[MatchArm], scrutinee_type: &CoreType) -> bool {
        if arms.iter().any(|arm| {
            matches!(
                arm.pattern,
                Pattern::Wildcard { .. } | Pattern::Binding { .. }
            )
        }) {
            return true;
        }

        let type_name = Self::adt_name_for_type(scrutinee_type);
        if !self.adt_variants.contains_key(&type_name) {
            return false;
        }

        self.collect_missing_variants(arms, scrutinee_type)
            .is_empty()
    }

    /// Collect fully-qualified variant names that remain uncovered by match arms.
    fn collect_missing_variants(
        &self,
        arms: &[MatchArm],
        scrutinee_type: &CoreType,
    ) -> Vec<String> {
        let type_name = Self::adt_name_for_type(scrutinee_type);
        let all_variants = self
            .adt_variants
            .get(&type_name)
            .cloned()
            .unwrap_or_default();

        let mut covered = Vec::new();
        let default_type_name = Self::adt_name_for_type(scrutinee_type);
        for arm in arms {
            if let Pattern::Variant {
                type_name: ref pattern_type_name,
                ref variant_name,
                ..
            } = arm.pattern
            {
                let resolved_type = pattern_type_name
                    .clone()
                    .unwrap_or_else(|| default_type_name.clone());
                covered.push(format!("{resolved_type}.{variant_name}"));
            }
        }

        all_variants
            .into_iter()
            .filter(|variant| !covered.contains(variant))
            .collect()
    }
}
