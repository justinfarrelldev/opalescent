//! Bounded fresh-process scheduling uses synchronization, not scenario locks.
#![cfg(target_os = "linux")]
use super::artifact::NativeTestArtifact;
use super::identity::CaseIdentity;
use super::scheduler::{NativeRunOptions, ScheduledCase, run_cases};
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

#[test]
fn cases_in_one_scenario_overlap_in_distinct_worker_processes() {
    let directory = tempfile::tempdir().expect("workers");
    let artifact = fixture(directory.path(), "barrier");
    let cases = selections(&artifact, false);
    let options = options(directory.path(), 2);
    let report = run_cases(&artifact, &cases, &options).expect("scheduler");
    assert_eq!(report.peak_workers, 2);
    assert!(report.cases.iter().all(|case| case.outcome.is_success()));
    assert_ne!(
        report.cases[0].attempts[0].process_id,
        report.cases[1].attempts[0].process_id
    );
}

#[test]
fn shared_exclusion_keys_serialize_cases_without_serializing_distinct_keys() {
    for shared in [true, false] {
        let directory = tempfile::tempdir().expect("workers");
        let artifact = fixture(directory.path(), "immediate");
        let cases = selections(&artifact, shared);
        let report =
            run_cases(&artifact, &cases, &options(directory.path(), 2)).expect("scheduler");
        assert!(report.cases.iter().all(|case| case.outcome.is_success()));
        assert_eq!(report.peak_workers, if shared { 1 } else { 2 });
    }
}

#[test]
fn retries_retain_failed_attempts_and_use_new_worker_processes() {
    let directory = tempfile::tempdir().expect("workers");
    let artifact = fixture(directory.path(), "retry");
    let cases = selections(&artifact, false);
    let mut options = options(directory.path(), 2);
    options.retries = 1;
    let report = run_cases(&artifact, &cases, &options).expect("scheduler");
    for case in &report.cases {
        assert_eq!(case.outcome, super::scheduler::CaseOutcome::FlakyPassed);
        assert_eq!(case.attempts.len(), 2);
        assert!(!case.attempts[0].outcome.is_success());
        assert!(case.attempts[1].outcome.is_success());
        assert_ne!(case.attempts[0].process_id, case.attempts[1].process_id);
    }
}

#[test]
fn hard_timeout_retains_last_phase_without_claiming_cleanup_finished() {
    let directory = tempfile::tempdir().expect("workers");
    let artifact = fixture(directory.path(), "timeout");
    let cases = selections(&artifact, false);
    let mut options = options(directory.path(), 2);
    options.timeout = Duration::from_millis(250);
    let report = run_cases(&artifact, &cases, &options).expect("scheduler");
    for case in &report.cases {
        assert_eq!(case.outcome, super::scheduler::CaseOutcome::TimedOut);
        assert_eq!(case.attempts[0].last_phase, Some(4));
        assert!(case.attempts[0].result.is_none());
    }
}

#[test]
fn crashed_workers_retain_last_phase_and_no_completed_result() {
    let directory = tempfile::tempdir().expect("workers");
    let artifact = fixture(directory.path(), "crash");
    let report = run_cases(
        &artifact,
        &selections(&artifact, false),
        &options(directory.path(), 2),
    )
    .expect("scheduler");
    for case in &report.cases {
        assert_eq!(case.outcome, super::scheduler::CaseOutcome::WorkerCrashed);
        assert_eq!(case.attempts[0].last_phase, Some(4));
        assert!(case.attempts[0].result.is_none());
    }
}

#[test]
fn a_worker_cannot_claim_pass_without_starting_its_test_body() {
    let directory = tempfile::tempdir().expect("workers");
    let artifact = fixture(directory.path(), "malformed");
    let cases = selections(&artifact, false);
    let report = run_cases(&artifact, &cases, &options(directory.path(), 2)).expect("scheduler");
    assert!(
        report
            .cases
            .iter()
            .all(|case| case.outcome == super::scheduler::CaseOutcome::InfrastructureFailed)
    );
}

/// Same module and scenario, with complete stable identities rather than ordinals.
fn fixture(root: &std::path::Path, mode: &str) -> NativeTestArtifact {
    let path = root.join("worker.py");
    std::fs::write(
        &path,
        format!("#!/usr/bin/env python3\nMODE={mode:?}\n{WORKER}"),
    )
    .expect("worker");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    NativeTestArtifact {
        executable: path,
        cases: ["one", "two"]
            .map(|test| CaseIdentity {
                module: "suite.test.op".to_owned(),
                scenarios: vec!["one shared scenario".to_owned()],
                test: test.to_owned(),
                example: None,
            })
            .to_vec(),
    }
}

/// The complete key set is scheduler-only metadata; subjects/Worlds are not locks.
fn selections(artifact: &NativeTestArtifact, shared: bool) -> Vec<ScheduledCase> {
    artifact
        .cases
        .iter()
        .map(|identity| ScheduledCase {
            identity: identity.clone(),
            exclusion_keys: vec![if shared {
                "device".to_owned()
            } else {
                identity.test.clone()
            }],
        })
        .collect()
}

/// Every test owns its coordinator output, not the resources created by source tests.
fn options(root: &std::path::Path, jobs: usize) -> NativeRunOptions {
    NativeRunOptions {
        jobs,
        retries: 0,
        timeout: Duration::from_secs(5),
        working_directory: root.to_path_buf(),
        application_executable: None,
        output_directory: root.join("results"),
    }
}

/// A two-sided readiness barrier fails under serial scheduling. Timeout workers
/// publish a phase before blocking; retry workers encode their attempt identity.
const WORKER: &str = r"
import json, os, pathlib, signal, sys, time
identity, attempt, output = sys.argv[1], int(sys.argv[2]), pathlib.Path(sys.argv[3])
case = json.loads(identity)
root = pathlib.Path(__file__).parent
with output.open('x') as stream:
    stream.write(json.dumps({'event':'phase','phase':4})+'\n'); stream.flush()
    if MODE == 'barrier':
        (root / ('ready-' + case['test'])).touch()
        deadline = time.monotonic() + 3
        while not all((root / ('ready-' + name)).exists() for name in ['one','two']):
            if time.monotonic() > deadline: sys.exit(2)
            time.sleep(.005)
    if MODE == 'timeout': time.sleep(30)
    if MODE == 'crash': os.kill(os.getpid(), signal.SIGKILL)
    failed = MODE == 'retry' and attempt == 1
    failures = [{'id':1,'primary_failure_id':1,'parent_failure_id':0,'kind':1,'phase':4,'message':'first attempt','file':'suite.test.op','offset':0,'length':1}] if failed else []
    stream.write(json.dumps({'event':'result','case_id':identity,'attempt':attempt,'status':'failed' if failed else 'passed','primary_failure_id':1 if failed else 0,'body_started':MODE != 'malformed','body_passed':not failed and MODE != 'malformed','initialized_worlds':0,'entered_scopes':0,'failures':failures})+'\n')
sys.exit(1 if failed else 0)
";
