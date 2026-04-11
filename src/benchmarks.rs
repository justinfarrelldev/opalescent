#![expect(
    clippy::pub_use,
    reason = "Task 40 requires a benchmark API surface exposed from src/benchmarks.rs"
)]

#[path = "benchmarks/compile_time.rs"]
pub mod compile_time;
#[path = "benchmarks/memory.rs"]
pub mod memory;
#[path = "benchmarks/regression.rs"]
pub mod regression;
#[path = "benchmarks/runtime_bench.rs"]
pub mod runtime_bench;
#[path = "benchmarks/suite.rs"]
pub mod suite;

pub use crate::testing::BenchmarkResult;
pub use memory::{MeasuredBenchmark, MemoryMetrics};
pub use regression::{
    check_regression, RegressionResult, RegressionThreshold, RegressionViolation,
};
pub use suite::{BenchmarkSuite, SuiteReport};

#[cfg(test)]
#[path = "benchmarks/tests.rs"]
mod tests;
