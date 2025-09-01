use crate::token::{Position, Span};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Errors that can occur during lexical analysis
#[derive(Error, Debug, Diagnostic)]
pub enum LexError {
    #[error("Unexpected character '{character}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::unexpected_character),
        help("Remove or replace this character with a valid token")
    )]
    UnexpectedCharacter {
        character: char,
        position: Position,
        #[label("unexpected character here")]
        span: SourceSpan,
    },

    #[error("Unterminated string literal starting at position {start:?}")]
    #[diagnostic(
        code(opalescent::lexer::unterminated_string),
        help("Add a closing quote to terminate the string literal")
    )]
    UnterminatedString {
        start: Position,
        #[label("string starts here")]
        span: SourceSpan,
    },

    #[error("Invalid escape sequence '\\{sequence}' in string at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_escape),
        help("Use a valid escape sequence like \\n, \\t, \\\\, or \\\"")
    )]
    InvalidEscapeSequence {
        sequence: String,
        position: Position,
        #[label("invalid escape sequence")]
        span: SourceSpan,
    },

    #[error("Mixed whitespace detected: both spaces and tabs are used in this file")]
    #[diagnostic(
        code(opalescent::lexer::mixed_whitespace),
        help("Use either spaces OR tabs consistently throughout the file, not both")
    )]
    MixedWhitespace {
        #[label("first tab found here")]
        tab_span: SourceSpan,
        #[label("first space found here")]
        space_span: SourceSpan,
    },

    #[error("Invalid number format '{number}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_number),
        help("Check the number format - it may be too large or have invalid characters")
    )]
    InvalidNumber {
        number: String,
        position: Position,
        #[label("invalid number")]
        span: SourceSpan,
    },

    #[error("Unterminated multi-line comment starting at position {start:?}")]
    #[diagnostic(
        code(opalescent::lexer::unterminated_comment),
        help("Add '##' to close the multi-line comment")
    )]
    UnterminatedComment {
        start: Position,
        #[label("comment starts here")]
        span: SourceSpan,
    },

    #[error("Invalid identifier '{identifier}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_identifier),
        help("Identifiers must be in snake_case and start with a letter or underscore")
    )]
    InvalidIdentifier {
        identifier: String,
        position: Position,
        #[label("invalid identifier")]
        span: SourceSpan,
    },

    #[error("Invalid type identifier '{identifier}' at position {position:?}")]
    #[diagnostic(
        code(opalescent::lexer::invalid_type_identifier),
        help("Type identifiers must be in PascalCase and start with a capital letter")
    )]
    InvalidTypeIdentifier {
        identifier: String,
        position: Position,
        #[label("invalid type identifier")]
        span: SourceSpan,
    },
}

impl LexError {
    pub fn span_from_position(pos: Position, len: usize) -> SourceSpan {
        SourceSpan::new(pos.offset.into(), len)
    }

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
    pub errors: Vec<LexError>,
}

impl LexErrors {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    pub fn push(&mut self, error: LexError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

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
