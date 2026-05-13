#![cfg(feature = "integration")]

use super::*;
use crate::tests::fs_helpers::{unique_probe_target_dir, wait_for_child_output_with_timeout};
use std::process::Stdio;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn run_binary_with_timeout(
    binary_path: &Path,
    context: &str,
) -> Result<std::process::Output, String> {
    let child = std::process::Command::new(binary_path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{context} should execute: {error}"))?;

    wait_for_child_output_with_timeout(child, GENERATED_BINARY_TEST_TIMEOUT, context)
}

#[test]
fn rc_basic_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/rc-basic");
    let temp_dir = unique_probe_target_dir("rc-basic");
    println!("rc-basic target dir: {}", temp_dir.display());
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "rc-basic target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "rc-basic project should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_with_timeout(&binary_path, "rc-basic compiled binary")?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "rc-basic: hello world" {
            return Err(format!(
                "rc-basic stdout should equal 'rc-basic: hello world', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "rc-basic binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "rc-basic target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "rc-basic should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn rc_reuse_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/rc-reuse");
    let temp_dir = unique_probe_target_dir("rc-reuse");
    println!("rc-reuse target dir: {}", temp_dir.display());
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "rc-reuse target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "rc-reuse project should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_with_timeout(&binary_path, "rc-reuse compiled binary")?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "rc-reuse: first second" {
            return Err(format!(
                "rc-reuse stdout should equal 'rc-reuse: first second', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "rc-reuse binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "rc-reuse target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "rc-reuse should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn iterative_drop_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/iterative-drop");
    let temp_dir = unique_probe_target_dir("iterative-drop");
    println!("iterative-drop target dir: {}", temp_dir.display());
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "iterative-drop target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "iterative-drop project should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_with_timeout(&binary_path, "iterative-drop compiled binary")?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "iterative-drop: done" {
            return Err(format!(
                "iterative-drop stdout should equal 'iterative-drop: done', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "iterative-drop binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "iterative-drop target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "iterative-drop should compile, run, and print expected output: {failure_message}"
    );
}

#[test]
fn weak_ref_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for integration tests"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/weak-ref");
    let temp_dir = unique_probe_target_dir("weak-ref");
    println!("weak-ref target dir: {}", temp_dir.display());
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "weak-ref target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        let binary_path = match binary_result {
            Ok(path) => path,
            Err(error) => {
                return Err(format!(
                    "weak-ref project should compile into a binary: {error}"
                ));
            }
        };

        let run_output = run_binary_with_timeout(&binary_path, "weak-ref compiled binary")?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "weak-ref: ok" {
            return Err(format!(
                "weak-ref stdout should equal 'weak-ref: ok', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "weak-ref binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "weak-ref target directory should be removed"
    );
    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "weak-ref should compile, run, and print expected output: {failure_message}"
    );
}
