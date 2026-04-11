//! String operations for the Opalescent standard library.
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
//! - [`replace`] — substitute all occurrences of a pattern
//! - [`split`] — divide by a delimiter
//! - [`trim`] — strip leading/trailing ASCII whitespace
//! - [`to_upper`] / [`to_lower`] — case conversion
//! - [`slice`] — Unicode-aware substring by char indices

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

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
