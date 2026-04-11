//! Checked and saturating arithmetic for all numeric types in the Opalescent standard library.
//!
//! All types are `no_std` compatible — only `core` and `alloc` are used.
//! This module provides the language-level numeric type operations that map to Opalescent's
//! `int8`, `int16`, `int32`, `int64`, `uint8`, `uint16`, `uint32`, `uint64`, `float32`, `float64`,
//! `bool`, `string`, and `void` runtime representations.
//!
//! # Design rationale
//!
//! Checked variants return `Option<T>` to avoid panics in all environments.
//! Saturating variants clamp to the type boundary instead of wrapping or trapping.
//! Both families are available per type for user choice based on the math spec.

// =====================================================================
// i64 operations
// =====================================================================

/// Checked addition for `int64`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_i64(a: i64, b: i64) -> Option<i64> {
    a.checked_add(b)
}

/// Checked subtraction for `int64`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_i64(a: i64, b: i64) -> Option<i64> {
    a.checked_sub(b)
}

/// Checked multiplication for `int64`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_i64(a: i64, b: i64) -> Option<i64> {
    a.checked_mul(b)
}

/// Checked division for `int64`: returns `None` on division by zero or overflow.
#[must_use]
pub const fn checked_div_i64(a: i64, b: i64) -> Option<i64> {
    a.checked_div(b)
}

/// Saturating addition for `int64`: clamps to `i64::MAX` on overflow, `i64::MIN` on underflow.
#[must_use]
pub const fn saturating_add_i64(a: i64, b: i64) -> i64 {
    a.saturating_add(b)
}

/// Saturating subtraction for `int64`: clamps at type boundaries.
#[must_use]
pub const fn saturating_sub_i64(a: i64, b: i64) -> i64 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `int64`: clamps at type boundaries.
#[must_use]
pub const fn saturating_mul_i64(a: i64, b: i64) -> i64 {
    a.saturating_mul(b)
}

// =====================================================================
// i32 operations
// =====================================================================

/// Checked addition for `int32`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_i32(a: i32, b: i32) -> Option<i32> {
    a.checked_add(b)
}

/// Checked subtraction for `int32`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_i32(a: i32, b: i32) -> Option<i32> {
    a.checked_sub(b)
}

/// Checked multiplication for `int32`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_i32(a: i32, b: i32) -> Option<i32> {
    a.checked_mul(b)
}

/// Checked division for `int32`: returns `None` on division by zero or overflow.
#[must_use]
pub const fn checked_div_i32(a: i32, b: i32) -> Option<i32> {
    a.checked_div(b)
}

/// Saturating addition for `int32`: clamps at type boundaries.
#[must_use]
pub const fn saturating_add_i32(a: i32, b: i32) -> i32 {
    a.saturating_add(b)
}

/// Saturating subtraction for `int32`: clamps at type boundaries.
#[must_use]
pub const fn saturating_sub_i32(a: i32, b: i32) -> i32 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `int32`: clamps at type boundaries.
#[must_use]
pub const fn saturating_mul_i32(a: i32, b: i32) -> i32 {
    a.saturating_mul(b)
}

// =====================================================================
// i16 operations
// =====================================================================

/// Checked addition for `int16`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_i16(a: i16, b: i16) -> Option<i16> {
    a.checked_add(b)
}

/// Checked subtraction for `int16`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_i16(a: i16, b: i16) -> Option<i16> {
    a.checked_sub(b)
}

/// Checked multiplication for `int16`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_i16(a: i16, b: i16) -> Option<i16> {
    a.checked_mul(b)
}

/// Checked division for `int16`: returns `None` on division by zero or overflow.
#[must_use]
pub const fn checked_div_i16(a: i16, b: i16) -> Option<i16> {
    a.checked_div(b)
}

/// Saturating addition for `int16`: clamps at type boundaries.
#[must_use]
pub const fn saturating_add_i16(a: i16, b: i16) -> i16 {
    a.saturating_add(b)
}

/// Saturating subtraction for `int16`: clamps at type boundaries.
#[must_use]
pub const fn saturating_sub_i16(a: i16, b: i16) -> i16 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `int16`: clamps at type boundaries.
#[must_use]
pub const fn saturating_mul_i16(a: i16, b: i16) -> i16 {
    a.saturating_mul(b)
}

// =====================================================================
// i8 operations
// =====================================================================

/// Checked addition for `int8`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_i8(a: i8, b: i8) -> Option<i8> {
    a.checked_add(b)
}

/// Checked subtraction for `int8`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_i8(a: i8, b: i8) -> Option<i8> {
    a.checked_sub(b)
}

/// Checked multiplication for `int8`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_i8(a: i8, b: i8) -> Option<i8> {
    a.checked_mul(b)
}

/// Checked division for `int8`: returns `None` on division by zero or overflow.
#[must_use]
pub const fn checked_div_i8(a: i8, b: i8) -> Option<i8> {
    a.checked_div(b)
}

/// Saturating addition for `int8`: clamps at type boundaries.
#[must_use]
pub const fn saturating_add_i8(a: i8, b: i8) -> i8 {
    a.saturating_add(b)
}

