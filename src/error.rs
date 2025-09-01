//! Error types and utilities for the Opalescent compiler
//!
//! This module defines error types for lexical analysis and parsing,
//! using the miette crate for rich error reporting.

#![expect(dead_code, reason = "Error types are being developed incrementally")]

use crate::token::{Position, Span};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Errors that can occur during lexical analysis
#[derive(Error, Debug, Diagnostic)]
pub enum LexError {
    /// Encountered a character that cannot be tokenized in the current context
    #[error("Unexpected character '{character}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::unexpected_character),
        help("Remove or replace this character with a valid token")
    )]
    UnexpectedCharacter {
        /// The character that could not be tokenized
        character: char,
        /// The position in the source where the character was found
        position: Position,
        #[label("unexpected character here")]
        /// Source span highlighting the unexpected character location
        span: SourceSpan,
    },

    /// String literal was started but never closed with a matching quote
    #[error("Unterminated string literal starting at position {start:?}")]
    #[diagnostic(
        code(opalescent::lexer::unterminated_string),
        help("Add a closing quote to terminate the string literal")
    )]
    UnterminatedString {
        /// The position where the unterminated string literal started
        start: Position,
        #[label("string starts here")]
        /// Source span highlighting where the string literal begins
        span: SourceSpan,
    },

    /// Invalid escape sequence found within a string literal
    #[error("Invalid escape sequence '\\{sequence}' in string at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_escape),
        help("Use a valid escape sequence like \\n, \\t, \\\\, or \\\"")
    )]
    InvalidEscapeSequence {
        /// The invalid escape sequence that was found
        sequence: String,
        /// The position in the source where the invalid escape sequence was found
        position: Position,
        #[label("invalid escape sequence")]
        /// Source span highlighting the invalid escape sequence location
        span: SourceSpan,
    },

    /// Both spaces and tabs are used for indentation in the same file
    #[error("Mixed whitespace detected: both spaces and tabs are used in this file")]
    #[diagnostic(
        code(opalescent::lexer::mixed_whitespace),
        help("Use either spaces OR tabs consistently throughout the file, not both")
    )]
    MixedWhitespace {
        #[label("first tab found here")]
        /// Source span highlighting the location of the first tab character
        tab_span: SourceSpan,
        #[label("first space found here")]
        /// Source span highlighting the location of the first space character
        space_span: SourceSpan,
    },

    /// Number literal has invalid format or cannot be parsed
    #[error("Invalid number format '{number}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_number),
        help("Check the number format - it may be too large or have invalid characters")
    )]
    InvalidNumber {
        /// The string representation of the invalid number literal
        number: String,
        /// The position in the source where the invalid number was found
        position: Position,
        #[label("invalid number")]
        /// Source span highlighting the invalid number location
        span: SourceSpan,
    },

    /// Multi-line comment was started but never closed
    #[error("Unterminated multi-line comment starting at position {start:?}")]
    #[diagnostic(
        code(opalescent::lexer::unterminated_comment),
        help("Add '##' to close the multi-line comment")
    )]
    UnterminatedComment {
        /// The position where the unterminated comment started
        start: Position,
        #[label("comment starts here")]
        /// Source span highlighting where the comment begins
        span: SourceSpan,
    },

    /// Identifier does not follow the required `snake_case` naming convention
    #[error("Invalid identifier '{identifier}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_identifier),
        help("Identifiers must be in snake_case and start with a letter or underscore")
    )]
    InvalidIdentifier {
        /// The invalid identifier that does not follow snake_case convention
        identifier: String,
        /// The position in the source where the invalid identifier was found
        position: Position,
        #[label("invalid identifier")]
        /// Source span highlighting the invalid identifier location
        span: SourceSpan,
    },

    /// Type identifier does not follow the required PascalCase naming convention
    #[error("Invalid type identifier '{identifier}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_type_identifier),
        help("Type identifiers must be in PascalCase and start with a capital letter")
    )]
    InvalidTypeIdentifier {
        /// The invalid type identifier that does not follow PascalCase convention
        identifier: String,
        /// The position in the source where the invalid type identifier was found
        position: Position,
        #[label("invalid type identifier")]
        /// Source span highlighting the invalid type identifier location
        span: SourceSpan,
    },
}

impl LexError {
    /// Create a SourceSpan from a Position and length
    ///
    /// # Arguments
    /// * `pos` - The starting position in the source text
    /// * `len` - The length of the span in bytes
    ///
    /// # Returns
    /// A SourceSpan that can be used for error reporting
    pub fn span_from_position(pos: Position, len: usize) -> SourceSpan {
        SourceSpan::new(pos.offset.into(), len)
    }

    /// Convert a parser Span to a miette SourceSpan
    ///
    /// # Arguments
    /// * `span` - The span from the parser containing start and end positions
    ///
    /// # Returns
    /// A SourceSpan that can be used for error reporting with miette
    pub fn span_from_span(span: Span) -> SourceSpan {
        let start = span.start.offset;
        let end = span.end.offset;
        let len = if end >= start { end - start + 1 } else { 1 };
        SourceSpan::new(start.into(), len)
    }
}

/// Result type for lexer operations
pub type LexResult<T> = Result<T, LexError>;

/// Collection of lexer errors for multiple error reporting
#[derive(Debug)]
pub struct LexErrors {
    /// Collection of lexical analysis errors
    pub errors: Vec<LexError>,
}

impl LexErrors {
    /// Create a new empty collection of lexical errors
    ///
    /// # Returns
    /// A new LexErrors instance with an empty error vector
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Add a lexical error to the collection
    pub fn push(&mut self, error: LexError) {
        self.errors.push(error);
    }

    /// Check if there are no errors in the collection
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors in the collection
    pub fn len(&self) -> usize {
        self.errors.len()
    }
}

impl Default for LexErrors {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_errors_creation() {
        let mut errors = LexErrors::new();
        assert!(errors.is_empty());
        assert_eq!(errors.len(), 0);

        let error = LexError::UnexpectedCharacter {
            character: '@',
            position: Position::start(),
            span: SourceSpan::new(0.into(), 1),
        };
        
        errors.push(error);
        assert!(!errors.is_empty());
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_span_conversion() {
        let pos = Position::new(1, 5, 10);
        let span = LexError::span_from_position(pos, 3);
        assert_eq!(span.offset(), 10);
        assert_eq!(span.len(), 3);
    }

    #[test]
    fn test_span_from_span() {
        let start = Position::new(1, 1, 5);
        let end = Position::new(1, 8, 12);
        let span = Span::new(start, end);
        
        let source_span = LexError::span_from_span(span);
        assert_eq!(source_span.offset(), 5);
        assert_eq!(source_span.len(), 8); // 12 - 5 + 1
    }
}
