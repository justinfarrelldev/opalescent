#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{strip_crlf, unique_probe_target_dir};
use super::*;
use serial_test::serial;

#[test]
#[serial(fs)]
fn process_cwd_fixture_restores_child_and_preserves_parent_cwd() {
    let parent_cwd_result = std::env::current_dir();
    assert!(
        parent_cwd_result.is_ok(),
        "parent process working directory should be readable before process-cwd fixture"
    );
    let Ok(parent_cwd_before) = parent_cwd_result else {
        return;
    };

    let project_dir = parent_cwd_before.join("test-projects/process-cwd");
    let temp_dir = unique_probe_target_dir("process-cwd-fixture");
    let binary_result = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    assert!(
        binary_result.is_ok(),
        "process-cwd fixture should compile into a binary: {}",
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

    let output_result = run_binary_in_dir_output_with_timeout(
        &binary_path,
        &parent_cwd_before,
        std::time::Duration::from_secs(10),
        "process-cwd compiled binary",
    );
    assert!(
        output_result.is_ok(),
        "process-cwd compiled binary should execute: {}",
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
        "process-cwd binary should exit with status code 0, got: {:?}, stdout={stdout:?}, stderr={}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    for marker in [
        "cwd_mutation_applied=true",
        "cwd_mutation_restored=true",
        "cwd_fixture_done=true",
    ] {
        assert!(
            stdout.lines().any(|line| line.trim() == marker),
            "process-cwd output should contain marker {marker:?}, got: {stdout:?}"
        );
    }

    let parent_cwd_after_result = std::env::current_dir();
    assert!(
        parent_cwd_after_result.is_ok(),
        "parent process working directory should remain readable after process-cwd fixture"
    );
    let Ok(parent_cwd_after) = parent_cwd_after_result else {
        return;
    };

    assert_eq!(
        parent_cwd_before, parent_cwd_after,
        "process-cwd fixture should not mutate parent process cwd"
    );

    let cleanup_target = cleanup_dir(&temp_dir);
    assert!(
        cleanup_target.is_ok(),
        "process-cwd target directory should be removed after fixture run"
    );
}
