//! Core test-case and suite execution for Opalescent-language tests.

extern crate alloc;

use crate::build_system::config::ProjectConfig;
use crate::build_system::targets::BuildTarget;
use crate::testing::assertions::AssertionError;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Outcome of a single test execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestResult {
    /// Test passed without assertion failures.
    Passed,
    /// Test failed with assertion details.
    Failed {
        /// Failure details.
        message: String,
    },
    /// Test was skipped with reason.
    Skipped {
        /// Skip reason.
        reason: String,
    },
}

/// One test case discovered from Opalescent test blocks.
#[derive(Debug, Clone)]
pub struct TestCase {
    /// Stable test-case name.
    pub name: String,
    /// Executable unit for this test case.
    pub run_fn: fn() -> Result<(), AssertionError>,
}

impl TestCase {
    /// Construct a new test case.
    #[must_use]
    pub fn new(name: impl Into<String>, run_fn: fn() -> Result<(), AssertionError>) -> Self {
        Self {
            name: name.into(),
            run_fn,
        }
    }

    /// Execute this test case and return [`TestResult`].
    #[must_use]
    pub fn run(&self) -> TestResult {
        match (self.run_fn)() {
            Ok(()) => TestResult::Passed,
            Err(error) => TestResult::Failed {
                message: error.message,
            },
        }
    }
}

/// A named collection of [`TestCase`] values.
#[derive(Debug, Clone)]
pub struct TestSuite {
    /// Suite name.
    pub name: String,
    /// Test cases in deterministic order.
    pub tests: Vec<TestCase>,
}

impl TestSuite {
    /// Construct an empty test suite.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            tests: Vec::new(),
        }
    }

    /// Add one test case into the suite.
    pub fn add_test(&mut self, test_case: TestCase) {
        self.tests.push(test_case);
    }
}

/// Aggregated execution report for a test suite run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestReport {
    /// Number of passing tests.
    pub passed: usize,
    /// Number of failing tests.
    pub failed: usize,
    /// Number of skipped tests.
    pub skipped: usize,
    /// Per-test outcomes keyed by test name.
    pub outcomes: BTreeMap<String, TestResult>,
}

impl TestReport {
    /// Create an empty report.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            passed: 0,
            failed: 0,
            skipped: 0,
            outcomes: BTreeMap::new(),
        }
    }

    /// Return whether all non-skipped tests passed.
    #[must_use]
    pub const fn is_success(&self) -> bool {
        self.failed == 0
    }
}

impl Default for TestReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Execute all test cases in `suite` and return a report.
#[must_use]
pub fn run_suite(suite: &TestSuite) -> TestReport {
    let mut report = TestReport::new();
    for test_case in &suite.tests {
        let result = test_case.run();
        match result {
            TestResult::Passed => {
                report.passed = report.passed.saturating_add(1);
            }
            TestResult::Failed { .. } => {
                report.failed = report.failed.saturating_add(1);
            }
            TestResult::Skipped { .. } => {
                report.skipped = report.skipped.saturating_add(1);
            }
        }
        report.outcomes.insert(test_case.name.clone(), result);
    }
    report
}

/// Build-system level command for invoking Opalescent tests (`opal test`).
#[derive(Debug, Clone)]
pub struct TestCommand {
    /// Parsed project configuration used for command context.
    pub project_config: ProjectConfig,
    /// Optional target override when cross-testing.
    pub target: Option<BuildTarget>,
    /// Optional substring filter applied to test names.
    pub name_filter: Option<String>,
}

impl TestCommand {
    /// Create a command from a project config with no target override.
    #[must_use]
    pub const fn new(project_config: ProjectConfig) -> Self {
        Self {
            project_config,
            target: None,
            name_filter: None,
        }
    }

    /// Set an explicit target override for this test command.
    #[must_use]
    pub const fn with_target(mut self, target: BuildTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set a name filter for test discovery and execution.
    #[must_use]
    pub fn with_filter(mut self, name_filter: impl Into<String>) -> Self {
        self.name_filter = Some(name_filter.into());
        self
    }

    /// Execute tests for `suite` and return a report.
    #[must_use]
    pub fn execute(&self, suite: &TestSuite) -> TestReport {
        self.name_filter.as_ref().map_or_else(
            || run_suite(suite),
            |filter| {
                let mut filtered = TestSuite::new(suite.name.clone());
                for test_case in &suite.tests {
                    if test_case.name.contains(filter) {
                        filtered.add_test(test_case.clone());
                    }
                }
                run_suite(&filtered)
            },
        )
    }
}
