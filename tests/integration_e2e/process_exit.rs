#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::time::Duration;

const PROCESS_EXIT_CODE: i32 = 42;
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
#[serial(fs)]
fn process_exit_runtime_function_terminates_child_with_requested_code() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for process-exit fixture test"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/process-exit-code");
    let temp_dir = unique_probe_target_dir("process-exit-code-fixture");
    let binary_result = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    assert!(
        binary_result.is_ok(),
        "process-exit-code fixture should compile into a binary: {}",
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

    let output_result = run_binary_output_with_timeout(
        &binary_path,
        PROCESS_EXIT_TIMEOUT,
        "process-exit-code compiled binary",
    );
    assert!(
        output_result.is_ok(),
        "process-exit-code compiled binary should execute: {}",
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

    assert_eq!(
        run_output.status.code(),
        Some(PROCESS_EXIT_CODE),
        "process-exit-code child should terminate with exit code {PROCESS_EXIT_CODE}, stdout={}, stderr={}",
        String::from_utf8_lossy(&run_output.stdout),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let cleanup_target = cleanup_dir(&temp_dir);
    assert!(
        cleanup_target.is_ok(),
        "process-exit-code target directory should be removed after fixture run"
    );
}
