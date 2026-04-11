//! Benchmark harness for Opalescent-language benchmark blocks.

extern crate alloc;

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
#[derive(Debug, Clone, PartialEq)]
pub struct BenchmarkResult {
    /// Benchmark name.
    pub name: &'static str,
    /// Total iterations executed.
    pub iterations: usize,
    /// Mean execution time in nanoseconds.
    pub mean_ns: f64,
    /// Standard deviation in nanoseconds.
    pub stddev_ns: f64,
}

/// Execute benchmark iterations and compute summary statistics.
#[expect(
    clippy::float_arithmetic,
    reason = "benchmark summary statistics require floating-point math"
)]
#[expect(
    clippy::suspicious_operation_groupings,
    reason = "variance formula intentionally subtracts squared mean"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "clarity of statistical formula is preferred here"
)]
#[must_use]
pub fn run_benchmark(benchmark: &Benchmark) -> BenchmarkResult {
    let mut count = 0_usize;
    let mut total = 0.0_f64;
    let mut total_squares = 0.0_f64;

    while count < benchmark.iterations {
        let elapsed = (benchmark.run_fn)();
        let elapsed_f64 = u64_to_f64(elapsed);
        total += elapsed_f64;
        total_squares += elapsed_f64 * elapsed_f64;
        count = count.saturating_add(1);
    }

    if benchmark.iterations == 0 {
        return BenchmarkResult {
            name: benchmark.name,
            iterations: 0,
            mean_ns: 0.0,
            stddev_ns: 0.0,
        };
    }

    let iteration_count = usize_to_f64(benchmark.iterations);
    let mean = total / iteration_count;
    let variance = (total_squares / iteration_count) - (mean * mean);
    let stddev = if variance <= 0.0_f64 {
        0.0_f64
    } else {
        variance.sqrt()
    };

    BenchmarkResult {
        name: benchmark.name,
        iterations: benchmark.iterations,
        mean_ns: mean,
        stddev_ns: stddev,
    }
}

/// Convert `usize` into `f64` using checked conversion.
fn usize_to_f64(value: usize) -> f64 {
    match u64::try_from(value) {
        Ok(converted) => u64_to_f64(converted),
        Err(_error) => f64::from(u32::MAX),
    }
}

/// Convert `u64` into `f64` without `as` casting.
#[expect(
    clippy::float_arithmetic,
    reason = "u64-to-f64 conversion combines high and low chunks with float math"
)]
#[expect(
    clippy::suboptimal_flops,
    reason = "conversion formula prioritizes readability over fused operations"
)]
fn u64_to_f64(value: u64) -> f64 {
    let low_mask = u64::from(u32::MAX);
    let low_bits_u64 = value & low_mask;
    let high_bits_u64 = value >> 32_u32;

    let low_bits = match u32::try_from(low_bits_u64) {
        Ok(converted) => converted,
        Err(_error) => u32::MAX,
    };
    let high_bits = match u32::try_from(high_bits_u64) {
        Ok(converted) => converted,
        Err(_error) => u32::MAX,
    };

    f64::from(high_bits) * 4_294_967_296.0 + f64::from(low_bits)
}
