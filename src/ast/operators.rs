//! Operator definitions and conversions for the Opalescent AST
//!
//! This module contains binary and unary operator enums along with their
//! conversions from token types and display implementations.

use crate::token::TokenType;
use core::fmt;

/// Binary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    // Arithmetic
    /// Addition operator (+)
    Add,
    /// Subtraction operator (-)
    Subtract,
    /// Multiplication operator (*)
    Multiply,
    /// Division operator (/)
    Divide,
    /// Modulo operator (%)
    Modulo,
    /// Exponentiation operator (^)
    Power,

    // Comparison
    /// Equality operator (is)
    Equal,
    /// Inequality operator (is not)
    NotEqual,
    /// Less than operator (<)
    Less,
    /// Less than or equal operator (<=)
    LessEqual,
    /// Greater than operator (>)
    Greater,
    /// Greater than or equal operator (>=)
    GreaterEqual,
    /// Identity comparison operator (is)
    Is,
    /// Negative identity comparison operator (is not)
    IsNot,

    // Logical
    /// Logical AND operator (and)
    And,
    /// Logical OR operator (or)
    Or,
    /// Logical XOR operator (xor)
    Xor,

    // Bitwise
    /// Bitwise AND operator (band)
    BitAnd,
    /// Bitwise OR operator (bor)
    BitOr,
    /// Bitwise XOR operator (bxor)
    BitXor,
    /// Bitwise left shift operator (bshl)
    BitShiftLeft,
    /// Bitwise right shift operator (bshr)
    BitShiftRight,
    /// Bitwise unsigned right shift operator (bushr)
    BitUnsignedShiftRight,

    // Assignment
    /// Assignment operator (=)
    Assign,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    /// Numeric negation operator (-x)
    Negate,
    /// Logical negation operator (not x)
    Not,
    /// Bitwise negation operator (bnot x)
    BitNot,
    /// Unary plus operator (+x)
    Plus,
}

impl fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match *self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulo => "%",
            Self::Power => "^",
            Self::Equal | Self::Is => "is",
            Self::NotEqual | Self::IsNot => "is not",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::And => "and",
            Self::Or => "or",
            Self::Xor => "xor",
            Self::BitAnd => "band",
            Self::BitOr => "bor",
            Self::BitXor => "bxor",
            Self::BitShiftLeft => "bshl",
            Self::BitShiftRight => "bshr",
            Self::BitUnsignedShiftRight => "bushr",
            Self::Assign => "=",
        };
        write!(f, "{symbol}")
    }
}

impl fmt::Display for UnaryOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbol = match *self {
            Self::Negate => "-",
            Self::Not => "not",
            Self::BitNot => "bnot",
            Self::Plus => "+",
        };
        write!(f, "{symbol}")
    }
}

impl TryFrom<TokenType> for BinaryOp {
    type Error = String;

    fn try_from(token_type: TokenType) -> Result<Self, Self::Error> {
        match token_type {
            TokenType::Plus => Ok(Self::Add),
            TokenType::Minus => Ok(Self::Subtract),
            TokenType::Multiply => Ok(Self::Multiply),
            TokenType::Divide => Ok(Self::Divide),
            TokenType::Modulo => Ok(Self::Modulo),
            TokenType::Power => Ok(Self::Power),
            TokenType::Less => Ok(Self::Less),
            TokenType::LessEqual => Ok(Self::LessEqual),
            TokenType::Greater => Ok(Self::Greater),
            TokenType::GreaterEqual => Ok(Self::GreaterEqual),
            TokenType::Is => Ok(Self::Is),
            TokenType::IsNot => Ok(Self::IsNot),
            TokenType::And => Ok(Self::And),
            TokenType::Or => Ok(Self::Or),
            TokenType::Xor => Ok(Self::Xor),
            TokenType::BitAnd => Ok(Self::BitAnd),
            TokenType::BitOr => Ok(Self::BitOr),
            TokenType::BitXor => Ok(Self::BitXor),
            TokenType::BitShiftLeft => Ok(Self::BitShiftLeft),
            TokenType::BitShiftRight => Ok(Self::BitShiftRight),
            TokenType::BitUnsignedShiftRight => Ok(Self::BitUnsignedShiftRight),
            TokenType::Assign => Ok(Self::Assign),
            _ => Err(format!("Cannot convert {token_type:?} to BinaryOp")),
        }
    }
}

impl TryFrom<TokenType> for UnaryOp {
    type Error = String;

    fn try_from(token_type: TokenType) -> Result<Self, Self::Error> {
        match token_type {
            TokenType::Minus => Ok(Self::Negate),
            TokenType::Plus => Ok(Self::Plus),
            TokenType::Not => Ok(Self::Not),
            TokenType::BitNot => Ok(Self::BitNot),
            _ => Err(format!("Cannot convert {token_type:?} to UnaryOp")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_op_display() {
        assert_eq!(format!("{}", BinaryOp::Add), "+");
        assert_eq!(format!("{}", BinaryOp::And), "and");
        assert_eq!(format!("{}", BinaryOp::BitShiftLeft), "bshl");
    }

    #[test]
    fn test_unary_op_display() {
        assert_eq!(format!("{}", UnaryOp::Negate), "-");
        assert_eq!(format!("{}", UnaryOp::Not), "not");
        assert_eq!(format!("{}", UnaryOp::BitNot), "bnot");
    }

    #[test]
    fn test_token_to_binary_op() {
        assert_eq!(BinaryOp::try_from(TokenType::Plus).unwrap(), BinaryOp::Add);
        assert_eq!(BinaryOp::try_from(TokenType::And).unwrap(), BinaryOp::And);
        assert_eq!(
            BinaryOp::try_from(TokenType::BitShiftLeft).unwrap(),
            BinaryOp::BitShiftLeft
        );
    }

    #[test]
    fn test_token_to_unary_op() {
        assert_eq!(
            UnaryOp::try_from(TokenType::Minus).unwrap(),
            UnaryOp::Negate
        );
        assert_eq!(UnaryOp::try_from(TokenType::Not).unwrap(), UnaryOp::Not);
        assert_eq!(
            UnaryOp::try_from(TokenType::BitNot).unwrap(),
            UnaryOp::BitNot
        );
    }
}
