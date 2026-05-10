#![cfg(feature = "integration")]

use super::*;
use crate::tests::fs_helpers::{unique_probe_target_dir, wait_for_child_output_with_timeout};
use std::process::{Command, Stdio};
use std::time::Duration;

fn write_op_cat_fixture(path: &std::path::Path, contents: &str) -> Result<(), String> {
    std::fs::write(path, contents)
        .map_err(|error| format!("op-cat fixture file should be writable: {error}"))
}

#[test]
fn op_cat_happy_path_prints_file_contents() {
    let project_dir = std::path::Path::new("test-projects/op-cat");
    let temp_dir = unique_probe_target_dir("op-cat-happy-path");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "op-cat target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let source_path = project_dir.join("src/main.op");
        let source_result = std::fs::read_to_string(&source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!("op-cat source file should be readable: {error}"));
            }
        };

        let valid_input = temp_dir.join("valid.txt");
        write_op_cat_fixture(&valid_input, "valid file contents\n")?;

        let binary_result = compile_program_for_tests(&source_path, source_str.as_str(), &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "op-cat source should compile into a binary: {error}"
                ));
            }
        };

        let mut command = Command::new(&binary_path);
        command
            .arg(valid_input.as_path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command
            .spawn()
            .map_err(|error| format!("op-cat compiled binary should execute: {error}"))?;
        let run_output = wait_for_child_output_with_timeout(
            child,
            Duration::from_secs(30),
            "op-cat happy path",
        )
        .map_err(|error| format!("op-cat compiled binary should complete: {error}"))?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("valid file contents") {
            return Err(format!(
                "op-cat happy path should print the file contents, got stdout: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "op-cat happy path should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "op-cat target directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "op-cat happy path should compile, run, and print the requested file contents: {failure_message}"
    );
}

#[test]
fn op_cat_error_path_continues_to_next_arg() {
    let project_dir = std::path::Path::new("test-projects/op-cat");
    let temp_dir = unique_probe_target_dir("op-cat-error-path");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "op-cat target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let source_path = project_dir.join("src/main.op");
        let source_result = std::fs::read_to_string(&source_path);
        let source_str = match source_result {
            Ok(contents) => contents,
            Err(error) => {
                return Err(format!("op-cat source file should be readable: {error}"));
            }
        };

        let valid_input = temp_dir.join("valid.txt");
        let missing_input = temp_dir.join("missing.txt");
        write_op_cat_fixture(&valid_input, "valid file contents\n")?;

        let binary_result = compile_program_for_tests(&source_path, source_str.as_str(), &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "op-cat source should compile into a binary: {error}"
                ));
            }
        };

        let mut command = Command::new(&binary_path);
        command
            .args([
                valid_input.as_path(),
                missing_input.as_path(),
                valid_input.as_path(),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let child = command
            .spawn()
            .map_err(|error| format!("op-cat compiled binary should execute: {error}"))?;
        let run_output = wait_for_child_output_with_timeout(
            child,
            Duration::from_secs(30),
            "op-cat error path",
        )
        .map_err(|error| format!("op-cat compiled binary should complete: {error}"))?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let valid_occurrences = stdout.matches("valid file contents").count();
        if valid_occurrences != 2 {
            return Err(format!(
                "op-cat error path should print valid content exactly twice, got {valid_occurrences} occurrences in stdout: '{stdout}'"
            ));
        }

        let handled_missing_file_error_occurrences = stdout
            .matches("An error occurred while catting a file: FileNotFoundError:")
            .count();
        if handled_missing_file_error_occurrences != 1 {
            return Err(format!(
                "op-cat error path should print handled missing-file error exactly once, got {handled_missing_file_error_occurrences} occurrences in stdout: '{stdout}'"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "op-cat error path should still exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "op-cat target directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "op-cat error path should continue past a missing file and print valid output twice with one handled error: {failure_message}"
    );
}
