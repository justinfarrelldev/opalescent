//! Real reusable native worker artifacts, selected by stable complete identities.
#![cfg(target_os = "linux")]
use super::checking::check_project;
use super::project::NativeTestProject;
use std::path::Path;

#[test]
fn module_value_accessors_are_lazy_cached_and_resolve_import_aliases_in_native_workers() {
    let directory = tempfile::tempdir().expect("project");
    std::fs::write(directory.path().join("values.op"),"##\n    Description: A shared computed value.\n##\npublic let cached: int64 = calculate()\n\nlet calculate = f(): int64 =>\n    print('initialized')\n    return 42\n").expect("value source");
    let source = directory.path().join("case.test.op");
    std::fs::write(&source,"import cached as answer from ./values\nlet broken: int64 = fail_initializer()\nlet fail_initializer = f(): int64 =>\n    expect false to be true\n    return 99\nscenario module values:\n    it should preserve lazy value caching: f(): void =>\n        expect answer to be 42\n        expect answer to be 42\n        return void\n    it should propagate initializer abort: f(): void =>\n        expect broken to be 99\n        print('not reached')\n        return void\n").expect("test source");
    let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let checked = check_project(&project).expect("checked");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("native artifact");
    let id = serde_json::to_string(&artifact.cases[0]).expect("identity");
    for index in 0..2_u32 {
        let output = std::process::Command::new(&artifact.executable)
            .arg(&id)
            .arg("1")
            .arg(directory.path().join(format!("attempt-{index}.jsonl")))
            .output()
            .expect("worker");
        assert!(output.status.success());
        assert_eq!(output.stdout, b"initialized\n");
    }
    let result_path = directory.path().join("abort.jsonl");
    let aborted = std::process::Command::new(&artifact.executable)
        .arg(serde_json::to_string(&artifact.cases[1]).expect("identity"))
        .arg("1")
        .arg(&result_path)
        .output()
        .expect("aborting worker");
    assert_eq!(aborted.status.code(), Some(1_i32));
    assert!(aborted.stdout.is_empty());
    let transport = std::fs::read_to_string(result_path).expect("failure transport");
    let result: serde_json::Value =
        serde_json::from_str(transport.lines().last().expect("result")).expect("result JSON");
    assert_eq!(result["failures"][0]["kind"], 1_u64);
    assert_eq!(
        result["failures"][0]["offset"],
        u64::try_from(
            std::fs::read_to_string(directory.path().join("case.test.op"))
                .expect("source")
                .find("expect false")
                .expect("initializer assertion")
        )
        .expect("offset")
    );
}

#[test]
fn built_worker_selects_independent_examples_from_one_native_executable() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("sources");
    let checked = check_project(&project).expect("checked source");
    let directory = tempfile::tempdir().expect("artifact output");
    let artifact = super::artifact::build_native_artifact(&project, &checked, directory.path())
        .expect("native worker artifact");
    assert_eq!(artifact.cases.len(), 3);
    for (index, case) in artifact.cases.iter().enumerate() {
        let output = directory.path().join(format!("attempt-{index}.jsonl"));
        let id = serde_json::to_string(case).expect("stable case identity");
        let status = std::process::Command::new(&artifact.executable)
            .arg(&id)
            .arg("1")
            .arg(&output)
            .status()
            .expect("fresh native worker");
        assert!(status.success());
        let transport = std::fs::read_to_string(output).expect("owned result transport");
        let events: Vec<serde_json::Value> = transport
            .lines()
            .map(|line| serde_json::from_str(line).expect("structured worker event"))
            .collect();
        assert!(events.iter().any(|event| event["event"] == "phase"));
        assert!(events.iter().any(|event| event["event"] == "callback"
            && event["stage"] == "finish"
            && event["phase"] == 4_u64
            && event["passed"] == true
            && event["duration_ms"].is_u64()));
        let result = events.last().expect("terminal result");
        assert_eq!(result["event"], "result");
        assert_eq!(result["case_id"], id);
        assert_eq!(result["attempt"], 1_u64);
        assert_eq!(result["status"], "passed");
        assert_eq!(result["failures"], serde_json::json!([]));
    }
}

