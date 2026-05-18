#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::{FunctionValue, PointerValue};

#[doc = "Emit early-return default for propagate error path."]
pub(super) fn emit_function_default_return<'context>(
    codegen_context: &CodegenContext<'context>,
    function: FunctionValue<'context>,
    forwarded_error: Option<PointerValue<'context>>,
) -> Result<(), CodegenError> {
    let return_type = function.get_type().get_return_type();
    if return_type.is_none() {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }
    let Some(return_basic_type) = return_type else {
        return Err(CodegenError::new(String::from(
            "invalid function return type",
        )));
    };
    if let Some(error_ptr) = forwarded_error {
        if return_basic_type.is_struct_type() {
            let return_struct_type = return_basic_type.into_struct_type();
            if return_struct_type.count_fields() == 2 {
                let success_type = return_struct_type
                    .get_field_type_at_index(0)
                    .ok_or_else(|| CodegenError::new(String::from("missing success field type")))?;
                let aggregate = crate::codegen::error_abi::build_error_aggregate(
                    codegen_context,
                    success_type,
                    error_ptr,
                )?;
                let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
                return Ok(());
            }
        }
    }
    let block_name = codegen_context
        .builder
        .get_insert_block()
        .and_then(|block| {
            block
                .get_name()
                .to_str()
                .ok()
                .map(alloc::borrow::ToOwned::to_owned)
        })
        .unwrap_or_else(|| String::from("ret"));
    let msg_name = format!("ret.msg.{block_name}");
    let call_name = format!("ret.call.{block_name}");
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context
        .builder
        .build_global_string_ptr("missing return statement", msg_name.as_str())?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue =
        codegen_context
            .builder
            .build_call(runtime_fn, &[msg.into()], call_name.as_str())?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;
    Ok(())
}

#[doc = "Fetch current LLVM function from builder insertion block."]
pub(super) fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let Some(block) = codegen_context.builder.get_insert_block() else {
        return Err(CodegenError::new(String::from(
            "builder is not positioned in a block",
        )));
    };
    block
        .get_parent()
        .ok_or_else(|| CodegenError::new(String::from("insert block does not have parent")))
}

pub(super) fn uses_aggregate_result_dispatch(function: FunctionValue<'_>) -> bool {
    function
        .get_type()
        .get_return_type()
        .is_some_and(|return_type| {
            return_type.is_struct_type() && return_type.into_struct_type().count_fields() >= 2
        })
        || function.get_type().get_return_type().is_none()
            && function
                .get_type()
                .get_param_types()
                .first()
                .is_some_and(|first_param| {
                    first_param.is_pointer_type()
                        && first_param
                            .into_pointer_type()
                            .get_element_type()
                            .is_struct_type()
                        && first_param
                            .into_pointer_type()
                            .get_element_type()
                            .into_struct_type()
                            .count_fields()
                            >= 2
                })
}

pub(super) fn caller_returns_error_aggregate(function: FunctionValue<'_>) -> bool {
    function
        .get_type()
        .get_return_type()
        .is_some_and(|return_type| {
            return_type.is_struct_type() && return_type.into_struct_type().count_fields() == 2
        })
}

#[doc = "Approximate core type mapping from LLVM basic value type."]
pub(super) fn llvm_basic_type_to_core_type(
    llvm_type: inkwell::types::BasicTypeEnum<'_>,
) -> CoreType {
    if llvm_type.is_int_type() {
        let int_type = llvm_type.into_int_type();
        return match int_type.get_bit_width() {
            1 => CoreType::Boolean,
            8 => CoreType::Int8,
            16 => CoreType::Int16,
            32 => CoreType::Int32,
            _ => CoreType::Int64,
        };
    }
    if llvm_type.is_float_type() {
        return CoreType::Float64;
    }
    if llvm_type.is_pointer_type() {
        return CoreType::String;
    }
    if llvm_type.is_array_type() {
        return CoreType::Array(alloc::boxed::Box::new(CoreType::Int64));
    }
    CoreType::Unit
}

#[doc = "Infer semantic core type for guard success binding from callee signature when possible."]
pub(super) fn infer_guard_binding_core_type(
    env: &CodegenEnv<'_>,
    guarded_expr: &Expr,
    success_value_type: inkwell::types::BasicTypeEnum<'_>,
) -> CoreType {
    if let Expr::Call { ref callee, .. } = *guarded_expr {
        if let Expr::Identifier { ref name, .. } = *callee.as_ref() {
            if let Some(first_return) =
                env.imported_signatures
                    .get(name.as_str())
                    .and_then(|signature| match signature {
                        &CoreType::Function {
                            ref return_types, ..
                        } => return_types.first(),
                        _ => None,
                    })
            {
                return first_return.clone();
            }

            if name == "bytes_from_hex" || name == "bytes_slice" {
                return CoreType::Generic {
                    name: String::from("Bytes"),
                    type_args: Vec::new(),
                };
            }
            if name == "frame_clock_new" {
                return CoreType::Generic {
                    name: String::from("FrameClock"),
                    type_args: Vec::new(),
                };
            }
            if name == "stdout_terminal" {
                return CoreType::Generic {
                    name: String::from("StdoutTerminal"),
                    type_args: Vec::new(),
                };
            }
            if name == "print_text_sync"
                || name == "flush_standard_output_sync"
                || name == "writer_write_sync"
                || name == "writer_flush_sync"
                || name == "sleep_ms_sync"
                || name == "frame_clock_wait_next_sync"
                || name == "terminal_clear_screen_on_sync"
                || name == "terminal_move_cursor_on_sync"
                || name == "terminal_draw_rows_sync"
                || name == "terminal_clear_screen_sync"
                || name == "terminal_move_cursor_sync"
            {
                return CoreType::Unit;
            }
        }
    }

    llvm_basic_type_to_core_type(success_value_type)
}
