#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, fs_project_root, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_markdown_roundtrip() {
    let project_name = "fs-markdown-roundtrip";
    let project_dir = fs_project_root(project_name);
    let output_path = project_dir.join("workspace/output.md");
    let expected_path = project_dir.join("fixtures/expected_output.md");

    {
        let _guard = FsStateGuard::new("test-projects/fs-markdown-roundtrip")
            .expect("fs-markdown-roundtrip guard should initialize and reset target/workspace");

        assert_workspace_empty(project_name);

        let junk_path = project_dir.join("workspace/leftover.txt");
        let junk_write = fs::write(&junk_path, b"stale workspace content\n");
        assert!(
            junk_write.is_ok(),
            "fs-markdown-roundtrip test should be able to seed workspace junk: {:?}",
            junk_write.err()
        );

        let temp_dir = unique_probe_target_dir("markdown-roundtrip-fixture");
        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "fs-markdown-roundtrip fixture should compile into a binary: {}",
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
            "fs-markdown-roundtrip compiled binary should execute: {}",
            output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                ToString::to_string
            )
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert!(
            run_output.status.success(),
            "fs-markdown-roundtrip binary should exit with status code 0, got: {:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert_eq!(
            stdout.trim_end(),
            "roundtrip: ok (547 bytes match)",
            "fs-markdown-roundtrip output should report exact byte count"
        );

        let actual_bytes = fs::read(&output_path);
        assert!(
            actual_bytes.is_ok(),
            "fs-markdown-roundtrip should create workspace/output.md"
        );
        let expected_bytes = fs::read(&expected_path);
        assert!(
            expected_bytes.is_ok(),
            "fs-markdown-roundtrip expected fixture should be readable"
        );
        assert_eq!(
            actual_bytes.ok(),
            expected_bytes.ok(),
            "fs-markdown-roundtrip output bytes should match expected_output.md exactly"
        );
    }

    assert_workspace_empty(project_name);
}