#[test]
fn scheduler_executes_real_native_cases_with_bounded_fresh_workers() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/addition.test.op")]).expect("sources");
    let checked = check_project(&project).expect("checked source");
    let directory = tempfile::tempdir().expect("artifact output");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("native artifact");
    let cases: Vec<_> = artifact
        .cases
        .iter()
        .map(|identity| super::scheduler::ScheduledCase {
            identity: identity.clone(),
            exclusion_keys: Vec::new(),
        })
        .collect();
    let options = super::scheduler::NativeRunOptions {
        jobs: 2,
        retries: 0,
        timeout: std::time::Duration::from_secs(5),
        working_directory: root,
        application_executable: None,
        output_directory: directory.path().join("attempts"),
    };
    let report =
        super::scheduler::run_cases(&artifact, &cases, &options).expect("native scheduling");
    assert_eq!(report.cases.len(), 3);
    assert!(
        report
            .cases
            .iter()
            .all(|case| case.outcome == super::scheduler::CaseOutcome::Passed)
    );
    assert!(report.peak_workers <= 2);
}

#[test]
fn timeout_retains_completed_callback_metrics_and_marks_only_the_active_boundary_interrupted() {
    let directory = tempfile::tempdir().expect("timeout project");
    let source = directory.path().join("timeout.test.op");
    std::fs::write(&source,"import attach_text from standard.testing\nworld Ready:\n    setup: f(): int64 =>\n        return 1\nscenario interrupted callback:\n    worlds: Ready\n    it should retain callback progress: f(ref ready: Ready, mutable ref context: TestContext): void =>\n        expect ready.state to be 1\n        attach_text(mutable ref context, 'before timeout', 'retained attachment')\n        while true:\n            continue\n        return void\n").expect("source");
    let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let checked = check_project(&project).expect("checked");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("artifact");
    let cases = vec![super::scheduler::ScheduledCase {
        identity: artifact.cases[0].clone(),
        exclusion_keys: Vec::new(),
    }];
    let options = super::scheduler::NativeRunOptions {
        jobs: 1,
        retries: 0,
        timeout: std::time::Duration::from_millis(500),
        working_directory: directory.path().to_owned(),
        application_executable: None,
        output_directory: directory.path().join("attempts"),
    };
    let report = super::scheduler::run_cases(&artifact, &cases, &options).expect("timeout report");
    let attempt = &report.cases[0].attempts[0];
    assert_eq!(attempt.outcome, super::scheduler::CaseOutcome::TimedOut);
    assert!(attempt.result.is_none());
    assert!(attempt.callbacks.iter().any(|callback| callback.phase == 2
        && callback.outcome == "passed"
        && callback.duration_ms.is_some()));
    let last = attempt.callbacks.last().expect("active callback");
    assert_eq!(last.phase, 4_u32);
    assert_eq!(last.outcome, "interrupted");
    assert!(last.duration_ms.is_none());
    assert_eq!(attempt.attachments.len(), 1);
    assert_eq!(attempt.attachments[0].text, "retained attachment");
}

#[test]
fn real_examples_with_one_world_subject_and_double_overlap_with_independent_configurations() {
    let template =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-parallel-doubles");
    let directory = tempfile::tempdir().expect("private barrier project");
    for relative in [
        "src/io.op",
        "src/app.op",
        "tests/parallel.types.op",
        "tests/parallel.test.op",
    ] {
        let destination = directory.path().join(relative);
        std::fs::create_dir_all(destination.parent().expect("source parent"))
            .expect("source directory");
        std::fs::copy(template.join(relative), destination).expect("fixture source");
    }
    let project = NativeTestProject::load(
        directory.path(),
        &[directory.path().join("tests/parallel.test.op")],
    )
    .expect("source project");
    let checked = check_project(&project).expect("checked project");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("native artifact");
    let cases: Vec<_> = artifact
        .cases
        .iter()
        .map(|identity| super::scheduler::ScheduledCase {
            identity: identity.clone(),
            exclusion_keys: Vec::new(),
        })
        .collect();
    let options = super::scheduler::NativeRunOptions {
        jobs: 2,
        retries: 0,
        timeout: std::time::Duration::from_secs(5),
        working_directory: directory.path().to_owned(),
        application_executable: None,
        output_directory: directory.path().join("attempts"),
    };
    let report = super::scheduler::run_cases(&artifact, &cases, &options)
        .expect("native concurrency report");
    assert_eq!(report.peak_workers, 2);
    assert!(
        report
            .cases
            .iter()
            .all(|case| case.outcome == super::scheduler::CaseOutcome::Passed)
    );
    assert_ne!(
        report.cases[0].attempts[0].process_id,
        report.cases[1].attempts[0].process_id
    );
}

