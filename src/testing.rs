#![expect(
    clippy::pub_use,
    reason = "Task 37 requires a testing API surface exposed from src/testing.rs"
)]

//! Testing framework surface for Opalescent-language test execution.

pub mod assertions;
pub mod bench;
pub mod coverage;
pub mod discovery;
pub mod property;
pub mod runner;

pub use assertions::{
    AssertionError, assert_eq, assert_false, assert_ne, assert_throws, assert_true,
};
pub use bench::{Benchmark, BenchmarkResult, run_benchmark};
pub use coverage::{CoverageMap, CoverageReport, generate_coverage_report};
pub use discovery::{TestSelection, discover_tests_by_name_pattern};
pub use property::{PropertyCheckResult, PropertyFailure, PropertyTest, check_property};
pub use runner::{TestCase, TestCommand, TestReport, TestResult, TestSuite};

#[cfg(test)]
mod tests;
