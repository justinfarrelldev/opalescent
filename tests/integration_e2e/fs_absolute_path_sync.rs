#![cfg(feature = "integration")]

use super::*;
use super::fs_helpers::{FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir};
use serial_test::serial;

fn stringify_error<E: core::fmt::Display>(error: E) -> String {
    format!("{error}")
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_absolute_path_sync_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_absolute_path_sync")
            .expect("_absolute_path_sync guard should initialize and reset target/workspace");

        assert_workspace_empty("_absolute_path_sync");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _absolute_path_sync fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_absolute_path_sync");
        let temp_dir = unique_probe_target_dir("absolute-path-sync-fixture");

        let binary_result = opalescent::compiler::compile_project(
            &project_dir,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "_absolute_path_sync fixture should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), stringify_error)
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = std::process::Command::new(&binary_path).output();
        assert!(
            output_result.is_ok(),
            "_absolute_path_sync compiled binary should execute: {}",
            output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), stringify_error)
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout
            .lines()
            .map(str::trim)
            .collect();

        assert_eq!(
            lines.len(),
            4,
            "_absolute_path_sync fixture should print exactly 4 lines"
        );

        let existing_line = lines.first().copied().unwrap_or_default();
        assert!(
            existing_line.starts_with("./test-projects/_absolute_path_sync/src/main.op -> ")
                && existing_line.contains("main.op"),
            "existing relative path should resolve to an absolute main.op path, got: {existing_line}"
        );

        let missing_line = lines.get(1).copied().unwrap_or_default();
        assert!(
            missing_line.starts_with("./test-projects/_absolute_path_sync/does_not_exist.txt -> ")
                && missing_line.contains("does_not_exist.txt"),
            "non-existing relative path should resolve lexically to an absolute path, got: {missing_line}"
        );

        let normalized_line = lines.get(2).copied().unwrap_or_default();
        assert!(
            normalized_line.starts_with("./test-projects/_absolute_path_sync/src/../README.md -> ")
                && normalized_line.contains("README.md"),
            "path containing '..' should collapse to README absolute path, got: {normalized_line}"
        );

        let root_line = lines.get(3).copied().unwrap_or_default();
        assert_eq!(root_line, "/ -> /", "already absolute root path should remain root");

        assert!(
            run_output.status.success(),
            "_absolute_path_sync binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_absolute_path_sync");
}
