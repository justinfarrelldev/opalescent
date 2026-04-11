extern crate alloc;

use alloc::format;
use alloc::string::String;
use miette::Diagnostic;
use thiserror::Error;

/// Runtime operation result type.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Runtime-level error variants shared by generated programs and host helpers.
#[derive(Debug, Clone, PartialEq, Eq, Error, Diagnostic)]
pub enum RuntimeError {
    /// Array index is outside valid bounds.
    #[error("index {index} is out of bounds for length {length}")]
    #[diagnostic(
        code(opalescent::runtime::index_out_of_bounds),
        help("Ensure the index is less than the array length before indexing")
    )]
    IndexOutOfBounds {
        /// Attempted index.
        index: usize,
        /// Array length used for bounds checking.
        length: usize,
    },

    /// Division attempted with a zero divisor.
    #[error("division by zero")]
    #[diagnostic(
        code(opalescent::runtime::division_by_zero),
        help("Ensure divisor is non-zero before division")
    )]
    DivisionByZero,

    /// Runtime stack usage exceeded supported depth.
    #[error("stack overflow")]
    #[diagnostic(
        code(opalescent::runtime::stack_overflow),
        help("Reduce recursion depth or increase stack size")
    )]
    StackOverflow,

    /// User-defined runtime error carrying code and message.
    #[error("runtime error ({code}): {message}")]
    #[diagnostic(
        code(opalescent::runtime::user_error),
        help("Inspect runtime error code and message for details")
    )]
    UserError {
        /// Stable error code.
        code: i64,
        /// Human-readable description.
        message: String,
    },
}

impl RuntimeError {
    /// Return stable numeric error code for this variant.
    #[must_use]
    pub const fn error_code(&self) -> i64 {
        match *self {
            Self::IndexOutOfBounds { .. } => 1_001,
            Self::DivisionByZero => 1_002,
            Self::StackOverflow => 1_003,
            Self::UserError { code, .. } => code,
        }
    }

    /// Render stable error message text for this variant.
    #[must_use]
    pub fn message(&self) -> String {
        match *self {
            Self::IndexOutOfBounds { index, length } => {
                format!("index {index} is out of bounds for length {length}")
            }
            Self::DivisionByZero => String::from("division by zero"),
            Self::StackOverflow => String::from("stack overflow"),
            Self::UserError { ref message, .. } => message.clone(),
        }
    }

    /// Construct user-defined runtime error variant.
    #[must_use]
    pub fn user_error(code: i64, message: impl Into<String>) -> Self {
        Self::UserError {
            code,
            message: message.into(),
        }
    }
}

/// Extension trait for mapping foreign errors into `RuntimeError::UserError`.
pub trait RuntimeResultExt<T> {
    /// Convert an arbitrary error into `RuntimeError::UserError` with context.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::UserError`] when the input result is `Err`.
    fn into_runtime_error(self, code: i64, context: &str) -> RuntimeResult<T>;
}

impl<T, ErrorType> RuntimeResultExt<T> for Result<T, ErrorType>
where
    ErrorType: core::fmt::Display,
{
    fn into_runtime_error(self, code: i64, context: &str) -> RuntimeResult<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => Err(RuntimeError::UserError {
                code,
                message: format!("{context}: {error}"),
            }),
        }
    }
}
