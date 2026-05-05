//! Helper functions for type checking operations

extern crate alloc;

use crate::ast::{BinaryOp, Expr, LiteralValue, UnaryOp};
use crate::token::Span;
use crate::type_system::arithmetic::fold_integer_binary_values;
use crate::type_system::errors::{TypeError, Warning};
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
        Expr::Array { ref elements, .. } => (elements.is_empty()
            && matches!(expected, &CoreType::Array(_)))
        .then(|| expected.clone()),
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
        BinaryOp::DivEuclid => "euclidean division",
        BinaryOp::ModEuclid => "euclidean modulo",
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

/// Return the bit width for concrete integer core types.
pub(super) const fn integer_bit_width(core_type: &CoreType) -> Option<u32> {
    match *core_type {
        CoreType::Int8 | CoreType::UInt8 => Some(8),
        CoreType::Int16 | CoreType::UInt16 => Some(16),
        CoreType::Int32 | CoreType::UInt32 => Some(32),
        CoreType::Int64 | CoreType::UInt64 => Some(64),
        _ => None,
    }
}

/// Return inclusive min/max bounds for concrete integer core types.
pub(super) const fn integer_bounds(core_type: &CoreType) -> Option<(i128, i128)> {
    match *core_type {
        CoreType::Int8 => Some((-128, 127)),
        CoreType::Int16 => Some((-0x8000, 0x7FFF)),
        CoreType::Int32 => Some((-0x8000_0000, 0x7FFF_FFFF)),
        CoreType::Int64 => Some((-0x8000_0000_0000_0000, 0x7FFF_FFFF_FFFF_FFFF)),
        CoreType::UInt8 => Some((0, 255)),
        CoreType::UInt16 => Some((0, 0xFFFF)),
        CoreType::UInt32 => Some((0, 0xFFFF_FFFF)),
        CoreType::UInt64 => Some((0, 0xFFFF_FFFF_FFFF_FFFF)),
        _ => None,
    }
}

pub(super) use crate::type_system::arithmetic::extract_integer_constant;

/// Check whether integer constant arithmetic exceeds destination type bounds.
pub(super) const fn integer_arithmetic_overflows(
    operator: &BinaryOp,
    operand_type: &CoreType,
    left: i128,
    right: i128,
) -> bool {
    if !matches!(
        *operator,
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply
    ) {
        return false;
    }

    let computed = fold_constant_binary_op(operator, left, right);

    let Some(result) = computed else {
        return true;
    };

    let Some((min_value, max_value)) = integer_bounds(operand_type) else {
        return false;
    };

    result < min_value || result > max_value
}

/// Build an arithmetic-overflow warning for constant integer binary expressions.
pub(super) fn constant_integer_overflow_warning(
    operator: &BinaryOp,
    left_expr: &Expr,
    right_expr: &Expr,
    result_type: &CoreType,
    span: Span,
) -> Option<Warning> {
    let left_value = extract_integer_constant(left_expr)?;
    let right_value = extract_integer_constant(right_expr)?;

    if !integer_arithmetic_overflows(operator, result_type, left_value, right_value) {
        return None;
    }

    let operation = match *operator {
        BinaryOp::Add => "addition",
        BinaryOp::Subtract => "subtraction",
        BinaryOp::Multiply => "multiplication",
        _ => return None,
    };

    Some(Warning::ArithmeticOverflow {
        operation: operation.to_owned(),
        type_name: result_type.to_string(),
        span: TypeError::span_from_span(span),
        suppression_annotation: None,
    })
}

/// Identify division-like operators with a compile-time constant zero divisor.
pub(super) fn zero_divisor_operation_name(
    operator: &BinaryOp,
    right_expr: &Expr,
) -> Option<&'static str> {
    let divisor = extract_integer_constant(right_expr)?;
    if divisor != 0 {
        return None;
    }

    match *operator {
        BinaryOp::Divide => Some("division"),
        BinaryOp::Modulo => Some("modulo"),
        _ => None,
    }
}

/// Validate compile-time shift count bounds for constant integer shifts.
pub(super) fn validate_constant_shift_bounds(
    operator: &BinaryOp,
    left_type: &CoreType,
    right_expr: &Expr,
    shift_count_span: Span,
) -> Result<(), TypeError> {
    if let Some(error) = check_shift_bounds(operator, left_type, right_expr, shift_count_span) {
        return Err(error);
    }
    Ok(())
}

/// Validate constant shift counts for shift operators and return a typed diagnostic on failure.
pub(super) fn check_shift_bounds(
    operator: &BinaryOp,
    left_type: &CoreType,
    right_expr: &Expr,
    shift_count_span: Span,
) -> Option<TypeError> {
    if !matches!(
        *operator,
        BinaryOp::BitShiftLeft | BinaryOp::BitShiftRight | BinaryOp::BitUnsignedShiftRight
    ) {
        return None;
    }

    let shift_count = extract_integer_constant(right_expr)?;
    let bit_width = integer_bit_width(left_type)?;

    if shift_count.is_negative() {
        return Some(invalid_shift_count_error(
            "negative",
            shift_count,
            bit_width,
            shift_count_span,
        ));
    }

    let Ok(shift_count_u32) = u32::try_from(shift_count) else {
        return Some(invalid_shift_count_error(
            "out of range",
            shift_count,
            bit_width,
            shift_count_span,
        ));
    };

    if shift_count_u32 >= bit_width {
        return Some(invalid_shift_count_error(
            "out of range",
            shift_count,
            bit_width,
            shift_count_span,
        ));
    }

    None
}

