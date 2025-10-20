//! Helper functions for type checking operations

extern crate alloc;

use crate::ast::{BinaryOp, Expr, LiteralValue, UnaryOp};
use crate::token::Span;
use crate::type_system::errors::TypeError;
use crate::type_system::types::{CoreType, NumericKind};

/// Determine the canonical [`CoreType`] for a literal value.
pub(super) const fn literal_to_core_type(value: &LiteralValue) -> CoreType {
    match *value {
        LiteralValue::Integer(_) => CoreType::Int64,
        LiteralValue::Float(_) => CoreType::Float64,
        LiteralValue::String(_) => CoreType::String,
        LiteralValue::Boolean(_) => CoreType::Boolean,
        LiteralValue::Void => CoreType::Unit,
    }
}

/// Categorize a core type into a numeric family when applicable.
pub(super) const fn classify_numeric(core_type: &CoreType) -> Option<NumericKind> {
    match *core_type {
        CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64 => {
            Some(NumericKind::SignedInt)
        }
        CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64 => {
            Some(NumericKind::UnsignedInt)
        }
        CoreType::Float32 | CoreType::Float64 => Some(NumericKind::Float),
        _ => None,
    }
}

/// Check whether the provided type belongs to any numeric family.
pub(super) const fn is_numeric_type(core_type: &CoreType) -> bool {
    classify_numeric(core_type).is_some()
}

/// Check whether the provided type is an integer (signed or unsigned).
pub(super) const fn is_integer_type(core_type: &CoreType) -> bool {
    matches!(
        classify_numeric(core_type),
        Some(NumericKind::SignedInt | NumericKind::UnsignedInt)
    )
}

/// Check whether the provided type is a floating point primitive.
pub(super) const fn is_float_type(core_type: &CoreType) -> bool {
    matches!(core_type, &CoreType::Float32 | &CoreType::Float64)
}

/// Check whether the provided type is the boolean primitive.
pub(super) const fn is_boolean_type(core_type: &CoreType) -> bool {
    matches!(core_type, &CoreType::Boolean)
}

/// Check whether the provided type is the string primitive.
pub(super) const fn is_string_type(core_type: &CoreType) -> bool {
    matches!(core_type, &CoreType::String)
}

/// Construct a type error describing an invalid operation on a type.
pub(super) fn invalid_operation_error(
    operation: &str,
    core_type: &CoreType,
    span: Span,
) -> TypeError {
    TypeError::InvalidOperation {
        operation: operation.to_owned(),
        type_name: core_type.to_string(),
        span: TypeError::span_from_span(span),
    }
}

/// Construct a type mismatch diagnostic with consistent formatting.
pub(super) fn type_mismatch_error(
    expected: &CoreType,
    expected_span: Option<Span>,
    found: &CoreType,
    found_span: Span,
) -> TypeError {
    TypeError::TypeMismatch {
        expected: expected.to_string(),
        found: found.to_string(),
        found_span: TypeError::span_from_span(found_span),
        expected_span: expected_span.map(TypeError::span_from_span),
    }
}

/// Attempt to coerce a literal expression's type to match an expected core type.
pub(super) fn coerce_literal_to_expected(
    expected: &CoreType,
    expr: &Expr,
    actual: &CoreType,
) -> Option<CoreType> {
    match *expr {
        Expr::Literal { ref value, .. } => match *value {
            LiteralValue::Integer(_) => {
                (is_integer_type(expected) && is_integer_type(actual)).then(|| expected.clone())
            }
            LiteralValue::Float(_) => {
                (is_float_type(expected) && is_float_type(actual)).then(|| expected.clone())
            }
            _ => None,
        },
        _ => None,
    }
}

/// Ensure that two resolved operand types are identical, capturing precise source spans
/// for a subsequent diagnostic when they differ.
pub(super) fn ensure_same_type(
    expected: &CoreType,
    expected_span: Span,
    actual: &CoreType,
    actual_span: Span,
) -> Result<(), TypeError> {
    if expected == actual {
        Ok(())
    } else {
        Err(type_mismatch_error(
            expected,
            Some(expected_span),
            actual,
            actual_span,
        ))
    }
}

/// Validate that a core type belongs to one of the numeric families prior to a numeric
/// operation, preserving architectural guarantees about arithmetic safety.
pub(super) fn ensure_numeric_type(
    core_type: &CoreType,
    span: Span,
    operation: &str,
) -> Result<(), TypeError> {
    if is_numeric_type(core_type) {
        Ok(())
    } else {
        Err(invalid_operation_error(operation, core_type, span))
    }
}

