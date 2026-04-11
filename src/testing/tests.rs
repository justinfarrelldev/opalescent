extern crate alloc;

use crate::build_system::config::{parse_config, ProjectConfig};
use crate::build_system::targets::{parse_target_triple, BuildTarget};
use crate::testing::assertions::{
    assert_eq as opal_assert_eq, assert_false, assert_ne, assert_throws, assert_true,
};
use crate::testing::bench::{run_benchmark, Benchmark};
use crate::testing::coverage::{generate_coverage_report, CoverageMap};
use crate::testing::discovery::{
    discover_tests_by_name_pattern, discover_tests_with_selection, TestSelection,
};
use crate::testing::property::{check_property, PropertyCheckResult, PropertyTest};
use crate::testing::runner::{run_suite, TestCase, TestCommand, TestResult, TestSuite};
use alloc::string::String;
use alloc::vec;

#[expect(
    clippy::unnecessary_wraps,
    reason = "test harness requires fn pointer returning Result for uniform execution"
)]
fn passing_test() -> Result<(), crate::testing::AssertionError> {
    Ok(())
}

fn failing_test() -> Result<(), crate::testing::AssertionError> {
    Err(crate::testing::AssertionError::new("intentional failure"))
}

fn error_operation() -> Result<u32, &'static str> {
    Err("failed")
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "assert_throws contract requires operation returning Result"
)]
fn ok_operation() -> Result<u32, &'static str> {
    Ok(7)
}

fn benchmark_sample_ns() -> u64 {
    1_000_u64
}

fn project_config_fixture() -> ProjectConfig {
    let parsed = parse_config(
        r#"
name = "opal_project"
version = "1.0.0"
"#,
    );
    let Ok(config) = parsed else {
        return ProjectConfig {
            name: String::from("fallback"),
            version: crate::build_system::Version {
                major: 1,
                minor: 0,
                patch: 0,
            },
            dependencies: vec![],
            build_targets: vec![],
        };
    };
    config
}

#[test]
fn assertions_return_expected_results() {
    assert!(assert_true(true).is_ok(), "assert_true(true) should pass");
    assert!(
        assert_false(false).is_ok(),
        "assert_false(false) should pass"
    );
    assert!(
        opal_assert_eq(&42_u32, &42_u32).is_ok(),
        "assert_eq should pass on equal values"
    );
    assert!(
        assert_ne(&1_u32, &2_u32).is_ok(),
        "assert_ne should pass on non-equal values"
    );
    assert!(
        assert_throws(error_operation).is_ok(),
        "assert_throws should pass on error result"
    );

    assert!(
        assert_true(false).is_err(),
        "assert_true(false) should fail"
    );
    assert!(
        assert_false(true).is_err(),
        "assert_false(true) should fail"
    );
    assert!(
        opal_assert_eq(&1_u32, &2_u32).is_err(),
        "assert_eq should fail on non-equal values"
    );
    assert!(
        assert_ne(&3_u32, &3_u32).is_err(),
        "assert_ne should fail on equal values"
    );
    assert!(
        assert_throws(ok_operation).is_err(),
        "assert_throws should fail on ok result"
    );
}

#[test]
fn discovery_filters_tests_by_pattern_and_selection() {
    let tests = vec![
        TestCase::new("math_add", passing_test),
        TestCase::new("math_subtract", passing_test),
        TestCase::new("string_concat", passing_test),
    ];

    let pattern_selected = discover_tests_by_name_pattern(&tests, "math");
    assert_eq!(
        pattern_selected.len(),
        2,
        "math filter should select two tests"
    );

    let selection = TestSelection::new()
        .with_name_pattern("math")
        .exclude("math_subtract")
        .include("math_add");
    let rich_selected = discover_tests_with_selection(&tests, &selection);
    assert_eq!(
        rich_selected.len(),
        1,
        "selection should include one filtered test"
    );
    assert_eq!(
        rich_selected[0].name, "math_add",
        "selected test should be math_add"
    );
}

