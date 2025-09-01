//! Token definitions and utilities for the Opalescent programming language
//!
//! This module defines the token types, positions, and spans used by the lexer
//! and parser.

#![expect(dead_code, reason = "Token types are being developed incrementally")]

use core::fmt;

/// Represents a position in the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    /// 1-based line number in the source file
    pub line: usize,
    /// 1-based column number within the current line
    pub column: usize,
    /// Byte offset from the start of the source
    pub offset: usize,
}

impl Position {
    /// Create a new Position
    pub const fn new(line: usize, column: usize, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    /// Starting position at the beginning of a source file
    pub const fn start() -> Self {
        Self::new(1, 1, 0)
    }
}

/// Represents a span in the source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Start position (inclusive)
    pub start: Position,
    /// End position (exclusive)
    pub end: Position,
}

impl Span {
    /// Create a new Span from start to end
    pub const fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    /// Create a zero-length span at `pos`
    pub const fn single(pos: Position) -> Self {
        Self::new(pos, pos)
    }
}

/// The different types of tokens in Opalescent
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    /// Integer number literal (e.g., 42, -17, 0)
    IntegerLiteral(i64),
    /// Floating-point number literal (e.g., 3.14, -2.5, 1.0)
    FloatLiteral(f64),
    /// String literal enclosed in quotes (e.g., "hello", "world")
    StringLiteral(String),
    /// Boolean literal (true or false)
    BooleanLiteral(bool),

    // Identifiers and Keywords
    /// User-defined identifier (e.g., variable names, function names)
    Identifier(String),

    // Keywords
    /// The 'let' keyword for variable declarations
    Let,
    /// The 'mutable' keyword for mutable variable declarations
    Mutable,
    /// The 'f' keyword for function declarations
    Function, // f
    /// The 'return' keyword for returning values from functions
    Return,
    /// The 'void' type keyword indicating no return value
    Void,
    /// The 'if' keyword for conditional statements
    If,
    /// The 'else' keyword for alternative branches in conditionals
    Else,
    /// The 'for' keyword for for-loop statements
    For,
    /// The 'while' keyword for while-loop statements
    While,
    /// The 'loop' keyword for infinite loop statements
    Loop,
    /// The 'in' keyword used in for-loops and membership tests
    In,
    /// The 'break' keyword to exit loops early
    Break,
    /// The 'continue' keyword to skip to the next loop iteration
    Continue,

    // Type keywords
    /// 8-bit signed integer type
    Int8,
    /// 16-bit signed integer type
    Int16,
    /// 32-bit signed integer type
    Int32,
    /// 64-bit signed integer type
    Int64,
    /// 8-bit unsigned integer type
    UInt8,
    /// 16-bit unsigned integer type
    UInt16,
    /// 32-bit unsigned integer type
    UInt32,
    /// 64-bit unsigned integer type
    UInt64,
    /// 32-bit floating-point type
    Float32,
    /// 64-bit floating-point type
    Float64,
    /// String type keyword
    String,
    /// Boolean type keyword
    Boolean,

    // Visibility
    /// The 'public' keyword for public visibility
    Public,
    /// The 'entry' keyword marking program entry points
    Entry,

    // Import/Export
    /// The 'import' keyword for importing modules
    Import,
    /// The 'from' keyword used in import statements
    From,
    /// The 'as' keyword for aliasing in imports and casts
    As,
    /// The 'type' keyword for type declarations
    Type,

    // Operators
    /// Addition operator (+)
    Plus,
    /// Subtraction operator (-)
    Minus,
    /// Multiplication operator (*)
    Multiply,
    /// Division operator (/)
    Divide,
    /// Exponentiation operator (^)
    Power, // ^
    /// Modulo operator (%)
    Modulo, // %

    // Assignment
    /// Assignment operator (=)
    Assign, // =

    // Comparison
    /// Less-than comparison operator (<)
    Less,
    /// Less-than-or-equal comparison operator (<=)
    LessEqual,
    /// Greater-than comparison operator (>)
    Greater,
    /// Greater-than-or-equal comparison operator (>=)
    GreaterEqual,
    /// Equality comparison operator (is)
    Is,
    /// Inequality comparison operator (is not)
    IsNot,

    // Logical
    /// Logical AND operator (and)
    And,
    /// Logical OR operator (or)
    Or,
    /// Logical NOT operator (not)
    Not,
    /// Logical XOR operator (xor)
    Xor,

    // Bitwise
    /// Bitwise AND operator (band)
    BitAnd, // band
    /// Bitwise OR operator (bor)
    BitOr, // bor
    /// Bitwise XOR operator (bxor)
    BitXor, // bxor
    /// Bitwise NOT operator (bnot)
    BitNot, // bnot
    /// Bitwise left shift operator (bshl)
    BitShiftLeft, // bshl
    /// Bitwise right shift operator (bshr)
    BitShiftRight, // bshr
    /// Bitwise unsigned right shift operator (bushr)
    BitUnsignedShiftRight, // bushr

    // Cast
    /// Type casting operator (as)
    Cast, // as

    // Type checking
    /// Type inspection function (`type_of`)
    TypeOf,

    // Punctuation
    /// Left parenthesis '('
    LeftParen,
    /// Right parenthesis ')'
    RightParen,
    /// Left square bracket '['
    LeftBracket,
    /// Right square bracket ']'
    RightBracket,
    /// Left curly brace '{'
    LeftBrace,
    /// Right curly brace '}'
    RightBrace,
    /// Colon punctuation ':'
    Colon,
    /// Comma punctuation ','
    Comma,
    /// Arrow punctuation '=>' used in function definitions
    Arrow, // =>
    /// Dot punctuation '.' for member access
    Dot,

    // Comments
    /// Single-line or multi-line comment
    Comment(String),
    /// Documentation comment for generating docs
    DocComment(String),

    // Special
    /// Newline character marking end of line
    Newline,
    /// Indentation increase token
    Indent,
    /// Indentation decrease token
    Dedent,
    /// End of file marker
    EndOfFile,
}

