//! Assertion helpers for Opalescent-language tests.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::fmt::Debug;

/// Assertion failure value returned by assertion helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssertionError {
    /// Human-readable assertion failure message.
    pub message: String,
}

impl AssertionError {
    /// Create a new assertion error with message text.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Assert that `value` is `true`.
///
/// # Errors
///
/// Returns [`AssertionError`] when `value` is `false`.
pub fn assert_true(value: bool) -> Result<(), AssertionError> {
    if value {
        return Ok(());
    }
    Err(AssertionError::new("expected condition to be true"))
}

/// Assert that `value` is `false`.
///
/// # Errors
///
/// Returns [`AssertionError`] when `value` is `true`.
pub fn assert_false(value: bool) -> Result<(), AssertionError> {
    if !value {
        return Ok(());
    }
    Err(AssertionError::new("expected condition to be false"))
}

/// Assert that `left` equals `right`.
///
/// # Errors
///
/// Returns [`AssertionError`] when values differ.
pub fn assert_eq<T>(left: &T, right: &T) -> Result<(), AssertionError>
where
    T: PartialEq + Debug,
{
    if left == right {
        return Ok(());
    }
    Err(AssertionError::new(format!(
        "expected values to be equal: left={left:?} right={right:?}"
    )))
}

/// Assert that `left` does not equal `right`.
///
/// # Errors
///
/// Returns [`AssertionError`] when values are equal.
pub fn assert_ne<T>(left: &T, right: &T) -> Result<(), AssertionError>
where
    T: PartialEq + Debug,
{
    if left != right {
        return Ok(());
    }
    Err(AssertionError::new(format!(
        "expected values to differ: left={left:?} right={right:?}"
    )))
}

/// Assert that `operation` returns an error.
///
/// # Errors
///
/// Returns [`AssertionError`] when `operation` returns `Ok`.
pub fn assert_throws<T, ErrorType>(
    operation: impl FnOnce() -> Result<T, ErrorType>,
) -> Result<(), AssertionError> {
    match operation() {
        Ok(_value) => Err(AssertionError::new("expected operation to return an error")),
        Err(_error) => Ok(()),
    }
}
