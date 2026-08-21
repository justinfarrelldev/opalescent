#![allow(
    clippy::missing_docs_in_private_items,
    reason = "private guard typing helpers are internal implementation details"
)]
//! Control-flow expression typing helpers for the type checker.

extern crate alloc;

use super::helpers::{ensure_boolean_type, type_mismatch_error};
use crate::ast::{AstNode, BinaryOp, Expr, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{format, string::String};

/// Context describing how a guard expression is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardUsage {
    /// Guard result feeds into a surrounding expression.
    Expression,
    /// Guard is used for control flow, typically within a statement position.
    Statement,
}

/// Metadata describing the binding introduced by a guard expression.
#[derive(Debug, Clone)]
pub struct GuardBindingInfo<'type_ref> {
    /// Name of the binding created when the guard succeeds.
    pub name: &'type_ref str,
    /// Optional user-provided type annotation for the binding.
    pub annotation: Option<&'type_ref Type>,
    /// Whether the binding is declared as mutable.
    pub is_mutable: bool,
    /// Source span that identifies the binding declaration site.
    pub span: Span,
}

/// Input bundle for guard typing to keep checker call sites compact.
#[derive(Debug)]
pub struct GuardCheckRequest<'expr, 'binding, 'error, 'stmt, 'expected> {
    pub expr: &'expr Expr,
    pub binding: &'binding GuardBindingInfo<'binding>,
    pub error_binding: Option<&'error str>,
    pub else_branch: &'stmt Stmt,
    pub usage: GuardUsage,
    pub expected_return: Option<&'expected [CoreType]>,
}

