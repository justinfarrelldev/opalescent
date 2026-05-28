#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{strip_crlf, unique_probe_target_dir};
use super::*;
use opalescent::error::LexError;
use opalescent::errors::reporter::CompilerError;
use serial_test::serial;
use std::path::Path;
use std::process::Command;

#[cfg(unix)]
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

fn build_invalid_name_source(name_literal: &str) -> String {
    format!(
        "import get_environment_variable from process\n\n##\n  Description: Integration probe that reports env invalid-name errors.\n##\nentry main = f(args: string[]): void errors EnvironmentVariableNotFoundError, InvalidEnvironmentVariableNameError, InvalidUtf8Error =>\n    guard get_environment_variable({name_literal}) into value else err =>\n        print(err)\n        propagate err\n\n    print('UNEXPECTED_SUCCESS')\n    print(value)\n    return void\n"
    )
}

fn build_invalid_utf8_source() -> String {
    String::from(
        "import get_environment_variable, get_environment_variable_or from process\n\n##\n  Description: Integration probe that reports env invalid-utf8 errors.\n##\nentry main = f(args: string[]): void errors EnvironmentVariableNotFoundError, InvalidEnvironmentVariableNameError, InvalidUtf8Error =>\n    guard get_environment_variable('OPAL_PROCESS_TEST_INVALID_UTF8') into value else err =>\n        print(err)\n        propagate err\n\n    let fallback_value = propagate get_environment_variable_or('OPAL_PROCESS_TEST_INVALID_UTF8', 'fallback-value')\n    print('UNEXPECTED_SUCCESS')\n    print(value)\n    print(fallback_value)\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program_for_tests(
        Path::new("test-projects/process-env/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "process-env inline probe source should compile into a binary: {error}"
            ));
        }
    };

    run_binary_output_with_timeout(
        &binary_path,
        std::time::Duration::from_secs(10),
        "compiled process-env inline probe",
    )
    .map_err(|error| format!("process-env inline probe should execute: {error}"))
}

#[test]
#[serial(fs)]
fn process_environment_runtime_functions_compile_and_run() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for process-env fixture test"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/process-env");
    let temp_dir = unique_probe_target_dir("process-env-fixture");
    let binary_result = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    assert!(
        binary_result.is_ok(),
        "process-env fixture should compile into a binary: {}",
        binary_result
            .as_ref()
            .err()
            .map_or_else(
                || String::from("unknown compile error"),
                alloc::string::ToString::to_string,
            )
    );
    let Ok(binary_path) = binary_result else {
        return;
    };

    let mut command = Command::new(&binary_path);
    command.env("OPAL_PROCESS_TEST_VALUE", "present-value");
    command.env("OPAL_PROCESS_TEST_EMPTY", "");
    command.env_remove("OPAL_PROCESS_TEST_MISSING");

    let output_result = run_command_output_with_timeout(
        &mut command,
        std::time::Duration::from_secs(10),
        "process-env compiled binary",
    );
    assert!(
        output_result.is_ok(),
        "process-env compiled binary should execute: {}",
        output_result
            .as_ref()
            .err()
            .map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string,
            )
    );
    let Ok(run_output) = output_result else {
        return;
    };

    let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
    assert!(
        run_output.status.success(),
        "process-env binary should exit with status code 0, got: {:?}, stdout={stdout:?}, stderr={}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let expected_markers = [
        "present_value=present-value",
        "present_exists=true",
        "missing_exists=false",
        "missing_default=fallback-value",
        "empty_present=",
        "empty_default=",
        "present_default=present-value",
    ];

    for marker in expected_markers {
        assert!(
            stdout.lines().any(|line| line.trim() == marker),
            "process-env output should contain marker {marker:?}, got: {stdout:?}"
        );
    }

    let cleanup_target = cleanup_dir(&temp_dir);
    assert!(
        cleanup_target.is_ok(),
        "process-env target directory should be removed after fixture run"
    );
}

#[test]
#[serial(fs)]
fn process_environment_invalid_names_fail_with_invalid_environment_variable_name_error() {
    let temp_dir = unique_probe_target_dir("process-env-invalid-name");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "process-env invalid-name temp directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        for (name_literal, expected) in [
            ("''", "InvalidEnvironmentVariableNameError: environment variable name must not be empty"),
            ("'BAD=NAME'", "InvalidEnvironmentVariableNameError: environment variable name must not contain '='"),
        ] {
            let source = build_invalid_name_source(name_literal);
            let run_output = compile_and_run_inline_program(&source, &temp_dir)?;
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let combined = format!("{stdout}\n{stderr}");
            if combined.contains("UNEXPECTED_SUCCESS") {
                return Err(format!(
                    "invalid-name probe unexpectedly succeeded for {name_literal}, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
            if !combined.contains(expected) {
                return Err(format!(
                    "invalid-name output should contain {expected:?} for {name_literal}, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "process-env invalid-name temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "invalid env names should fail with InvalidEnvironmentVariableNameError: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn process_environment_name_with_embedded_nul_is_rejected_during_compilation() {
    let temp_dir = unique_probe_target_dir("process-env-embedded-nul-name");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "process-env embedded-nul temp directory should be created"
    );

    let source = build_invalid_name_source("'BAD\0NAME'");
    let binary_result = compile_program_for_tests(
        Path::new("test-projects/process-env/src/main.op"),
        &source,
        &temp_dir,
        &TargetTriple::host(),
    );

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "process-env embedded-nul temp directory should be removed"
    );

    assert!(
        binary_result.is_err(),
        "process-env source with embedded NUL env name should fail compilation, but it compiled"
    );
    let Err(compile_error) = binary_result else {
        return;
    };

    assert!(
        matches!(&compile_error, &CompileError::Report { .. }),
        "process-env embedded NUL should fail with a lexical report, got: {compile_error}"
    );
    let CompileError::Report { report, .. } = compile_error else {
        return;
    };

    assert!(
        report.entries().iter().any(|entry| {
            matches!(
                entry,
                &(
                    _,
                    CompilerError::Lexer(LexError::UnexpectedCharacter {
                        character: '\0',
                        ..
                    })
                )
            )
        }),
        "embedded NUL env name should be rejected before runtime as an unexpected NUL character"
    );
}

#[cfg(unix)]
#[test]
#[serial(fs)]
fn process_environment_invalid_utf8_value_reports_invalid_utf8_error() {
    let temp_dir = unique_probe_target_dir("process-env-invalid-utf8");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "process-env invalid-utf8 temp directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source = build_invalid_utf8_source();
        let binary_result = compile_program_for_tests(
            Path::new("test-projects/process-env/src/main.op"),
            &source,
            &temp_dir,
            &TargetTriple::host(),
        );
        let binary_path = binary_result.map_err(|error| {
            format!("process-env invalid-utf8 probe should compile into a binary: {error}")
        })?;

        let mut command = Command::new(&binary_path);
        command.env(
            "OPAL_PROCESS_TEST_INVALID_UTF8",
            OsString::from_vec(vec![0xFF_u8]),
        );
        let run_output = run_command_output_with_timeout(
            &mut command,
            std::time::Duration::from_secs(10),
            "process-env invalid-utf8 compiled binary",
        )?;
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "invalid-utf8 env probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("InvalidUtf8Error: 0") {
            return Err(format!(
                "invalid-utf8 env output should contain 'InvalidUtf8Error: 0', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if combined.contains("fallback-value") {
            return Err(format!(
                "invalid-utf8 env probe should not use missing-variable fallback, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "process-env invalid-utf8 temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "invalid UTF-8 env values should fail with InvalidUtf8Error: {failure_message}"
    );
}
