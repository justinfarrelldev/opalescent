#![cfg(feature = "integration")]
#![allow(
    clippy::match_same_arms,
    clippy::pattern_type_mismatch,
    reason = "integration test control-flow matches are intentionally explicit"
)]

use super::*;
use super::fs_helpers::unique_probe_target_dir;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn run_guard_stmt_project(project_name: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("current working directory should be readable for integration tests: {error}")
    })?;
    let project_dir = cwd.join(format!("test-projects/{project_name}"));
    let temp_dir = unique_probe_target_dir(&format!("guard-stmt-{project_name}"));
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be created: {error}"))?;

    let execution_result: Result<String, String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("{project_name} project should compile into a binary: {error}")
            })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            &format!("{project_name} compiled binary"),
        )?;
        if !run_output.status.success() {
            return Err(format!(
                "{project_name} binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(String::from_utf8_lossy(&run_output.stdout).into_owned())
    })();

    let cleanup_result = cleanup_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be removed: {error}"));
    match (execution_result, cleanup_result) {
        (Ok(stdout), Ok(())) => Ok(stdout),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Err(exec_error), Err(cleanup_error)) => Err(format!("{exec_error}; {cleanup_error}")),
    }
}

fn read_expected_stdout(project_name: &str) -> Result<String, String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("current working directory should be readable for integration tests: {error}")
    })?;
    let expected_path = cwd.join(format!("test-projects/{project_name}/expected/stdout.txt"));
    fs::read_to_string(&expected_path).map_err(|error| {
        format!("{project_name} expected stdout fixture should be readable: {error}")
    })
}

fn compile_guard_stmt_project_failure(project_name: &str) -> Result<CompileError, String> {
    let cwd = std::env::current_dir().map_err(|error| {
        format!("current working directory should be readable for integration tests: {error}")
    })?;
    let project_dir = cwd.join(format!("test-projects/{project_name}"));
    let temp_dir = unique_probe_target_dir(&format!("guard-stmt-compile-{project_name}"));
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be created: {error}"))?;

    let compile_result =
        compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    let cleanup_result = cleanup_dir(&temp_dir)
        .map_err(|error| format!("{project_name} target directory should be removed: {error}"));

    match (compile_result, cleanup_result) {
        (Ok(_), Ok(())) => Err(format!(
            "{project_name} project should fail to compile, but compilation succeeded"
        )),
        (Err(error), Ok(())) => Ok(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(_), Err(cleanup_error)) => Err(cleanup_error),
    }
}

fn assert_constraint_reason_contains(
    project_name: &str,
    compile_error: CompileError,
    expected_reason: &str,
) -> Result<(), String> {
    match compile_error {
        CompileError::Report { report, .. } => {
            let has_expected_error = report.entries().iter().any(|entry| {
                if let CompilerError::TypeChecker(TypeError::ConstraintSolvingFailed {
                    reason,
                    ..
                }) = &entry.1
                {
                    reason.contains(expected_reason)
                } else {
                    false
                }
            });

            if has_expected_error {
                Ok(())
            } else {
                Err(format!(
                    "{project_name} should emit expected diagnostic substring '{expected_reason}', got: {:?}",
                    report.entries()
                ))
            }
        }
        CompileError::Type(TypeError::ConstraintSolvingFailed { reason, .. }) => {
            if reason.contains(expected_reason) {
                Ok(())
            } else {
                Err(format!(
                    "{project_name} should emit expected diagnostic substring '{expected_reason}', got type error reason: {reason}"
                ))
            }
        }
        other => Err(format!(
            "{project_name} should fail with a guard constraint diagnostic, got: {other}"
        )),
    }
}