/// Ensure that the provided type is an integer (signed or unsigned) before executing an
/// integer-only operation, preventing silent lossy conversions.
pub(super) fn ensure_integer_type(
    core_type: &CoreType,
    span: Span,
    operation: &str,
) -> Result<(), TypeError> {
    if is_integer_type(core_type) {
        Ok(())
    } else {
        Err(invalid_operation_error(operation, core_type, span))
    }
}

/// Guard boolean-only operations so that only strict `boolean` operands are permitted,
/// preserving logical semantics for control-flow constructs.
pub(super) fn ensure_boolean_type(
    core_type: &CoreType,
    span: Span,
    operation: &str,
) -> Result<(), TypeError> {
    if is_boolean_type(core_type) {
        Ok(())
    } else {
        Err(invalid_operation_error(operation, core_type, span))
    }
}

/// Provide a human-readable description for a binary operator, feeding into diagnostics
/// and future telemetry without repeating strings across the code base.
pub(super) const fn binary_operation_name(operator: &BinaryOp) -> &'static str {
    match *operator {
        BinaryOp::Add => "addition",
        BinaryOp::Subtract => "subtraction",
        BinaryOp::Multiply => "multiplication",
        BinaryOp::Divide => "division",
        BinaryOp::Modulo => "modulo",
        BinaryOp::Power => "exponentiation",
        BinaryOp::Equal => "equality comparison",
        BinaryOp::NotEqual => "inequality comparison",
        BinaryOp::Less => "less-than comparison",
        BinaryOp::LessEqual => "less-or-equal comparison",
        BinaryOp::Greater => "greater-than comparison",
        BinaryOp::GreaterEqual => "greater-or-equal comparison",
        BinaryOp::Is => "identity comparison",
        BinaryOp::IsNot => "negative identity comparison",
        BinaryOp::And => "logical and",
        BinaryOp::Or => "logical or",
        BinaryOp::Xor => "logical xor",
        BinaryOp::BitAnd => "bitwise and",
        BinaryOp::BitOr => "bitwise or",
        BinaryOp::BitXor => "bitwise xor",
        BinaryOp::BitShiftLeft => "left shift",
        BinaryOp::BitShiftRight => "right shift",
        BinaryOp::BitUnsignedShiftRight => "unsigned right shift",
        BinaryOp::Assign => "assignment",
    }
}

/// Provide a human-readable description for unary operators so that diagnostics can
/// reference intent rather than symbolic tokens alone.
pub(super) const fn unary_operation_name(operator: &UnaryOp) -> &'static str {
    match *operator {
        UnaryOp::Negate => "numeric negation",
        UnaryOp::Not => "logical not",
        UnaryOp::BitNot => "bitwise not",
        UnaryOp::Plus => "unary plus",
    }
}

/// Determine the numeric family and bit width for cast validation, enabling widening rules
/// that mirror the language specification while keeping the data in a const context.
pub(super) const fn numeric_bit_width(core_type: &CoreType) -> Option<(NumericKind, u8)> {
    match *core_type {
        CoreType::Int8 => Some((NumericKind::SignedInt, 8)),
        CoreType::Int16 => Some((NumericKind::SignedInt, 16)),
        CoreType::Int32 => Some((NumericKind::SignedInt, 32)),
        CoreType::Int64 => Some((NumericKind::SignedInt, 64)),
        CoreType::UInt8 => Some((NumericKind::UnsignedInt, 8)),
        CoreType::UInt16 => Some((NumericKind::UnsignedInt, 16)),
        CoreType::UInt32 => Some((NumericKind::UnsignedInt, 32)),
        CoreType::UInt64 => Some((NumericKind::UnsignedInt, 64)),
        CoreType::Float32 => Some((NumericKind::Float, 32)),
        CoreType::Float64 => Some((NumericKind::Float, 64)),
        _ => None,
    }
}

/// Determine whether an implicit cast between numeric types is permitted under the
/// language's widening rules. This intentionally excludes narrowing conversions and
/// mixed-family casts unless explicitly sanctioned by the specification.
pub(super) fn is_cast_allowed(from: &CoreType, to: &CoreType) -> bool {
    if from == to {
        return true;
    }
    match (numeric_bit_width(from), numeric_bit_width(to)) {
        (Some((NumericKind::SignedInt, from_bits)), Some((NumericKind::SignedInt, to_bits)))
        | (
            Some((NumericKind::UnsignedInt, from_bits)),
            Some((NumericKind::UnsignedInt, to_bits)),
        )
        | (Some((NumericKind::Float, from_bits)), Some((NumericKind::Float, to_bits))) => {
            from_bits <= to_bits
        }
        (
            Some((NumericKind::SignedInt | NumericKind::UnsignedInt, _)),
            Some((NumericKind::Float, _)),
        ) => true,
        _ => false,
    }
}
