#![expect(
    clippy::pub_use,
    reason = "Task 37 requires a testing API surface exposed from src/testing.rs"
)]

//! Testing framework surface for Opalescent-language test execution.

#[path = "testing/assertions.rs"]
pub mod assertions;
#[path = "testing/bench.rs"]
pub mod bench;
#[path = "testing/coverage.rs"]
pub mod coverage;
#[path = "testing/discovery.rs"]
pub mod discovery;
#[path = "testing/property.rs"]
pub mod property;
#[path = "testing/runner.rs"]
pub mod runner;

pub use assertions::{
    assert_eq, assert_false, assert_ne, assert_throws, assert_true, AssertionError,
};
pub use bench::{run_benchmark, Benchmark, BenchmarkResult};
pub use coverage::{generate_coverage_report, CoverageMap, CoverageReport};
pub use discovery::{discover_tests_by_name_pattern, TestSelection};
pub use property::{check_property, PropertyCheckResult, PropertyFailure, PropertyTest};
pub use runner::{TestCase, TestCommand, TestReport, TestResult, TestSuite};

#[cfg(test)]
#[path = "testing/tests.rs"]
mod tests;
