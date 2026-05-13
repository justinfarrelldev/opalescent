#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use alloc::string::ToString;
use serial_test::serial;
use std::path::Path;

fn append_log_path() -> String {
    std::env::temp_dir()
        .join(format!(
            "opalescent-fs-append-log-{}.txt",
            std::process::id()
        ))
        .to_string_lossy()
        .into_owned()
}

fn build_single_append_source(path: &str, line: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_line = line.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, append_text_sync from standard\n\n##\n  Description: Appends one line for monotonic file-size checks.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidPathError, WriteFailureError, FilesystemFullError =>\n    propagate append_text_sync(path_from('{escaped_path}'), '{escaped_line}\\n')\n    return void\n"
    )
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_append_log() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_append_log")
            .expect("_fs_append_log guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_append_log");

        let log_path = append_log_path();
        drop(fs::remove_file(&log_path));

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_append_log fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_append_log");
        let temp_dir = unique_probe_target_dir("append-log-fixture");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_append_log fixture should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                ToString::to_string
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let fixture_log_path = std::env::temp_dir().join("opalescent-fs-append-log.txt");
        drop(fs::remove_file(&fixture_log_path));

        let output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(10),
            "compiled binary",
        );
        assert!(
            output_result.is_ok(),
            "_fs_append_log compiled binary should execute: {}",
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
            stdout.contains("appended 5 lines; readback confirmed"),
            "_fs_append_log output should contain confirmation line, got: {stdout:?}"
        );
        assert!(
            run_output.status.success(),
            "_fs_append_log binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    let fixture_log_path = std::env::temp_dir().join("opalescent-fs-append-log.txt");
    drop(fs::remove_file(&fixture_log_path));
    drop(fs::remove_file(append_log_path()));
    assert_workspace_empty("_fs_append_log");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_append_log_monotonic() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_append_log")
            .expect("_fs_append_log guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_append_log");

        let log_path = append_log_path();
        drop(fs::remove_file(&log_path));

        let temp_dir = unique_probe_target_dir("append-log-monotonic");
        let prepare = prepare_dir(&temp_dir);
        assert!(
            prepare.is_ok(),
            "_fs_append_log target directory should be created"
        );

        let mut previous_size: u64 = 0;
        for idx in 0_usize..5_usize {
            let line = format!("monotonic-line-{}", idx + 1_usize);
            let source = build_single_append_source(&log_path, &line);

            let binary_result = compile_program_for_tests(
                Path::new("test-projects/_fs_append_log/src/main.op"),
                &source,
                &temp_dir,
                &TargetTriple::host(),
            );
            assert!(
                binary_result.is_ok(),
                "monotonic append inline source should compile: {}",
                binary_result.as_ref().err().map_or_else(
                    || String::from("unknown compile error"),
                    alloc::string::ToString::to_string
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
                "monotonic append binary should execute: {}",
                output_result.as_ref().err().map_or_else(
                    || String::from("unknown execution error"),
                    alloc::string::ToString::to_string
                )
            );
            let Ok(run_output) = output_result else {
                return;
            };
            assert!(
                run_output.status.success(),
                "monotonic append run {} should exit 0, got status {:?}, stderr={}",
                idx + 1_usize,
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            );

            let metadata_result = fs::metadata(&log_path);
            assert!(
                metadata_result.is_ok(),
                "log file metadata should be readable after append {}: {}",
                idx + 1_usize,
                metadata_result.as_ref().err().map_or_else(
                    || String::from("unknown metadata error"),
                    alloc::string::ToString::to_string
                )
            );
            let Ok(metadata) = metadata_result else {
                return;
            };
            let current_size = metadata.len();
            assert!(
                current_size > previous_size,
                "log file size should grow monotonically after append {}: previous={}, current={}",
                idx + 1_usize,
                previous_size,
                current_size
            );
            previous_size = current_size;
        }

        let cleanup = cleanup_dir(&temp_dir);
        assert!(
            cleanup.is_ok(),
            "_fs_append_log target directory should be removed"
        );
    }

    drop(fs::remove_file(append_log_path()));
    assert_workspace_empty("_fs_append_log");
}
