//! Return statement helpers shared by control-flow lowering.
//!
//! These routines keep the main control-flow module focused on block emission
//! while concentrating return-value transfer and error-wrapper inspection here.

extern crate alloc;

use crate::ast::{Expr, LabeledValue};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::is_error_abi_struct_type;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::basic_block::BasicBlock;
use inkwell::types::StructType;
use inkwell::values::FunctionValue;

/// Collect identifier names whose bindings are transferred directly by a return.
pub(crate) fn collect_transferred_return_identifier_names(values: &[LabeledValue]) -> Vec<String> {
    values
        .iter()
        .filter(|value| value.label != "err")
        .filter_map(|value| match value.value.clone() {
            Expr::Identifier { name, .. } => Some(name),
            _ => None,
        })
        .collect::<Vec<_>>()
}

/// Return the active function's error ABI return type when it has one.
pub(crate) fn current_error_return_type<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<Option<StructType<'context>>, CodegenError> {
    let function = current_function(codegen_context)?;
    let Some(return_type) = function.get_type().get_return_type() else {
        return Ok(None);
    };
    if !return_type.is_struct_type() {
        return Ok(None);
    }

    let struct_type = return_type.into_struct_type();
    if is_error_abi_struct_type(struct_type) {
        Ok(Some(struct_type))
    } else {
        Ok(None)
    }
}

/// Extract the wrapped error variant for a guard-wrapper constructor if present.
pub(crate) fn extract_guard_wrapper_error_variant<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
) -> Result<Option<String>, CodegenError> {
    let Expr::Constructor { callee, fields, .. } = expr.clone() else {
        return Ok(None);
    };

    let Some(source_field) = fields.iter().find(|field| field.name == "source") else {
        return Ok(None);
    };

    let Expr::Identifier { name, .. } = source_field.value.clone() else {
        return Ok(None);
    };

    let Some(active_guard_error_slot) = env.current_guard_error_slot() else {
        return Ok(None);
    };
    let Some(source_binding) = env.variables.get(name.as_str()) else {
        return Ok(None);
    };
    if source_binding.alloca != active_guard_error_slot {
        return Ok(None);
    }

    for field in fields {
        if field.name == "source" {
            continue;
        }
        let _unused: inkwell::values::BasicValueEnum<'_> =
            codegen_expression(codegen_context, env, &field.value, None)?;
    }

    Ok(Some(extract_error_variant_name(callee.as_ref())?))
}

/// Extract a variant name from an error return expression.
pub(crate) fn extract_error_variant_name(expr: &Expr) -> Result<String, CodegenError> {
    match expr.clone() {
        Expr::Identifier { name, .. } => Ok(name),
        Expr::Member { member, .. } => Ok(member),
        Expr::Constructor { fields, .. } if !fields.is_empty() => {
            Err(CodegenError::new(String::from(
                "payload-bearing error variants not yet supported in user-defined functions",
            )))
        }
        Expr::Constructor { callee, .. } => extract_error_variant_name(callee.as_ref()),
        _ => Err(CodegenError::new(String::from(
            "error returns must use `return err: VariantName`",
        ))),
    }
}

/// Return the function that owns the current insertion block.
fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    codegen_context
        .builder
        .get_insert_block()
        .and_then(BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new(String::from("codegen currently has no active function")))
}
