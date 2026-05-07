#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_path_manipulation() {
    {
        let _guard = FsStateGuard::new("test-projects/fs-path-manipulation")
            .expect("fs-path-manipulation guard should initialize and reset target/workspace");

        assert_workspace_empty("fs-path-manipulation");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for fs-path-manipulation fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/fs-path-manipulation");
        let temp_dir = unique_probe_target_dir("path-manipulation-fixture");

        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "fs-path-manipulation fixture should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                |error| format!("{error}")
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = std::process::Command::new(&binary_path).output();
        assert!(
            output_result.is_ok(),
            "fs-path-manipulation compiled binary should execute: {}",
            output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                |error| format!("{error}")
            )
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert!(
            run_output.status.success(),
            "fs-path-manipulation binary should exit with status code 0, got: {:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            stdout.trim_end(),
            "passed 50/50",
            "fs-path-manipulation output should report the full pass count"
        );
    }

    assert_workspace_empty("fs-path-manipulation");
}
