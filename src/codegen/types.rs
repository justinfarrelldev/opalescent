//! Core type to LLVM type mapping.

use crate::codegen::expressions::CodegenError;
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

/// Returns true when the core type is a signed integer type (i8, i16, i32, i64).
pub const fn is_signed_core_type(core_type: &CoreType) -> bool {
    matches!(
        *core_type,
        CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64
    )
}

/// Encode signed integer literals as two's-complement bit patterns.
///
/// Non-negative integers are returned as-is. Negative integers are encoded
/// as their two's complement representation in u64.
pub fn integer_literal_bits(number: i64) -> Result<u64, CodegenError> {
    if number >= 0 {
        return u64::try_from(number).map_err(|conversion_error| {
            CodegenError::new(format!(
                "failed converting non-negative integer literal to u64: {conversion_error}"
            ))
        });
    }

    let magnitude = number.unsigned_abs();
    Ok((!magnitude).wrapping_add(1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_literal_bits_positive() {
        assert_eq!(integer_literal_bits(42).unwrap(), 42_u64);
    }

    #[test]
    fn test_integer_literal_bits_zero() {
        assert_eq!(integer_literal_bits(0).unwrap(), 0_u64);
    }

    #[test]
    fn test_integer_literal_bits_negative_one() {
        assert_eq!(integer_literal_bits(-1).unwrap(), u64::MAX);
    }

    #[test]
    fn test_is_signed_core_type_signed() {
        assert!(is_signed_core_type(&CoreType::Int8));
        assert!(is_signed_core_type(&CoreType::Int16));
        assert!(is_signed_core_type(&CoreType::Int32));
        assert!(is_signed_core_type(&CoreType::Int64));
    }

    #[test]
    fn test_is_signed_core_type_unsigned() {
        assert!(!is_signed_core_type(&CoreType::UInt8));
        assert!(!is_signed_core_type(&CoreType::UInt16));
        assert!(!is_signed_core_type(&CoreType::UInt32));
        assert!(!is_signed_core_type(&CoreType::UInt64));
    }

    #[test]
    fn test_is_signed_core_type_float() {
        assert!(!is_signed_core_type(&CoreType::Float32));
        assert!(!is_signed_core_type(&CoreType::Float64));
    }
}
