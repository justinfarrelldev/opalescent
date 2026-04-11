//! Test discovery and filtering support.

extern crate alloc;

use crate::testing::runner::TestCase;
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;

/// User-provided criteria for selecting test cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSelection {
    /// Optional substring pattern matched against test names.
    pub name_pattern: Option<String>,
    /// Explicitly included test names.
    pub include_names: BTreeSet<String>,
    /// Explicitly excluded test names.
    pub exclude_names: BTreeSet<String>,
}

impl TestSelection {
    /// Construct an empty selection that accepts all tests.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            name_pattern: None,
            include_names: BTreeSet::new(),
            exclude_names: BTreeSet::new(),
        }
    }

    /// Configure a name-pattern filter.
    #[must_use]
    pub fn with_name_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.name_pattern = Some(pattern.into());
        self
    }

    /// Include one exact test name.
    #[must_use]
    pub fn include(mut self, test_name: impl Into<String>) -> Self {
        self.include_names.insert(test_name.into());
        self
    }

    /// Exclude one exact test name.
    #[must_use]
    pub fn exclude(mut self, test_name: impl Into<String>) -> Self {
        self.exclude_names.insert(test_name.into());
        self
    }

    /// Return whether `test_case` satisfies this selection.
    #[must_use]
    pub fn matches(&self, test_case: &TestCase) -> bool {
        if self.exclude_names.contains(&test_case.name) {
            return false;
        }
        if !self.include_names.is_empty() && !self.include_names.contains(&test_case.name) {
            return false;
        }
        self.name_pattern
            .as_ref()
            .is_none_or(|pattern| test_case.name.contains(pattern))
    }
}

impl Default for TestSelection {
    fn default() -> Self {
        Self::new()
    }
}

/// Discover tests that match `name_pattern`.
#[must_use]
pub fn discover_tests_by_name_pattern(tests: &[TestCase], name_pattern: &str) -> Vec<TestCase> {
    let mut selected = Vec::new();
    for test_case in tests {
        if test_case.name.contains(name_pattern) {
            selected.push(test_case.clone());
        }
    }
    selected
}

/// Discover tests satisfying a rich [`TestSelection`].
#[must_use]
pub fn discover_tests_with_selection(
    tests: &[TestCase],
    selection: &TestSelection,
) -> Vec<TestCase> {
    let mut selected = Vec::new();
    for test_case in tests {
        if selection.matches(test_case) {
            selected.push(test_case.clone());
        }
    }
    selected
}
