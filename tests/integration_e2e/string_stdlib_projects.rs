#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn string_search_stdlib_project_compiles_and_runs() {
    assert_project_stdout(
        "string-search-stdlib",
        "found:2\nfallback:7\nlast:5\n",
        "compile, run, and print search helper results",
    );
}

#[test]
fn string_lines_whitespace_stdlib_project_compiles_and_runs() {
    assert_project_stdout(
        "string-lines-whitespace-stdlib",
        "lines:2:alpha:beta\nblank-spaces:true\nblank-text:false\ntrimmed:[hi]\n",
        "compile, run, and print line splitting plus whitespace helper results",
    );
}

#[test]
fn string_ranges_stdlib_project_compiles_and_runs() {
    assert_project_stdout(
        "string-ranges-stdlib",
        "prefix:[hé🙂]\nsuffix:[🙂z]\nrange:[é🙂]\n",
        "compile, run, and print Unicode-scalar range helper results",
    );
}

fn assert_project_stdout(project_name: &str, expected_stdout: &str, expectation: &str) {
    let cwd = std::env::current_dir().expect("current working directory should be readable");
    let project_dir = cwd.join(format!("test-projects/{project_name}"));
    let temp_dir = unique_probe_target_dir(project_name);
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "{project_name} target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("{project_name} project should compile into a binary: {error}")
            })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            &format!("{project_name} compiled binary"),
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout != expected_stdout {
            return Err(format!(
                "{project_name} binary stdout should equal {expected_stdout:?}, got: {stdout:?}"
            ));
        }

        if !run_output.status.success() {
            return Err(format!(
                "{project_name} binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "{project_name} target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "{project_name} project should {expectation}: {failure_message}"
    );
}