/// Saturating subtraction for `int8`: clamps at type boundaries.
#[must_use]
pub const fn saturating_sub_i8(a: i8, b: i8) -> i8 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `int8`: clamps at type boundaries.
#[must_use]
pub const fn saturating_mul_i8(a: i8, b: i8) -> i8 {
    a.saturating_mul(b)
}

// =====================================================================
// u64 operations
// =====================================================================

/// Checked addition for `uint64`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_u64(a: u64, b: u64) -> Option<u64> {
    a.checked_add(b)
}

/// Checked subtraction for `uint64`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_u64(a: u64, b: u64) -> Option<u64> {
    a.checked_sub(b)
}

/// Checked multiplication for `uint64`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_u64(a: u64, b: u64) -> Option<u64> {
    a.checked_mul(b)
}

/// Checked division for `uint64`: returns `None` on division by zero.
#[must_use]
pub const fn checked_div_u64(a: u64, b: u64) -> Option<u64> {
    a.checked_div(b)
}

/// Saturating addition for `uint64`: clamps at `u64::MAX`.
#[must_use]
pub const fn saturating_add_u64(a: u64, b: u64) -> u64 {
    a.saturating_add(b)
}

/// Saturating subtraction for `uint64`: clamps at `0`.
#[must_use]
pub const fn saturating_sub_u64(a: u64, b: u64) -> u64 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `uint64`: clamps at `u64::MAX`.
#[must_use]
pub const fn saturating_mul_u64(a: u64, b: u64) -> u64 {
    a.saturating_mul(b)
}

// =====================================================================
// u32 operations
// =====================================================================

/// Checked addition for `uint32`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_add(b)
}

/// Checked subtraction for `uint32`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_sub(b)
}

/// Checked multiplication for `uint32`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_mul(b)
}

/// Checked division for `uint32`: returns `None` on division by zero.
#[must_use]
pub const fn checked_div_u32(a: u32, b: u32) -> Option<u32> {
    a.checked_div(b)
}

/// Saturating addition for `uint32`: clamps at `u32::MAX`.
#[must_use]
pub const fn saturating_add_u32(a: u32, b: u32) -> u32 {
    a.saturating_add(b)
}

/// Saturating subtraction for `uint32`: clamps at `0`.
#[must_use]
pub const fn saturating_sub_u32(a: u32, b: u32) -> u32 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `uint32`: clamps at `u32::MAX`.
#[must_use]
pub const fn saturating_mul_u32(a: u32, b: u32) -> u32 {
    a.saturating_mul(b)
}

// =====================================================================
// u16 operations
// =====================================================================

/// Checked addition for `uint16`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_u16(a: u16, b: u16) -> Option<u16> {
    a.checked_add(b)
}

/// Checked subtraction for `uint16`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_u16(a: u16, b: u16) -> Option<u16> {
    a.checked_sub(b)
}

/// Checked multiplication for `uint16`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_u16(a: u16, b: u16) -> Option<u16> {
    a.checked_mul(b)
}

/// Checked division for `uint16`: returns `None` on division by zero.
#[must_use]
pub const fn checked_div_u16(a: u16, b: u16) -> Option<u16> {
    a.checked_div(b)
}

/// Saturating addition for `uint16`: clamps at `u16::MAX`.
#[must_use]
pub const fn saturating_add_u16(a: u16, b: u16) -> u16 {
    a.saturating_add(b)
}

/// Saturating subtraction for `uint16`: clamps at `0`.
#[must_use]
pub const fn saturating_sub_u16(a: u16, b: u16) -> u16 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `uint16`: clamps at `u16::MAX`.
#[must_use]
pub const fn saturating_mul_u16(a: u16, b: u16) -> u16 {
    a.saturating_mul(b)
}

// =====================================================================
// u8 operations
// =====================================================================

/// Checked addition for `uint8`: returns `None` on overflow.
#[must_use]
pub const fn checked_add_u8(a: u8, b: u8) -> Option<u8> {
    a.checked_add(b)
}

/// Checked subtraction for `uint8`: returns `None` on underflow.
#[must_use]
pub const fn checked_sub_u8(a: u8, b: u8) -> Option<u8> {
    a.checked_sub(b)
}

/// Checked multiplication for `uint8`: returns `None` on overflow.
#[must_use]
pub const fn checked_mul_u8(a: u8, b: u8) -> Option<u8> {
    a.checked_mul(b)
}

/// Checked division for `uint8`: returns `None` on division by zero.
#[must_use]
pub const fn checked_div_u8(a: u8, b: u8) -> Option<u8> {
    a.checked_div(b)
}

/// Saturating addition for `uint8`: clamps at `u8::MAX`.
#[must_use]
pub const fn saturating_add_u8(a: u8, b: u8) -> u8 {
    a.saturating_add(b)
}

/// Saturating subtraction for `uint8`: clamps at `0`.
#[must_use]
pub const fn saturating_sub_u8(a: u8, b: u8) -> u8 {
    a.saturating_sub(b)
}

/// Saturating multiplication for `uint8`: clamps at `u8::MAX`.
#[must_use]
pub const fn saturating_mul_u8(a: u8, b: u8) -> u8 {
    a.saturating_mul(b)
}
