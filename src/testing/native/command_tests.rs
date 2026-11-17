//! Native compiler CLI discovery/listing/selection/reporting executes real cases.
#![cfg(target_os = "linux")]
use super::command::execute;
use std::path::Path;

#[test]
fn list_discovers_independent_examples_without_creating_build_products() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--list".to_owned(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native list");
    assert_eq!(output.exit_code, 0_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("case report");
    assert_eq!(report["cases"].as_array().expect("cases").len(), 3);
    assert_eq!(report["coverage"], "not_collected");
    assert!(
        report["by_world"][0]["documentation"]
            .as_str()
            .expect("retained World documentation")
            .contains("per-attempt nominal environment")
    );
    assert!(
        report["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .all(|case| case["execution"].is_null())
    );
}

#[test]
fn listing_a_world_never_builds_or_invokes_its_source_setup() {
    let directory = tempfile::tempdir().expect("private source root");
    std::fs::create_dir_all(directory.path().join("tests")).expect("test root");
    std::fs::write(directory.path().join("tests/world.test.op"), "world Environment:\n    setup: f(): int64 =>\n        print('setup must not execute during discovery')\n        return 0\nscenario listing:\n    worlds: Environment\n    it should only be discovered: f(ref environment: Environment): void =>\n        expect environment.state to be 0\n        return void\n").expect("source");
    let output = execute(&[
        "--root".to_owned(),
        directory.path().display().to_string(),
        "--list".to_owned(),
    ])
    .expect("listing");
    assert_eq!(output.exit_code, 0_i32);
    assert!(!directory.path().join("target").exists());
    assert!(
        !output
            .text
            .contains("setup must not execute during discovery")
    );
}

#[test]
fn exact_case_selection_executes_one_example_and_reports_other_cases_skipped() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let base = vec![
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ];
    let mut listing = base.clone();
    listing.push("--list".to_owned());
    let listed: serde_json::Value =
        serde_json::from_str(&execute(&listing).expect("list").text).expect("listing JSON");
    let case = listed["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| !case["identity"]["example"].is_null())
        .expect("example");
    let mut args = base;
    args.extend([
        "--case".to_owned(),
        case["id"].as_str().expect("case id").to_owned(),
        "--jobs".to_owned(),
        "2".to_owned(),
    ]);
    let output = execute(&args).expect("native execution");
    assert_eq!(output.exit_code, 0_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("result JSON");
    assert_eq!(report["totals"]["passed"], 1_u64);
    assert_eq!(report["totals"]["skipped"], 2_u64);
}

#[test]
fn no_matches_is_nonzero_unless_explicitly_allowed() {
    let directory = tempfile::tempdir().expect("empty project");
    let mut args = vec!["--root".to_owned(), directory.path().display().to_string()];
    assert_ne!(execute(&args).expect("selection outcome").exit_code, 0_i32);
    args.push("--allow-empty".to_owned());
    assert_eq!(
        execute(&args).expect("allowed empty selection").exit_code,
        0_i32
    );
}

#[test]
fn junit_output_is_validly_escaped_and_counts_independent_cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--list".to_owned(),
        "--reporter".to_owned(),
        "junit".to_owned(),
    ])
    .expect("JUnit list");
    assert!(output.text.contains("<testsuite"));
    assert!(output.text.contains("tests=\"3\""));
    assert_eq!(output.text.matches("<testcase ").count(), 3);
    let directory = tempfile::tempdir().expect("XML validation");
    let path = directory.path().join("report.xml");
    std::fs::write(&path, output.text).expect("XML report");
    let parsed = std::process::Command::new("python3").args(["-c", "import sys, xml.etree.ElementTree as E; r=E.parse(sys.argv[1]).getroot(); assert len(r.findall('testcase')) == 3"]).arg(path).status().expect("XML parser");
    assert!(parsed.success());
}

#[test]
fn canonical_association_filters_and_report_files_preserve_case_totals() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let directory = tempfile::tempdir().expect("report output");
    for (flag, value, count) in [
        ("--unit", "src/arithmetic.op::add", 3_u64),
        ("--world", "tests/addition.test.op::StartingNumber", 1_u64),
        ("--kind", "integration", 0_u64),
    ] {
        let path = directory.path().join(format!("{flag}.json"));
        let args = [
            "--root".to_owned(),
            root.display().to_string(),
            "--list".to_owned(),
            flag.to_owned(),
            value.to_owned(),
            "--reporter".to_owned(),
            "json".to_owned(),
            "--output".to_owned(),
            path.display().to_string(),
        ];
        let output = execute(&args).expect("filtered report");
        let report: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("written report"))
                .expect("JSON report");
        assert_eq!(report["totals"]["listed"], count);
        assert_eq!(output.written_to, Some(path));
        assert_eq!(output.exit_code, i32::from(count == 0));
    }
}

