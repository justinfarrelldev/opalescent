extern crate alloc;

use crate::benchmarks::compile_time::{bench_codegen, bench_parse, bench_typecheck};
use crate::benchmarks::memory::{estimate_memory_metrics, MeasuredBenchmark};
use crate::benchmarks::regression::{check_regression, RegressionThreshold};
use crate::benchmarks::runtime_bench::{
    bench_array_sort_runtime, bench_fibonacci_runtime, bench_string_ops_runtime,
};
use crate::benchmarks::suite::BenchmarkSuite;
use alloc::string::String;

#[test]
fn compile_time_benchmarks_return_named_results() {
    let source = "public main = f(): void => { return void }";

    let parse_result = bench_parse(source);
    let typecheck_result = bench_typecheck(source);
    let codegen_result = bench_codegen(source);

    assert_eq!(
        parse_result.name,
        String::from("compile_parse"),
        "parse benchmark should report canonical name"
    );
    assert_eq!(
        typecheck_result.name,
        String::from("compile_typecheck"),
        "typecheck benchmark should report canonical name"
    );
    assert_eq!(
        codegen_result.name,
        String::from("compile_codegen"),
        "codegen benchmark should report canonical name"
    );

    assert!(parse_result.iterations > 0, "parse benchmark should run");
    assert!(
        typecheck_result.iterations > 0,
        "typecheck benchmark should run"
    );
    assert!(
        codegen_result.iterations > 0,
        "codegen benchmark should run"
    );
}

#[test]
fn runtime_benchmarks_return_named_results() {
    let fibonacci_result = bench_fibonacci_runtime();
    let sort_result = bench_array_sort_runtime();
    let string_result = bench_string_ops_runtime();

    assert_eq!(
        fibonacci_result.name,
        String::from("runtime_fibonacci"),
        "fibonacci benchmark should report canonical name"
    );
    assert_eq!(
        sort_result.name,
        String::from("runtime_array_sort"),
        "array sort benchmark should report canonical name"
    );
    assert_eq!(
        string_result.name,
        String::from("runtime_string_ops"),
        "string benchmark should report canonical name"
    );

    assert!(
        fibonacci_result.mean_ns > 0 || fibonacci_result.stddev_ns == 0,
        "fibonacci benchmark should return non-negative timing stats"
    );
}

#[test]
fn suite_collects_results_and_reports_by_name() {
    let mut suite = BenchmarkSuite::new();
    suite.add_result(bench_parse("public main = f(): void => { return void }"));
    suite.add_result(bench_fibonacci_runtime());

    let report = suite.report();
    assert_eq!(report.results.len(), 2, "suite should collect two results");
    assert!(
        report.by_name.contains_key("compile_parse"),
        "report should index parse benchmark"
    );
    assert!(
        report.by_name.contains_key("runtime_fibonacci"),
        "report should index runtime benchmark"
    );
}

#[test]
fn regression_detection_reports_violation_and_success() {
    let result = bench_parse("public main = f(): void => { return void }");

    let permissive_threshold = RegressionThreshold {
        name: String::from("compile_parse"),
        max_mean_ns: u64::MAX,
        max_stddev_ns: u64::MAX,
    };
    let permissive_check = check_regression(&result, &permissive_threshold);
    assert!(
        permissive_check.is_ok(),
        "permissive threshold should not fail"
    );

    let strict_threshold = RegressionThreshold {
        name: String::from("compile_parse"),
        max_mean_ns: 0,
        max_stddev_ns: 0,
    };
    let strict_check = check_regression(&result, &strict_threshold);
    assert!(
        strict_check.is_err(),
        "strict threshold should fail for real benchmark data"
    );
}

#[test]
fn measured_benchmark_wraps_timing_and_memory_metrics() {
    let timing = bench_string_ops_runtime();
    let memory = estimate_memory_metrics(0x4000, 42);
    let measured = MeasuredBenchmark::new(timing.clone(), memory);

    assert_eq!(
        measured.result, timing,
        "measured benchmark should retain timing result"
    );
    assert_eq!(
        measured.memory.peak_bytes, 0x4000,
        "measured benchmark should retain peak byte estimate"
    );
    assert_eq!(
        measured.memory.allocations, 42,
        "measured benchmark should retain allocation estimate"
    );
}

#[test]
fn hot_reload_swap_measurement_returns_named_result() {
    let result = BenchmarkSuite::measure_hot_reload_swap();

    assert_eq!(
        result.name,
        String::from("hot_reload_swap"),
        "hot reload benchmark should report canonical name"
    );
    assert!(
        result.iterations > 0,
        "hot reload benchmark should execute iterations"
    );
}
