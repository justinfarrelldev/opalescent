//! Parse error types and error collection utilities
//!
//! This module defines the error types that can occur during parsing,
//! including detailed diagnostic information for helpful error messages.

use crate::error::LexError;
use crate::token::Token;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    /// Found a token that doesn't match what was expected at this position
    #[error("Unexpected token: expected {expected}, found {found}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_token),
        help("Check the syntax around this location")
    )]
    UnexpectedToken {
        /// The token type that was expected at this position
        expected: String,
        /// The actual token that was found instead
        found: String,
        #[label("unexpected token")]
        /// Source span highlighting the unexpected token location
        span: SourceSpan,
    },

    /// Expected a specific token but it was not found
    #[error("Missing token: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::missing_token),
        help("Add the missing {expected}")
    )]
    MissingToken {
        /// The token type that was expected but not found
        expected: String,
        #[label("expected {expected} here")]
        /// Source span indicating where the missing token should be
        span: SourceSpan,
    },

    /// The syntax is invalid according to the language grammar
    #[error("Invalid syntax: {message}")]
    #[diagnostic(
        code(opalescent::parser::invalid_syntax),
        help("Check the language specification for correct syntax")
    )]
    InvalidSyntax {
        /// Description of what makes the syntax invalid
        message: String,
        #[label("invalid syntax")]
        /// Source span highlighting the location of invalid syntax
        span: SourceSpan,
    },

    /// Reached end of file while expecting more tokens
    #[error("Unexpected end of file: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_eof),
        help("Complete the {expected}")
    )]
    UnexpectedEof {
        /// The token or construct that was expected before EOF
        expected: String,
        #[label("file ends here")]
        /// Source span indicating the end of file location
        span: SourceSpan,
    },

    /// Duplicate label in a break or continue statement
    #[error("Duplicate label '{label}' in control flow statement")]
    #[diagnostic(
        code(opalescent::parser::duplicate_label),
        help("Each label must be unique within a single break or continue statement")
    )]
    DuplicateLabel {
        /// The duplicated label name
        label: String,
        #[label("duplicate label here")]
        /// Source span indicating where the duplicate label appears
        span: SourceSpan,
    },
}

impl ParseError {
    /// Creates a source span from a token's span information
    /// Used for error reporting to highlight the token location in source code
    pub fn span_from_token(token: &Token) -> SourceSpan {
        LexError::span_from_span(token.span)
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Collection of parse errors for multiple error reporting
#[derive(Debug)]
pub struct ParseErrors {
    /// Vector containing all parse errors encountered during parsing
    pub errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Creates a new empty collection of parse errors
    pub const fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add a parse error to the collection
    pub fn push(&mut self, error: ParseError) {
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

impl Default for ParseErrors {
    fn default() -> Self {
        Self::new()
    }
}
