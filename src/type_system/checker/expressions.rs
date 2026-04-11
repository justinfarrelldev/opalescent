//! Expression type checking for the Opalescent type system
extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardUsage};
use super::helpers::{
    binary_operation_name, coerce_literal_to_expected, constant_integer_overflow_warning,
    ensure_boolean_type, ensure_integer_type, ensure_numeric_type, ensure_same_type,
    invalid_operation_error, is_boolean_type, is_integer_type, is_numeric_type, is_string_type,
    literal_to_core_type, type_mismatch_error, unary_operation_name,
    validate_constant_shift_bounds, zero_divisor_operation_name,
};
use crate::ast::{AstNode, BinaryOp, Expr, LambdaBody, Parameter, Stmt, StringPart, Type, UnaryOp};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{boxed::Box, format, string::String, vec::Vec};

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
    /// Type check an expression and return its [`CoreType`]
    ///
    /// # Errors
    /// Returns `TypeError` variants when expression typing fails.
    #[expect(
        clippy::too_many_lines,
        reason = "Central expression dispatcher intentionally enumerates all expression variants"
    )]
    pub fn type_check_expr(&mut self, expr: &Expr) -> Result<CoreType, TypeError> {
        match *expr {
            Expr::Literal { ref value, .. } => Ok(literal_to_core_type(value)),
            Expr::Identifier { ref name, span, .. } => self.resolve_identifier(name, span),
            Expr::Parenthesized { ref expr, .. } => self.type_check_expr(expr),
            Expr::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                span,
                ..
            } => {
                self.type_check_if_expr(condition, then_branch, else_branch.as_deref(), span, None)
            }
            Expr::Binary {
                ref left,
                ref operator,
                ref right,
                span,
                ..
            } => self.type_check_binary_expr(left.as_ref(), operator, right.as_ref(), span),
            Expr::Unary {
                ref operator,
                ref operand,
                span,
                ..
            } => self.type_check_unary_expr(operator, operand.as_ref(), span),
            Expr::Call {
                ref callee,
                ref generic_args,
                ref args,
                span,
                ..
            } => self.type_check_call_expr(
                callee.as_ref(),
                generic_args.as_deref(),
                args.as_slice(),
                span,
            ),
            Expr::Index {
                ref object,
                ref index,
                span,
                ..
            } => self.type_check_index_expr(object.as_ref(), index.as_ref(), span),
            Expr::Member {
                ref object,
                ref member,
                span,
                ..
            } => {
                let object_type = self.type_check_expr(object.as_ref())?;
                if let Expr::Identifier {
                    ref name,
                    span: object_span,
                    ..
                } = **object
                {
                    let qualified_member = format!("{name}.{member}");
                    if let Some(symbol) = self.symbol_table().lookup(&qualified_member) {
                        return Ok(symbol.core_type.clone());
                    }

                    let nominal_member = format!("{object_type}.{member}");
                    if let Some(symbol) = self.symbol_table().lookup(&nominal_member) {
                        return Ok(symbol.core_type.clone());
                    }

                    return Err(TypeError::SymbolNotFound {
                        name: qualified_member,
                        span: TypeError::span_from_span(object_span),
                    });
                }

                let nominal_member = format!("{object_type}.{member}");
                if let Some(symbol) = self.symbol_table().lookup(&nominal_member) {
                    return Ok(symbol.core_type.clone());
                }

                Err(TypeError::SymbolNotFound {
                    name: nominal_member,
                    span: TypeError::span_from_span(span),
                })
            }
            Expr::Cast {
                ref expr,
                ref target_type,
                span,
                ..
            } => self.type_check_cast_expr(expr.as_ref(), target_type, span),
            Expr::TypeOf { ref expr, .. } => {
                self.type_check_expr(expr.as_ref())?;
                Ok(CoreType::String)
            }
            Expr::StringInterpolation {
                ref parts, span, ..
            } => {
                self.type_check_string_interpolation(parts.as_slice(), span)?;
                Ok(CoreType::String)
            }
            Expr::Array {
                ref elements, span, ..
            } => self.type_check_array_expr(elements.as_slice(), span),
            Expr::Lambda {
                ref generic_params,
                ref generic_constraints,
                ref params,
                ref return_types,
                ref error_types,
                ref body,
                span,
                ..
            } => self.type_check_lambda_expr(
                params.as_slice(),
                return_types.as_slice(),
                error_types.as_slice(),
                body,
                span,
                generic_params.as_deref(),
                generic_constraints.as_deref(),
            ),
            Expr::Guard {
                ref expr,
                ref binding_name,
                ref binding_type,
                is_mutable,
                ref else_branch,
                span,
                ..
            } => {
                let binding_info = GuardBindingInfo {
                    name: binding_name,
                    annotation: binding_type.as_ref(),
                    is_mutable,
                    span,
                };
                self.type_check_guard_expr(
                    expr,
                    &binding_info,
                    else_branch,
                    GuardUsage::Expression,
                    None,
                )
            }
            Expr::Propagate { ref call, span, .. } => {
                self.type_check_propagate_expr(call.as_ref(), span)
            }
        }
    }

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

        self.type_check_call_expr(callee_expr, None, args, call_span)?;

        if let Some(annotated_type_ast) = binding.annotation {
            let annotated_type = Self::ast_type_to_core_type(annotated_type_ast)?;
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
                        if matches!(expr, &Expr::Propagate { .. }) {
                            // Propagate implicitly transfers control by bubbling the error upward.
                            Ok(GuardElseOutcome::ControlFlow)
                        } else if self.types_compatible(&CoreType::Unit, &handler_type) {
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
                // Future phases may introduce additional guard handler forms.
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
    fn guard_error_type_sets_match(left: &[CoreType], right: &[CoreType]) -> bool {
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
    fn format_error_type_list(error_types: &[CoreType]) -> String {
        if error_types.is_empty() {
            return "<none>".to_owned();
        }

        let mut rendered = Vec::with_capacity(error_types.len());
        for error_type in error_types {
            rendered.push(error_type.to_string());
        }

        rendered.join(", ")
    }

    /// Type-check a `propagate` expression.
    ///
    /// This function ensures that:
    /// 1. The `propagate` expression is used inside a function that declares error types.
    /// 2. The inner expression is a function call.
    /// 3. The error types produced by the inner call are a subset of the error types
    ///    declared by the enclosing function.
    ///
    /// # Errors
    ///
    /// - `PropagateOutsideErrorFunction`: If used outside a function declaring errors.
    /// - `PropagateErrorMismatch`: If the propagated errors are not a subset of the
    ///   enclosing function's declared errors.
    fn type_check_propagate_expr(
        &mut self,
        call: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        // 1. Ensure we are inside a function that can handle errors.
        // Treat both "no current function" and "current function with zero declared errors"
        // as outside-of-error-function contexts, since `propagate` would be meaningless.
        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(), // Clone to release the borrow.
        };

        // 2. Ensure the inner expression is a function call and fetch its function type.
        if let Expr::Call {
            ref callee,
            ref args,
            ..
        } = *call
        {
            let callee_type = self.type_check_expr(callee)?;
            if let CoreType::Function {
                parameters: _parameters,
                return_types,
                error_types: callee_error_types,
                ..
            } = callee_type
            {
                if callee_error_types.is_empty() {
                    return Err(TypeError::PropagateOnNonErrorExpression {
                        span: TypeError::span_from_span(span),
                    });
                }

                // Validate the call arguments against the parameters (reuse call typing logic)
                // We intentionally call the existing checker to enforce argument checks
                self.type_check_call_expr(callee, None, args.as_slice(), call.span())?;

                if let Some(active_errors) = self.guard_error_stack.last() {
                    if !Self::guard_error_type_sets_match(
                        active_errors.as_slice(),
                        &callee_error_types,
                    ) {
                        return Err(TypeError::GuardChainedErrorMismatch {
                            expected: Self::format_error_type_list(active_errors.as_slice()),
                            found: Self::format_error_type_list(&callee_error_types),
                            span: TypeError::span_from_span(span),
                        });
                    }
                }

                // 3. Check subset relation for error types declared by the enclosing function.
                let is_subset = callee_error_types
                    .iter()
                    .all(|error_type| current_fn_error_types.contains(error_type));

                if !is_subset {
                    return Err(TypeError::PropagateErrorMismatch {
                        expected: Self::format_error_type_list(&current_fn_error_types),
                        found: Self::format_error_type_list(&callee_error_types),
                        span: TypeError::span_from_span(
                            self.symbol_table.current_function_span().unwrap_or(span),
                        ),
                        callee_span: TypeError::span_from_span(call.span()),
                    });
                }

                // Propagate expression yields the success type of the inner call
                if return_types.len() != 1 {
                    return Err(TypeError::ArityMismatch {
                        expected: 1,
                        found: return_types.len(),
                        span: TypeError::span_from_span(span),
                    });
                }
                return_types
                    .first()
                    .cloned()
                    .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                        reason: "propagate callee has no declared return type".to_owned(),
                        span: TypeError::span_from_span(span),
                    })
            } else {
                Err(TypeError::PropagateOnNonErrorExpression {
                    span: TypeError::span_from_span(span),
                })
            }
        } else {
            // Parser should ensure this path is unreachable; defensively handle anyway.
            Err(TypeError::PropagateOnNonErrorExpression {
                span: TypeError::span_from_span(span),
            })
        }
    }

    /// Resolve an identifier to its registered core type or emit a symbol error.
    fn resolve_identifier(&mut self, name: &str, span: Span) -> Result<CoreType, TypeError> {
        if let Some(info) = self.symbol_table_mut().lookup_mut(name) {
            info.read_count = info.read_count.saturating_add(1);
            return Ok(info.core_type.clone());
        }

        Err(TypeError::SymbolNotFound {
            name: name.to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Type check a binary expression, enforcing operand compatibility, recording inference
    /// constraints, and returning the resulting core type for subsequent analysis.
    #[expect(
        clippy::too_many_lines,
        reason = "Binary expression typing keeps all operator cases in one place"
    )]
    pub(super) fn type_check_binary_expr(
        &mut self,
        left: &Expr,
        operator: &BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if matches!(*operator, BinaryOp::Is | BinaryOp::IsNot)
            && matches!(left, &Expr::Identifier { .. })
            && matches!(right, &Expr::Identifier { .. })
        {
            return Ok(CoreType::Boolean);
        }

        let left_type = self.type_check_expr(left)?;
        let right_type = self.type_check_expr(right)?;
        let op_name = binary_operation_name(operator);

        match *operator {
            BinaryOp::Add
            | BinaryOp::Subtract
            | BinaryOp::Multiply
            | BinaryOp::Divide
            | BinaryOp::Power => {
                if is_string_type(&left_type) && is_string_type(&right_type) {
                    return Ok(CoreType::String);
                }
                ensure_numeric_type(&left_type, left.span(), op_name)?;
                ensure_numeric_type(&right_type, right.span(), op_name)?;
                Self::ensure_non_zero_divisor(operator, right)?;
                ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                if matches!(
                    *operator,
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply
                ) && is_integer_type(&result_type)
                {
                    if let Some(warning) =
                        constant_integer_overflow_warning(operator, left, right, &result_type, span)
                    {
                        self.push_warning(warning);
                    }
                }
                self.add_constraint(TypeConstraint::equality(
                    left_type,
                    right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(result_type)
            }
            BinaryOp::Modulo => {
                ensure_integer_type(&left_type, left.span(), op_name)?;
                ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_non_zero_divisor(operator, right)?;
                ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::equality(
                    left_type,
                    right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(result_type)
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Is | BinaryOp::IsNot => {
                if !self.types_compatible(&left_type, &right_type) {
                    return Err(type_mismatch_error(
                        &left_type,
                        Some(left.span()),
                        &right_type,
                        right.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::equality(
                    left_type,
                    right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(CoreType::Boolean)
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                ensure_numeric_type(&left_type, left.span(), op_name)?;
                ensure_numeric_type(&right_type, right.span(), op_name)?;
                ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
                ensure_boolean_type(&left_type, left.span(), op_name)?;
                ensure_boolean_type(&right_type, right.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
                ensure_integer_type(&left_type, left.span(), op_name)?;
                ensure_integer_type(&right_type, right.span(), op_name)?;
                ensure_same_type(&left_type, left.span(), &right_type, right.span())?;
                let result_type = left_type.clone();
                self.add_constraint(TypeConstraint::equality(
                    left_type,
                    right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(result_type)
            }
            BinaryOp::BitShiftLeft | BinaryOp::BitShiftRight | BinaryOp::BitUnsignedShiftRight => {
                ensure_integer_type(&left_type, left.span(), op_name)?;
                ensure_integer_type(&right_type, right.span(), op_name)?;
                validate_constant_shift_bounds(&left_type, right, span)?;
                Ok(left_type)
            }
            BinaryOp::Assign => Err(invalid_operation_error(op_name, &left_type, span)),
        }
    }

    /// Reject compile-time constant zero divisors for division-like operators.
    fn ensure_non_zero_divisor(operator: &BinaryOp, right: &Expr) -> Result<(), TypeError> {
        let Some(operation) = zero_divisor_operation_name(operator, right) else {
            return Ok(());
        };

        Err(TypeError::DivisionByZero {
            operation: operation.to_owned(),
            span: TypeError::span_from_span(right.span()),
        })
    }

    /// Type check a unary expression, returning the deduced result type while enforcing the
    /// operator's domain constraints.
    fn type_check_unary_expr(
        &mut self,
        operator: &UnaryOp,
        operand: &Expr,
        _span: Span,
    ) -> Result<CoreType, TypeError> {
        let operand_type = self.type_check_expr(operand)?;
        let op_name = unary_operation_name(operator);
        match *operator {
            UnaryOp::Negate | UnaryOp::Plus => {
                ensure_numeric_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
            UnaryOp::Not => {
                ensure_boolean_type(&operand_type, operand.span(), op_name)?;
                Ok(CoreType::Boolean)
            }
            UnaryOp::BitNot => {
                ensure_integer_type(&operand_type, operand.span(), op_name)?;
                Ok(operand_type)
            }
        }
    }

    /// Validate a function call, ensuring arity matches, arguments conform to parameter types,
    /// and recording equality constraints for the inference engine.
    fn type_check_call_expr(
        &mut self,
        callee: &Expr,
        generic_args: Option<&[Type]>,
        args: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        self.type_check_call_expr_impl(callee, generic_args, args, span)
    }

    /// Type check an array indexing operation, confirming integer indices and yielding the
    /// element type for subsequent evaluation.
    fn type_check_index_expr(
        &mut self,
        object: &Expr,
        index: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let object_type = self.type_check_expr(object)?;
        let index_type = self.type_check_expr(index)?;
        ensure_integer_type(&index_type, index.span(), "indexing")?;
        match object_type {
            CoreType::Array(element_type) => Ok(*element_type),
            other => Err(invalid_operation_error("indexing", &other, span)),
        }
    }

    /// Type check an explicit cast expression, validating safety and compatibility.
    ///
    /// This method validates the cast using the type system's cast validation rules,
    /// which distinguish between:
    /// - Safe casts (widening conversions) - always allowed
    /// - Unsafe casts (narrowing conversions) - allowed with warning, runtime trap in debug
    /// - Invalid casts (non-numeric types) - compilation error
    ///
    /// See [`TypeChecker::validate_cast`] for detailed cast safety rules.
    fn type_check_cast_expr(
        &mut self,
        expr: &Expr,
        target_type: &Type,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let source_type = self.type_check_expr(expr)?;
        let target_core_type = Self::ast_type_to_core_type(target_type)?;

        // Validate the cast according to language spec (math.md)
        // Invalid casts remain errors; unsafe casts are collected as warnings.
        self.validate_cast_with_warnings(&source_type, &target_core_type, span)?;

        Ok(target_core_type)
    }

    /// Validate each interpolated expression, ensuring only display-safe primitives appear
    /// inside a string literal interpolation sequence.
    fn type_check_string_interpolation(
        &mut self,
        parts: &[StringPart],
        _span: Span,
    ) -> Result<(), TypeError> {
        for part in parts {
            if let StringPart::Expression(ref expr) = *part {
                let expr_type = self.type_check_expr(expr)?;
                if !(is_numeric_type(&expr_type)
                    || is_boolean_type(&expr_type)
                    || is_string_type(&expr_type))
                {
                    return Err(invalid_operation_error(
                        "string interpolation",
                        &expr_type,
                        expr.span(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Type check an array literal, deriving a unified element type and generating equality
    /// constraints between each element and the inferred element type.
    fn type_check_array_expr(
        &mut self,
        elements: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let mut element_type: Option<(CoreType, Span)> = None;
        for element in elements {
            let element_span = element.span();
            let element_core_type = self.type_check_expr(element)?;
            if let Some(existing) = element_type.as_ref() {
                let existing_type = &existing.0;
                let existing_span = existing.1;
                if !self.types_compatible(existing_type, &element_core_type) {
                    return Err(type_mismatch_error(
                        existing_type,
                        Some(existing_span),
                        &element_core_type,
                        element_span,
                    ));
                }
                self.add_constraint(TypeConstraint::equality(
                    existing_type.clone(),
                    element_core_type,
                    Some(existing_span),
                    Some(element_span),
                ));
            } else {
                element_type = Some((element_core_type, element_span));
            }
        }

        let resolved = match element_type {
            Some((core_type, _)) => core_type,
            None => self.fresh_type_var_auto(span)?,
        };

        Ok(CoreType::Array(Box::new(resolved)))
    }

    /// Type check a lambda expression by establishing a scoped environment for its parameters and body.
    #[expect(
        clippy::too_many_arguments,
        reason = "Lambda typing requires signature, body, span, and optional generic metadata"
    )]
    pub(super) fn type_check_lambda_expr(
        &mut self,
        parameters: &[Parameter],
        return_types: &[Type],
        error_types: &[alloc::string::String],
        body: &LambdaBody,
        span: Span,
        generic_params: Option<&[alloc::string::String]>,
        generic_constraints: Option<&[crate::ast::TypeParameter]>,
    ) -> Result<CoreType, TypeError> {
        if let Some(params) = generic_params {
            if !params.is_empty() {
                return Err(TypeError::NotImplementedYet {
                    feature: "generic lambda type checking".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
        }
        if let Some(constraints) = generic_constraints {
            if !constraints.is_empty() {
                return Err(TypeError::NotImplementedYet {
                    feature: "generic lambda constraints type checking".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core_types: Vec<CoreType> = return_types
            .iter()
            .map(Self::ast_type_to_core_type)
            .collect::<Result<Vec<_>, _>>()?;

        let core_errors = self.resolve_error_types(error_types, span)?;
        self.symbol_table.enter_function(core_errors.clone(), span);
        self.begin_return_context();

        let body_result = self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                    is_let_binding: false,
                    is_mutable: false,
                    read_count: 0,
                });
            }

            match *body {
                LambdaBody::Expression(ref expr) => {
                    if return_core_types.len() != 1 {
                        return Err(TypeError::ArityMismatch {
                            expected: return_core_types.len(),
                            found: 1,
                            span: TypeError::span_from_span(expr.span()),
                        });
                    }
                    let expr_type = checker.type_check_expr(expr)?;
                    if !checker.types_compatible(&return_core_types[0], &expr_type) {
                        return Err(type_mismatch_error(
                            &return_core_types[0],
                            return_types.first().map(Type::span),
                            &expr_type,
                            expr.span(),
                        ));
                    }
                    checker.add_constraint(TypeConstraint::equality(
                        return_core_types[0].clone(),
                        expr_type,
                        return_types.first().map(Type::span),
                        Some(expr.span()),
                    ));
                    Ok(())
                }
                LambdaBody::Block(ref statements) => {
                    checker.type_check_statements(statements, Some(return_core_types.as_slice()))
                }
            }
        });

        self.end_return_context();
        self.symbol_table.exit_function();
        body_result?;

        // Map lambda-declared error types into nominal core types
        Ok(CoreType::Function {
            generic_params: Vec::new(),
            parameters: parameter_types,
            return_types: return_core_types,
            error_types: core_errors,
        })
    }
}
