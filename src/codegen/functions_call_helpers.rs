extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::values::FunctionValue;

#[doc = "Emit early-return default for propagate error path."]
pub(super) fn emit_function_default_return<'context>(
    codegen_context: &CodegenContext<'context>,
    function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    let return_type = function.get_type().get_return_type();
    if return_type.is_none() {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }
    let Some(_return_basic_type) = return_type else {
        return Err(CodegenError::new(String::from(
            "invalid function return type",
        )));
    };
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
