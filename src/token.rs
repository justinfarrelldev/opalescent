//! Token definitions and utilities for the Opalescent programming language
//!
//! This module defines the token types, positions, and spans used by the lexer
//! and parser.

#![expect(dead_code, reason = "Token types are being developed incrementally")]

use std::fmt;

/// Represents a position in the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
    pub offset: usize,
}

impl Position {
    pub fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub fn start() -> Self {
        Self::new(1, 1, 0)
    }
}

/// Represents a span in the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: Position,
    pub end: Position,
}

impl Span {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn single(pos: Position) -> Self {
        Self::new(pos, pos)
    }
}

/// The different types of tokens in Opalescent
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    IntegerLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    BooleanLiteral(bool),

    // Identifiers and Keywords
    Identifier(String),

    // Keywords
    Let,
    Mutable,
    Function, // f
    Return,
    Void,
    If,
    Else,
    For,
    While,
    In,
    Break,
    Continue,

    // Type keywords
    Int8,
    Int16,
    Int32,
    Int64,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    Float32,
    Float64,
    String,
    Boolean,

    // Visibility
    Public,
    Entry,

    // Import/Export
    Import,
    From,
    As,
    Type,

    // Operators
    Plus,
    Minus,
    Multiply,
    Divide,
    Power,  // ^
    Modulo, // %

    // Assignment
    Assign, // =

    // Comparison
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Is,
    IsNot,

    // Logical
    And,
    Or,
    Not,
    Xor,

    // Bitwise
    BitAnd,                // band
    BitOr,                 // bor
    BitXor,                // bxor
    BitNot,                // bnot
    BitShiftLeft,          // bshl
    BitShiftRight,         // bshr
    BitUnsignedShiftRight, // bushr

    // Cast
    Cast, // as

    // Type checking
    TypeOf,

    // Punctuation
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Colon,
    Comma,
    Arrow, // =>
    Dot,

    // Comments
    Comment(String),
    DocComment(String),

    // Special
    Newline,
    Indent,
    Dedent,
    EndOfFile,
}

/// A token with its type, value, and location information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub span: Span,
    pub lexeme: String,
}

