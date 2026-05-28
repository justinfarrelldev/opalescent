#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{strip_crlf, unique_probe_target_dir};
use super::*;
use serial_test::serial;

#[test]
#[serial(fs)]
fn process_paths_runtime_functions_compile_and_run() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for process-paths fixture test"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/process-paths");
    let temp_dir = unique_probe_target_dir("process-paths-fixture");
    let binary_result = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    assert!(
        binary_result.is_ok(),
        "process-paths fixture should compile into a binary: {}",
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
        std::time::Duration::from_secs(10),
        "process-paths compiled binary",
    );
    assert!(
        output_result.is_ok(),
        "process-paths compiled binary should execute: {}",
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
        "process-paths binary should exit with status code 0, got: {:?}, stdout={stdout:?}, stderr={}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    let expected_markers = [
        "cwd_non_empty=true",
        "cwd_exists=true",
        "cwd_is_directory=true",
        "exe_path_non_empty=true",
        "exe_path_exists=true",
        "exe_dir_non_empty=true",
        "exe_dir_exists=true",
        "exe_dir_is_directory=true",
        "cwd_changed=true",
        "changed_cwd_exists=true",
        "changed_cwd_is_directory=true",
        "cwd_restored=true",
    ];

    for marker in expected_markers {
        assert!(
            stdout.lines().any(|line| line.trim() == marker),
            "process-paths output should contain marker {marker:?}, got: {stdout:?}"
        );
    }

    let cleanup_target = cleanup_dir(&temp_dir);
    assert!(
        cleanup_target.is_ok(),
        "process-paths target directory should be removed after fixture run"
    );
}
