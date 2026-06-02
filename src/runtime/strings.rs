use crate::runtime::errors::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{OpalString, RuntimeAllocator};
use core::cmp::Ordering;

/// Concatenate two runtime strings into a newly allocated runtime string.
///
/// # Errors
///
/// Returns allocator-provided runtime errors when string allocation fails.
pub fn string_concat<Allocator>(
    allocator: &Allocator,
    left: &OpalString,
    right: &OpalString,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    let mut combined = left.as_str().to_owned();
    combined.push_str(right.as_str());
    allocator.allocate_string(&combined)
}

/// Return length as count of Unicode scalar values.
#[must_use]
pub fn string_length(value: &OpalString) -> usize {
    value.as_str().chars().count()
}

/// Read a single Unicode scalar by zero-based scalar index.
///
/// # Errors
///
/// Returns [`RuntimeError::IndexOutOfBounds`] when `index` is invalid.
pub fn string_index<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
    index: usize,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    let length = string_length(value);
    let Some(scalar) = value.as_str().chars().nth(index) else {
        return Err(RuntimeError::IndexOutOfBounds { index, length });
    };
    let mut encoded = [0_u8; 4];
    allocator.allocate_string(scalar.encode_utf8(&mut encoded))
}

/// Compare two runtime strings lexicographically.
#[must_use]
pub fn string_compare(left: &OpalString, right: &OpalString) -> Ordering {
    left.as_str().cmp(right.as_str())
}

/// Return true when both runtime strings are equal.
#[must_use]
pub fn string_equals(left: &OpalString, right: &OpalString) -> bool {
    string_compare(left, right) == Ordering::Equal
}
