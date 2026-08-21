//! Error propagation helpers for call lowering.

use super::functions_call_helpers::current_function;
use super::using_cleanup::emit_cleanup_aware_error_return;
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::attach_error_cause;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

/// Lower an expression that must produce an error pointer.
pub(super) fn lower_error_pointer_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    role: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    let value = codegen_expression(codegen_context, env, expr, None)?;
    if value.is_pointer_value() {
        return Ok(value.into_pointer_value());
    }
    Err(CodegenError::new(format!(
        "{role} must lower to an error pointer"
    )))
}

/// Return whether an expression is a direct error value accepted by the checker.
pub(super) fn is_direct_error_value_expression(expr: &Expr) -> bool {
    match expr {
        Expr::Identifier { .. } => true,
        Expr::Parenthesized { expr, .. } | Expr::BorrowArgument { target: expr, .. } => {
            is_direct_error_value_expression(expr.as_ref())
        }
        _ => false,
    }
}

/// Lower `propagate error_value [cause cause_value]` to an immediate cleanup-aware return.
pub(super) fn propagate_error_value_return<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    error_expr: &Expr,
    requested_cause: Option<PointerValue<'context>>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let primary_error =
        lower_error_pointer_expression(codegen_context, env, error_expr, "propagate primary")?;
    let forwarded_error =
        attach_requested_error_cause(codegen_context, env, primary_error, requested_cause)?;
    emit_cleanup_aware_error_return(
        codegen_context,
        env,
        current_function(codegen_context)?,
        Some(forwarded_error),
        &[],
    )?;
    Ok(codegen_context
        .context
        .struct_type(&[], false)
        .const_zero()
        .as_basic_value_enum())
}

/// Attach an optional cause to a primary error pointer.
pub(super) fn attach_requested_error_cause<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    primary: PointerValue<'context>,
    cause: Option<PointerValue<'context>>,
) -> Result<PointerValue<'context>, CodegenError> {
    cause.map_or(Ok(primary), |cause_value| {
        attach_error_cause(codegen_context, env, primary, cause_value)
    })
}
