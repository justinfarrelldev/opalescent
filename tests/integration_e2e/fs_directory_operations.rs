#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_directory_operations() {
    {
        let _guard = FsStateGuard::new("test-projects/fs-directory-operations")
            .expect("fs-directory-operations guard should initialize and reset target/workspace");

        assert_workspace_empty("fs-directory-operations");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for fs-directory-operations fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/fs-directory-operations");
        let temp_dir = unique_probe_target_dir("directory-operations-fixture");

        let binary_result =
            opalescent::compiler::compile_project(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "fs-directory-operations fixture should compile into a binary: {}",
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
            "fs-directory-operations compiled binary should execute: {}",
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
            "fs-directory-operations binary should exit with status code 0, got: {:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert!(
            stdout.contains("fs-directory-operations: created"),
            "fs-directory-operations output should contain success prefix, got: {stdout:?}"
        );
        assert!(
            stdout.contains("all match"),
            "fs-directory-operations output should report all match, got: {stdout:?}"
        );
    }

    assert_workspace_empty("fs-directory-operations");
}
