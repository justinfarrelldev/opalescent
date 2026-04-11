extern crate alloc;

use crate::runtime::errors::{RuntimeError, RuntimeResult};
use crate::runtime::memory::OpalArray;
use alloc::string::String;
use alloc::vec::Vec;
use core::convert::TryFrom;
use core::ops::Range;
use core::sync::atomic::{AtomicU64, Ordering};

/// Global state for the runtime linear-congruential random generator.
static RANDOM_STATE: AtomicU64 = AtomicU64::new(0x4d59_5df4_d0f3_3173);

/// Random source abstraction to keep runtime randomness testable.
pub trait RandomIntSource {
    /// Produce next pseudorandom 32-bit value.
    fn next_u32(&mut self) -> u32;
}

/// Default runtime pseudorandom source.
#[derive(Debug, Default)]
pub struct DefaultRandomIntSource;

impl DefaultRandomIntSource {}

impl RandomIntSource for DefaultRandomIntSource {
    fn next_u32(&mut self) -> u32 {
        let previous = RANDOM_STATE.load(Ordering::Relaxed);
        let next = previous
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        RANDOM_STATE.store(next, Ordering::Relaxed);
        let shifted = next >> 32_u32;
        u32::try_from(shifted).ok().map_or(0_u32, |value| value)
    }
}

/// Parse runtime string text into an `i32` value.
///
/// # Errors
///
/// Returns [`RuntimeError::ParseError`] when parsing fails.
pub fn string_to_int32(input: &str) -> RuntimeResult<i32> {
    input
        .parse::<i32>()
        .map_err(|_parse_error| RuntimeError::ParseError {
            message: alloc::format!("failed to parse int32 from '{input}'"),
        })
}

/// Generate random `i32` in inclusive range `[min, max]`.
///
/// # Errors
///
/// Returns [`RuntimeError::UserError`] when `min > max`.
pub fn random_int32(min: i32, max: i32) -> RuntimeResult<i32> {
    let mut source = DefaultRandomIntSource;
    random_int32_with_source(&mut source, min, max)
}

/// Generate random `i32` in inclusive range `[min, max]` with injected source.
///
/// # Errors
///
/// Returns [`RuntimeError::UserError`] when `min > max` or range conversion fails.
pub fn random_int32_with_source(
    source: &mut impl RandomIntSource,
    min: i32,
    max: i32,
) -> RuntimeResult<i32> {
    if min > max {
        return Err(RuntimeError::UserError {
            code: 2_001,
            message: String::from("random_int32 requires min <= max"),
        });
    }

    let span_range_i64 = i64::from(max)
        .checked_sub(i64::from(min))
        .and_then(|difference| difference.checked_add(1_i64))
        .ok_or_else(|| RuntimeError::UserError {
            code: 2_002,
            message: String::from("random_int32 range conversion failed"),
        })?;
    let span_count_u32 =
        u32::try_from(span_range_i64).map_err(|_conversion_error| RuntimeError::UserError {
            code: 2_002,
            message: String::from("random_int32 range conversion failed"),
        })?;

    let offset = source
        .next_u32()
        .checked_rem(span_count_u32)
        .ok_or_else(|| RuntimeError::UserError {
            code: 2_003,
            message: String::from("random_int32 result conversion failed"),
        })?;
    let result_value_i64 = i64::from(min)
        .checked_add(i64::from(offset))
        .ok_or_else(|| RuntimeError::UserError {
            code: 2_003,
            message: String::from("random_int32 result conversion failed"),
        })?;
    i32::try_from(result_value_i64).map_err(|_conversion_error| RuntimeError::UserError {
        code: 2_003,
        message: String::from("random_int32 result conversion failed"),
    })
}

/// Format interpolated string by replacing each `{...}` placeholder with next value.
///
/// Placeholder names are ignored; values are consumed in encounter order.
///
/// # Errors
///
/// Returns [`RuntimeError::UserError`] when placeholder count mismatches `values`.
pub fn format_interpolated_string(format: &str, values: &[String]) -> RuntimeResult<String> {
    let placeholder_count = count_placeholders(format);
    if placeholder_count != values.len() {
        return Err(RuntimeError::UserError {
            code: 2_004,
            message: alloc::format!(
                "placeholder count mismatch: expected {placeholder_count} values, received {}",
                values.len()
            ),
        });
    }

    let mut output = String::new();
    let mut chars = format.chars();
    let mut value_index = 0_usize;

    while let Some(ch) = chars.next() {
        if ch == '{' {
            for next_char in chars.by_ref() {
                if next_char == '}' {
                    break;
                }
            }
            if let Some(value) = values.get(value_index) {
                output.push_str(value);
                value_index =
                    value_index
                        .checked_add(1_usize)
                        .ok_or_else(|| RuntimeError::UserError {
                            code: 2_005,
                            message: String::from("interpolation value index overflow"),
                        })?;
            }
        } else {
            output.push(ch);
        }
    }

    Ok(output)
}

/// Slice a runtime array by half-open range `[start, end)`.
///
/// # Errors
///
/// Returns [`RuntimeError::IndexOutOfBounds`] when range is invalid.
pub fn opal_array_slice<T>(
    arr: &OpalArray<T>,
    start: usize,
    end: usize,
) -> RuntimeResult<OpalArray<T>>
where
    T: Clone,
{
    if start > end || end > arr.len() {
        return Err(RuntimeError::IndexOutOfBounds {
            index: start,
            length: arr.len(),
        });
    }

    let indices: Range<usize> = start..end;
    let mut values = Vec::with_capacity(indices.len());
    for index in indices {
        if let Some(value) = arr.get(index) {
            values.push(value.clone());
        }
    }

    Ok(OpalArray::new(values))
}

/// Count interpolation placeholders of the form `{...}` in source text.
fn count_placeholders(format: &str) -> usize {
    let mut count = 0_usize;
    let mut chars = format.chars();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            for next_char in chars.by_ref() {
                if next_char == '}' {
                    count = count.saturating_add(1_usize);
                    break;
                }
            }
        }
    }

    count
}
