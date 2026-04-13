//! Guard expression typing helpers extracted from `expressions.rs`.

extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardUsage};
use super::helpers::coerce_literal_to_expected;
use crate::ast::{AstNode, Expr, Stmt};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Result of typing the `else` branch of a guard expression.
///
/// Statement-context handlers typically short-circuit control flow, while
/// expression-context handlers yield fallback values compatible with the
/// function's success type.
#[derive(Debug, Clone, PartialEq)]
enum GuardElseOutcome {
    /// Handler yielded a fallback value that can substitute for the success type.
    FallbackValue(CoreType),
    /// Handler performs control-flow actions instead of yielding a value.
    ControlFlow,
}

impl TypeChecker {
    /// Type-check a `guard` expression.
    ///
    /// This function ensures that:
    /// 1. The guarded expression is a function call that can produce errors.
    /// 2. The success value is correctly bound to a new variable.
    /// 3. The `else` branch type-checks in isolation so that guard scopes remain precise.
    pub(super) fn type_check_guard_expr(
        &mut self,
        expr: &Expr,
        binding: &GuardBindingInfo<'_>,
        else_branch: &Stmt,
        usage: GuardUsage,
        expected_return: Option<&[CoreType]>,
    ) -> Result<CoreType, TypeError> {
        let (callee_expr, args, call_span) = match *expr {
            Expr::Call {
                ref callee,
                ref args,
                span: call_span,
                ..
            } => (callee.as_ref(), args.as_slice(), call_span),
            _ => {
                return Err(TypeError::GuardOnNonErrorExpression {
                    span: TypeError::span_from_span(expr.span()),
                });
            }
        };

        let (success_type, callee_error_types) =
            self.resolve_guard_callee_signature(expr, callee_expr)?;

        self.type_check_call_expr(callee_expr, None, args, call_span, expr.node_id().0)?;

        if let Some(annotated_type_ast) = binding.annotation {
            let annotated_type =
                ast_type_to_core_type(annotated_type_ast).map_err(TypeError::from)?;
            if !self.types_compatible(&success_type, &annotated_type) {
                return Err(TypeError::GuardBindingTypeMismatch {
                    expected: success_type.to_string(),
                    found: annotated_type.to_string(),
                    span: TypeError::span_from_span(annotated_type_ast.span()),
                });
            }
        }

        if let Some(active_errors) = self.guard_error_stack.last() {
            if !Self::guard_error_type_sets_match(active_errors.as_slice(), &callee_error_types) {
                return Err(TypeError::GuardChainedErrorMismatch {
                    expected: Self::format_error_type_list(active_errors.as_slice()),
                    found: Self::format_error_type_list(&callee_error_types),
                    span: TypeError::span_from_span(expr.span()),
                });
            }
        }

        let else_outcome = self.type_check_guard_else_with_scope(
            else_branch,
            callee_error_types.as_slice(),
            usage,
            &success_type,
            expected_return,
        )?;

        let _: GuardElseOutcome = else_outcome;

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };
        self.symbol_table.register(SymbolInfo {
            name: binding.name.to_owned(),
            symbol_type,
            core_type: success_type.clone(),
            visibility: Visibility::Private,
            source_location: binding.span,
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });

