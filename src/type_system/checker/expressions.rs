#![allow(
    clippy::pattern_type_mismatch,
    reason = "expression matcher patterns intentionally work on borrowed AST nodes"
)]
//! Expression type checking for the Opalescent type system
extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardCheckRequest, GuardUsage};
use super::helpers::{
    binary_operation_name, coerce_literal_to_expected, constant_integer_overflow_warning,
    ensure_boolean_type, ensure_integer_type, ensure_numeric_type, ensure_same_type,
    invalid_operation_error, is_integer_type, is_string_type, literal_to_core_type,
    type_mismatch_error, unary_operation_name, validate_constant_shift_bounds,
    zero_divisor_operation_name,
};
use crate::ast::{AstNode, BinaryOp, Expr, LambdaBody, Parameter, Type, TypeParameter, UnaryOp};
use crate::errors::suggestions::{SUGGESTION_DISTANCE_THRESHOLD, closest_identifier_suggestion};
use crate::token::Span;
use crate::type_system::arithmetic::{
    ArithmeticMode, fold_integer_binary_expr, mode_for_binary_operator, mode_for_intrinsic_member,
};
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::substitution::Substitution;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::{CoreType, GenericTypeParameter};
use alloc::{format, string::String, vec::Vec};

impl TypeChecker {
    /// Resolve a generic ADT field type against concrete owner type arguments.
    fn resolve_generic_adt_field_type(
        &self,
        owner_name: &str,
        owner_type_args: &[CoreType],
        field_type: &CoreType,
    ) -> CoreType {
        let Some(generic_params) = self.adt_generic_params_for(owner_name) else {
            return field_type.clone();
        };
        if generic_params.len() != owner_type_args.len() {
            return field_type.clone();
        }

        let mut substitution = Substitution::empty();
        for (generic_param, type_arg) in generic_params.iter().zip(owner_type_args.iter()) {
            substitution = Substitution::single(generic_param.type_var.id, type_arg.clone())
                .compose(&substitution);
        }
        substitution.apply(field_type)
    }

