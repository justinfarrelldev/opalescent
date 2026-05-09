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
use crate::ast::{AstNode, Expr, LabeledValue, LiteralValue, Stmt};
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GuardElseValidation {
    handled_bound_error: bool,
    terminal_propagate_seen: bool,
    handling_statement_count: usize,
    referenced_active_error_binding: bool,
}

impl GuardElseValidation {
    const fn with_handled_bound_error(handled_bound_error: bool) -> Self {
        Self {
            handled_bound_error,
            terminal_propagate_seen: false,
            handling_statement_count: 0,
            referenced_active_error_binding: false,
        }
    }
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
        let else_outcome = self.type_check_guard_else_with_scope(GuardElseScopeRequest {
            callee_error_types: callee_error_types.as_slice(),
            else_branch,
            error_binding,
            pending_success_binding,
            usage,
            success_type: &success_type,
            expected_return,
        })?;

        let _: GuardElseOutcome = else_outcome;

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
                .push(error_binding_name.to_owned());
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
                        let validation = self.type_check_guard_error_clause_statement(
                            else_branch,
                            expected_return,
                            true,
                        )?;
                        if validation.terminal_propagate_seen
                            && validation.handling_statement_count == 0
                        {
                            return Err(TypeError::ConstraintSolvingFailed {
                                reason: "guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed"
                                    .to_owned(),
                                span: TypeError::span_from_span(span),
                            });
                        }
                        if !validation.handled_bound_error {
                            return Err(TypeError::ConstraintSolvingFailed {
                                reason:
                                    "guard error clause must handle or propagate the bound error"
                                        .to_owned(),
                                span: TypeError::span_from_span(span),
                            });
                        }
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
                    let validation = self.type_check_guard_error_clause_statements(
                        statements.as_slice(),
                        expected_return,
                        span,
                    )?;
                    if !validation.handled_bound_error {
                        return Err(TypeError::ConstraintSolvingFailed {
                            reason: "guard error clause must handle or propagate the bound error"
                                .to_owned(),
                            span: TypeError::span_from_span(span),
                        });
                    }
                } else {
                    self.type_check_statements(statements, expected_return)?;
                }
                Ok(GuardElseOutcome::ControlFlow)
            }
            ref other => {
                if usage == GuardUsage::Statement && error_binding.is_some() {
                    let validation =
                        self.type_check_guard_error_clause_statement(other, expected_return, true)?;
                    if validation.terminal_propagate_seen
                        && validation.handling_statement_count == 0
                    {
                        return Err(TypeError::ConstraintSolvingFailed {
                            reason: "guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed"
                                .to_owned(),
                            span: TypeError::span_from_span(other.span()),
                        });
                    }
                    if !validation.handled_bound_error {
                        return Err(TypeError::ConstraintSolvingFailed {
                            reason: "guard error clause must handle or propagate the bound error"
                                .to_owned(),
                            span: TypeError::span_from_span(other.span()),
                        });
                    }
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
        clause_span: crate::token::Span,
    ) -> Result<GuardElseValidation, TypeError> {
        self.type_check_guard_error_clause_statements_with_terminal_mode(
            statements,
            expected_return,
            clause_span,
            true,
        )
    }

    fn type_check_guard_error_clause_statements_with_terminal_mode(
        &mut self,
        statements: &[Stmt],
        expected_return: Option<&[CoreType]>,
        clause_span: crate::token::Span,
        allow_terminal_propagate: bool,
    ) -> Result<GuardElseValidation, TypeError> {
        let mut validation = GuardElseValidation::with_handled_bound_error(false);

        for (index, statement) in statements.iter().enumerate() {
            let is_last = allow_terminal_propagate && (index + 1 == statements.len());
            let statement_validation =
                self.type_check_guard_error_clause_statement(statement, expected_return, is_last)?;

            if statement_validation.terminal_propagate_seen {
                if !is_last {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "propagate err is only valid as the final statement of a guard error clause"
                            .to_owned(),
                        span: TypeError::span_from_span(statement.span()),
                    });
                }
                if validation.handling_statement_count == 0 {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "guard error clause must perform handling before propagating; replace this guard with shorthand propagate <call>() when no handling is needed"
                            .to_owned(),
                        span: TypeError::span_from_span(statement.span()),
                    });
                }
                validation.handled_bound_error = true;
                validation.terminal_propagate_seen = true;
                validation.referenced_active_error_binding = true;
            } else if statement_validation.handling_statement_count > 0 {
                validation.handling_statement_count +=
                    statement_validation.handling_statement_count;
            }

            if statement_validation.handled_bound_error {
                validation.handled_bound_error = true;
            }
            if statement_validation.referenced_active_error_binding {
                validation.referenced_active_error_binding = true;
            }
        }

        if validation.terminal_propagate_seen {
            validation.handled_bound_error = true;
        }

        if !validation.handled_bound_error {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "guard error clause must handle or propagate the bound error".to_owned(),
                span: TypeError::span_from_span(clause_span),
            });
        }

        Ok(validation)
    }

    pub(super) fn type_check_guard_error_clause_statement(
        &mut self,
        statement: &Stmt,
        expected_return: Option<&[CoreType]>,
        allow_terminal_propagate: bool,
    ) -> Result<GuardElseValidation, TypeError> {
        match *statement {
            Stmt::PropagateGuardError {
                ref error_binding,
                span,
                ..
            } => self.type_check_guard_error_propagate_terminal(
                error_binding,
                span,
                allow_terminal_propagate,
            ),
            Stmt::Return {
                ref values, span, ..
            } => {
                if self.return_statement_forwards_active_guard_error(values.as_slice()) {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "return err is not valid in a guard error clause; use propagate err to forward the guard error"
                            .to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                }
                self.type_check_stmt_with_return(statement, expected_return)?;
                Ok(GuardElseValidation {
                    handled_bound_error: true,
                    terminal_propagate_seen: false,
                    handling_statement_count: 1,
                    referenced_active_error_binding: self
                        .stmt_references_active_guard_error_binding(statement),
                })
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {
                self.type_check_stmt_with_return(statement, expected_return)?;
                Ok(GuardElseValidation {
                    handled_bound_error: true,
                    terminal_propagate_seen: false,
                    handling_statement_count: 1,
                    referenced_active_error_binding: self
                        .stmt_references_active_guard_error_binding(statement),
                })
            }
            Stmt::Expression { ref expr, span, .. } => {
                let handler_type = self.type_check_expr(expr)?;
                if !self.types_compatible(&CoreType::Unit, &handler_type) {
                    return Err(TypeError::GuardElseIncompatibleError {
                        expected: CoreType::Unit.to_string(),
                        found: handler_type.to_string(),
                        span: TypeError::span_from_span(span),
                    });
                }
                let referenced_active_error_binding =
                    self.stmt_references_active_guard_error_binding(statement);
                Ok(GuardElseValidation {
                    handled_bound_error: referenced_active_error_binding
                        || self.expression_counts_as_guard_error_handling(expr),
                    terminal_propagate_seen: false,
                    handling_statement_count: usize::from(!Self::expression_is_guard_error_noop(
                        expr,
                    )),
                    referenced_active_error_binding,
                })
            }
            Stmt::Block {
                ref statements,
                span,
                ..
            } => {
                self.symbol_table.enter_scope();
                let nested_validation = self
                    .type_check_guard_error_clause_statements_with_terminal_mode(
                        statements.as_slice(),
                        expected_return,
                        span,
                        false,
                    );
                self.symbol_table.exit_scope();
                nested_validation.map(|validation| GuardElseValidation {
                    handled_bound_error: validation.handled_bound_error,
                    terminal_propagate_seen: false,
                    handling_statement_count: 1,
                    referenced_active_error_binding: validation.referenced_active_error_binding,
                })
            }
            _ => {
                self.type_check_stmt_with_return(statement, expected_return)?;
                Ok(GuardElseValidation {
                    handled_bound_error: true,
                    terminal_propagate_seen: false,
                    handling_statement_count: 1,
                    referenced_active_error_binding: self
                        .stmt_references_active_guard_error_binding(statement),
                })
            }
        }
    }

    fn type_check_guard_error_propagate_terminal(
        &mut self,
        error_binding: &str,
        span: crate::token::Span,
        allow_terminal_propagate: bool,
    ) -> Result<GuardElseValidation, TypeError> {
        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason:
                    "propagate err is only valid as the final statement of a guard error clause"
                        .to_owned(),
                span: TypeError::span_from_span(span),
            });
        };

        if error_binding != active_error_binding || !allow_terminal_propagate {
            return Err(TypeError::ConstraintSolvingFailed {
                reason:
                    "propagate err is only valid as the final statement of a guard error clause"
                        .to_owned(),
                span: TypeError::span_from_span(span),
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

        Ok(GuardElseValidation {
            handled_bound_error: true,
            terminal_propagate_seen: true,
            handling_statement_count: 0,
            referenced_active_error_binding: true,
        })
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
            Expr::Identifier { name, .. } if name == active_error_binding
        )
    }

    fn stmt_references_active_guard_error_binding(&self, statement: &Stmt) -> bool {
        let Some(active_error_binding) = self.context.active_guard_error_bindings.last() else {
            return false;
        };

        self.stmt_references_identifier(statement, active_error_binding)
    }

    fn expression_counts_as_guard_error_handling(&self, expr: &Expr) -> bool {
        !Self::expression_is_guard_error_noop(expr)
    }

    fn expression_is_guard_error_noop(expr: &Expr) -> bool {
        match expr {
            Expr::Literal {
                value: LiteralValue::Void,
                ..
            } => true,
            Expr::Parenthesized { expr: inner, .. } => Self::expression_is_guard_error_noop(inner),
            _ => false,
        }
    }

    fn expr_references_identifier(&self, expr: &Expr, identifier: &str) -> bool {
        match expr {
            Expr::Literal { .. } => false,
            Expr::Identifier { name, .. } => name == identifier,
            Expr::Binary { left, right, .. } => {
                self.expr_references_identifier(left, identifier)
                    || self.expr_references_identifier(right, identifier)
            }
            Expr::Unary { operand, .. }
            | Expr::Parenthesized { expr: operand, .. }
            | Expr::TypeOf { expr: operand, .. }
            | Expr::Propagate { call: operand, .. } => {
                self.expr_references_identifier(operand, identifier)
            }
            Expr::Call { callee, args, .. } => {
                self.expr_references_identifier(callee, identifier)
                    || args
                        .iter()
                        .any(|arg| self.expr_references_identifier(arg, identifier))
            }
            Expr::Constructor { callee, fields, .. } => {
                self.expr_references_identifier(callee, identifier)
                    || fields
                        .iter()
                        .any(|field| self.expr_references_identifier(&field.value, identifier))
            }
            Expr::Index { object, index, .. } => {
                self.expr_references_identifier(object, identifier)
                    || self.expr_references_identifier(index, identifier)
            }
            Expr::Member { object, .. } => self.expr_references_identifier(object, identifier),
            Expr::Cast { expr, .. } => self.expr_references_identifier(expr, identifier),
            Expr::StringInterpolation { parts, .. } => parts.iter().any(|part| match part {
                crate::ast::StringPart::Literal(_) => false,
                crate::ast::StringPart::Expression(expr) => {
                    self.expr_references_identifier(expr, identifier)
                }
            }),
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.expr_references_identifier(condition, identifier)
                    || self.stmt_references_identifier(then_branch, identifier)
                    || else_branch
                        .as_ref()
                        .is_some_and(|branch| self.stmt_references_identifier(branch, identifier))
            }
            Expr::Array { elements, .. } => elements
                .iter()
                .any(|element| self.expr_references_identifier(element, identifier)),
            Expr::Match {
                scrutinee, arms, ..
            } => {
                self.expr_references_identifier(scrutinee, identifier)
                    || arms.iter().any(|arm| {
                        arm.guard
                            .as_ref()
                            .is_some_and(|guard| self.expr_references_identifier(guard, identifier))
                            || self.expr_references_identifier(&arm.body, identifier)
                    })
            }
            Expr::Loop { body, .. } => self.stmt_references_identifier(body, identifier),
            Expr::Lambda {
                body: crate::ast::LambdaBody::Expression(expr),
                ..
            } => self.expr_references_identifier(expr, identifier),
            Expr::Lambda {
                body: crate::ast::LambdaBody::Block(body),
                ..
            } => body
                .iter()
                .any(|statement| self.stmt_references_identifier(statement, identifier)),
            Expr::Guard {
                expr, else_branch, ..
            } => {
                self.expr_references_identifier(expr, identifier)
                    || self.stmt_references_identifier(else_branch, identifier)
            }
        }
    }

    fn stmt_references_identifier(&self, statement: &Stmt, identifier: &str) -> bool {
        match statement {
            Stmt::Let { initializer, .. } => initializer
                .as_ref()
                .is_some_and(|expr| self.expr_references_identifier(expr, identifier)),
            Stmt::LetDestructure { initializer, .. } => {
                self.expr_references_identifier(initializer, identifier)
            }
            Stmt::Assignment { target, value, .. } => {
                self.expr_references_identifier(target, identifier)
                    || self.expr_references_identifier(value, identifier)
            }
            Stmt::Return { values, .. }
            | Stmt::Break { values, .. }
            | Stmt::Continue { values, .. } => values
                .iter()
                .any(|value| self.expr_references_identifier(&value.value, identifier)),
            Stmt::Expression { expr, .. } => self.expr_references_identifier(expr, identifier),
            Stmt::Block { statements, .. } => statements
                .iter()
                .any(|statement| self.stmt_references_identifier(statement, identifier)),
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.expr_references_identifier(condition, identifier)
                    || self.stmt_references_identifier(then_branch, identifier)
                    || else_branch
                        .as_ref()
                        .is_some_and(|branch| self.stmt_references_identifier(branch, identifier))
            }
            Stmt::For { iterable, body, .. } => {
                self.expr_references_identifier(iterable, identifier)
                    || self.stmt_references_identifier(body, identifier)
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.expr_references_identifier(condition, identifier)
                    || self.stmt_references_identifier(body, identifier)
            }
            Stmt::Guard {
                expression,
                else_body,
                ..
            } => {
                self.expr_references_identifier(expression, identifier)
                    || self.stmt_references_identifier(else_body, identifier)
            }
            Stmt::PropagateGuardError { error_binding, .. } => error_binding == identifier,
            Stmt::Loop { body, .. } => self.stmt_references_identifier(body, identifier),
            Stmt::Comment { .. } => false,
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
