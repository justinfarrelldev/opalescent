#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{strip_crlf, unique_probe_target_dir};
use super::*;
use serial_test::serial;
use std::process::Command;

#[test]
#[serial(fs)]
fn process_api_smoke_fixture_compiles_and_runs() {
    let cwd = std::env::current_dir();
    assert!(
        cwd.is_ok(),
        "current working directory should be readable for process-api-smoke fixture test"
    );
    let Ok(cwd_path) = cwd else {
        return;
    };

    let project_dir = cwd_path.join("test-projects/process-api-smoke");
    let temp_dir = unique_probe_target_dir("process-api-smoke-fixture");
    let binary_result = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
    assert!(
        binary_result.is_ok(),
        "process-api-smoke fixture should compile into a binary: {}",
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
    command.current_dir(&cwd_path);
    command.env("OPAL_PROCESS_TEST_VALUE", "present-value");
    command.env_remove("OPAL_PROCESS_TEST_MISSING");

    let output_result = run_command_output_with_timeout(
        &mut command,
        std::time::Duration::from_secs(10),
        "process-api-smoke compiled binary",
    );
    assert!(
        output_result.is_ok(),
        "process-api-smoke compiled binary should execute: {}",
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
        "process-api-smoke binary should exit with status code 0, got: {:?}, stdout={stdout:?}, stderr={}",
        run_output.status.code(),
        String::from_utf8_lossy(&run_output.stderr)
    );

    for marker in [
        "process_api_smoke_paths_ok=true",
        "process_api_smoke_env_ok=true",
        "process_api_smoke_get_ok=true",
        "process_api_smoke_done=true",
    ] {
        assert!(
            stdout.lines().any(|line| line.trim() == marker),
            "process-api-smoke output should contain marker {marker:?}, got: {stdout:?}"
        );
    }

    let cleanup_target = cleanup_dir(&temp_dir);
    assert!(
        cleanup_target.is_ok(),
        "process-api-smoke target directory should be removed after fixture run"
    );
}
