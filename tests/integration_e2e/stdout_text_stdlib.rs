#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn print_text_flush_writes_without_newline() {
    let temp_dir = unique_probe_target_dir("print-text-flush-without-newline");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "print-text-flush-without-newline target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/print-text-flush-without-newline/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("print-text-flush-without-newline source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("print-text-flush-without-newline source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "print-text-flush-without-newline compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "print-text-flush-without-newline binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "abc";
        if stdout != expected {
            return Err(format!(
                "print-text-flush-without-newline stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "print-text-flush-without-newline target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "print-text-flush-without-newline should compile, run, and write exact raw bytes: \
         {failure_message}"
    );
}
