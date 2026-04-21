#![expect(
    clippy::pub_use,
    reason = "Task 40 requires a benchmark API surface exposed from src/benchmarks.rs"
)]

pub mod compile_time;
pub mod memory;
pub mod regression;
pub mod runtime_bench;
pub mod suite;

pub use crate::testing::BenchmarkResult;
pub use memory::{MeasuredBenchmark, MemoryMetrics};
pub use regression::{
    RegressionResult, RegressionThreshold, RegressionViolation, check_regression,
};
pub use suite::{BenchmarkSuite, SuiteReport};

#[cfg(test)]
mod tests;
