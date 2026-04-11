extern crate alloc;

use crate::ast::{BinaryOp, Expr, LiteralValue, UnaryOp};

/// Arithmetic overflow behavior metadata attached to typed expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArithmeticMode {
    /// Language default behavior (debug trap, release wrap per specification).
    Default,
    /// Checked arithmetic returns optional result (`Option<T>` style intrinsics).
    Checked,
    /// Wrapping arithmetic never traps and wraps by type width.
    Wrapping,
    /// Saturating arithmetic clamps to numeric bounds.
    Saturating,
}

/// Resolve arithmetic mode for intrinsic binary operators.
pub const fn mode_for_binary_operator(operator: &BinaryOp) -> Option<ArithmeticMode> {
    match *operator {
        BinaryOp::Add
        | BinaryOp::Subtract
        | BinaryOp::Multiply
        | BinaryOp::Divide
        | BinaryOp::Modulo
        | BinaryOp::BitShiftLeft
        | BinaryOp::BitShiftRight
        | BinaryOp::BitUnsignedShiftRight => Some(ArithmeticMode::Default),
        _ => None,
    }
}

/// Resolve arithmetic mode for member-call intrinsics by naming convention.
pub fn mode_for_intrinsic_member(member_name: &str) -> Option<ArithmeticMode> {
    if member_name.starts_with("checked_") {
        return Some(ArithmeticMode::Checked);
    }

    if member_name.starts_with("wrapping_") {
        return Some(ArithmeticMode::Wrapping);
    }

    if member_name.starts_with("masked_") {
        return Some(ArithmeticMode::Wrapping);
    }

    if member_name.ends_with("_masked") {
        return Some(ArithmeticMode::Wrapping);
    }

    if member_name.starts_with("saturating_") {
        return Some(ArithmeticMode::Saturating);
    }

    None
}

/// Fold integer binary expression operands when both sides are constants.
pub fn fold_integer_binary_expr(
    operator: &BinaryOp,
    left_expr: &Expr,
    right_expr: &Expr,
) -> Option<i128> {
    let left_value = extract_integer_constant(left_expr)?;
    let right_value = extract_integer_constant(right_expr)?;
    fold_integer_binary_values(operator, left_value, right_value)
}

/// Extract a signed integer constant from literal-like expression forms.
pub fn extract_integer_constant(expr: &Expr) -> Option<i128> {
    match *expr {
        Expr::Literal {
            value: LiteralValue::Integer(value),
            ..
        } => Some(i128::from(value)),
        Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
            extract_integer_constant(expr)
        }
        Expr::Unary {
            operator: UnaryOp::Negate,
            ref operand,
            ..
        } => extract_integer_constant(operand).and_then(i128::checked_neg),
        Expr::Unary {
            operator: UnaryOp::Plus,
            ref operand,
            ..
        } => extract_integer_constant(operand),
        Expr::Binary {
            ref left,
            ref operator,
            ref right,
            ..
        } => {
            let left_value = extract_integer_constant(left)?;
            let right_value = extract_integer_constant(right)?;
            fold_integer_binary_values(operator, left_value, right_value)
        }
        _ => None,
    }
}

/// Fold integer binary values using checked arithmetic semantics.
pub const fn fold_integer_binary_values(
    operator: &BinaryOp,
    left_value: i128,
    right_value: i128,
) -> Option<i128> {
    match *operator {
        BinaryOp::Add => left_value.checked_add(right_value),
        BinaryOp::Subtract => left_value.checked_sub(right_value),
        BinaryOp::Multiply => left_value.checked_mul(right_value),
        BinaryOp::Divide => left_value.checked_div(right_value),
        BinaryOp::Modulo => left_value.checked_rem(right_value),
        _ => None,
    }
}
