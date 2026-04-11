//! Error types for the Opalescent code formatter.
//!
//! All public formatter operations return [`FormatterResult`] which is an alias
//! for `Result<T, FormatterError>`.

extern crate alloc;

use alloc::string::String;
use core::fmt;

/// Convenience alias for a `Result` carrying a [`FormatterError`].
pub type FormatterResult<Type> = Result<Type, FormatterError>;

/// Errors that can arise during formatter operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormatterError {
    /// The source code could not be parsed.
    ///
    /// Contains a human-readable description of the parse failure.
    ParseError(String),

    /// The provided [`FormatterConfig`] contains an invalid value.
    ///
    /// Contains a human-readable description of what is wrong.
    InvalidConfig(String),
}

impl fmt::Display for FormatterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::ParseError(ref msg) => write!(f, "parse error: {msg}"),
            Self::InvalidConfig(ref msg) => write!(f, "invalid config: {msg}"),
        }
    }
}