impl TokenType {
    /// Returns the raw type name for type tokens, used in parsing
    pub const fn type_name(&self) -> Option<&'static str> {
        match *self {
            Self::Int8 => Some("int8"),
            Self::Int16 => Some("int16"),
            Self::Int32 => Some("int32"),
            Self::Int64 => Some("int64"),
            Self::UInt8 => Some("uint8"),
            Self::UInt16 => Some("uint16"),
            Self::UInt32 => Some("uint32"),
            Self::UInt64 => Some("uint64"),
            Self::Float32 => Some("float32"),
            Self::Float64 => Some("float64"),
            Self::String => Some("string"),
            Self::Boolean => Some("boolean"),
            Self::Void => Some("void"),
            _ => None,
        }
    }
}

/// A token with its type, value, and location information
#[derive(Debug, Clone, PartialEq)]
#[expect(
    clippy::struct_field_names,
    reason = "Field names follow domain naming convention"
)]
pub struct Token {
    /// The semantic token type
    pub token_type: TokenType,
    /// Source span covered by this token
    pub span: Span,
    /// Raw lexeme text
    pub lexeme: String,
}

impl Token {
    /// Create a new token
    pub const fn new(token_type: TokenType, span: Span, lexeme: String) -> Self {
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
            Self::IntegerLiteral(n) => write!(f, "integer literal '{n}'"),
            Self::FloatLiteral(n) => write!(f, "float literal '{n}'"),
            Self::StringLiteral(s) => write!(f, "string literal '{s}'"),
            Self::BooleanLiteral(b) => write!(f, "boolean literal '{b}'"),
            Self::Identifier(name) => write!(f, "identifier '{name}'"),
            Self::Let => write!(f, "keyword 'let'"),
            Self::Mutable => write!(f, "keyword 'mutable'"),
            Self::Function => write!(f, "keyword 'f'"),
            Self::Return => write!(f, "keyword 'return'"),
            Self::Void => write!(f, "keyword 'void'"),
            Self::If => write!(f, "keyword 'if'"),
            Self::Else => write!(f, "keyword 'else'"),
            Self::For => write!(f, "keyword 'for'"),
            Self::While => write!(f, "keyword 'while'"),
            Self::Loop => write!(f, "keyword 'loop'"),
            Self::In => write!(f, "keyword 'in'"),
            Self::Break => write!(f, "keyword 'break'"),
            Self::Continue => write!(f, "keyword 'continue'"),
            Self::Int8 => write!(f, "type 'int8'"),
            Self::Int16 => write!(f, "type 'int16'"),
            Self::Int32 => write!(f, "type 'int32'"),
            Self::Int64 => write!(f, "type 'int64'"),
            Self::UInt8 => write!(f, "type 'uint8'"),
            Self::UInt16 => write!(f, "type 'uint16'"),
            Self::UInt32 => write!(f, "type 'uint32'"),
            Self::UInt64 => write!(f, "type 'uint64'"),
            Self::Float32 => write!(f, "type 'float32'"),
            Self::Float64 => write!(f, "type 'float64'"),
            Self::String => write!(f, "type 'string'"),
            Self::Boolean => write!(f, "type 'boolean'"),
            Self::Public => write!(f, "keyword 'public'"),
            Self::Entry => write!(f, "keyword 'entry'"),
            Self::Import => write!(f, "keyword 'import'"),
            Self::From => write!(f, "keyword 'from'"),
            Self::As => write!(f, "keyword 'as'"),
            Self::Type => write!(f, "keyword 'type'"),
            Self::Plus => write!(f, "operator '+'"),
            Self::Minus => write!(f, "operator '-'"),
            Self::Multiply => write!(f, "operator '*'"),
            Self::Divide => write!(f, "operator '/'"),
            Self::Power => write!(f, "operator '^'"),
            Self::Modulo => write!(f, "operator '%'"),
            Self::Assign => write!(f, "operator '='"),
            Self::Less => write!(f, "operator '<'"),
            Self::LessEqual => write!(f, "operator '<='"),
            Self::Greater => write!(f, "operator '>'"),
            Self::GreaterEqual => write!(f, "operator '>='"),
            Self::Is => write!(f, "operator 'is'"),
            Self::IsNot => write!(f, "operator 'is not'"),
            Self::And => write!(f, "operator 'and'"),
            Self::Or => write!(f, "operator 'or'"),
            Self::Not => write!(f, "operator 'not'"),
            Self::Xor => write!(f, "operator 'xor'"),
            Self::BitAnd => write!(f, "operator 'band'"),
            Self::BitOr => write!(f, "operator 'bor'"),
            Self::BitXor => write!(f, "operator 'bxor'"),
            Self::BitNot => write!(f, "operator 'bnot'"),
            Self::BitShiftLeft => write!(f, "operator 'bshl'"),
            Self::BitShiftRight => write!(f, "operator 'bshr'"),
            Self::BitUnsignedShiftRight => write!(f, "operator 'bushr'"),
            Self::Cast => write!(f, "operator 'as'"),
            Self::TypeOf => write!(f, "function 'type_of'"),
            Self::LeftParen => write!(f, "'('"),
            Self::RightParen => write!(f, "')'"),
            Self::LeftBracket => write!(f, "'['"),
            Self::RightBracket => write!(f, "']'"),
            Self::LeftBrace => write!(f, "'{{'"),
            Self::RightBrace => write!(f, "'}}'"),
            Self::Colon => write!(f, "':'"),
            Self::Comma => write!(f, "','"),
            Self::Arrow => write!(f, "'=>'"),
            Self::Dot => write!(f, "'.'"),
            Self::Comment(_) => write!(f, "comment"),
            Self::DocComment(_) => write!(f, "documentation comment"),
            Self::Newline => write!(f, "newline"),
            Self::Indent => write!(f, "indent"),
            Self::Dedent => write!(f, "dedent"),
            Self::EndOfFile => write!(f, "end of file"),
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
        let lexeme = "42".to_owned();

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
