#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn string_builder_push_finish() {
    let temp_dir = unique_probe_target_dir("string-builder-push-finish");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-builder-push-finish target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/string-builder-push-finish/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("string-builder-push-finish source should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("string-builder-push-finish source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "string-builder-push-finish compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "string-builder-push-finish binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "row-1\nrow-2\n";
        if stdout != expected {
            return Err(format!(
                "string-builder-push-finish stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-builder-push-finish target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "string-builder-push-finish should compile, run, and print the finished builder text: {failure_message}"
    );
}

#[test]
fn string_builder_use_after_finish_errors() {
    let temp_dir = unique_probe_target_dir("string-builder-use-after-finish-errors");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-builder-use-after-finish-errors target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path =
            Path::new("test-projects/string-builder-use-after-finish-errors/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("string-builder-use-after-finish-errors source should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "string-builder-use-after-finish-errors source should compile into a binary: {error}"
            )
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "string-builder-use-after-finish-errors compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "string-builder-use-after-finish-errors unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("BuilderFinishedError") {
            return Err(format!(
                "string-builder-use-after-finish-errors output should contain BuilderFinishedError, status={:?}, stdout='{}', stderr='{}'",
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
        "string-builder-use-after-finish-errors target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "string-builder-use-after-finish-errors should compile, run, and report BuilderFinishedError: {failure_message}"
    );
}