    /// Resolve lambda signature/core AST types with in-scope lambda generic bindings.
    fn ast_type_to_core_type_with_lambda_generics(
        ast_type: &Type,
        generic_bindings: &[(alloc::string::String, CoreType)],
    ) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, .. } => {
                if let Some(core_type) = generic_bindings
                    .iter()
                    .find_map(|binding| (&binding.0 == name).then_some(&binding.1))
                {
                    return Ok(core_type.clone());
                }
                match ast_type_to_core_type(ast_type).map_err(TypeError::from) {
                    Ok(core_type) => Ok(core_type),
                    Err(TypeError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
                        name: type_name,
                        type_args: Vec::new(),
                    }),
                    Err(other) => Err(other),
                }
            }
            Type::Array {
                ref element_type, ..
            } => Ok(CoreType::Array(alloc::boxed::Box::new(
                Self::ast_type_to_core_type_with_lambda_generics(element_type, generic_bindings)?,
            ))),
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let mut resolved_args = Vec::new();
                for type_arg in type_args {
                    resolved_args.push(Self::ast_type_to_core_type_with_lambda_generics(
                        type_arg,
                        generic_bindings,
                    )?);
                }
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: resolved_args,
                })
            }
            Type::Function {
                ref parameters,
                ref return_types,
                ref errors,
                ..
            } => {
                let mut resolved_params = Vec::new();
                for parameter in parameters {
                    resolved_params.push(Self::ast_type_to_core_type_with_lambda_generics(
                        parameter,
                        generic_bindings,
                    )?);
                }
                let mut resolved_returns = Vec::new();
                for return_type in return_types {
                    resolved_returns.push(Self::ast_type_to_core_type_with_lambda_generics(
                        return_type,
                        generic_bindings,
                    )?);
                }
                let mut resolved_errors = Vec::new();
                if let Some(ref error_types) = *errors {
                    for error_type in error_types {
                        resolved_errors.push(Self::ast_type_to_core_type_with_lambda_generics(
                            error_type,
                            generic_bindings,
                        )?);
                    }
                }
                Ok(CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: resolved_params,
                    return_types: resolved_returns,
                    error_types: resolved_errors,
                })
            }
        }
    }

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
                id,
            } => self.type_check_binary_expr(left.as_ref(), operator, right.as_ref(), span, id.0),
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
                id,
            } => self.type_check_call_expr(
                callee.as_ref(),
                generic_args.as_deref(),
                args.as_slice(),
                span,
                id.0,
            ),
            Expr::Constructor {
                ref callee,
                ref fields,
                span,
                ..
            } => self.type_check_constructor_expr(callee.as_ref(), fields.as_slice(), span),
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
                if let Some(collection_member_type) =
                    self.resolve_collection_member_call(&object_type, member)
                {
                    return Ok(collection_member_type);
                }
                if let CoreType::Generic {
                    ref name,
                    ref type_args,
                } = object_type
                {
                    if let Some(field_type) = self.adt_field_type(name, member) {
                        let resolved_field_type =
                            self.resolve_generic_adt_field_type(name, type_args, field_type);
                        self.add_constraint(TypeConstraint::HasField {
                            owner: CoreType::Generic {
                                name: name.clone(),
                                type_args: type_args.clone(),
                            },
                            field_name: member.clone(),
                            field_type: resolved_field_type.clone(),
                            owner_span: Some(object.span()),
                            field_span: Some(span),
                        });
                        return Ok(resolved_field_type);
                    }
                }
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
                        suggestion: self.suggest_visible_identifier(name.as_str()),
                        span: TypeError::span_from_span(object_span),
                    });
                }

                let nominal_member = format!("{object_type}.{member}");
                if let Some(symbol) = self.symbol_table().lookup(&nominal_member) {
                    return Ok(symbol.core_type.clone());
                }

                Err(TypeError::SymbolNotFound {
                    name: nominal_member,
                    suggestion: self.suggest_visible_identifier(member.as_str()),
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
            Expr::Match {
                ref scrutinee,
                ref arms,
                span,
                ..
            } => self.type_check_match_expr(scrutinee, arms.as_slice(), span),
            Expr::Loop { ref body, span, .. } => {
                self.context.loop_break_type_stack.push(None);
                self.type_check_stmt_with_return(body.as_ref(), None)?;
                let break_types = self.context.loop_break_type_stack.pop();
                match break_types {
                    Some(Some(mut types)) if types.len() == 1 => Ok(types.remove(0)),
                    Some(Some(types)) if types.len() > 1 => Err(TypeError::ArityMismatch {
                        expected: 1,
                        found: types.len(),
                        span: TypeError::span_from_span(span),
                    }),
                    _ => Ok(CoreType::Unit),
                }
            }
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
                self.type_check_guard_expr(GuardCheckRequest {
                    expr,
                    binding: &binding_info,
                    error_binding: None,
                    else_branch,
                    usage: GuardUsage::Expression,
                    expected_return: None,
                })
            }
            Expr::Propagate { ref call, span, .. } => {
                self.type_check_propagate_expr(call.as_ref(), span)
            }
        }
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
                let previous_propagate_context = self.context.in_propagate_context;
                self.context.in_propagate_context = true;
                let call_result = self.type_check_call_expr(
                    callee,
                    None,
                    args.as_slice(),
                    call.span(),
                    call.node_id().0,
                );
                self.context.in_propagate_context = previous_propagate_context;
                call_result?;

                if let Some(active_errors) = self.context.guard_error_stack.last() {
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

        if self.context.guard_else_depth > 0 {
            if self
                .context
                .pending_guard_success_bindings
                .iter()
                .any(|hidden_name| hidden_name == name)
            {
                return Err(TypeError::ConstraintSolvingFailed {
                    reason: "success binding is not available inside guard error clause".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }

            if self
                .context
                .active_guard_error_bindings
                .last()
                .is_some_and(|active_error_binding| active_error_binding.name == name)
            {
                if let Some(active_errors) = self.context.guard_error_stack.last() {
                    if let [single_error_type] = active_errors.as_slice() {
                        return Ok(single_error_type.clone());
                    }
                    return Ok(CoreType::Generic {
                        name: "GuardErrorContext".to_owned(),
                        type_args: active_errors.clone(),
                    });
                }
            }
        }

        Err(TypeError::SymbolNotFound {
            name: name.to_owned(),
            suggestion: self.suggest_visible_identifier(name),
            span: TypeError::span_from_span(span),
        })
    }

    /// Suggest the closest visible symbol name for unresolved identifiers.
    fn suggest_visible_identifier(&self, unresolved_name: &str) -> Option<String> {
        let visible = self.symbol_table().visible_symbol_names();
        closest_identifier_suggestion(unresolved_name, visible.as_slice()).and_then(|ranked| {
            (ranked.distance <= SUGGESTION_DISTANCE_THRESHOLD).then_some(ranked.suggestion)
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
        expr_id: usize,
    ) -> Result<CoreType, TypeError> {
        if let Some(arithmetic_mode) = mode_for_binary_operator(operator) {
            self.record_arithmetic_mode(expr_id, arithmetic_mode);
        }

        if let Some(constant_value) = fold_integer_binary_expr(operator, left, right) {
            self.record_constant_integer_value(expr_id, constant_value);
        }

        if matches!(*operator, BinaryOp::Is | BinaryOp::IsNot)
            && matches!(left, &Expr::Identifier { .. })
            && matches!(right, &Expr::Identifier { .. })
        {
            return Ok(CoreType::Boolean);
        }

        let left_type = self.type_check_expr(left)?;
        let right_type = self.type_check_expr(right)?;
        let op_name = binary_operation_name(operator);

        let normalized_left_type = coerce_literal_to_expected(&right_type, left, &left_type)
            .unwrap_or_else(|| left_type.clone());
        let normalized_right_type = coerce_literal_to_expected(&left_type, right, &right_type)
            .unwrap_or_else(|| right_type.clone());

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
                ensure_same_type(
                    &normalized_left_type,
                    left.span(),
                    &normalized_right_type,
                    right.span(),
                )?;
                let result_type = normalized_left_type.clone();
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
                    normalized_left_type,
                    normalized_right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(result_type)
            }
            BinaryOp::Modulo | BinaryOp::DivEuclid | BinaryOp::ModEuclid => {
                ensure_integer_type(&left_type, left.span(), op_name)?;
                ensure_integer_type(&right_type, right.span(), op_name)?;
                Self::ensure_non_zero_divisor(operator, right)?;
                ensure_same_type(
                    &normalized_left_type,
                    left.span(),
                    &normalized_right_type,
                    right.span(),
                )?;
                let result_type = normalized_left_type.clone();
                self.add_constraint(TypeConstraint::equality(
                    normalized_left_type,
                    normalized_right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(result_type)
            }
            BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Is | BinaryOp::IsNot => {
                if matches!(*operator, BinaryOp::Is | BinaryOp::IsNot)
                    && matches!(*right, Expr::Identifier { .. })
                {
                    return Ok(CoreType::Boolean);
                }
                if !self.types_compatible(&normalized_left_type, &normalized_right_type) {
                    return Err(type_mismatch_error(
                        &normalized_left_type,
                        Some(left.span()),
                        &normalized_right_type,
                        right.span(),
                    ));
                }
                self.add_constraint(TypeConstraint::equality(
                    normalized_left_type,
                    normalized_right_type,
                    Some(left.span()),
                    Some(right.span()),
                ));
                Ok(CoreType::Boolean)
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                ensure_numeric_type(&left_type, left.span(), op_name)?;
                ensure_numeric_type(&right_type, right.span(), op_name)?;
                if !((is_integer_type(&left_type) && is_integer_type(&right_type))
                    || self.types_compatible(&normalized_left_type, &normalized_right_type))
                {
                    ensure_same_type(
                        &normalized_left_type,
                        left.span(),
                        &normalized_right_type,
                        right.span(),
                    )?;
                }
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
                validate_constant_shift_bounds(operator, &left_type, right, right.span())?;
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
    pub(super) fn type_check_call_expr(
        &mut self,
        callee: &Expr,
        generic_args: Option<&[Type]>,
        args: &[Expr],
        span: Span,
        expr_id: usize,
    ) -> Result<CoreType, TypeError> {
        if let Expr::Member { ref member, .. } = *callee {
            if let Some(arithmetic_mode) = mode_for_intrinsic_member(member) {
                self.record_arithmetic_mode(expr_id, arithmetic_mode);
                if arithmetic_mode != ArithmeticMode::Default {
                    self.clear_constant_integer_value(expr_id);
                }
            }
        }

        let result = self.type_check_call_expr_impl(callee, generic_args, args, span);
        if let Err(error) = result {
            if let Expr::Member { ref member, .. } = *callee {
                if member == "filter" && args.len() == 1 {
                    let predicate_type = self.type_check_expr(&args[0]);
                    if let Ok(CoreType::Function { return_types, .. }) = predicate_type {
                        if let Some(first_return_type) = return_types.first() {
                            if first_return_type != &CoreType::Boolean {
                                return Err(invalid_operation_error(
                                    "filter predicate must return boolean",
                                    first_return_type,
                                    args[0].span(),
                                ));
                            }
                        }
                    }
                }
            }
            return Err(error);
        }

        result
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
        let target_core_type = ast_type_to_core_type(target_type).map_err(TypeError::from)?;
        // Validate the cast according to language spec (math.md)
        // Invalid casts remain errors; unsafe casts are collected as warnings.
        self.validate_cast_with_warnings(&source_type, &target_core_type, span)?;
        Ok(target_core_type)
    }

    /// Type check a lambda expression by establishing a scoped environment for its parameters and body.
    #[expect(
        clippy::too_many_arguments,
        reason = "Lambda typing requires signature, body, span, and optional generic metadata"
    )]
    #[expect(
        clippy::too_many_lines,
        reason = "Lambda typing handles generic binding resolution and body validation in one path"
    )]
    pub(super) fn type_check_lambda_expr(
        &mut self,
        parameters: &[Parameter],
        return_types: &[Type],
        error_types: &[alloc::string::String],
        body: &LambdaBody,
        span: Span,
        generic_params: Option<&[alloc::string::String]>,
        generic_constraints: Option<&[TypeParameter]>,
    ) -> Result<CoreType, TypeError> {
        // NOTE: Lambdas do NOT push their own modifier context. They inherit
        // the enclosing function's modifiers from function_modifier_stack.
        // This means lambdas inside `pure` functions are implicitly pure.
        // This is intentional — a lambda created in a pure context should
        // not be able to perform impure operations.
        let mut lambda_generic_params: Vec<GenericTypeParameter> = Vec::new();
        let mut generic_bindings: Vec<(alloc::string::String, CoreType)> = Vec::new();
        if let Some(constraints) = generic_constraints {
            for declaration in constraints {
                let variable_core =
                    self.fresh_type_var(declaration.name.clone(), declaration.span)?;
                let CoreType::Variable(type_var) = variable_core else {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "failed to allocate generic type variable".to_owned(),
                        span: TypeError::span_from_span(declaration.span),
                    });
                };
                generic_bindings.push((
                    declaration.name.clone(),
                    CoreType::Variable(type_var.clone()),
                ));
                lambda_generic_params.push(GenericTypeParameter {
                    name: declaration.name.clone(),
                    type_var,
                    constraints: Vec::new(),
                });
            }

            for (index, declaration) in constraints.iter().enumerate() {
                let mut constraint_types = Vec::new();
                for constraint in &declaration.constraints {
                    constraint_types.push(Self::ast_type_to_core_type_with_lambda_generics(
                        constraint,
                        generic_bindings.as_slice(),
                    )?);
                }
                if let Some(generic_param) = lambda_generic_params.get_mut(index) {
                    generic_param.constraints = constraint_types;
                }
            }
        } else if let Some(params) = generic_params {
            for param_name in params {
                let variable_core = self.fresh_type_var(param_name.clone(), span)?;
                let CoreType::Variable(type_var) = variable_core else {
                    return Err(TypeError::ConstraintSolvingFailed {
                        reason: "failed to allocate generic type variable".to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                };
                generic_bindings.push((param_name.clone(), CoreType::Variable(type_var.clone())));
                lambda_generic_params.push(GenericTypeParameter {
                    name: param_name.clone(),
                    type_var,
                    constraints: Vec::new(),
                });
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type_with_lambda_generics(
                &param.param_type,
                generic_bindings.as_slice(),
            )?);
        }
        let return_core_types: Vec<CoreType> = return_types
            .iter()
            .map(|return_type| {
                Self::ast_type_to_core_type_with_lambda_generics(
                    return_type,
                    generic_bindings.as_slice(),
                )
            })
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
                    is_pure: false,
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
            generic_params: lambda_generic_params,
            parameters: parameter_types,
            return_types: return_core_types,
            error_types: core_errors,
        })
    }
}
