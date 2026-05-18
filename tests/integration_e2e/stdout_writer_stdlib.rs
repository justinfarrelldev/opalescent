#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn stdout_writer_write_flush() {
    let temp_dir = unique_probe_target_dir("stdout-writer-write-flush");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "stdout-writer-write-flush target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/stdout-writer-write-flush/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("stdout-writer-write-flush source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("stdout-writer-write-flush source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "stdout-writer-write-flush compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "stdout-writer-write-flush binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "frame";
        if stdout != expected {
            return Err(format!(
                "stdout-writer-write-flush stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "stdout-writer-write-flush target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "stdout-writer-write-flush should compile, run, and write exact raw bytes: \
         {failure_message}"
    );
}

#[test]
fn stdout_writer_interleaves_with_print_text() {
    let temp_dir = unique_probe_target_dir("stdout-writer-interleaves-with-print-text");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "stdout-writer-interleaves-with-print-text target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path =
            Path::new("test-projects/stdout-writer-interleaves-with-print-text/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!(
                "stdout-writer-interleaves-with-print-text source file should be readable: {error}"
            )
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "stdout-writer-interleaves-with-print-text source should compile into a binary: {error}"
            )
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "stdout-writer-interleaves-with-print-text compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "stdout-writer-interleaves-with-print-text binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "A1B2";
        if stdout != expected {
            return Err(format!(
                "stdout-writer-interleaves-with-print-text stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "stdout-writer-interleaves-with-print-text target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "stdout-writer-interleaves-with-print-text should compile, run, and preserve stream ordering: \
         {failure_message}"
    );
}
