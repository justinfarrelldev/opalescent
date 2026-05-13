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
fn join_handles_absolute_reset() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("join absolute-reset guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, join_path_components, path_parent_directory, path_file_name from standard

##
  Description: Validates absolute reset behavior of join_path_components.
##
entry main = f(args: string[]): void =>
    let result = join_path_components(path_from('a'), ['/b', 'c'])
    let parent_name = path_file_name(path_parent_directory(result))
    let name = path_file_name(result)
    print('join-reset parent_name={parent_name} name={name}')
    return void
";

        let temp_dir = unique_probe_target_dir("join-absolute-reset");

        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_fs_path_from/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "join absolute-reset source should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), stringify_error)
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
            "join absolute-reset compiled binary should execute: {}",
            output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), stringify_error)
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert_eq!(
            stdout.trim_end(),
            "join-reset parent_name=b name=c",
            "join_path_components should reset to absolute component and continue joining"
        );
        assert!(
            run_output.status.success(),
            "join absolute-reset binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_join_path_components_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_join_path_components")
            .expect("_fs_join_path_components guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_join_path_components");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_join_path_components fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_join_path_components");
        let temp_dir = unique_probe_target_dir("join-fixture-showcase");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_join_path_components fixture should compile into a binary: {}",
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
            "_fs_join_path_components compiled binary should execute: {}",
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

        let expected = vec![
            "home + [user, docs] -> home/user/docs",
            "a/ + [b] -> a/b",
            "a + [/b, c] -> /b/c",
            "a + [] -> a",
            "`` + [x] -> x",
        ];

        assert_eq!(
            lines, expected,
            "_fs_join_path_components fixture should print the 5 locked join cases"
        );

        assert!(
            run_output.status.success(),
            "_fs_join_path_components binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_join_path_components");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn join_windows_absolute_components_reset_accumulator() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("join windows-absolute guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, join_path_components, path_to_string from standard

##
  Description: Verifies Windows absolute join components replace the accumulator.
##
entry main = f(args: string[]): void =>
    let drive_reset = join_path_components(path_from('base'), ['C:\\\\abs'])
    print('drive-reset={path_to_string(drive_reset)}')

    let mixed_drive_reset = join_path_components(path_from('base/child'), ['C:/abs', 'leaf'])
    print('mixed-drive-reset={path_to_string(mixed_drive_reset)}')

    let unc_reset = join_path_components(path_from('base'), ['\\\\\\\\server\\\\share\\\\dir', 'file.ext'])
    print('unc-reset={path_to_string(unc_reset)}')
    return void
";

        let temp_dir = unique_probe_target_dir("join-windows-absolute-reset");

        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_fs_path_from/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "join windows-absolute source should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), stringify_error)
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
            "join windows-absolute compiled binary should execute: {}",
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
            .filter(|line| !line.is_empty())
            .collect();

        let expected = vec![
            "drive-reset=C:/abs",
            "mixed-drive-reset=C:/abs/leaf",
            "unc-reset=//server/share/dir/file.ext",
        ];

        assert_eq!(
            lines, expected,
            "join_path_components should treat drive and UNC inputs as absolute resets"
        );

        assert!(
            run_output.status.success(),
            "join windows-absolute binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn join_canonical_matrix() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_from")
            .expect("join matrix guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_from");

        let source = "
import path_from, join_path_components, path_parent_directory, path_file_name from standard

##
  Description: Prints join matrix for lexical component joining.
##
entry main = f(args: string[]): void =>
    let r0 = join_path_components(path_from('a'), ['b', 'c'])
    print('a+[b,c] -> parent_name={path_file_name(path_parent_directory(r0))} name={path_file_name(r0)}')

    let r1 = join_path_components(path_from('a'), ['/b', 'c'])
    print('a+[/b,c] -> parent_name={path_file_name(path_parent_directory(r1))} name={path_file_name(r1)}')

    let r2 = join_path_components(path_from('a/'), ['b'])
    print('a/+[b] -> parent_name={path_file_name(path_parent_directory(r2))} name={path_file_name(r2)}')

    let r3 = join_path_components(path_from('a'), ['', '.'])
    print('a+[empty,dot] -> parent_name={path_file_name(path_parent_directory(r3))} name={path_file_name(r3)}')

    let r4 = join_path_components(path_from(''), ['x'])
    print('empty+[x] -> parent_name={path_file_name(path_parent_directory(r4))} name={path_file_name(r4)}')

    return void
";

        let temp_dir = unique_probe_target_dir("join-canonical-matrix");

        let binary_result = compile_program_for_tests(
            Path::new("test-projects/_fs_path_from/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "join matrix source should compile into a binary: {}",
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
            "join matrix compiled binary should execute: {}",
            output_result.as_ref().err().map_or_else(
                || String::from("unknown execution error"),
                alloc::string::ToString::to_string
            )
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect();

        let expected = vec![
            "a+[b,c] -> parent_name=b name=c",
            "a+[/b,c] -> parent_name=b name=c",
            "a/+[b] -> parent_name=a name=b",
            "a+[empty,dot] -> parent_name=. name=a",
            "empty+[x] -> parent_name=. name=x",
        ];

        assert_eq!(
            lines, expected,
            "join_path_components should satisfy the canonical join matrix"
        );

        assert!(
            run_output.status.success(),
            "join matrix binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_from");
}