fn assert_guard_variant(
    project_name: &str,
    compile_error: CompileError,
    variant_matches: impl Fn(&TypeError) -> bool,
) -> Result<(), String> {
    match compile_error {
        CompileError::Report { report, .. } => {
            let has_expected_error = report.entries().iter().any(|entry| match &entry.1 {
                CompilerError::TypeChecker(type_error) => variant_matches(type_error),
                _ => false,
            });

            if has_expected_error {
                Ok(())
            } else {
                Err(format!(
                    "{project_name} should emit expected strict-guard type diagnostic, got: {:?}",
                    report.entries()
                ))
            }
        }
        CompileError::Type(type_error) => {
            if variant_matches(&type_error) {
                Ok(())
            } else {
                Err(format!(
                    "{project_name} should emit expected strict-guard type diagnostic, got: {type_error:?}"
                ))
            }
        }
        other => Err(format!(
            "{project_name} should fail with a strict-guard type diagnostic, got: {other}"
        )),
    }
}

#[test]
fn guard_stmt_typed_binding_project_compiles_links_and_runs() {
    let expected_stdout = read_expected_stdout("guard-stmt-typed-binding");
    assert!(
        expected_stdout.is_ok(),
        "guard-stmt-typed-binding expected stdout should be readable"
    );
    let Ok(expected_stdout) = expected_stdout else {
        return;
    };

    let execution_result = run_guard_stmt_project("guard-stmt-typed-binding");
    let failure_message = match execution_result {
        Ok(stdout) => {
            if stdout != expected_stdout {
                format!(
                    "guard-stmt-typed-binding stdout should match expected fixture exactly, got: '{stdout}'"
                )
            } else if stdout.contains("UNEXPECTED_TYPED_BINDING_ERROR=") {
                format!(
                    "guard-stmt-typed-binding success path should not print unexpected error marker, got: '{stdout}'"
                )
            } else {
                String::new()
            }
        }
        Err(message) => message,
    };

    assert!(
        failure_message.is_empty(),
        "guard-stmt-typed-binding project should compile, run, and prove typed mutable success bindings stay available after guard completion: {failure_message}"
    );
}

#[test]
fn guard_stmt_propagate_err_project_compiles_links_and_runs() {
    let expected_stdout = read_expected_stdout("guard-stmt-propagate-err");
    assert!(
        expected_stdout.is_ok(),
        "guard-stmt-propagate-err expected stdout should be readable"
    );
    let Ok(expected_stdout) = expected_stdout else {
        return;
    };

    let execution_result = run_guard_stmt_project("guard-stmt-propagate-err");
    let failure_message = match execution_result {
        Ok(stdout) => {
            if stdout != expected_stdout {
                format!(
                    "guard-stmt-propagate-err stdout should match expected fixture exactly, got: '{stdout}'"
                )
            } else if stdout.contains("UNEXPECTED_GUARD_PROPAGATE_SUCCESS") {
                format!(
                    "guard-stmt-propagate-err failure path should not reach success marker, got: '{stdout}'"
                )
            } else {
                String::new()
            }
        }
        Err(message) => message,
    };

    assert!(
        failure_message.is_empty(),
        "guard-stmt-propagate-err project should compile, run, perform side effects, and forward the original guard error: {failure_message}"
    );
}

#[test]
fn guard_stmt_success_binding_leak_project_emits_scope_diagnostic() {
    let compile_error = compile_guard_stmt_project_failure("guard-stmt-success-binding-leak");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-success-binding-leak should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification = assert_constraint_reason_contains(
        "guard-stmt-success-binding-leak",
        compile_error,
        "success binding is not available inside guard error clause",
    );
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-success-binding-leak should emit the exact success-binding leak diagnostic: {failure_message}"
    );
}

#[test]
fn guard_stmt_only_propagate_project_emits_shorthand_guidance() {
    let compile_error = compile_guard_stmt_project_failure("guard-stmt-only-propagate");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-only-propagate should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification = assert_guard_variant("guard-stmt-only-propagate", compile_error, |error| {
        matches!(error, TypeError::GuardShorthandRequired { .. })
    });
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-only-propagate should emit the exact shorthand guidance diagnostic: {failure_message}"
    );
}

