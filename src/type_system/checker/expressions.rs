//! Expression type checking for the Opalescent type system

extern crate alloc;

use super::helpers::{
    binary_operation_name, coerce_literal_to_expected, ensure_boolean_type, ensure_integer_type,
    ensure_numeric_type, ensure_same_type, invalid_operation_error, is_boolean_type,
    is_numeric_type, is_string_type, literal_to_core_type, type_mismatch_error,
    unary_operation_name,
};
use crate::ast::{AstNode, BinaryOp, Expr, LambdaBody, Parameter, StringPart, Type, UnaryOp};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{boxed::Box, vec::Vec};

impl TypeChecker {
    /// Type check an expression and return its [`CoreType`]
    ///
    /// # Errors
    /// Returns `TypeError` variants when expression typing fails.
    pub fn type_check_expr(&mut self, expr: &Expr) -> Result<CoreType, TypeError> {
        match *expr {
            Expr::Literal { ref value, .. } => Ok(literal_to_core_type(value)),
            Expr::Identifier { ref name, span, .. } => self.resolve_identifier(name, span),
            Expr::Parenthesized { ref expr, .. } => self.type_check_expr(expr),
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
                ref args,
                span,
                ..
            } => self.type_check_call_expr(callee.as_ref(), args.as_slice(), span),
            Expr::Index {
                ref object,
                ref index,
                span,
                ..
            } => self.type_check_index_expr(object.as_ref(), index.as_ref(), span),
            Expr::Member { span, .. } => Err(TypeError::NotImplementedYet {
                feature: "member access type checking".to_owned(),
                span: TypeError::span_from_span(span),
            }),
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
                ref params,
                ref return_type,
                ref body,
                span,
                ..
            } => self.type_check_lambda_expr(
                generic_params.as_deref(),
                params.as_slice(),
                return_type,
                body,
                span,
            ),
        }
    }

    /// Resolve an identifier to its registered core type or emit a symbol error.
    fn resolve_identifier(&self, name: &str, span: Span) -> Result<CoreType, TypeError> {
        self.symbol_table()
            .lookup(name)
            .map(|info| info.core_type.clone())
            .ok_or_else(|| TypeError::SymbolNotFound {
                name: name.to_owned(),
                span: TypeError::span_from_span(span),
            })
    }

    /// Type check a binary expression, enforcing operand compatibility, recording inference
    /// constraints, and returning the resulting core type for subsequent analysis.
    pub(super) fn type_check_binary_expr(
        &mut self,
        left: &Expr,
        operator: &BinaryOp,
        right: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
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
            BinaryOp::Modulo => {
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
                Ok(left_type)
            }
            BinaryOp::Assign => Err(invalid_operation_error(op_name, &left_type, span)),
        }
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
        args: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let callee_type = self.type_check_expr(callee)?;
        match callee_type {
            CoreType::Function {
                parameters,
                return_type,
            } => {
                if parameters.len() != args.len() {
                    return Err(TypeError::InvalidOperation {
                        operation: alloc::format!(
                            "function call expected {} arguments but received {}",
                            parameters.len(),
                            args.len()
                        ),
                        type_name: "function".to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                }

                for (index, arg_expr) in args.iter().enumerate() {
                    let param_type = parameters[index].clone();
                    let arg_type = self.type_check_expr(arg_expr)?;
                    let reconciled_type = if self.types_compatible(&param_type, &arg_type) {
                        arg_type
                    } else if let Some(adjusted) =
                        coerce_literal_to_expected(&param_type, arg_expr, &arg_type)
                    {
                        adjusted
                    } else {
                        return Err(type_mismatch_error(
                            &param_type,
                            None,
                            &arg_type,
                            arg_expr.span(),
                        ));
                    };
                    self.add_constraint(TypeConstraint::equality(
                        param_type.clone(),
                        reconciled_type,
                        None,
                        Some(arg_expr.span()),
                    ));
                }

                Ok(*return_type)
            }
            other => Err(invalid_operation_error("function call", &other, span)),
        }
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
        // This will error on invalid casts and warn on unsafe casts
        Self::validate_cast(&source_type, &target_core_type, span)?;

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
    pub(super) fn type_check_lambda_expr(
        &mut self,
        generic_params: Option<&[alloc::string::String]>,
        parameters: &[Parameter],
        return_type: &Type,
        body: &LambdaBody,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if let Some(params) = generic_params {
            if !params.is_empty() {
                return Err(TypeError::NotImplementedYet {
                    feature: "generic lambda type checking".to_owned(),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        let mut parameter_types = Vec::with_capacity(parameters.len());
        for param in parameters {
            parameter_types.push(Self::ast_type_to_core_type(&param.param_type)?);
        }

        let return_core = Self::ast_type_to_core_type(return_type)?;
        let return_span = return_type.span();

        self.within_new_scope(|checker| -> Result<(), TypeError> {
            for (param, core_type) in parameters.iter().zip(parameter_types.iter()) {
                checker.symbol_table.register(SymbolInfo {
                    name: param.name.clone(),
                    symbol_type: SymbolType::Variable,
                    core_type: core_type.clone(),
                    visibility: Visibility::Private,
                    source_location: param.span(),
                });
            }

            match *body {
                LambdaBody::Expression(ref expr) => {
                    let expr_type = checker.type_check_expr(expr)?;
                    if !checker.types_compatible(&return_core, &expr_type) {
                        return Err(type_mismatch_error(
                            &return_core,
                            Some(return_span),
                            &expr_type,
                            expr.span(),
                        ));
                    }
                    checker.add_constraint(TypeConstraint::equality(
                        return_core.clone(),
                        expr_type,
                        Some(return_span),
                        Some(expr.span()),
                    ));
                    Ok(())
                }
                LambdaBody::Block(ref statements) => {
                    checker.type_check_statements(statements, Some(&return_core))
                }
            }
        })?;

        Ok(CoreType::Function {
            parameters: parameter_types,
            return_type: Box::new(return_core),
        })
    }
}