        Ok(success_type)
    }

    /// Resolve and validate the guarded call signature, returning success and error types.
    fn resolve_guard_callee_signature(
        &mut self,
        expr: &Expr,
        callee_expr: &Expr,
    ) -> Result<(CoreType, Vec<CoreType>), TypeError> {
        let callee_type = self.type_check_expr(callee_expr)?;
        match callee_type {
            CoreType::Function {
                return_types,
                error_types,
                ..
            } => {
                if error_types.is_empty() {
                    return Err(TypeError::GuardOnNonErrorExpression {
                        span: TypeError::span_from_span(expr.span()),
                    });
                }
                if return_types.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        found: return_types.len(),
                        span: TypeError::span_from_span(expr.span()),
                    });
                }
                let Some(return_type) = return_types.first() else {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "guard callee has no declared return type".to_owned(),
                        span: TypeError::span_from_span(expr.span()),
                    });
                };
                Ok((return_type.clone(), error_types))
            }
            _ => Err(TypeError::GuardOnNonErrorExpression {
                span: TypeError::span_from_span(expr.span()),
            }),
        }
    }

    /// Type-check a guard else-branch inside an isolated scope and guard context.
    fn type_check_guard_else_with_scope(
        &mut self,
        else_branch: &Stmt,
        callee_error_types: &[CoreType],
        usage: GuardUsage,
        success_type: &CoreType,
        expected_return: Option<&[CoreType]>,
    ) -> Result<GuardElseOutcome, TypeError> {
        self.symbol_table.enter_scope();
        self.guard_else_depth = self.guard_else_depth.saturating_add(1);
        self.guard_error_stack.push(callee_error_types.to_vec());

        let else_result = self.type_check_guard_else_branch(
            else_branch,
            callee_error_types,
            usage,
            success_type,
            expected_return,
        );

        let popped = self.guard_error_stack.pop();
        debug_assert!(
            popped.is_some(),
            "guard error stack underflow when exiting guard else handling"
        );
        debug_assert!(
            self.guard_else_depth > 0,
            "guard_else_depth should be positive when exiting guard else scope"
        );
        self.guard_else_depth = self.guard_else_depth.saturating_sub(1);
        self.symbol_table.exit_scope();

        else_result
    }

    /// Type-check the `else` branch of a guard expression.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::GuardElseIncompatibleError`] when the handler fails to
    /// align with the guard's success type or declared error types.
    fn type_check_guard_else_branch(
        &mut self,
        else_branch: &Stmt,
        error_types: &[CoreType],
        usage: GuardUsage,
        success_type: &CoreType,
        expected_return: Option<&[CoreType]>,
    ) -> Result<GuardElseOutcome, TypeError> {
        match *else_branch {
            Stmt::Expression { ref expr, span, .. } => {
                let handler_type = self.type_check_expr(expr)?;
                match usage {
                    GuardUsage::Expression => {
                        let fallback_type = if self.types_compatible(success_type, &handler_type) {
                            handler_type
                        } else if let Some(adjusted) =
                            coerce_literal_to_expected(success_type, expr, &handler_type)
                        {
                            adjusted
                        } else {
                            return Err(TypeError::GuardElseIncompatibleError {
                                expected: success_type.to_string(),
                                found: handler_type.to_string(),
                                span: TypeError::span_from_span(span),
                            });
                        };

                        if !self.guard_error_types_are_homogeneous(error_types) {
                            return Err(TypeError::GuardElseIncompatibleError {
                                expected: Self::format_error_type_list(error_types),
                                found: fallback_type.to_string(),
                                span: TypeError::span_from_span(span),
                            });
                        }

                        Ok(GuardElseOutcome::FallbackValue(fallback_type))
                    }
                    GuardUsage::Statement => {
                        if matches!(expr, &Expr::Propagate { .. })
                            || self.types_compatible(&CoreType::Unit, &handler_type)
                        {
                            Ok(GuardElseOutcome::ControlFlow)
                        } else {
                            Err(TypeError::GuardElseIncompatibleError {
                                expected: CoreType::Unit.to_string(),
                                found: handler_type.to_string(),
                                span: TypeError::span_from_span(span),
                            })
                        }
                    }
                }
            }
            Stmt::Block { ref statements, .. } => {
                self.type_check_statements(statements, expected_return)?;
                Ok(GuardElseOutcome::ControlFlow)
            }
            ref other => {
                self.type_check_stmt_with_return(other, expected_return)?;
                Ok(GuardElseOutcome::ControlFlow)
            }
        }
    }

    /// Determine whether all error types declared by the guard's callee are mutually compatible.
    fn guard_error_types_are_homogeneous(&self, error_types: &[CoreType]) -> bool {
        if error_types.is_empty() {
            return true;
        }

        let reference = &error_types[0];
        error_types.iter().all(|error_ty| {
            self.types_compatible(reference, error_ty) && self.types_compatible(error_ty, reference)
        })
    }

    /// Determine whether two error type sets are equivalent irrespective of ordering.
    pub(super) fn guard_error_type_sets_match(left: &[CoreType], right: &[CoreType]) -> bool {
        if left.len() != right.len() {
            return false;
        }

        let mut left_rendered: Vec<String> = left.iter().map(ToString::to_string).collect();
        let mut right_rendered: Vec<String> = right.iter().map(ToString::to_string).collect();
        left_rendered.sort();
        right_rendered.sort();
        left_rendered == right_rendered
    }

    /// Render a deterministic, comma-separated list of error type names for diagnostics.
    pub(super) fn format_error_type_list(error_types: &[CoreType]) -> String {
        if error_types.is_empty() {
            return "<none>".to_owned();
        }

        let mut rendered = Vec::with_capacity(error_types.len());
        for error_type in error_types {
            rendered.push(error_type.to_string());
        }

        rendered.join(", ")
    }
}
