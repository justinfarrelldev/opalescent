#![cfg(feature = "integration")]

extern crate alloc;

use alloc::string::ToString;
use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;

fn stringify_error<E: core::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn normalize_canonical_matrix() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("normalize matrix guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, normalize_path, path_parent_directory, path_file_name from standard

##
  Description: Prints normalization matrix for lexical path canonicalization.
##
entry main = f(args: string[]): void =>
    let n0 = normalize_path(path_from('a//b'))
    print('a//b -> parent_name={path_file_name(path_parent_directory(n0))} name={path_file_name(n0)}')

    let n1 = normalize_path(path_from('a/./b'))
    print('a/./b -> parent_name={path_file_name(path_parent_directory(n1))} name={path_file_name(n1)}')

    let n2 = normalize_path(path_from('a/b/..'))
    print('a/b/.. -> parent_name={path_file_name(path_parent_directory(n2))} name={path_file_name(n2)}')

    let n3 = normalize_path(path_from('a/b/../../c'))
    print('a/b/../../c -> parent_name={path_file_name(path_parent_directory(n3))} name={path_file_name(n3)}')

    let n4 = normalize_path(path_from('./a'))
    print('./a -> parent_name={path_file_name(path_parent_directory(n4))} name={path_file_name(n4)}')

    let n5 = normalize_path(path_from('/a/b/../../..'))
    print('/a/b/../../.. -> parent_name={path_file_name(path_parent_directory(n5))} name={path_file_name(n5)}')

    let n6 = normalize_path(path_from(''))
    print('<empty> -> parent_name={path_file_name(path_parent_directory(n6))} name={path_file_name(n6)}')

    return void
";

        let temp_dir = unique_probe_target_dir("normalize-canonical-matrix");

        let binary_result = compile_program_for_tests(Path::new("test-projects/_fs_path_from/src/main.op"), source, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "normalize matrix source should compile into a binary: {}",
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
            "normalize matrix compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "normalize matrix compiled binary should execute: {}",
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
            "a//b -> parent_name=a name=b",
            "a/./b -> parent_name=a name=b",
            "a/b/.. -> parent_name=. name=a",
            "a/b/../../c -> parent_name=. name=c",
            "./a -> parent_name=. name=a",
            "/a/b/../../.. -> parent_name=. name=",
            "<empty> -> parent_name=. name=",
        ];

        assert_eq!(
            lines, expected,
            "normalize_path should satisfy the canonical lexical normalization matrix"
        );

        assert!(
            !stdout.contains("InvalidPathError"),
            "normalize_path matrix output should not emit fallible error discriminants"
        );

        assert!(
            run_output.status.success(),
            "normalize matrix binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_normalize_path_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_normalize_path")
            .expect("_fs_normalize_path guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_normalize_path");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_normalize_path fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_normalize_path");
        let temp_dir = unique_probe_target_dir("normalize-fixture-showcase");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_normalize_path fixture should compile into a binary: {}",
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
            "_fs_normalize_path compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "_fs_normalize_path compiled binary should execute: {}",
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
            "a//b -> a/b",
            "a/./b -> a/b",
            "a/b/.. -> a",
            "a/b/../../c -> c",
            "./a -> a",
            "/a/b/../../.. -> empty-sentinel",
        ];
        assert_eq!(
            lines, expected,
            "_fs_normalize_path fixture should print the 6 canonical normalize_path showcase lines"
        );

        assert!(
            run_output.status.success(),
            "_fs_normalize_path binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_normalize_path");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn normalize_windows_roots_and_mixed_separators() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("normalize windows-root guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, normalize_path, path_to_string from standard

##
  Description: Verifies Windows lexical roots and mixed separators normalize correctly.
##
entry main = f(args: string[]): void =>
    let drive_backslash = normalize_path(path_from('C:\\\\Users\\\\foo\\\\..\\\\bar.txt'))
    print('drive-backslash={path_to_string(drive_backslash)}')

    let drive_slash = normalize_path(path_from('C:/Users/foo/../bar.txt'))
    print('drive-slash={path_to_string(drive_slash)}')

    let unc = normalize_path(path_from('\\\\\\\\server\\\\share\\\\dir\\\\..\\\\file.ext'))
    print('unc={path_to_string(unc)}')

    let posix = normalize_path(path_from('/tmp/../file.ext'))
    print('posix={path_to_string(posix)}')
    return void
";

        let temp_dir = unique_probe_target_dir("normalize-windows-roots");

        let binary_result = compile_program_for_tests(Path::new("test-projects/_fs_path_from/src/main.op"), source, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "normalize windows-root source should compile into a binary: {}",
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
            "normalize windows-root compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "normalize windows-root compiled binary should execute: {}",
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
            "drive-backslash=C:/Users/bar.txt",
            "drive-slash=C:/Users/bar.txt",
            "unc=//server/share/file.ext",
            "posix=/file.ext",
        ];

        assert_eq!(
            lines, expected,
            "normalize_path should preserve Windows roots and POSIX root collapse semantics"
        );

        assert!(
            run_output.status.success(),
            "normalize windows-root binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn normalize_root_escape_returns_empty() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("normalize root-escape guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, normalize_path, absolute_path_sync from standard

##
  Description: Verifies root escape returns empty sentinel.
##
entry main = f(args: string[]): void =>
    let normalized = normalize_path(path_from('/a/b/../../..'))
    guard absolute_path_sync(normalized) into abs else err =>
        print('root-escape=ok')
        return void

    print('root-escape=bad')
    return void
";

        let temp_dir = unique_probe_target_dir("normalize-root-escape");

        let binary_result = compile_program_for_tests(Path::new("test-projects/_fs_path_from/src/main.op"), source, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "normalize root-escape source should compile into a binary: {}",
            binary_result.as_ref().err().map_or_else(
                || String::from("unknown compile error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let run_output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(30),
            "normalize root-escape compiled binary",
        );
        assert!(
            run_output_result.is_ok(),
            "normalize root-escape compiled binary should execute: {}",
            run_output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(run_output) = run_output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert_eq!(
            stdout.trim_end(),
            "root-escape=ok",
            "normalize_path should return empty sentinel when absolute path escapes root"
        );
        assert!(
            !stdout.contains("InvalidPathError"),
            "normalize_path root-escape should remain infallible and not emit discriminant errors"
        );

        assert!(
            run_output.status.success(),
            "normalize root-escape binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}
