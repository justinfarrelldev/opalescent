//! String operations for the Opalescent standard library.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::map_err_ignore,
    clippy::string_slice,
    reason = "string helpers slice only on Unicode scalar boundaries and preserve explicit range errors"
)]
//!
//! This module provides the language-level string API. The lower-level `OpalString`
//! runtime primitives live in `crate::runtime::strings`. These higher-level helpers
//! operate on bare `&str` and `String` values (the public language API surface) while
//! keeping all allocations inside `alloc` for `no_std` compatibility.
//!
//! # Operations
//!
//! - [`concat`] — join two string slices
//! - [`length`] — Unicode scalar count (not byte count)
//! - [`find`] — byte-position of first substring match, returned as char offset
//! - [`find_index_or`] — first substring match or caller-provided fallback
//! - [`find_last_index_of_text`] — last substring match as a char offset
//! - [`split_lines`] — logical line splitting on LF/CRLF/CR
//! - [`is_blank`] — empty or Unicode `White_Space`-only strings
//! - [`trim_whitespace`] — strip leading/trailing Unicode `White_Space`
//! - [`replace`] — substitute all occurrences of a pattern
//! - [`split`] — divide by a delimiter
//! - [`trim`] — strip leading/trailing ASCII whitespace
//! - [`to_upper`] / [`to_lower`] — case conversion
//! - [`slice`] — Unicode-aware substring by char indices

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringRangeError {
    NegativeCount,
    OutOfBounds,
    RangeOrder,
}

/// Match the Unicode `White_Space` property without depending on locale-sensitive helpers.
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

/// Concatenate two string slices into a new owned `String`.
#[must_use]
pub fn concat(left: &str, right: &str) -> String {
    let mut result = String::with_capacity(left.len().saturating_add(right.len()));
    result.push_str(left);
    result.push_str(right);
    result
}

/// Return the number of Unicode scalar values in `value`.
///
/// This differs from `str::len()` which counts UTF-8 bytes.
#[must_use]
pub fn length(value: &str) -> usize {
    value.chars().count()
}

/// Return the char-index of the first occurrence of `needle` in `haystack`.
///
/// Returns `None` when `needle` is not found.
/// Returns `Some(0)` when `needle` is empty (matches at start).
#[must_use]
pub fn find(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0_usize);
    }

    haystack.find(needle).map(|byte_offset| {
        haystack
            .get(..byte_offset)
            .map_or(0_usize, |s| s.chars().count())
    })
}

/// Return the char-index of the first occurrence of `needle` or `fallback_index`.
#[must_use]
pub fn find_index_or(haystack: &str, needle: &str, fallback_index: usize) -> usize {
    if needle.is_empty() {
        return fallback_index;
    }

    find(haystack, needle).unwrap_or(fallback_index)
}

/// Return the char-index of the last occurrence of `needle`.
///
/// Returns `None` when `needle` is empty or not found.
#[must_use]
pub fn find_last_index_of_text(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return None;
    }

    haystack.rfind(needle).map(|byte_offset| {
        haystack
            .get(..byte_offset)
            .map_or(0_usize, |s| s.chars().count())
    })
}

/// Split `source` into logical lines using `\n`, `\r\n`, and `\r`.
#[must_use]
pub fn split_lines(source: &str) -> Vec<String> {
    if source.is_empty() {
        return Vec::new();
    }

    let bytes = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0_usize;
    let mut index = 0_usize;

    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(String::from(&source[start..index]));
                index += 1;
                start = index;
            }
            b'\r' => {
                lines.push(String::from(&source[start..index]));
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
        lines.push(String::from(&source[start..]));
    }

    lines
}

/// Return true when `source` is empty or contains only Unicode `White_Space`.
#[must_use]
pub fn is_blank(source: &str) -> bool {
    source.chars().all(is_unicode_white_space)
}

/// Trim leading and trailing Unicode `White_Space`.
#[must_use]
pub fn trim_whitespace(source: &str) -> String {
    let mut start = None;
    let mut end = 0_usize;

    for (index, value) in source.char_indices() {
        if !is_unicode_white_space(value) {
            if start.is_none() {
                start = Some(index);
            }
            end = index + value.len_utf8();
        }
    }

    start.map_or_else(String::new, |start_index| String::from(&source[start_index..end]))
}

/// Return the first `count` Unicode scalar values.
pub fn take_prefix(source: &str, count: i64) -> Result<String, StringRangeError> {
    if count < 0 {
        return Err(StringRangeError::NegativeCount);
    }
    let count = usize::try_from(count).map_err(|_| StringRangeError::OutOfBounds)?;
    let char_count = source.chars().count();
    if count > char_count {
        return Err(StringRangeError::OutOfBounds);
    }
    Ok(source.chars().take(count).collect())
}

/// Return the last `count` Unicode scalar values.
pub fn take_suffix(source: &str, count: i64) -> Result<String, StringRangeError> {
    if count < 0 {
        return Err(StringRangeError::NegativeCount);
    }
    let count = usize::try_from(count).map_err(|_| StringRangeError::OutOfBounds)?;
    let char_count = source.chars().count();
    if count > char_count {
        return Err(StringRangeError::OutOfBounds);
    }
    Ok(source
        .chars()
        .skip(char_count.saturating_sub(count))
        .collect())
}

/// Return the Unicode scalar range `[start, end)`.
pub fn extract_range(source: &str, start: i64, end: i64) -> Result<String, StringRangeError> {
    if start < 0 || end < 0 {
        return Err(StringRangeError::OutOfBounds);
    }
    if end < start {
        return Err(StringRangeError::RangeOrder);
    }
    let start = usize::try_from(start).map_err(|_| StringRangeError::OutOfBounds)?;
    let end = usize::try_from(end).map_err(|_| StringRangeError::OutOfBounds)?;
    let char_count = source.chars().count();
    if end > char_count {
        return Err(StringRangeError::OutOfBounds);
    }
    Ok(source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect())
}

/// Replace all occurrences of `from` in `source` with `to`.
///
/// Returns `source` unmodified when `from` is not found.
#[must_use]
pub fn replace(source: &str, from: &str, to: &str) -> String {
    source.replace(from, to)
}

/// Split `source` by `delimiter` and return each part as a `String`.
///
/// If `delimiter` is not present, a single-element vec containing the whole input is returned.
#[must_use]
pub fn split(source: &str, delimiter: &str) -> Vec<String> {
    source.split(delimiter).map(ToOwned::to_owned).collect()
}

/// Remove leading and trailing whitespace from `source`.
#[must_use]
pub fn trim(source: &str) -> String {
    source.trim().to_owned()
}

/// Convert `source` to uppercase using Unicode case mapping.
#[must_use]
pub fn to_upper(source: &str) -> String {
    source.to_uppercase()
}

/// Convert `source` to lowercase using Unicode case mapping.
#[must_use]
pub fn to_lower(source: &str) -> String {
    source.to_lowercase()
}

/// Return the substring of `source` covering char indices `[start, end)`.
///
/// Returns `None` when `start > end` or either index is out of range.
#[must_use]
pub fn slice(source: &str, start: usize, end: usize) -> Option<String> {
    if start > end {
        return None;
    }

    let char_count = source.chars().count();

    if end > char_count {
        return None;
    }

    let result: String = source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect();

    Some(result)
}