#[test]
fn compiled_string_matchers_preserve_utf8_values_and_owned_failure_diagnostics() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-matchers");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native string matchers");
    assert_eq!(output.exit_code, 1_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("JSON results");
    assert_eq!(report["totals"]["passed"], 2_u64);
    assert_eq!(report["totals"]["failed"], 2_u64);
    let failed = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| {
            case["identity"]["module"] == "tests/strings.test.op"
                && case["execution"]["outcome"] == "assertion_failed"
        })
        .expect("intentional mismatch");
    let message = failed["execution"]["attempts"][0]["result"]["failures"][0]["message"]
        .as_str()
        .expect("owned diagnostic");
    assert!(message.contains("café actual"));
    assert!(message.contains("different expected"));
    assert!(message.contains("string"));
    let observed = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["identity"]["test"] == "evaluate both operands once in source order")
        .expect("observation mismatch");
    let stdout = observed["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("capture path");
    assert_eq!(
        std::fs::read_to_string(stdout)
            .expect("evaluation trace")
            .lines()
            .collect::<Vec<_>>(),
        ["actual evaluated", "expected evaluated"]
    );
    let captured = observed["execution"]["attempts"][0]["result"]["failures"][0]["message"]
        .as_str()
        .expect("owned managed string snapshot");
    assert!(captured.contains("observed actual 41"));
    assert!(captured.contains("observed expected 42"));
}

#[test]
fn compiled_array_matchers_compare_typed_elements_lengths_order_and_multiplicity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-arrays");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native array matchers");
    assert_eq!(output.exit_code, 1_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("JSON array results");
    assert_eq!(report["totals"]["passed"], 1_u64);
    assert_eq!(report["totals"]["failed"], 2_u64);
}

#[test]
fn compiled_products_use_field_equality_and_variants_use_nominal_tags() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-products");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native product and variant matchers");
    assert_eq!(output.exit_code, 1_i32);
    let report: serde_json::Value =
        serde_json::from_str(&output.text).expect("JSON product results");
    let diagnostic_text = report.to_string();
    assert!(
        diagnostic_text.contains("Document{"),
        "product fields must be captured before lexical cleanup"
    );
    assert!(
        diagnostic_text.contains("Status.Open"),
        "variant diagnostics must capture genuine observed identity"
    );
    assert_eq!(report["totals"]["passed"], 2_u64);
    assert_eq!(report["totals"]["failed"], 2_u64);
}

#[test]
fn compiled_polling_evaluates_configuration_once_and_retains_latest_observation() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-polling");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native polling");
    assert_eq!(output.exit_code, 1_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("polling results");
    assert_eq!(report["totals"]["passed"], 3_u64);
    assert_eq!(report["totals"]["failed"], 4_u64);
    let snapshots = report.to_string();
    assert!(snapshots.contains("actual (int64) -42"));
    assert!(snapshots.contains("[[41, 42"));
    let passed = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["execution"]["outcome"] == "passed")
        .expect("successful retry");
    let path = passed["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("trace");
    assert_eq!(
        std::fs::read_to_string(path)
            .expect("evaluation trace")
            .lines()
            .collect::<Vec<_>>(),
        [
            "expected evaluated",
            "deadline evaluated",
            "observation evaluated",
            "observation evaluated",
            "observation evaluated"
        ]
    );
    let failed = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["execution"]["outcome"] == "assertion_failed")
        .expect("expired observation");
    let message = failed["execution"]["attempts"][0]["result"]["failures"][0]["message"]
        .as_str()
        .expect("latest snapshot");
    assert!(message.contains("latest observed 42"));
    let failed_path = failed["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("failed trace");
    assert_eq!(
        std::fs::read_to_string(failed_path)
            .expect("observation trace")
            .lines()
            .count(),
        1
    );
    let error_case = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| case["execution"]["outcome"] == "application_error")
        .expect("immediate application error");
    let error_path = error_case["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("application trace");
    assert_eq!(
        std::fs::read_to_string(error_path)
            .expect("application observation trace")
            .lines()
            .collect::<Vec<_>>(),
        ["failing observation evaluated", "after failed observation"]
    );
}

