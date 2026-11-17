//! Real reusable native worker artifacts, selected by stable complete identities.
#![cfg(target_os = "linux")]
use super::checking::check_project;
use super::project::NativeTestProject;
use std::path::Path;

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
    std::fs::write(&source,"world Ready:\n    setup: f(): int64 =>\n        return 1\nscenario interrupted callback:\n    worlds: Ready\n    it should retain callback progress: f(ref ready: Ready): void =>\n        expect ready.state to be 1\n        while true:\n            continue\n        return void\n").expect("source");
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
