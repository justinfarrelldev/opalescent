//! Math functions for the Opalescent standard library.
//!
//! All functions are `no_std` compatible — only `core` numeric primitives are used.
//! This module exposes the mathematical constants and functions specified in
//! `language-spec/requirements/math.md`, including trigonometric functions,
//! logarithms, rounding, and power operations.
//!
//! # Design rationale
//!
//! Integer overloads use `saturating_abs` to avoid the `i64::MIN.abs()` panic
//! that exists in standard Rust. Float operations follow IEEE-754 semantics per
//! the language specification.

/// The mathematical constant π (pi) — ratio of a circle's circumference to its diameter.
pub const PI: f64 = core::f64::consts::PI;

/// The mathematical constant e — base of the natural logarithm.
pub const E: f64 = core::f64::consts::E;

/// Positive floating-point infinity.
pub const INFINITY: f64 = f64::INFINITY;

/// IEEE-754 Not-a-Number value.
pub const NAN: f64 = f64::NAN;

/// Absolute value of an `int64` integer using saturating semantics.
///
/// `i64::MIN.abs()` would overflow; `saturating_abs` clamps to `i64::MAX` instead.
#[must_use]
pub const fn abs_i64(x: i64) -> i64 {
    x.saturating_abs()
}

/// Absolute value of a `float64` floating-point number following IEEE-754.
#[must_use]
pub const fn abs_f64(x: f64) -> f64 {
    x.abs()
}

/// Ceiling — smallest integer ≥ `x`.
#[must_use]
pub fn ceil(x: f64) -> f64 {
    x.ceil()
}

/// Floor — largest integer ≤ `x`.
#[must_use]
pub fn floor(x: f64) -> f64 {
    x.floor()
}

/// Round — nearest integer, with ties rounding away from zero (IEEE-754 default).
#[must_use]
pub fn round(x: f64) -> f64 {
    x.round()
}

/// Square root of `x`.
///
/// Returns NaN when `x < 0.0` per IEEE-754.
#[must_use]
pub fn sqrt(x: f64) -> f64 {
    x.sqrt()
}

/// Raise `base` to the power `exp` using IEEE-754 `pow`.
#[must_use]
pub fn pow(base: f64, exp: f64) -> f64 {
    base.powf(exp)
}

/// Natural logarithm (base e) of `x`.
///
/// Returns NaN for negative `x` and negative infinity for `x = 0.0` per IEEE-754.
#[must_use]
pub fn log(x: f64) -> f64 {
    x.ln()
}

/// Base-10 logarithm of `x`.
///
/// Returns NaN for negative `x` and negative infinity for `x = 0.0` per IEEE-754.
#[must_use]
pub fn log10(x: f64) -> f64 {
    x.log10()
}

/// Sine of `x` (in radians).
#[must_use]
pub fn sin(x: f64) -> f64 {
    x.sin()
}

/// Cosine of `x` (in radians).
#[must_use]
pub fn cos(x: f64) -> f64 {
    x.cos()
}

/// Tangent of `x` (in radians).
#[must_use]
pub fn tan(x: f64) -> f64 {
    x.tan()
}

/// Two-argument arctangent: angle of the vector from the origin to `(x, y)`.
///
/// Returns a value in radians in `(-π, π]`.
#[must_use]
pub fn atan2(y: f64, x: f64) -> f64 {
    y.atan2(x)
}

/// Return the minimum of two `int64` values.
#[must_use]
pub const fn min_i64(a: i64, b: i64) -> i64 {
    if a < b {
        a
    } else {
        b
    }
}

/// Return the maximum of two `int64` values.
#[must_use]
pub const fn max_i64(a: i64, b: i64) -> i64 {
    if a > b {
        a
    } else {
        b
    }
}

/// Return the minimum of two `float64` values.
///
/// If either argument is NaN, the result follows `f64::min` semantics (returns the non-NaN value).
#[must_use]
pub const fn min_f64(a: f64, b: f64) -> f64 {
    a.min(b)
}

/// Return the maximum of two `float64` values.
///
/// If either argument is NaN, the result follows `f64::max` semantics (returns the non-NaN value).
#[must_use]
pub const fn max_f64(a: f64, b: f64) -> f64 {
    a.max(b)
}
