//! Core type to LLVM type mapping.

use crate::type_system::types::CoreType;
use inkwell::context::Context;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::AddressSpace;

/// Convert a [`CoreType`] into the nearest LLVM [`BasicTypeEnum`].
#[must_use]
pub fn core_type_to_llvm<'context>(
    context: &'context Context,
    core_type: &CoreType,
) -> BasicTypeEnum<'context> {
    match *core_type {
        CoreType::Int8 | CoreType::UInt8 => context.i8_type().into(),
        CoreType::Int16 | CoreType::UInt16 => context.i16_type().into(),
        CoreType::Int32 | CoreType::UInt32 => context.i32_type().into(),
        CoreType::Int64 | CoreType::UInt64 => context.i64_type().into(),
        CoreType::Float32 => context.f32_type().into(),
        CoreType::Float64 => context.f64_type().into(),
        CoreType::Boolean => context.bool_type().into(),
        CoreType::String
        | CoreType::Variable(_)
        | CoreType::Function { .. }
        | CoreType::Generic { .. } => context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .as_basic_type_enum(),
        CoreType::Array(ref element_type) => core_type_to_llvm(context, element_type)
            .array_type(0)
            .as_basic_type_enum(),
        CoreType::Unit => context.struct_type(&[], false).as_basic_type_enum(),
    }
}
