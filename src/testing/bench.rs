//! Benchmark harness for Opalescent-language benchmark blocks.

extern crate alloc;

use alloc::string::String;
use core::convert::TryFrom;

/// Benchmark definition.
#[derive(Debug, Clone)]
pub struct Benchmark {
    /// Benchmark name.
    pub name: &'static str,
    /// Number of benchmark iterations.
    pub iterations: usize,
    /// Benchmark function returning elapsed nanoseconds for one run.
    pub run_fn: fn() -> u64,
}

/// Benchmark statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkResult {
    /// Benchmark name.
    pub name: String,
    /// Total iterations executed.
    pub iterations: usize,
    /// Mean execution time in nanoseconds.
    pub mean_ns: u64,
    /// Standard deviation in nanoseconds.
    pub stddev_ns: u64,
}

/// Execute benchmark iterations and compute summary statistics.
#[must_use]
pub fn run_benchmark(benchmark: &Benchmark) -> BenchmarkResult {
    let mut count = 0_usize;
    let mut samples = alloc::vec::Vec::with_capacity(benchmark.iterations);

    while count < benchmark.iterations {
        let elapsed = (benchmark.run_fn)();
        samples.push(elapsed);
        count = count.saturating_add(1);
    }

    if benchmark.iterations == 0 {
        return BenchmarkResult {
            name: benchmark.name.to_owned(),
            iterations: 0,
            mean_ns: 0,
            stddev_ns: 0,
        };
    }

    let (mean, stddev) = summarize_samples(samples.as_slice());

    BenchmarkResult {
        name: benchmark.name.to_owned(),
        iterations: benchmark.iterations,
        mean_ns: mean,
        stddev_ns: stddev,
    }
}

/// Produces mean and standard deviation from raw sample nanoseconds.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "benchmark variance and mean calculations require numeric operators"
)]
#[expect(
    clippy::integer_division,
    reason = "integer nanosecond summary intentionally uses truncated integer moments"
)]
#[must_use]
pub fn summarize_samples(samples: &[u64]) -> (u64, u64) {
    if samples.is_empty() {
        return (0, 0);
    }

    let mut total = 0_u128;
    let mut total_squares = 0_u128;

    for sample in samples {
        let value = u128::from(*sample);
        total = total.saturating_add(value);
        total_squares = total_squares.saturating_add(value.saturating_mul(value));
    }

    let count = u128::try_from(samples.len()).unwrap_or(u128::MAX);
    let mean = total / count;
    let second_moment = total_squares / count;
    let mean_square = mean.saturating_mul(mean);
    let variance = second_moment.saturating_sub(mean_square);
    let stddev = integer_sqrt(variance);

    (
        u64::try_from(mean).unwrap_or(u64::MAX),
        u64::try_from(stddev).unwrap_or(u64::MAX),
    )
}

/// Computes integer square root with Newton iteration.
#[expect(
    clippy::arithmetic_side_effects,
    reason = "Newton iteration updates rely on integer arithmetic for deterministic sqrt"
)]
#[expect(
    clippy::integer_division,
    reason = "integer Newton iteration intentionally performs integer division"
)]
#[must_use]
const fn integer_sqrt(value: u128) -> u128 {
    if value <= 1 {
        return value;
    }

    let mut x0 = value;
    let mut x1 = (x0.saturating_add(1)) / 2;

    while x1 < x0 {
        x0 = x1;
        x1 = (x1.saturating_add(value / x1)) / 2;
    }

    x0
}
