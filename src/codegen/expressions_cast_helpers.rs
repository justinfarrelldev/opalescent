#![allow(clippy::all, reason = "internal codegen implementation module")]
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::type_system::types::CoreType;

pub(super) fn integer_type_for<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
) -> Result<inkwell::types::IntType<'context>, CodegenError> {
    match *core_type {
        CoreType::Int8 | CoreType::UInt8 => Ok(codegen_context.context.i8_type()),
        CoreType::Int16 | CoreType::UInt16 => Ok(codegen_context.context.i16_type()),
        CoreType::Int32 | CoreType::UInt32 => Ok(codegen_context.context.i32_type()),
        CoreType::Int64 | CoreType::UInt64 => Ok(codegen_context.context.i64_type()),
        _ => Err(CodegenError::new(format!(
            "{core_type} is not an integer type"
        ))),
    }
}

pub(super) fn float_type_for<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
) -> Result<inkwell::types::FloatType<'context>, CodegenError> {
    match *core_type {
        CoreType::Float32 => Ok(codegen_context.context.f32_type()),
        CoreType::Float64 => Ok(codegen_context.context.f64_type()),
        _ => Err(CodegenError::new(format!(
            "{core_type} is not a float type"
        ))),
    }
}

pub(super) const fn is_integer_core_type(core_type: &CoreType) -> bool {
    matches!(
        *core_type,
        CoreType::Int8
            | CoreType::Int16
            | CoreType::Int32
            | CoreType::Int64
            | CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
    )
}

pub(super) const fn is_float_core_type(core_type: &CoreType) -> bool {
    matches!(*core_type, CoreType::Float32 | CoreType::Float64)
}