/// Build a compile-time invalid-shift diagnostic with spec-defined reason/value metadata.
pub(super) fn invalid_shift_count_error(
    reason: &str,
    shift_count: i128,
    bit_width: u32,
    span: Span,
) -> TypeError {
    TypeError::InvalidShiftCount {
        reason: reason.to_owned(),
        count_value: shift_count,
        shift_count,
        bit_width,
        span: TypeError::span_from_span(span),
    }
}

/// Fold compile-time integer binary arithmetic for overflow/division/shift analyses.
pub(super) const fn fold_constant_binary_op(
    operator: &BinaryOp,
    left: i128,
    right: i128,
) -> Option<i128> {
    fold_integer_binary_values(operator, left, right)
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

/// Determine whether a cast from one type to another is safe (widening).
///
/// This function is used by [`TypeChecker::validate_cast`](crate::type_system::checker::TypeChecker::validate_cast)
/// to classify casts and determine the appropriate behavior.
///
/// Safe casts are those that:
/// - Widen within the same numeric family (e.g., int32 -> int64)
/// - Convert from integer to float (may lose precision for very large integers, but no overflow)
/// - Are identity casts (same type)
///
/// # Cast Safety Rules
///
/// ## Safe Casts (Widening):
/// - `int8` -> `int16`, `int32`, `int64`
/// - `int16` -> `int32`, `int64`
/// - `int32` -> `int64`
/// - `uint8` -> `uint16`, `uint32`, `uint64`
/// - `uint16` -> `uint32`, `uint64`
/// - `uint32` -> `uint64`
/// - `float32` -> `float64`
/// - Any integer type -> any float type (may lose precision for very large integers)
///
/// ## Unsafe Casts (Narrowing):
/// - `int64` -> `int32`, `int16`, `int8`
/// - `int32` -> `int16`, `int8`
/// - `int16` -> `int8`
/// - `uint64` -> `uint32`, `uint16`, `uint8`
/// - `uint32` -> `uint16`, `uint8`
/// - `uint16` -> `uint8`
/// - `float64` -> `float32`
/// - Any float type -> any integer type
/// - Signed <-> unsigned conversions
///
/// # Overflow Detection Strategy
///
/// For unsafe casts (per language spec math.md):
/// - **Debug mode**: Runtime traps on overflow/out-of-range values
/// - **Release mode**: Wrapping behavior (no trap, wraps around)
/// - Compile-time overflow detection for constant expressions
///
/// # Related Functions
///
/// - [`TypeChecker::validate_cast`](crate::type_system::checker::TypeChecker::validate_cast) - Uses this to validate casts in expressions
/// - [`is_valid_cast`] - Checks if a cast is valid at all (includes unsafe casts)
///
/// Future phases will add:
/// - Compile-time constant folding with overflow checking
/// - Runtime trap generation in debug builds
/// - Warning levels for unsafe casts
pub(super) fn is_safe_cast(from: &CoreType, to: &CoreType) -> bool {
    // Identity casts are always safe
    if from == to {
        return true;
    }

    // Get numeric classifications and bit widths
    let Some((from_kind, from_bits)) = numeric_bit_width(from) else {
        return false; // Non-numeric types cannot be cast
    };

    let Some((to_kind, to_bits)) = numeric_bit_width(to) else {
        return false; // Non-numeric types cannot be cast
    };

    // Safe casts within the same numeric family (widening)
    if from_kind == to_kind && from_bits <= to_bits {
        return true;
    }

    // Safe casts from integer to float (may lose precision but no overflow)
    if matches!(from_kind, NumericKind::SignedInt | NumericKind::UnsignedInt)
        && matches!(to_kind, NumericKind::Float)
    {
        return true;
    }

    // All other casts are unsafe
    false
}

/// Check if a cast is valid (allowed but potentially unsafe).
///
/// Valid but unsafe casts include:
/// - Narrowing within same family (int64 -> int32)
/// - Float to integer conversions
/// - Signed <-> unsigned conversions
/// - Float narrowing (float64 -> float32)
pub(super) fn is_valid_cast(from: &CoreType, to: &CoreType) -> bool {
    // Identity casts are valid
    if from == to {
        return true;
    }

    // Get numeric classifications
    let Some((from_kind, _)) = numeric_bit_width(from) else {
        return false; // Non-numeric types cannot be cast
    };

    let Some((to_kind, _)) = numeric_bit_width(to) else {
        return false; // Non-numeric types cannot be cast
    };

    // All numeric-to-numeric casts are valid (but may be unsafe)
    matches!(
        (from_kind, to_kind),
        (
            NumericKind::SignedInt | NumericKind::UnsignedInt | NumericKind::Float,
            NumericKind::SignedInt | NumericKind::UnsignedInt | NumericKind::Float
        )
    )
}
