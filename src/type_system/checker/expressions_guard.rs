#![allow(
    clippy::missing_docs_in_private_items,
    clippy::pattern_type_mismatch,
    clippy::needless_pass_by_value,
    clippy::needless_pass_by_ref_mut,
    clippy::unused_self,
    clippy::match_same_arms,
    clippy::shadow_unrelated,
    clippy::arithmetic_side_effects,
    clippy::too_many_lines,
    reason = "guard typing helpers are internal and intentionally verbose"
)]
//! Guard expression typing helpers extracted from `expressions.rs`.

extern crate alloc;

use super::control_flow::{GuardCheckRequest, GuardUsage};
use super::helpers::coerce_literal_to_expected;
use crate::ast::{AstNode, Expr, LabeledValue, Stmt};
use crate::token::Span;
use crate::type_system::checker::{ActiveGuardErrorBinding, TypeChecker};
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

#[derive(Debug, Clone, PartialEq)]
enum GuardReturnWrapperShape {
    Valid { expr: Expr },
    NotWrapper,
}

#[derive(Debug, Clone, Copy)]
struct GuardElseScopeRequest<'types, 'branch, 'binding, 'expected> {
    callee_error_types: &'types [CoreType],
    else_branch: &'branch Stmt,
    error_binding: Option<&'binding str>,
    pending_success_binding: Option<&'binding str>,
    usage: GuardUsage,
    success_type: &'types CoreType,
    expected_return: Option<&'expected [CoreType]>,
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
        request: GuardCheckRequest<'_, '_, '_, '_, '_>,
    ) -> Result<CoreType, TypeError> {
        let GuardCheckRequest {
            expr,
            binding,
            error_binding,
            else_branch,
            usage,
            expected_return,
        } = request;
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

        let previous_guard_subject_context = self.context.in_guard_subject_context;
        self.context.in_guard_subject_context = true;
        let call_result =
            self.type_check_call_expr(callee_expr, None, args, call_span, expr.node_id().0);
        self.context.in_guard_subject_context = previous_guard_subject_context;
        call_result?;

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

        let should_chain_guard_errors = usage != GuardUsage::Statement || error_binding.is_none();
        if should_chain_guard_errors {
            if let Some(active_errors) = self.context.guard_error_stack.last() {
                if !Self::guard_error_type_sets_match(active_errors.as_slice(), &callee_error_types)
                {
                    return Err(TypeError::GuardChainedErrorMismatch {
                        expected: Self::format_error_type_list(active_errors.as_slice()),
                        found: Self::format_error_type_list(&callee_error_types),
                        span: TypeError::span_from_span(expr.span()),
                    });
                }
            }
        }

        let should_register_success_binding =
            usage != GuardUsage::Statement || matches!(binding.name, name if name != "_");
        let pending_success_binding = (usage == GuardUsage::Statement
            && should_register_success_binding)
            .then_some(binding.name);
        self.type_check_guard_else_with_scope(GuardElseScopeRequest {
            callee_error_types: callee_error_types.as_slice(),
            else_branch,
            error_binding,
            pending_success_binding,
            usage,
            success_type: &success_type,
            expected_return,
        })?;

        if should_register_success_binding {
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
                is_pure: false,
            });
        }

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
                    return Err(TypeError::GuardWrapperSourceInvalid {
                        source_span: TypeError::span_from_span(expr.span()),
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
        request: GuardElseScopeRequest<'_, '_, '_, '_>,
    ) -> Result<GuardElseOutcome, TypeError> {
        let GuardElseScopeRequest {
            callee_error_types,
            else_branch,
            error_binding,
            pending_success_binding,
            usage,
            success_type,
            expected_return,
        } = request;
        self.symbol_table.enter_scope();
        self.context.guard_else_depth = self.context.guard_else_depth.saturating_add(1);
        self.context
            .guard_error_stack
            .push(callee_error_types.to_vec());
        if let Some(success_binding_name) = pending_success_binding {
            self.context
                .pending_guard_success_bindings
                .push(success_binding_name.to_owned());
        }

        if let Some(error_binding_name) = error_binding {
            let error_binding_type = if let [single_error_type] = callee_error_types {
                single_error_type.clone()
            } else {
                CoreType::Generic {
                    name: "GuardErrorContext".to_owned(),
                    type_args: callee_error_types.to_vec(),
                }
            };
            self.symbol_table.register(SymbolInfo {
                name: error_binding_name.to_owned(),
                symbol_type: SymbolType::Constant,
                core_type: error_binding_type,
                visibility: Visibility::Private,
                source_location: else_branch.span(),
                is_let_binding: true,
                is_mutable: false,
                read_count: 0,
                is_pure: false,
            });
            self.context
                .active_guard_error_bindings
                .push(ActiveGuardErrorBinding {
                    name: error_binding_name.to_owned(),
                    source_location: else_branch.span(),
                });
        }

        let else_result = self.type_check_guard_else_branch(
            else_branch,
            callee_error_types,
            error_binding,
            usage,
            success_type,
            expected_return,
        );

        if error_binding.is_some() {
            let popped_error_binding = self.context.active_guard_error_bindings.pop();
            debug_assert!(
                popped_error_binding.is_some(),
                "active guard error binding stack underflow when exiting guard else scope"
            );
        }

        if pending_success_binding.is_some() {
            let hidden_binding = self.context.pending_guard_success_bindings.pop();
            debug_assert!(
                hidden_binding.is_some(),
                "pending guard success binding stack underflow when exiting guard else scope"
            );
        }

        let popped = self.context.guard_error_stack.pop();
        debug_assert!(
            popped.is_some(),
            "guard error stack underflow when exiting guard else handling"
        );
        debug_assert!(
            self.context.guard_else_depth > 0,
            "guard_else_depth should be positive when exiting guard else scope"
        );
        self.context.guard_else_depth = self.context.guard_else_depth.saturating_sub(1);
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
        error_binding: Option<&str>,
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
                    GuardUsage::Statement if error_binding.is_some() => {
                        self.type_check_named_guard_error_clause(else_branch, expected_return)?;
                        Ok(GuardElseOutcome::ControlFlow)
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
            Stmt::Block {
                ref statements,
                span,
                ..
            } => {
                if usage == GuardUsage::Statement && error_binding.is_some() {
                    self.type_check_guard_error_clause_statements(
                        statements.as_slice(),
                        expected_return,
                        span,
                    )?;
                } else {
                    self.type_check_statements(statements, expected_return)?;
                }
                Ok(GuardElseOutcome::ControlFlow)
            }
            ref other => {
                if usage == GuardUsage::Statement && error_binding.is_some() {
                    self.type_check_named_guard_error_clause(other, expected_return)?;
                } else {
                    self.type_check_stmt_with_return(other, expected_return)?;
                }
                Ok(GuardElseOutcome::ControlFlow)
            }
        }
    }

    pub(super) fn type_check_guard_error_clause_statements(
        &mut self,
        statements: &[Stmt],
        expected_return: Option<&[CoreType]>,
        clause_span: Span,
    ) -> Result<(), TypeError> {
        let meaningful_len = statements
            .iter()
            .rposition(|statement| !matches!(statement, Stmt::Comment { .. }))
            .map_or(0, |index| index + 1);
        let meaningful_statements = &statements[..meaningful_len];

        let Some((terminal, prelude)) = meaningful_statements.split_last() else {
            return Err(TypeError::GuardErrorClauseMissingTerminal {
                clause_span: TypeError::span_from_span(clause_span),
            });
        };

        for statement in prelude {
            self.type_check_guard_error_clause_prelude_statement(statement, expected_return)?;
        }

        if prelude.is_empty() && matches!(terminal, Stmt::PropagateGuardError { .. }) {
            return Err(TypeError::GuardShorthandRequired {
                span: TypeError::span_from_span(terminal.span()),
            });
        }

        self.type_check_guard_error_clause_terminal_statement(terminal, expected_return)?;
        Ok(())
    }

    pub(super) fn type_check_guard_error_clause_statement(
        &mut self,
        statement: &Stmt,
        expected_return: Option<&[CoreType]>,
        allow_terminal_propagate: bool,
    ) -> Result<(), TypeError> {
        if allow_terminal_propagate {
            self.type_check_guard_error_clause_terminal_statement(statement, expected_return)
        } else {
            self.type_check_guard_error_clause_prelude_statement(statement, expected_return)?;
            Ok(())
        }
    }

    fn type_check_named_guard_error_clause(
        &mut self,
        clause: &Stmt,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        match clause {
            Stmt::Block {
                statements, span, ..
            } => self.type_check_guard_error_clause_statements(
                statements.as_slice(),
                expected_return,
                *span,
            ),
            Stmt::PropagateGuardError { span, .. } => Err(TypeError::GuardShorthandRequired {
                span: TypeError::span_from_span(*span),
            }),
            other => self.type_check_guard_error_clause_terminal_statement(other, expected_return),
        }
    }

    fn type_check_guard_error_clause_prelude_statement(
        &mut self,
        statement: &Stmt,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        match statement {
            Stmt::Block {
                statements, span, ..
            } => {
                self.symbol_table.enter_scope();
                let nested_validation = self.type_check_guard_error_clause_statements(
                    statements.as_slice(),
                    expected_return,
                    *span,
                );
                self.symbol_table.exit_scope();

                match nested_validation {
                    Err(
                        TypeError::GuardErrorClauseMissingTerminal { .. }
                        | TypeError::GuardPropagateErrNotFinal { .. }
                        | TypeError::GuardReturnErrInvalid { .. }
                        | TypeError::GuardWrapperSourceInvalid { .. }
                        | TypeError::GuardShorthandRequired { .. },
                    )
                    | Ok(()) => Ok(()),
                    Err(other) => Err(other),
                }
            }
            Stmt::PropagateGuardError { span, .. } => Err(TypeError::GuardPropagateErrNotFinal {
                propagate_span: TypeError::span_from_span(*span),
            }),
            Stmt::Return { values, span, .. } => {
                if self.return_statement_forwards_active_guard_error(values.as_slice()) {
                    return Err(TypeError::GuardReturnErrInvalid {
                        return_span: TypeError::span_from_span(*span),
                    });
                }
                self.type_check_stmt_with_return(statement, expected_return)
            }
            Stmt::Expression { expr, span, .. } => {
                let handler_type = self.type_check_expr(expr)?;
                if !self.types_compatible(&CoreType::Unit, &handler_type) {
                    return Err(TypeError::GuardElseIncompatibleError {
                        expected: CoreType::Unit.to_string(),
                        found: handler_type.to_string(),
                        span: TypeError::span_from_span(*span),
                    });
                }
                Ok(())
            }
            _ => self.type_check_stmt_with_return(statement, expected_return),
        }
    }

    fn type_check_guard_error_clause_terminal_statement(
        &mut self,
        statement: &Stmt,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        match statement {
            Stmt::PropagateGuardError {
                error_binding,
                span,
                ..
            } => self.type_check_guard_error_propagate_terminal(error_binding, *span, true),
            Stmt::Return { values, span, .. } => {
                if self.return_statement_forwards_active_guard_error(values.as_slice()) {
                    return Err(TypeError::GuardReturnErrInvalid {
                        return_span: TypeError::span_from_span(*span),
                    });
                }
                let wrapper_shape =
                    self.classify_guard_error_wrapper_shape(values.as_slice(), *span)?;
                match wrapper_shape {
                    GuardReturnWrapperShape::Valid { expr } => {
                        self.type_check_guard_error_wrapper_return(expr, *span)?;
                        Ok(())
                    }
                    GuardReturnWrapperShape::NotWrapper => {
                        self.type_check_stmt_with_return(statement, expected_return)?;
                        Err(TypeError::GuardErrorClauseMissingTerminal {
                            clause_span: TypeError::span_from_span(*span),
                        })
                    }
                }
            }
            Stmt::Expression { expr, span, .. } => {
                let handler_type = self.type_check_expr(expr)?;
                if !self.types_compatible(&CoreType::Unit, &handler_type) {
                    return Err(TypeError::GuardElseIncompatibleError {
                        expected: CoreType::Unit.to_string(),
                        found: handler_type.to_string(),
                        span: TypeError::span_from_span(*span),
                    });
                }
                if matches!(expr, Expr::Propagate { .. }) {
                    return Err(TypeError::GuardShorthandRequired {
                        span: TypeError::span_from_span(*span),
                    });
                }
                Err(TypeError::GuardErrorClauseMissingTerminal {
                    clause_span: TypeError::span_from_span(*span),
                })
            }
            Stmt::Block {
                statements, span, ..
            } => {
                self.symbol_table.enter_scope();
                let result = self.type_check_guard_error_clause_statements(
                    statements.as_slice(),
                    expected_return,
                    *span,
                );
                self.symbol_table.exit_scope();
                result
            }
            Stmt::Continue { .. } | Stmt::Break { .. } => {
                self.type_check_stmt_with_return(statement, expected_return)?;
                Ok(())
            }
            Stmt::Let {
                binding,
                initializer,
                ..
            } if self.guard_clause_is_error_alias_discard(
                binding.name.as_str(),
                initializer.as_ref(),
            ) =>
            {
                self.type_check_stmt_with_return(statement, expected_return)?;
                Ok(())
            }
            _ => {
                self.type_check_stmt_with_return(statement, expected_return)?;
                Err(TypeError::GuardErrorClauseMissingTerminal {
                    clause_span: TypeError::span_from_span(statement.span()),
                })
            }
        }
    }

    fn type_check_guard_error_propagate_terminal(
        &mut self,
        error_binding: &str,
        span: crate::token::Span,
        allow_terminal_propagate: bool,
    ) -> Result<(), TypeError> {
        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return Err(TypeError::GuardPropagateErrNotFinal {
                propagate_span: TypeError::span_from_span(span),
            });
        };

        if error_binding != active_error_binding.name || !allow_terminal_propagate {
            return Err(TypeError::GuardPropagateErrNotFinal {
                propagate_span: TypeError::span_from_span(span),
            });
        }

        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(),
        };

        let active_guard_errors = self
            .context
            .guard_error_stack
            .last()
            .cloned()
            .unwrap_or_default();
        let is_subset = active_guard_errors
            .iter()
            .all(|error_type| current_fn_error_types.contains(error_type));
        if !is_subset {
            return Err(TypeError::PropagateErrorMismatch {
                expected: Self::format_error_type_list(&current_fn_error_types),
                found: Self::format_error_type_list(&active_guard_errors),
                span: TypeError::span_from_span(
                    self.symbol_table.current_function_span().unwrap_or(span),
                ),
                callee_span: TypeError::span_from_span(span),
            });
        }

        Ok(())
    }

    fn return_statement_forwards_active_guard_error(&self, values: &[LabeledValue]) -> bool {
        if values.len() != 1 {
            return false;
        }

        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return false;
        };

        let value = &values[0].value;
        matches!(
            value,
            Expr::Identifier { name, .. } if name == &active_error_binding.name
        )
    }

    fn classify_guard_error_wrapper_shape(
        &self,
        values: &[LabeledValue],
        span: Span,
    ) -> Result<GuardReturnWrapperShape, TypeError> {
        if values.len() != 1 {
            return Ok(GuardReturnWrapperShape::NotWrapper);
        }

        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return Err(TypeError::GuardWrapperSourceInvalid {
                source_span: TypeError::span_from_span(span),
            });
        };

        let value = &values[0].value;
        self.classify_guard_error_wrapper_expr_shape(value, active_error_binding, span)
    }

    fn classify_guard_error_wrapper_expr_shape(
        &self,
        expr: &Expr,
        active_binding: &ActiveGuardErrorBinding,
        span: Span,
    ) -> Result<GuardReturnWrapperShape, TypeError> {
        match expr {
            Expr::Constructor { fields, .. } => {
                let mut source_field_seen = false;

                for field in fields {
                    if field.name == "source" {
                        source_field_seen = true;
                        if !self.wrapper_source_matches_active_guard_binding(
                            &field.value,
                            active_binding,
                        ) {
                            return Err(TypeError::GuardWrapperSourceInvalid {
                                source_span: TypeError::span_from_span(field.span),
                            });
                        }
                    }
                }

                if source_field_seen {
                    Ok(GuardReturnWrapperShape::Valid { expr: expr.clone() })
                } else {
                    Err(TypeError::GuardWrapperSourceInvalid {
                        source_span: TypeError::span_from_span(span),
                    })
                }
            }
            Expr::Parenthesized { expr: inner, .. } => {
                self.classify_guard_error_wrapper_expr_shape(inner, active_binding, span)
            }
            _ => Ok(GuardReturnWrapperShape::NotWrapper),
        }
    }

    fn type_check_guard_error_wrapper_return(
        &mut self,
        expr: Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(),
        };

        let wrapper_type = self.type_check_expr(&expr)?;
        if current_fn_error_types
            .iter()
            .any(|declared_error| self.types_compatible(declared_error, &wrapper_type))
        {
            Ok(())
        } else {
            Err(TypeError::PropagateErrorMismatch {
                expected: Self::format_error_type_list(&current_fn_error_types),
                found: wrapper_type.to_string(),
                span: TypeError::span_from_span(
                    self.symbol_table.current_function_span().unwrap_or(span),
                ),
                callee_span: TypeError::span_from_span(span),
            })
        }
    }

    fn wrapper_source_matches_active_guard_binding(
        &self,
        expr: &Expr,
        active_binding: &ActiveGuardErrorBinding,
    ) -> bool {
        let Expr::Identifier { name, .. } = expr else {
            return false;
        };

        if name != &active_binding.name {
            return false;
        }

        self.symbol_table()
            .lookup(name)
            .is_some_and(|symbol| symbol.source_location == active_binding.source_location)
    }

    fn guard_clause_is_error_alias_discard(
        &self,
        binding_name: &str,
        initializer: Option<&Expr>,
    ) -> bool {
        if !binding_name.starts_with('_') {
            return false;
        }

        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return false;
        };

        let Some(Expr::Identifier { name, .. }) = initializer else {
            return false;
        };

        name == &active_error_binding.name
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