#[test]
fn runner_produces_report_with_pass_fail_counts() {
    let mut suite = TestSuite::new("unit_suite");
    suite.add_test(TestCase::new("unit_pass", passing_test));
    suite.add_test(TestCase::new("unit_fail", failing_test));

    let report = run_suite(&suite);
    assert_eq!(report.passed, 1, "report should count one pass");
    assert_eq!(report.failed, 1, "report should count one failure");
    assert_eq!(report.skipped, 0, "report should count zero skipped");
    assert!(!report.is_success(), "report should not be successful");

    let maybe_fail = report.outcomes.get("unit_fail");
    assert!(maybe_fail.is_some(), "unit_fail result should be present");
    let Some(result) = maybe_fail else {
        return;
    };
    assert!(
        matches!(*result, TestResult::Failed { .. }),
        "unit_fail should be marked as failed"
    );
}

#[test]
fn property_check_runs_iterations_and_shrinks_counterexample() {
    let property = PropertyTest {
        name: "value less than ten",
        generate: |iteration| iteration,
        shrink: |value| {
            if *value > 0 {
                vec![value.saturating_sub(1)]
            } else {
                vec![]
            }
        },
        property_fn: |value| *value < 10,
    };

    let result = check_property(&property, 20);
    assert!(
        matches!(result, PropertyCheckResult::Failed(_)),
        "property should fail when generated value reaches ten"
    );

    let PropertyCheckResult::Failed(failure) = result else {
        return;
    };
    assert_eq!(
        failure.iteration, 10,
        "first failing iteration should be ten"
    );
    assert_eq!(
        failure.counter_example, 10,
        "shrunk failing value should remain minimum failing case"
    );
}

#[test]
fn benchmark_computes_mean_and_stddev() {
    let benchmark = Benchmark {
        name: "constant_benchmark",
        iterations: 5,
        run_fn: benchmark_sample_ns,
    };
    let result = run_benchmark(&benchmark);
    assert_eq!(result.iterations, 5, "benchmark should run five iterations");
    assert!(
        result.mean_ns == 1_000_u64,
        "constant benchmark should have expected mean"
    );
    assert!(
        result.stddev_ns == 0_u64,
        "constant benchmark should have zero stddev"
    );
}

#[test]
fn coverage_report_computes_percentage() {
    let mut coverage_map = CoverageMap::new();
    coverage_map.mark_executed(1);
    coverage_map.mark_unexecuted(2);
    coverage_map.mark_executed(3);

    let report = generate_coverage_report(&coverage_map);
    assert_eq!(report.total_lines, 3, "coverage should track three lines");
    assert_eq!(
        report.executed_lines, 2,
        "coverage should track two executed lines"
    );
    assert!(
        (report.coverage_percent - 66.666_666_666_666_66_f64).abs() < 0.000_1_f64,
        "coverage percent should be approximately 66.67"
    );
}

#[test]
fn test_command_executes_with_filter_and_target() {
    let mut suite = TestSuite::new("filtered_suite");
    suite.add_test(TestCase::new("alpha_case", passing_test));
    suite.add_test(TestCase::new("beta_case", failing_test));

    let parsed_target = parse_target_triple("x86_64-linux");
    assert!(parsed_target.is_ok(), "target triple should parse");
    let Ok(triple) = parsed_target else {
        return;
    };
    let target = BuildTarget { triple };

    let command = TestCommand::new(project_config_fixture())
        .with_target(target)
        .with_filter("alpha");
    let report = command.execute(&suite);

    assert_eq!(
        report.passed, 1,
        "filtered command should run one passing test"
    );
    assert_eq!(
        report.failed, 0,
        "filtered command should run zero failing tests"
    );
    assert_eq!(
        report.skipped, 0,
        "filtered command should run zero skipped tests"
    );
    assert!(report.is_success(), "filtered report should be successful");
}