impl TypeChecker {
    /// Type-check an `if` expression and return its resulting type.
    pub(super) fn type_check_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
        span: Span,
        expected_type: Option<&CoreType>,
    ) -> Result<CoreType, TypeError> {
        let condition_type = self.type_check_expr(condition)?;
        ensure_boolean_type(&condition_type, condition.span(), "if condition")?;

        let then_type = self.within_new_scope(|checker| {
            checker.apply_true_branch_type_narrowing(condition);
            checker.infer_stmt_value_type(then_branch)
        })?;

        if let Some(else_stmt) = else_branch {
            let else_type =
                self.within_new_scope(|checker| checker.infer_stmt_value_type(else_stmt))?;

            self.add_constraint(TypeConstraint::equality(
                then_type.clone(),
                else_type.clone(),
                Some(then_branch.span()),
                Some(else_stmt.span()),
            ));

            if !self.types_compatible(&then_type, &else_type) {
                return Err(type_mismatch_error(
                    &then_type,
                    Some(then_branch.span()),
                    &else_type,
                    else_stmt.span(),
                ));
            }

            Ok(then_type)
        } else {
            if let Some(required_type) = expected_type {
                if !matches!(required_type, &CoreType::Unit)
                    && !matches!(required_type, &CoreType::Variable(_))
                {
                    return Err(TypeError::MissingElseBranch {
                        expected_type: required_type.to_string(),
                        span: TypeError::span_from_span(span),
                    });
                }
            }
            self.type_check_stmt_with_return(then_branch, None)?;
            let unit = CoreType::Unit;
            self.add_constraint(TypeConstraint::equality(
                unit.clone(),
                unit.clone(),
                Some(span),
                Some(span),
            ));
            Ok(unit)
        }
    }

    /// Infer the resulting value type produced by a statement in expression context.
    pub(super) fn infer_stmt_value_type(&mut self, stmt: &Stmt) -> Result<CoreType, TypeError> {
        match *stmt {
            Stmt::Expression { ref expr, .. } => self.type_check_expr(expr),
            Stmt::Block { ref statements, .. } => self.infer_block_value_type(statements),
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                span,
                ..
            } => {
                self.type_check_if_expr(condition, then_branch, else_branch.as_deref(), span, None)
            }
            _ => {
                self.type_check_stmt_with_return(stmt, None)?;
                Ok(CoreType::Unit)
            }
        }
    }

    /// Infer the resulting value type of a block in expression position.
    fn infer_block_value_type(&mut self, statements: &[Stmt]) -> Result<CoreType, TypeError> {
        let Some((last_stmt, prefix)) = statements.split_last() else {
            return Ok(CoreType::Unit);
        };

        for statement in prefix {
            self.type_check_stmt_with_return(statement, None)?;
        }

        self.infer_stmt_value_type(last_stmt)
    }

    /// Type-check a nominal variant refinement condition created by `is ... into`.
    pub(super) fn type_check_refinement_expr(
        &mut self,
        value: &Expr,
        variant: &Expr,
        payload_binding: &str,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let Some(variable_name) = Self::direct_identifier_name(value) else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "left side of `is ... into` must be a direct identifier".to_owned(),
                span: TypeError::span_from_span(value.span()),
            });
        };
        let (family_name, variant_owner, _) = self.refinement_variant_identity(variant, span)?;
        let Some(fields) = self.adt_fields_for_owner(variant_owner.as_str()) else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!("unknown variant '{variant_owner}' in `is ... into` refinement"),
                span: TypeError::span_from_span(variant.span()),
            });
        };
        if fields.is_empty() {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "payloadless variant '{variant_owner}' cannot bind `into {payload_binding}`"
                ),
                span: TypeError::span_from_span(span),
            });
        }

        let value_type = self.type_check_expr(value)?;
        if !self.refinement_value_type_allows_family(
            &value_type,
            family_name.as_str(),
            variant_owner.as_str(),
        ) {
            return Err(type_mismatch_error(
                &Self::nominal_type(family_name.as_str()),
                Some(value.span()),
                &value_type,
                value.span(),
            ));
        }
        if self.symbol_table().lookup(variable_name).is_none() {
            return Err(TypeError::SymbolNotFound {
                name: variable_name.to_owned(),
                suggestion: self.suggest_visible_identifier(variable_name),
                span: TypeError::span_from_span(value.span()),
            });
        }
        Ok(CoreType::Boolean)
    }

    /// Extract the direct identifier name required by nominal variant refinement.
    fn direct_identifier_name(value: &Expr) -> Option<&str> {
        match *value {
            Expr::Identifier { ref name, .. } => Some(name.as_str()),
            Expr::Parenthesized { ref expr, .. } => Self::direct_identifier_name(expr.as_ref()),
            _ => None,
        }
    }

    /// Resolve `Family.Variant` to its family and fully-qualified variant owner.
    fn refinement_variant_identity(
        &self,
        variant: &Expr,
        fallback_span: Span,
    ) -> Result<(String, String, Span), TypeError> {
        let normalized_variant = Self::unwrapped_expr(variant);
        let &Expr::Member {
            object: ref object_expr,
            member: ref member_name,
            span,
            ..
        } = normalized_variant
        else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "right side of `is ... into` must name a known Type.Variant".to_owned(),
                span: TypeError::span_from_span(fallback_span),
            });
        };
        let &Expr::Identifier {
            name: ref family_name,
            ..
        } = Self::unwrapped_expr(object_expr.as_ref())
        else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "right side of `is ... into` must name a known Type.Variant".to_owned(),
                span: TypeError::span_from_span(span),
            });
        };
        let variant_owner = format!("{family_name}.{member_name}");
        let is_registered_variant = self
            .adt_variants
            .get(family_name)
            .is_some_and(|variants| variants.iter().any(|candidate| candidate == &variant_owner));
        if !is_registered_variant {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!("unknown variant '{variant_owner}' in `is ... into` refinement"),
                span: TypeError::span_from_span(span),
            });
        }
        Ok((family_name.clone(), variant_owner, span))
    }

    /// Return an expression with transparent parentheses removed.
    fn unwrapped_expr(expr: &Expr) -> &Expr {
        match *expr {
            Expr::Parenthesized { ref expr, .. } => Self::unwrapped_expr(expr.as_ref()),
            _ => expr,
        }
    }

    /// Check whether a value's static type can contain the requested nominal family.
    fn refinement_value_type_allows_family(
        &self,
        value_type: &CoreType,
        family_name: &str,
        variant_owner: &str,
    ) -> bool {
        let family_type = Self::nominal_type(family_name);
        let variant_type = Self::nominal_type(variant_owner);
        if self.types_compatible(value_type, &family_type)
            || self.types_compatible(value_type, &variant_type)
        {
            return true;
        }
        let &CoreType::Generic {
            name: ref type_name,
            type_args: ref union_members,
        } = value_type
        else {
            return false;
        };
        if type_name != "GuardErrorContext" {
            return false;
        }
        union_members
            .iter()
            .any(|error_type| Self::declared_error_type_covers(&family_type, error_type))
    }

    /// Build a non-generic nominal core type by name.
    pub(super) fn nominal_type(name: &str) -> CoreType {
        CoreType::Generic {
            name: name.to_owned(),
            type_args: Vec::new(),
        }
    }

    /// Build the branch-local symbol used for narrowed variants and payload bindings.
    const fn refinement_symbol(
        name: String,
        core_type: CoreType,
        source_location: Span,
    ) -> SymbolInfo {
        SymbolInfo {
            name,
            symbol_type: SymbolType::Constant,
            core_type,
            visibility: Visibility::Private,
            source_location,
            is_let_binding: true,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        }
    }

    /// Register the narrowed direct identifier and optional `into` payload binding.
    fn apply_variant_refinement_scope(
        &mut self,
        variable_name: &str,
        payload_binding: Option<&str>,
        variant_owner: &str,
        source_location: Span,
    ) {
        let narrowed_type = Self::nominal_type(variant_owner);
        if let Some(mut symbol) = self.symbol_table().lookup(variable_name).cloned() {
            symbol.core_type = narrowed_type.clone();
            symbol.source_location = source_location;
            self.symbol_table.register(symbol);
        }
        if let Some(binding_name) = payload_binding.filter(|name| *name != "_") {
            self.symbol_table.register(Self::refinement_symbol(
                binding_name.to_owned(),
                narrowed_type,
                source_location,
            ));
        }
    }

    /// Extract nominal variant refinement metadata from an `if` condition.
    fn extract_variant_refinement(
        &self,
        condition: &Expr,
    ) -> Option<(String, Option<String>, String, Span)> {
        let condition = Self::unwrapped_expr(condition);
        match *condition {
            Expr::Refinement {
                value: ref value_expr,
                variant: ref variant_expr,
                payload_binding: ref payload_binding_name,
                span,
                ..
            } => {
                let variable_name = Self::direct_identifier_name(value_expr.as_ref())?.to_owned();
                let (_, variant_owner, _) = self
                    .refinement_variant_identity(variant_expr.as_ref(), span)
                    .ok()?;
                Some((
                    variable_name,
                    Some(payload_binding_name.clone()),
                    variant_owner,
                    span,
                ))
            }
            Expr::Binary {
                left: ref left_expr,
                operator: BinaryOp::Is,
                right: ref right_expr,
                span,
                ..
            } => {
                let variable_name = Self::direct_identifier_name(left_expr.as_ref())?.to_owned();
                let (_, variant_owner, _) = self
                    .refinement_variant_identity(right_expr.as_ref(), span)
                    .ok()?;
                Some((variable_name, None, variant_owner, span))
            }
            _ => None,
        }
    }

    /// Apply narrowing for `if x is TypeName` and nominal variant refinements in the true branch scope.
    pub(super) fn apply_true_branch_type_narrowing(&mut self, condition: &Expr) {
        if let Some((variable_name, payload_binding, variant_owner, source_location)) =
            self.extract_variant_refinement(condition)
        {
            self.apply_variant_refinement_scope(
                variable_name.as_str(),
                payload_binding.as_deref(),
                variant_owner.as_str(),
                source_location,
            );
            return;
        }

        let narrowed = self.extract_is_type_narrowing(condition);
        let Some((variable_name, narrowed_type, source_location)) = narrowed else {
            return;
        };

        let existing_symbol = self.symbol_table().lookup(variable_name.as_str()).cloned();
        if let Some(mut symbol) = existing_symbol {
            symbol.core_type = narrowed_type;
            symbol.source_location = source_location;
            self.symbol_table.register(symbol);
        }
    }

    /// Extract `(variable_name, narrowed_type, span)` from `x is TypeName`.
    fn extract_is_type_narrowing(&self, condition: &Expr) -> Option<(String, CoreType, Span)> {
        let condition = match *condition {
            Expr::Parenthesized { ref expr, .. } => expr.as_ref(),
            _ => condition,
        };

        let &Expr::Binary {
            ref left,
            operator: BinaryOp::Is,
            ref right,
            span,
            ..
        } = condition
        else {
            return None;
        };

        let Expr::Identifier {
            name: ref variable_name,
            ..
        } = *left.as_ref()
        else {
            return None;
        };

        let Expr::Identifier {
            name: ref type_name,
            ..
        } = *right.as_ref()
        else {
            return None;
        };

        self.environment()
            .lookup_type(type_name.as_str(), span)
            .ok()
            .cloned()
            .map(|narrowed_type| (variable_name.clone(), narrowed_type, span))
    }

    /// Type-check a guard expression and return its success type.
    pub(super) fn type_check_guard_expression(
        &mut self,
        expr: &Expr,
        binding_name: &str,
        binding_type: Option<&Type>,
        is_mutable: bool,
        else_branch: &Stmt,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let binding_info = GuardBindingInfo {
            name: binding_name,
            annotation: binding_type,
            is_mutable,
            span,
        };
        self.type_check_guard_expr(GuardCheckRequest {
            expr,
            binding: &binding_info,
            error_binding: None,
            else_branch,
            usage: GuardUsage::Expression,
            expected_return: None,
        })
    }
}
