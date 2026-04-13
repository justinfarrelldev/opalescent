//! Operator precedence definitions for expression parsing
//!
//! This module defines the precedence levels for all operators in the language,
//! which is used by the precedence-climbing parser for correct operator parsing.

use crate::token::TokenType;

/// Operator precedence levels (higher number = higher precedence)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None = 0,
    Assignment = 1, // =
    Or = 2,         // or
    Xor = 3,        // xor
    And = 4,        // and
    BitOr = 5,      // bor
    BitXor = 6,     // bxor
    BitAnd = 7,     // band
    Equality = 8,   // is, is not
    Comparison = 9, // <, <=, >, >=
    Cast = 10,
    Shift = 11,   // bshl, bshr, bushr
    Term = 12,    // +, -
    Factor = 13,  // *, /, %
    Power = 14,   // ^ (right-associative)
    Unary = 15,   // +x, -x, not x, bnot x
    Call = 16,    // function calls, array access
    Primary = 17, // literals, identifiers, parentheses
}

impl Precedence {
    /// Determines the precedence level for a given token type
    /// Returns the appropriate precedence for binary operators
    pub const fn from_token(token_type: &TokenType) -> Self {
        match *token_type {
            // Remove assignment from expression precedence since it's a statement
            TokenType::Or => Self::Or,
            TokenType::Xor => Self::Xor,
            TokenType::And => Self::And,
            TokenType::BitOr => Self::BitOr,
            TokenType::BitXor => Self::BitXor,
            TokenType::BitAnd => Self::BitAnd,
            TokenType::Is | TokenType::IsNot => Self::Equality,
            TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual => Self::Comparison,
            TokenType::Cast => Self::Cast,
            TokenType::BitShiftLeft
            | TokenType::BitShiftRight
            | TokenType::BitUnsignedShiftRight => Self::Shift,
            TokenType::Plus | TokenType::Minus => Self::Term,
            TokenType::Multiply | TokenType::Divide | TokenType::Modulo => Self::Factor,
            TokenType::Power => Self::Power,
            TokenType::LeftParen | TokenType::LeftBracket | TokenType::Dot => Self::Call,
            _ => Self::None,
        }
    }

    /// Get the next higher precedence level for left-associative operators
    /// Used in precedence climbing to determine when to stop parsing at current level
    pub const fn next(self) -> Self {
        match self {
            Self::Assignment => Self::Or,
            Self::Or => Self::Xor,
            Self::Xor => Self::And,
            Self::And => Self::BitOr,
            Self::BitOr => Self::BitXor,
            Self::BitXor => Self::BitAnd,
            Self::BitAnd => Self::Equality,
            Self::Equality => Self::Comparison,
            Self::Comparison => Self::Cast,
            Self::Cast => Self::Shift,
            Self::Shift => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Power,
            Self::Power => Self::Unary,
            _ => self,
        }
    }
}
