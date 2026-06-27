#![allow(
    clippy::arithmetic_side_effects,
    clippy::map_err_ignore,
    clippy::string_slice,
    reason = "string helpers slice only on Unicode scalar boundaries and map runtime ABI errors intentionally"
)]

extern crate alloc;

use crate::runtime::errors::{RuntimeError, RuntimeResult};
use crate::runtime::memory::{OpalArray, OpalString, RuntimeAllocator};
use core::cmp::Ordering;

/// Match the Unicode `White_Space` property without depending on `std` helpers.
#[must_use]
const fn is_unicode_white_space(value: char) -> bool {
    matches!(
        value,
        '\u{0009}'
            | '\u{000A}'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{000D}'
            | '\u{0020}'
            | '\u{0085}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'
            | '\u{2001}'
            | '\u{2002}'
            | '\u{2003}'
            | '\u{2004}'
            | '\u{2005}'
            | '\u{2006}'
            | '\u{2007}'
            | '\u{2008}'
            | '\u{2009}'
            | '\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    )
}

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

/// Return the first Unicode-scalar index of `search_text` or `fallback_index`.
#[must_use]
pub fn string_find_index_or(
    value: &OpalString,
    search_text: &OpalString,
    fallback_index: i64,
) -> i64 {
    if search_text.as_str().is_empty() {
        return fallback_index;
    }

    value
        .as_str()
        .find(search_text.as_str())
        .map_or(fallback_index, |byte_offset| {
            i64::try_from(value.as_str()[..byte_offset].chars().count())
                .unwrap_or(fallback_index)
        })
}

/// Return the last Unicode-scalar index of `search_text`.
///
/// # Errors
///
/// Returns a user error carrying `StringEmptySearchTextError` or
/// `StringPatternNotFoundError` when the contract fails.
pub fn string_find_last_index_of_text(
    value: &OpalString,
    search_text: &OpalString,
) -> RuntimeResult<i64> {
    if search_text.as_str().is_empty() {
        return Err(RuntimeError::user_error(
            1_101,
            "StringEmptySearchTextError",
        ));
    }

    value
        .as_str()
        .rfind(search_text.as_str())
        .map(|byte_offset| {
            i64::try_from(value.as_str()[..byte_offset].chars().count()).unwrap_or(i64::MAX)
        })
        .ok_or_else(|| RuntimeError::user_error(1_102, "StringPatternNotFoundError"))
}

/// Split runtime strings into logical lines using LF/CRLF/CR semantics.
pub fn string_split_lines<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
) -> RuntimeResult<OpalArray<OpalString>>
where
    Allocator: RuntimeAllocator,
{
    let source = value.as_str();
    if source.is_empty() {
        return allocator.allocate_array::<OpalString>(&[]);
    }

    let bytes = source.as_bytes();
    let mut lines = alloc::vec::Vec::new();
    let mut start = 0_usize;
    let mut index = 0_usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(allocator.allocate_string(&source[start..index])?);
                index += 1;
                start = index;
            }
            b'\r' => {
                lines.push(allocator.allocate_string(&source[start..index])?);
                index += 1;
                if index < bytes.len() && bytes[index] == b'\n' {
                    index += 1;
                }
                start = index;
            }
            _ => index += 1,
        }
    }

    if start < bytes.len() {
        lines.push(allocator.allocate_string(&source[start..])?);
    }

    allocator.allocate_array(lines.as_slice())
}

/// Return true when the runtime string is empty or contains only Unicode `White_Space`.
#[must_use]
pub fn string_is_blank(value: &OpalString) -> bool {
    value.as_str().chars().all(is_unicode_white_space)
}

/// Trim leading and trailing Unicode `White_Space` from a runtime string.
pub fn string_trim_whitespace<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    let source = value.as_str();
    let mut start = None;
    let mut end = 0_usize;

    for (index, scalar) in source.char_indices() {
        if !is_unicode_white_space(scalar) {
            if start.is_none() {
                start = Some(index);
            }
            end = index + scalar.len_utf8();
        }
    }

    start.map_or_else(
        || allocator.allocate_string(""),
        |start_index| allocator.allocate_string(&source[start_index..end]),
    )
}

/// Return the first `count` Unicode scalar values from a runtime string.
pub fn string_take_prefix<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
    count: i64,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    if count < 0 {
        return Err(RuntimeError::user_error(1_103, "StringNegativeCountError"));
    }
    let count = usize::try_from(count)
        .map_err(|_| RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"))?;
    let source = value.as_str();
    let char_count = source.chars().count();
    if count > char_count {
        return Err(RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"));
    }
    let result: alloc::string::String = source.chars().take(count).collect();
    allocator.allocate_string(&result)
}

/// Return the last `count` Unicode scalar values from a runtime string.
pub fn string_take_suffix<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
    count: i64,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    if count < 0 {
        return Err(RuntimeError::user_error(1_103, "StringNegativeCountError"));
    }
    let count = usize::try_from(count)
        .map_err(|_| RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"))?;
    let source = value.as_str();
    let char_count = source.chars().count();
    if count > char_count {
        return Err(RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"));
    }
    let result: alloc::string::String = source
        .chars()
        .skip(char_count.saturating_sub(count))
        .collect();
    allocator.allocate_string(&result)
}

/// Return the Unicode scalar range `[start, end)` from a runtime string.
pub fn string_extract_range<Allocator>(
    allocator: &Allocator,
    value: &OpalString,
    start: i64,
    end: i64,
) -> RuntimeResult<OpalString>
where
    Allocator: RuntimeAllocator,
{
    if start < 0 || end < 0 {
        return Err(RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"));
    }
    if end < start {
        return Err(RuntimeError::user_error(1_105, "StringRangeOrderError"));
    }
    let start = usize::try_from(start)
        .map_err(|_| RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"))?;
    let end = usize::try_from(end)
        .map_err(|_| RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"))?;
    let source = value.as_str();
    let char_count = source.chars().count();
    if end > char_count {
        return Err(RuntimeError::user_error(1_104, "StringRangeOutOfBoundsError"));
    }
    let result: alloc::string::String = source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();
    allocator.allocate_string(&result)
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
