#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;

fn stringify_error<E: core::fmt::Display>(error: E) -> String {
    format!("{error}")
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_path_helpers_query_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_path_helpers_query")
            .expect("_fs_path_helpers_query guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_path_helpers_query");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_path_helpers_query fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_path_helpers_query");
        let temp_dir = unique_probe_target_dir("path-helpers-query-fixture");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_fs_path_helpers_query fixture should compile into a binary: {}",
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
            "_fs_path_helpers_query compiled binary should execute: {}",
            output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), stringify_error)
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();

        let expected = vec![
            "/home/user/doc.pdf: ext=pdf, name=doc.pdf, parent=/home/user",
            "/home/user/: ext=, name=, parent=/home/user",
            "noext: ext=, name=noext, parent=.",
            "a/b/c.tar.gz: ext=gz, name=c.tar.gz, parent=a/b",
            "/: ext=, name=, parent=/",
            r"C:\Users\foo\bar.txt: ext=txt, name=bar.txt, parent=C:\Users\foo",
            "C:/Users/foo/bar.txt: ext=txt, name=bar.txt, parent=C:/Users/foo",
            r"\\server\share\dir\file.ext: ext=ext, name=file.ext, parent=\\server\share\dir",
            "relative/path/file: ext=, name=file, parent=relative/path",
            "relative/path/noext: ext=, name=noext, parent=relative/path",
        ];

        assert_eq!(
            lines, expected,
            "_fs_path_helpers_query fixture should print the exact 10-case helper matrix"
        );

        assert!(
            run_output.status.success(),
            "_fs_path_helpers_query binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_fs_path_helpers_query");
}
