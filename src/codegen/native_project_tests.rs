//! Checked source projects compile into reusable executable native case artifacts.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::testing::native::{checking::check_project, project::NativeTestProject};
use inkwell::context::Context;
use std::path::Path;

#[test]
fn checked_basic_project_emits_and_executes_all_independent_cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-basic");
    let project = NativeTestProject::load(&root, &[root.join("tests/addition.test.op")])
        .expect("source project");
    compile_and_execute(&project, 3, 3, false);
}

#[test]
fn checked_lifecycle_project_executes_canonical_dependency_slots_and_source_hooks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-lifecycle");
    let project = NativeTestProject::load(&root, &[root.join("tests/diamond.test.op")])
        .expect("lifecycle source project");
    compile_and_execute(&project, 1, 9, false);
}

#[test]
fn checked_source_abort_runs_fresh_after_hook_and_declared_cleanup_boundaries() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("test-projects/native-testing-lifecycle");
    let project = NativeTestProject::load(&root, &[root.join("tests/failure.test.op")])
        .expect("failure source project");
    compile_and_execute(&project, 1, 4, true);
}

/// Compile one checked artifact, then run each descriptor in its own process.
fn compile_and_execute(
    project: &NativeTestProject,
    cases: usize,
    callbacks: usize,
    expected_failure: bool,
) {
    let checked = check_project(project).expect("checked source");
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_basic_project");
    let compiled = super::native_project::emit_checked_project(&codegen, project, &checked)
        .expect("compiled native case artifacts");
    assert_eq!(compiled.cases.len(), cases);
    assert_eq!(compiled.source_callbacks.len(), callbacks);
    codegen
        .module
        .verify()
        .expect("valid whole-project native LLVM artifact");
    let mut declarations = String::new();
    let mut entries = Vec::new();
    for case in &compiled.cases {
        use core::fmt::Write;
        writeln!(
            declarations,
            "extern const OpalTestCaseDescriptor {};",
            case.symbol
        )
        .expect("declaration");
        entries.push(format!("&{}", case.symbol));
    }
    let table = entries.join(",");
    let expected = u8::from(expected_failure);
    let driver_text = format!(
        "#define EXPECTED_FAILURE {expected}\n#include \"opal_testing_lifecycle.h\"\n{declarations}\nstatic const OpalTestCaseDescriptor *cases[] = {{{table}}};\n{DRIVER}"
    );
    let directory = tempfile::tempdir().expect("driver");
    let driver = directory.path().join("driver.c");
    std::fs::write(&driver, driver_text).expect("native driver");
    let runtime = Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    execute_module_with_sources(
        &codegen,
        &[
            driver,
            runtime.join("opal_testing_runtime.c"),
            runtime.join("opal_testing_lifecycle.c"),
            runtime.join("opal_error.c"),
            runtime.join("opal_rc.c"),
        ],
    );
}

/// Every descriptor uses the same linked artifact in a separate native process.
const DRIVER: &str = r#"
#include <sys/wait.h>
#include <unistd.h>
#include <stdio.h>
int main(void) {
    for (size_t index = 0; index < sizeof(cases)/sizeof(cases[0]); ++index) {
        pid_t child = fork();
        if (child < 0) return 1;
        if (!child) {
            if (!opal_test_attempt_begin("compiled project case", 1)) _exit(2);
            OpalTestLifecycleResult result;
            int failed = opal_test_run_case(cases[index], &result);
            int wrong = failed != EXPECTED_FAILURE || opal_test_failure_count() != EXPECTED_FAILURE;
            if (wrong) {
                const OpalTestFailure *failure = opal_test_failure_at(0);
                if (failure) fprintf(stderr, "case %zu: %s\n", index, failure->message);
            }
            opal_test_attempt_end(); _exit(wrong ? 3 : 0);
        }
        int status = 0;
        if (waitpid(child, &status, 0) != child || !WIFEXITED(status) || WEXITSTATUS(status)) return 4;
    }
    return 0;
}
"#;