#[test]
fn worker_protocol_is_separate_from_stdout_and_preserves_unicode_identities() {
    let directory = tempfile::tempdir().expect("source and output");
    let path = directory.path().join("logs.test.op");
    std::fs::write(&path, "scenario people's # β observations:\n    it should write diagnostics: f(): void =>\n        print('worker stdout is not protocol')\n        expect true to be true\n        return void\n").expect("source");
    let project = NativeTestProject::load(directory.path(), &[path]).expect("source project");
    let checked = check_project(&project).expect("checked source");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("artifact");
    let id = serde_json::to_string(&artifact.cases[0]).expect("identity");
    let result_path = directory.path().join("result.jsonl");
    let output = std::process::Command::new(&artifact.executable)
        .arg(&id)
        .arg("1")
        .arg(&result_path)
        .output()
        .expect("worker");
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("worker stdout is not protocol"));
    let transport = std::fs::read_to_string(&result_path).expect("result transport");
    let result: serde_json::Value =
        serde_json::from_str(transport.lines().last().expect("terminal event"))
            .expect("result JSON");
    assert_eq!(result["case_id"], id);
    let rejected_path = directory.path().join("unknown.jsonl");
    let rejected = std::process::Command::new(&artifact.executable)
        .arg("not a case")
        .arg("1")
        .arg(&rejected_path)
        .status()
        .expect("unknown selection");
    assert_eq!(rejected.code(), Some(2_i32));
    let infrastructure: serde_json::Value = serde_json::from_str(
        std::fs::read_to_string(rejected_path)
            .expect("infrastructure transport")
            .trim(),
    )
    .expect("infrastructure JSON");
    assert_eq!(infrastructure["event"], "infrastructure");
    // Never overwrite or reinterpret a previous attempt's result as fresh output.
    let stale = std::process::Command::new(&artifact.executable)
        .arg(&id)
        .arg("1")
        .arg(&result_path)
        .status()
        .expect("stale result rejection");
    assert_eq!(stale.code(), Some(2_i32));
    assert_eq!(
        std::fs::read_to_string(result_path).expect("original result"),
        transport
    );
}

#[test]
fn worker_failure_transport_preserves_primary_and_completed_cleanup_state() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-lifecycle");
    let project =
        NativeTestProject::load(&root, &[root.join("tests/failure.test.op")]).expect("source");
    let checked = check_project(&project).expect("checked source");
    let directory = tempfile::tempdir().expect("artifact output");
    let artifact = super::artifact::build_native_artifact(&project, &checked, directory.path())
        .expect("native artifact");
    let output = directory.path().join("failed.jsonl");
    let id = serde_json::to_string(&artifact.cases[0]).expect("identity");
    let status = std::process::Command::new(&artifact.executable)
        .arg(&id)
        .arg("2")
        .arg(&output)
        .status()
        .expect("worker");
    assert_eq!(status.code(), Some(1_i32));
    let transport = std::fs::read_to_string(output).expect("failure transport");
    let result: serde_json::Value = serde_json::from_str(transport.lines().last().expect("result"))
        .expect("structured failure");
    assert_eq!(result["attempt"], 2_u64);
    assert_eq!(result["status"], "failed");
    assert_eq!(result["failures"].as_array().expect("failures").len(), 1);
    assert_eq!(result["failures"][0]["id"], result["primary_failure_id"]);
    assert_eq!(result["failures"][0]["kind"], 1_u64);
    assert!(result["body_started"].as_bool().expect("body status"));
}
