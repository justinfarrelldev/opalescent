//! Top-level expression codegen dispatcher.

use super::{
    BasicValueEnum, CodegenContext, CodegenEnv, CodegenError, CoreType, Expr, String,
    codegen_array_access, codegen_array_literal, codegen_binary, codegen_call_expression,
    codegen_cast, codegen_constructor_expression, codegen_field_access_expression,
    codegen_guard_expression, codegen_identifier, codegen_if_expression, codegen_literal,
    codegen_match_expression, codegen_propagate_expression, codegen_string_access,
    codegen_string_interpolation, codegen_unary, format, infer_expression_core_type,
};

pub fn codegen_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match *expr {
        Expr::Literal { ref value, .. } => {
            codegen_literal(codegen_context, env, value, expected_type)
        }
        Expr::Identifier { ref name, .. } => {
            codegen_identifier(codegen_context, env, name, expected_type)
        }
        Expr::Parenthesized { ref expr, .. } => {
            codegen_expression(codegen_context, env, expr, expected_type)
        }
        Expr::BorrowArgument { ref target, .. } => {
            codegen_expression(codegen_context, env, target.as_ref(), expected_type)
        }
        Expr::Binary {
            ref left,
            ref operator,
            ref right,
            ..
        } => codegen_binary(codegen_context, env, left, operator, right, expected_type),
        Expr::Unary {
            ref operator,
            ref operand,
            ..
        } => codegen_unary(codegen_context, env, operator, operand, expected_type),
        Expr::Cast {
            ref expr,
            ref target_type,
            ..
        } => codegen_cast(codegen_context, env, expr, target_type),
        Expr::Constrain { .. } | Expr::Refinement { .. } => Err(CodegenError::new(String::from(
            "affine expression semantics are not implemented yet",
        ))),
        Expr::Array { ref elements, .. } => {
            codegen_array_literal(codegen_context, env, elements.as_slice(), expected_type)
        }
        Expr::Index {
            ref object,
            ref index,
            span,
            ..
        } => match infer_expression_core_type(env, object.as_ref()) {
            Some(CoreType::String) => {
                codegen_string_access(codegen_context, env, object, index, span, expected_type)
            }
            Some(CoreType::Array(_)) => {
                codegen_array_access(codegen_context, env, object, index, expected_type)
            }
            Some(other) => Err(CodegenError::new(format!(
                "index access expects array or string receiver, found '{other}'"
            ))),
            None => Err(CodegenError::new(String::from(
                "index access receiver type could not be inferred",
            ))),
        },
        Expr::Call {
            ref callee,
            ref generic_args,
            ref args,
            ..
        } => codegen_call_expression(
            codegen_context,
            env,
            callee.as_ref(),
            generic_args.as_deref(),
            args.as_slice(),
            expected_type,
        ),
        Expr::Constructor { .. } => {
            codegen_constructor_expression(codegen_context, env, expr, expected_type)
        }
        Expr::Match { .. } => codegen_match_expression(codegen_context, env, expr),
        Expr::Loop { .. } => Err(CodegenError::new(String::from(
            "loop expressions are lowered in statement context",
        ))),
        Expr::Member { .. } => codegen_field_access_expression(codegen_context, env, expr),
        Expr::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => codegen_if_expression(
            codegen_context,
            env,
            condition.as_ref(),
            then_branch.as_ref(),
            else_branch.as_deref(),
        ),
        Expr::Guard {
            ref expr,
            ref binding_name,
            ref else_branch,
            ..
        } => codegen_guard_expression(
            codegen_context,
            env,
            expr.as_ref(),
            binding_name.as_str(),
            else_branch.as_ref(),
            expected_type,
        ),
        Expr::Propagate {
            ref call,
            ref cause,
            ..
        } => codegen_propagate_expression(
            codegen_context,
            env,
            call.as_ref(),
            cause.as_deref(),
            expected_type,
        ),
        Expr::StringInterpolation { ref parts, .. } => {
            codegen_string_interpolation(codegen_context, env, parts.as_slice())
        }
        _ => Err(CodegenError::new(String::from(
            "unsupported expression kind",
        ))),
    }
}