#[test]
fn compiled_double_activation_restores_handles_and_aborts_conflicts_independently() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-activation");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("native double activation");
    assert_eq!(output.exit_code, 2_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("activation report");
    assert_eq!(report["totals"]["passed"], 5_u64);
    assert_eq!(report["totals"]["failed"], 3_u64);
    assert!(
        report
            .to_string()
            .contains("unexpected invocation of active double")
    );
    assert!(report.to_string().contains("conflicting active doubles"));
    let aborted = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| {
            case["identity"]["test"]
                == "abort an unconfigured active edge before returning an application value"
        })
        .expect("unconfigured call");
    let trace = aborted["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("argument trace");
    assert_eq!(
        std::fs::read_to_string(trace).expect("argument and cleanup trace"),
        "argument evaluated\nworld writer restored\n"
    );
    let world_case = report["cases"]
        .as_array()
        .expect("activation cases")
        .iter()
        .find(|case| {
            case["identity"]["test"] == "restore an initialized World after an activation abort"
        })
        .expect("World case");
    let path = world_case["execution"]["attempts"][0]["stdout_path"]
        .as_str()
        .expect("cleanup trace");
    assert_eq!(
        std::fs::read_to_string(path).expect("World cleanup trace"),
        "world writer restored\n"
    );
}

#[test]
fn compiled_exact_double_rules_return_values_and_preserve_guarded_nominal_errors() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-doubles");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("compiled exact rules");
    assert_eq!(output.exit_code, 0_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("double report");
    assert_eq!(report["totals"]["passed"], 2_u64);
    assert_eq!(report["totals"]["failed"], 0_u64);
}

#[test]
fn compiled_scalar_rules_snapshot_borrows_and_reject_duplicates_unused_rules_and_missing_history() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-double-values");
    super::project::NativeTestProject::load(&root, &[root.join("tests/rules.test.op")])
        .expect("valid typed scalar fixture");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("typed scalar rules");
    assert_eq!(output.exit_code, 2_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("scalar rule report");
    assert_eq!(report["totals"]["passed"], 3_u64);
    assert_eq!(report["totals"]["failed"], 3_u64);
    let diagnostics = report.to_string();
    assert!(diagnostics.contains("duplicate or overlapping exact double rules"));
    assert!(diagnostics.contains("never exercised"));
    assert!(diagnostics.contains("call history did not contain"));
    let unused = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| {
            case["identity"]["test"] == "diagnose unused exact rules at their source location"
        })
        .expect("unused rule case");
    let offset = unused["execution"]["attempts"][0]["result"]["failures"][0]["offset"]
        .as_u64()
        .expect("original source offset");
    assert!(offset > 0);
}

#[test]
fn compiled_replacements_adapt_success_preserve_errors_and_propagate_test_aborts() {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-replacements");
    let output = execute(&[
        "--root".to_owned(),
        root.display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("compiled replacement fallback");
    assert_eq!(output.exit_code, 1_i32);
    let report: serde_json::Value = serde_json::from_str(&output.text).expect("replacement report");
    assert_eq!(report["totals"]["passed"], 4_u64);
    assert_eq!(report["totals"]["failed"], 1_u64);
    let aborted = report["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|case| {
            case["identity"]["test"]
                == "propagate replacement assertion aborts before application guards"
        })
        .expect("aborted replacement");
    assert_eq!(aborted["execution"]["outcome"], "assertion_failed");
}

#[test]
fn compiler_errors_are_machine_distinguishable_from_assertion_failures() {
    let directory = tempfile::tempdir().expect("invalid source");
    std::fs::create_dir_all(directory.path().join("tests")).expect("test root");
    std::fs::write(directory.path().join("tests/invalid.test.op"), "scenario invalid:\n    unit under test: missing_function\n    it should fail compilation: f(): void =>\n        return void\n").expect("invalid subject source");
    let output = execute(&[
        "--root".to_owned(),
        directory.path().display().to_string(),
        "--reporter".to_owned(),
        "json".to_owned(),
    ])
    .expect("compilation report");
    assert_eq!(output.exit_code, 2_i32);
    let report: serde_json::Value =
        serde_json::from_str(&output.text).expect("JSON compiler error");
    assert_eq!(report["error"]["kind"], "compilation");
    assert!(report["cases"].as_array().expect("cases").is_empty());
}
