extern crate alloc;

use crate::testing::bench::BenchmarkResult;
use crate::testing::bench::{Benchmark, run_benchmark};
use core::convert::TryFrom;

/// Measures recursive Fibonacci-like runtime workload.
#[must_use]
pub fn bench_fibonacci_runtime() -> BenchmarkResult {
    run_benchmark(&Benchmark {
        name: "runtime_fibonacci",
        iterations: 20,
        run_fn: fibonacci_timing_ns,
    })
}

/// Measures array sort workload representative of runtime operations.
#[must_use]
pub fn bench_array_sort_runtime() -> BenchmarkResult {
    run_benchmark(&Benchmark {
        name: "runtime_array_sort",
        iterations: 20,
        run_fn: sort_timing_ns,
    })
}

/// Measures string concatenation and scanning workload.
#[must_use]
pub fn bench_string_ops_runtime() -> BenchmarkResult {
    run_benchmark(&Benchmark {
        name: "runtime_string_ops",
        iterations: 20,
        run_fn: string_ops_timing_ns,
    })
}

/// Computes one Fibonacci runtime sample in nanoseconds.
fn fibonacci_timing_ns() -> u64 {
    let start = std::time::Instant::now();
    consume(fibonacci(24));
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Computes one array sort runtime sample in nanoseconds.
fn sort_timing_ns() -> u64 {
    let start = std::time::Instant::now();
    let mut values = [50_i64, 1, 70, 4, 99, 12, 7, 44, 18, 2];
    values.sort_unstable();
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Computes one string operation runtime sample in nanoseconds.
fn string_ops_timing_ns() -> u64 {
    let start = std::time::Instant::now();
    let mut text = String::from("opal");
    for segment in ["es", "cent", "-", "bench"] {
        text.push_str(segment);
    }

    consume(text.contains("bench"));
    consume(text.len());
    u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Computes Fibonacci recursively for runtime stress simulation.
#[must_use]
fn fibonacci(index: u32) -> u64 {
    if index <= 1 {
        return u64::from(index);
    }

    fibonacci(index.saturating_sub(1)).saturating_add(fibonacci(index.saturating_sub(2)))
}

/// Consumes a value to make benchmark side effects explicit.
fn consume<T>(value: T) {
    drop(value);
}