impl Token {
    pub fn new(token_type: TokenType, span: Span, lexeme: String) -> Self {
        Self {
            token_type,
            span,
            lexeme,
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenType::IntegerLiteral(n) => write!(f, "integer literal '{}'", n),
            TokenType::FloatLiteral(n) => write!(f, "float literal '{}'", n),
            TokenType::StringLiteral(s) => write!(f, "string literal '{}'", s),
            TokenType::BooleanLiteral(b) => write!(f, "boolean literal '{}'", b),
            TokenType::Identifier(name) => write!(f, "identifier '{}'", name),
            TokenType::Let => write!(f, "keyword 'let'"),
            TokenType::Mutable => write!(f, "keyword 'mutable'"),
            TokenType::Function => write!(f, "keyword 'f'"),
            TokenType::Return => write!(f, "keyword 'return'"),
            TokenType::Void => write!(f, "keyword 'void'"),
            TokenType::If => write!(f, "keyword 'if'"),
            TokenType::Else => write!(f, "keyword 'else'"),
            TokenType::For => write!(f, "keyword 'for'"),
            TokenType::While => write!(f, "keyword 'while'"),
            TokenType::In => write!(f, "keyword 'in'"),
            TokenType::Break => write!(f, "keyword 'break'"),
            TokenType::Continue => write!(f, "keyword 'continue'"),
            TokenType::Int8 => write!(f, "type 'int8'"),
            TokenType::Int16 => write!(f, "type 'int16'"),
            TokenType::Int32 => write!(f, "type 'int32'"),
            TokenType::Int64 => write!(f, "type 'int64'"),
            TokenType::UInt8 => write!(f, "type 'uint8'"),
            TokenType::UInt16 => write!(f, "type 'uint16'"),
            TokenType::UInt32 => write!(f, "type 'uint32'"),
            TokenType::UInt64 => write!(f, "type 'uint64'"),
            TokenType::Float32 => write!(f, "type 'float32'"),
            TokenType::Float64 => write!(f, "type 'float64'"),
            TokenType::String => write!(f, "type 'string'"),
            TokenType::Boolean => write!(f, "type 'boolean'"),
            TokenType::Public => write!(f, "keyword 'public'"),
            TokenType::Entry => write!(f, "keyword 'entry'"),
            TokenType::Import => write!(f, "keyword 'import'"),
            TokenType::From => write!(f, "keyword 'from'"),
            TokenType::As => write!(f, "keyword 'as'"),
            TokenType::Type => write!(f, "keyword 'type'"),
            TokenType::Plus => write!(f, "operator '+'"),
            TokenType::Minus => write!(f, "operator '-'"),
            TokenType::Multiply => write!(f, "operator '*'"),
            TokenType::Divide => write!(f, "operator '/'"),
            TokenType::Power => write!(f, "operator '^'"),
            TokenType::Modulo => write!(f, "operator '%'"),
            TokenType::Assign => write!(f, "operator '='"),
            TokenType::Less => write!(f, "operator '<'"),
            TokenType::LessEqual => write!(f, "operator '<='"),
            TokenType::Greater => write!(f, "operator '>'"),
            TokenType::GreaterEqual => write!(f, "operator '>='"),
            TokenType::Is => write!(f, "operator 'is'"),
            TokenType::IsNot => write!(f, "operator 'is not'"),
            TokenType::And => write!(f, "operator 'and'"),
            TokenType::Or => write!(f, "operator 'or'"),
            TokenType::Not => write!(f, "operator 'not'"),
            TokenType::Xor => write!(f, "operator 'xor'"),
            TokenType::BitAnd => write!(f, "operator 'band'"),
            TokenType::BitOr => write!(f, "operator 'bor'"),
            TokenType::BitXor => write!(f, "operator 'bxor'"),
            TokenType::BitNot => write!(f, "operator 'bnot'"),
            TokenType::BitShiftLeft => write!(f, "operator 'bshl'"),
            TokenType::BitShiftRight => write!(f, "operator 'bshr'"),
            TokenType::BitUnsignedShiftRight => write!(f, "operator 'bushr'"),
            TokenType::Cast => write!(f, "operator 'as'"),
            TokenType::TypeOf => write!(f, "function 'type_of'"),
            TokenType::LeftParen => write!(f, "'('"),
            TokenType::RightParen => write!(f, "')'"),
            TokenType::LeftBracket => write!(f, "'['"),
            TokenType::RightBracket => write!(f, "']'"),
            TokenType::LeftBrace => write!(f, "'{{'"),
            TokenType::RightBrace => write!(f, "'}}'"),
            TokenType::Colon => write!(f, "':'"),
            TokenType::Comma => write!(f, "','"),
            TokenType::Arrow => write!(f, "'=>'"),
            TokenType::Dot => write!(f, "'.'"),
            TokenType::Comment(_) => write!(f, "comment"),
            TokenType::DocComment(_) => write!(f, "documentation comment"),
            TokenType::Newline => write!(f, "newline"),
            TokenType::Indent => write!(f, "indent"),
            TokenType::Dedent => write!(f, "dedent"),
            TokenType::EndOfFile => write!(f, "end of file"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(5, 10, 100);
        assert_eq!(pos.line, 5);
        assert_eq!(pos.column, 10);
        assert_eq!(pos.offset, 100);
    }

    #[test]
    fn test_position_start() {
        let pos = Position::start();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.column, 1);
        assert_eq!(pos.offset, 0);
    }

    #[test]
    fn test_span_creation() {
        let start = Position::new(1, 1, 0);
        let end = Position::new(1, 5, 4);
        let span = Span::new(start, end);
        assert_eq!(span.start, start);
        assert_eq!(span.end, end);
    }

    #[test]
    fn test_span_single() {
        let pos = Position::new(2, 3, 10);
        let span = Span::single(pos);
        assert_eq!(span.start, pos);
        assert_eq!(span.end, pos);
    }

    #[test]
    fn test_token_creation() {
        let token_type = TokenType::IntegerLiteral(42);
        let span = Span::single(Position::new(1, 1, 0));
        let lexeme = "42".to_string();

        let token = Token::new(token_type.clone(), span, lexeme.clone());
        assert_eq!(token.token_type, token_type);
        assert_eq!(token.span, span);
        assert_eq!(token.lexeme, lexeme);
    }

    #[test]
    fn test_token_type_display() {
        assert_eq!(format!("{}", TokenType::Let), "keyword 'let'");
        assert_eq!(
            format!("{}", TokenType::IntegerLiteral(42)),
            "integer literal '42'"
        );
        assert_eq!(format!("{}", TokenType::Plus), "operator '+'");
        assert_eq!(format!("{}", TokenType::LeftParen), "'('");
    }
}