#[test]
fn guard_stmt_return_err_banned_project_emits_return_err_diagnostic() {
    let compile_error = compile_guard_stmt_project_failure("guard-stmt-return-err-banned");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-return-err-banned should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification =
        assert_guard_variant("guard-stmt-return-err-banned", compile_error, |error| {
            matches!(error, TypeError::GuardReturnErrInvalid { .. })
        });
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-return-err-banned should emit the exact return-err rejection diagnostic: {failure_message}"
    );
}

#[test]
fn guard_stmt_wrapper_valid_project_compiles_links_and_runs() {
    let execution_result = run_guard_stmt_project("guard-stmt-wrapper-valid");
    let failure_message = match execution_result {
        Ok(stdout) => {
            if !stdout.contains("WRAPPER_VALUE=42") || !stdout.contains("wrapper-valid-ok") {
                format!(
                    "guard-stmt-wrapper-valid should prove direct wrapper returns compile and run, got: '{stdout}'"
                )
            } else if stdout.contains("UNEXPECTED_WRAPPER_ERROR=") {
                format!(
                    "guard-stmt-wrapper-valid success path should not print unexpected error marker, got: '{stdout}'"
                )
            } else {
                String::new()
            }
        }
        Err(message) => message,
    };

    assert!(
        failure_message.is_empty(),
        "guard-stmt-wrapper-valid should compile and run with a direct wrapper source return: {failure_message}"
    );
}

#[test]
fn guard_stmt_wrapper_invalid_alias_project_emits_wrapper_source_diagnostic() {
    let compile_error = compile_guard_stmt_project_failure("guard-stmt-wrapper-invalid-alias");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-wrapper-invalid-alias should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification =
        assert_guard_variant("guard-stmt-wrapper-invalid-alias", compile_error, |error| {
            matches!(error, TypeError::GuardWrapperSourceInvalid { .. })
        });
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-wrapper-invalid-alias should emit the exact wrapper-source rejection diagnostic: {failure_message}"
    );
}

#[test]
fn guard_stmt_wrapper_invalid_shadowed_project_emits_wrapper_source_diagnostic() {
    let compile_error = compile_guard_stmt_project_failure("guard-stmt-wrapper-invalid-shadowed");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-wrapper-invalid-shadowed should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification = assert_guard_variant(
        "guard-stmt-wrapper-invalid-shadowed",
        compile_error,
        |error| matches!(error, TypeError::GuardWrapperSourceInvalid { .. }),
    );
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-wrapper-invalid-shadowed should emit the exact wrapper-source rejection diagnostic: {failure_message}"
    );
}

#[test]
fn guard_stmt_wrapper_invalid_missing_source_project_emits_wrapper_source_diagnostic() {
    let compile_error =
        compile_guard_stmt_project_failure("guard-stmt-wrapper-invalid-missing-source");
    assert!(
        compile_error.is_ok(),
        "guard-stmt-wrapper-invalid-missing-source should produce a compile error"
    );
    let Ok(compile_error) = compile_error else {
        return;
    };

    let verification = assert_guard_variant(
        "guard-stmt-wrapper-invalid-missing-source",
        compile_error,
        |error| matches!(error, TypeError::GuardWrapperSourceInvalid { .. }),
    );
    let failure_message = verification.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "guard-stmt-wrapper-invalid-missing-source should emit the exact wrapper-source rejection diagnostic: {failure_message}"
    );
}

#[test]
fn delete_downloads_project_compiles_and_runs_with_strict_terminal_handlers() {
    let execution_result = run_guard_stmt_project("delete-downloads");
    let failure_message = match execution_result {
        Ok(stdout) => {
            if !stdout.contains("LIST_ERR=") && !stdout.contains("removed_or_attempted=") {
                format!(
                    "delete-downloads should print LIST_ERR or removed_or_attempted marker after strict fixture fix, got: '{stdout}'"
                )
            } else {
                String::new()
            }
        }
        Err(message) => message,
    };

    assert!(
        failure_message.is_empty(),
        "delete-downloads project should compile and run with strict named-guard terminals: {failure_message}"
    );
}
