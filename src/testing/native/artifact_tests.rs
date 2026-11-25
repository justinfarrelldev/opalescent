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
fn native_workers_execute_imported_generic_functions_with_distinct_concrete_types() {
    let directory = tempfile::tempdir().expect("project");
    std::fs::write(directory.path().join("helper.op"), "##\n    Description: Returns the input without changing its type.\n##\npublic let identity = f<T>(value: T): T =>\n    let result: T = value\n    return result\n\n##\n    Description: Requests another specialization in the defining module.\n##\npublic let forward = f<T>(value: T): T =>\n    return identity<T>(value)\n").expect("generic helper");
    let source = directory.path().join("case.test.op");
    std::fs::write(&source, "import identity as identity_value, forward from ./helper\nscenario generic application calls:\n    it should instantiate both concrete types: f(): void =>\n        expect identity_value<int64>(17) to be 17\n        expect identity_value<string>('café') to be 'café'\n        let inferred = forward('été')\n        expect inferred to be 'été'\n        return void\n").expect("test source");
    let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let checked = check_project(&project).expect("checked project");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("native artifact");
    let output = std::process::Command::new(&artifact.executable)
        .arg(serde_json::to_string(&artifact.cases[0]).expect("case identity"))
        .arg("1")
        .arg(directory.path().join("generic.jsonl"))
        .output()
        .expect("worker");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ordinary_function_parameters_preserve_native_callback_results_and_aborts() {
    let directory = tempfile::tempdir().expect("callback project");
    let source = directory.path().join("callbacks.test.op");
    std::fs::write(&source,"let apply = f(callback: f(int64): int64, input: int64): int64 =>\n    return callback(input)\nlet apply_fallible = f(callback: f(string): int32 errors ParseError): int32 errors ParseError =>\n    return propagate callback('invalid')\nlet parse = f(input: string): int32 errors ParseError =>\n    return propagate string_to_int32(input)\nlet twice = f(input: int64): int64 =>\n    return input + input\nlet fail = f(input: int64): int64 =>\n    expect false to be true\n    return input\nscenario source callbacks:\n    it should execute a typed function parameter: f(): void =>\n        expect apply(twice, 21) to be 42\n        let handled = guard apply_fallible(parse) into value: int32 else\n            9 as int32\n        expect handled to be (9 as int32)\n        return void\n    it should propagate callback aborts: f(): void =>\n        expect apply(fail, 21) to be 21\n        print('not reached')\n        return void\n").expect("source");
    let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let checked = check_project(&project).expect("checked");
    let artifact =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect("artifact");
    for (index, case) in artifact.cases.iter().enumerate() {
        let path = directory.path().join(format!("result-{index}.jsonl"));
        let output = std::process::Command::new(&artifact.executable)
            .arg(serde_json::to_string(case).expect("identity"))
            .arg("1")
            .arg(&path)
            .output()
            .expect("worker");
        assert_eq!(output.status.code(), Some(i32::from(index == 1)));
        assert!(output.stdout.is_empty());
        let transport = std::fs::read_to_string(path).expect("transport");
        let result: serde_json::Value =
            serde_json::from_str(transport.lines().last().expect("result")).expect("JSON");
        if index == 1 {
            assert_eq!(result["failures"][0]["kind"], 1_u64);
        }
    }
}

#[test]
fn foreign_function_parameters_are_rejected_before_executing_native_workers() {
    let directory = tempfile::tempdir().expect("project");
    let source = directory.path().join("foreign.test.op");
    std::fs::write(&source,"import int64_to_string from standard\nlet format_value = f(callback: f(int64): string): string =>\n    return callback(42)\nscenario foreign callback ABI:\n    it should reject a foreign callback: f(): void =>\n        expect format_value(int64_to_string) to be '42'\n        return void\n").expect("source");
    let project = NativeTestProject::load(directory.path(), &[source]).expect("project");
    let checked = check_project(&project).expect("checked");
    let error =
        super::artifact::build_native_artifact(&project, &checked, &directory.path().join("build"))
            .expect_err("foreign function cannot enter native callback ABI");
    assert!(
        error
            .to_string()
            .contains("foreign or incompatible source callback ABI"),
        "{error}"
    );
}

#[test]
fn native_test_workers_execute_local_lambdas_and_returned_source_functions() {
    use super::command::execute;
    let project = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(project.path().join("tests")).expect("test root");
    std::fs::write(project.path().join("tests/lambda.test.op"),"let plus_one = f(value: int64): int64 =>\n    return value + 1\nlet select = f(): f(int64): int64 =>\n    return plus_one\nscenario callable values:\n    it should call local and returned functions: f(): void =>\n        let local = f(value: int64): int64 =>\n            return value + 2\n        expect local(40) to be 42\n        let chosen = select()\n        expect chosen(41) to be 42\n        let alias = plus_one\n        expect alias(41) to be 42\n        expect (f(value: int64): int64 => value + 1)(41) to be 42\n        let increment = 2\n        expect (f(value: int64): int64 => value + increment)(40) to be 42\n        let caption = 'été {increment}'\n        expect (f(): string => caption)() to be 'été 2'\n        expect caption to be 'été 2'\n        return void\n    it should retain an assertion from a local function: f(): void =>\n        let local = f(): int64 =>\n            expect false to be true\n            return 1\n        expect local() to be 1\n        return void\n").expect("source");
    let result =
        execute(&["--root".to_owned(), project.path().display().to_string()]).expect("run command");
    assert_eq!(result.exit_code, 1_i32, "{}", result.text);
    assert!(
        result.text.contains("1 passed, 0 flaky, 1 failed"),
        "{}",
        result.text
    );
    assert!(result.text.contains("assertion_failed:"), "{}", result.text);
}

#[test]
fn native_array_callbacks_preserve_completed_values_and_assertion_aborts() {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("tests")).expect("tests");
    std::fs::write(root.path().join("tests/arrays.test.op"),"let twice = f(value: int64): int64 =>\n    return value + value\nlet fails = f(value: int64): int64 =>\n    expect false to be true\n    return value\nlet keep = f(value: string): boolean =>\n    expect value not to be 'fail'\n    return true\nscenario source array callbacks:\n    it should map typed values: f(): void =>\n        let input: int64[] = [1, 2]\n        expect input.map(twice) to contain exactly [2, 4] in order\n        let multiplier = 3\n        expect input.map(f(value: int64): int64 => value * multiplier) to be [3, 6]\n        expect input.filter(f(value: int64): boolean => value < multiplier) to be [1, 2]\n        expect input.reduce(0, f(acc: int64, value: int64): int64 => acc + value * multiplier) to be 9\n        let words: string[] = ['café', 'été']\n        expect words.filter(keep) to be ['café', 'été']\n        expect words to be ['café', 'été']\n        return void\n    it should abort rather than store an application payload: f(): void =>\n        let input: int64[] = [1]\n        expect input.map(fails) to be [1]\n        return void\n    it should abort inside an inline array callback: f(): void =>\n        let input: int64[] = [1]\n        let threshold = 2\n        let result = input.map(f(value: int64): int64 =>\n            expect value to be threshold\n            return value\n        )\n        expect result to be [1]\n        return void\n    it should abort after keeping a managed element: f(): void =>\n        let words: string[] = ['café', 'fail']\n        expect words.filter(keep) to be ['café']\n        return void\n").expect("source");
    let project = NativeTestProject::load(root.path(), &[root.path().join("tests/arrays.test.op")])
        .expect("project");
    let checked = check_project(&project).expect("checked");
    let llvm = inkwell::context::Context::create();
    let ctx = crate::codegen::context::CodegenContext::new(&llvm, "array_abort_cleanup");
    crate::codegen::native_project::emit_checked_project(&ctx, &project, &checked)
        .expect("LLVM suite");
    let ir = ctx.module.print_to_string().to_string();
    assert!(
        ir.lines()
            .filter(|line| line.contains("load") && line.contains("map.partial.owner"))
            .count()
            >= 2,
        "each map callback abort must release its partial result"
    );
    let result = super::command::execute(&["--root".to_owned(), root.path().display().to_string()])
        .expect("run");
    assert_eq!(result.exit_code, 1_i32, "{}", result.text);
    assert!(
        result.text.contains("1 passed, 0 flaky, 3 failed"),
        "{}",
        result.text
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
