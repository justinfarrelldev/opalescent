#![cfg(feature = "integration")]

extern crate alloc;

use alloc::string::ToString;
use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;
use std::path::Path;

fn atomic_target_path() -> &'static str {
    "/tmp/opalescent-fs-write-text-atomic.txt"
}

fn atomic_tmp_path() -> &'static str {
    "/tmp/opalescent-fs-write-text-atomic.txt.tmp.1"
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_write_text_atomic() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_write_text_atomic")
            .expect("_fs_write_text_atomic guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_write_text_atomic");

        drop(fs::remove_file(atomic_target_path()));
        drop(fs::remove_file(atomic_tmp_path()));

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_write_text_atomic fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_write_text_atomic");
        let temp_dir = unique_probe_target_dir("write-text-atomic-fixture");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_write_text_atomic fixture should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                ToString::to_string
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = run_binary_output_with_timeout(&binary_path, std::time::Duration::from_secs(10), "compiled binary");
        assert!(
            output_result.is_ok(),
            "_fs_write_text_atomic compiled binary should execute: {}",
            output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert!(
            stdout.contains("wrote atomically: 14"),
            "_fs_write_text_atomic output should contain confirmation line, got: {stdout:?}"
        );
        assert!(
            run_output.status.success(),
            "_fs_write_text_atomic binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );

        assert!(
            !Path::new(atomic_tmp_path()).exists(),
            "_fs_write_text_atomic should leave no temporary .tmp. path after run"
        );
    }

    drop(fs::remove_file(atomic_target_path()));
    drop(fs::remove_file(atomic_tmp_path()));
    assert_workspace_empty("_fs_write_text_atomic");
}
