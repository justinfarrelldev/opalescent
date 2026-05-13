#![cfg(feature = "integration")]

extern crate alloc;

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use alloc::string::ToString;
use serial_test::serial;

fn stringify_error<E: core::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn path_from_handles_empty_via_sentinel() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("path_from empty-sentinel guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, path_parent_directory, path_file_name from standard

##
  Description: Validates path_from empty input sentinel semantics.
##
entry main = f(args: string[]): void =>
    let empty = path_from('')
    let hello = path_from('hello')
    let nested = path_from('hello/world')

    let empty_name = path_file_name(empty)
    print('empty_name={empty_name}')

    let hello_name = path_file_name(hello)
    print('hello_name={hello_name}')

    let nested_parent_name = path_file_name(path_parent_directory(nested))
    print('nested_parent_name={nested_parent_name}')

    let nested_name = path_file_name(nested)
    print('nested_name={nested_name}')

    return void
";

        let temp_dir = unique_probe_target_dir("path-from-empty-sentinel");

        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_fs_path_from/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "path_from empty-sentinel source should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), stringify_error)
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let run_output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(30),
            "path_from empty-sentinel compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "path_from empty-sentinel compiled binary should execute: {}",
            run_output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), stringify_error)
        );
        let Ok(run_output) = run_output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        let expected = vec![
            "empty_name=",
            "hello_name=hello",
            "nested_parent_name=hello",
            "nested_name=world",
        ];
        assert_eq!(
            lines, expected,
            "path_from should preserve non-empty input and emit empty sentinel for empty input"
        );

        assert!(
            !stdout.contains("InvalidPathError"),
            "path_from should remain infallible and never emit InvalidPathError discriminants"
        );

        assert!(
            run_output.status.success(),
            "path_from empty-sentinel binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_path_from_smoke() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("fs_path_from guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for integration tests"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_path_from");
        let temp_dir = unique_probe_target_dir("path-from-smoke");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_path_from fixture should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                ToString::to_string
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let run_output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(30),
            "_fs_path_from compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "_fs_path_from compiled binary should execute: {}",
            run_output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(run_output) = run_output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();

        let expected = vec![
            "error: invalid",
            "path=hello",
            "path=hello/world",
            "path=hello/",
        ];
        assert_eq!(
            lines, expected,
            "_fs_path_from should exercise 4 cases: empty (error), simple, nested, trailing slash"
        );

        assert!(
            run_output.status.success(),
            "_fs_path_from binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}
