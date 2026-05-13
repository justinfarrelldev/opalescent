#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use alloc::string::ToString;
use serial_test::serial;

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_read_text_lines_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_read_text_lines")
            .expect("_fs_read_text_lines guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_read_text_lines");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_read_text_lines fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_read_text_lines");
        let temp_dir = unique_probe_target_dir("read-text-lines-fixture");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_read_text_lines fixture should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                ToString::to_string
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(10),
            "compiled binary",
        );
        assert!(
            output_result.is_ok(),
            "_fs_read_text_lines compiled binary should execute: {}",
            output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();

        assert!(
            lines.contains(&"lines=4"),
            "_fs_read_text_lines output should include 'lines=4', got: {lines:?}"
        );
        assert!(
            lines.contains(&"first=alpha"),
            "_fs_read_text_lines output should include 'first=alpha', got: {lines:?}"
        );
        assert!(
            lines.contains(&"match=true"),
            "_fs_read_text_lines output should include 'match=true', got: {lines:?}"
        );

        assert!(
            run_output.status.success(),
            "_fs_read_text_lines binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_read_text_lines");
}
