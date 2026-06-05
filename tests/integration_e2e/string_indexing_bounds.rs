#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

#[test]
fn string_indexing_bounds_runtime_error_surfaces_index_out_of_bounds_error() {
    let cwd = std::env::current_dir().expect("current working directory should be readable");
    let project_dir = cwd.join("test-projects/string-indexing-bounds");
    let temp_dir = unique_probe_target_dir("string-indexing-bounds");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-indexing-bounds target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("string-indexing-bounds project should compile into a binary: {error}")
            })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            Duration::from_secs(30),
            "string-indexing-bounds compiled binary",
        )?;

        assert_index_out_of_bounds_error_stderr(
            &run_output,
            "string-indexing-bounds/src/main.op",
            "let first: string = propagate message.at(message.length)",
            "message.at(message.length)",
        )
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-indexing-bounds target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "string-indexing-bounds runtime error should surface IndexOutOfBoundsError on stderr: {failure_message}"
    );
}

#[test]
fn string_indexing_empty_string_runtime_error_surfaces_index_out_of_bounds_error() {
    const SOURCE_PATH: &str = "test-projects/string-indexing-bounds/src/empty-string.main.op";
    const SOURCE: &str = "import print from standard\n\n##\n    Description: Entry function validates empty-string indexing diagnostics\n##\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let message = ''\n    let first: string = propagate message.at(0)\n    print(first)\n    return void\n}\n";
    let temp_dir = unique_probe_target_dir("string-indexing-bounds-empty-string");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-indexing-bounds-empty-string target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            SOURCE,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("empty-string string-indexing source should compile into a binary: {error}")
        })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            Duration::from_secs(30),
            "string-indexing-bounds-empty-string compiled binary",
        )?;

        assert_index_out_of_bounds_error_stderr(
            &run_output,
            SOURCE_PATH,
            "let first: string = propagate message.at(0)",
            "message.at(0)",
        )
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-indexing-bounds-empty-string target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "empty-string string indexing should surface IndexOutOfBoundsError on stderr: {failure_message}"
    );
}

#[test]
fn string_indexing_negative_expression_runtime_error_surfaces_index_out_of_bounds_error() {
    const SOURCE_PATH: &str =
        "test-projects/string-indexing-bounds/src/negative-expression.main.op";
    const SOURCE: &str = "import print from standard\n\n##\n    Description: Entry function validates negative-expression indexing diagnostics\n##\nentry main = f(): void errors IndexOutOfBoundsError => {\n    let message = 'opal'\n    let first: string = propagate message.at(0 - 1)\n    print(first)\n    return void\n}\n";
    let temp_dir = unique_probe_target_dir("string-indexing-bounds-negative-expression");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-indexing-bounds-negative-expression target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            SOURCE,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "negative-expression string-indexing source should compile into a binary: {error}"
            )
        })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            Duration::from_secs(30),
            "string-indexing-bounds-negative-expression compiled binary",
        )?;

        assert_index_out_of_bounds_error_stderr(
            &run_output,
            SOURCE_PATH,
            "let first: string = propagate message.at(0 - 1)",
            "message.at(0 - 1)",
        )
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-indexing-bounds-negative-expression target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "negative-expression string indexing should surface IndexOutOfBoundsError on stderr: {failure_message}"
    );
}

fn assert_index_out_of_bounds_error_stderr(
    run_output: &std::process::Output,
    source_path: &str,
    source_line: &str,
    indexed_expression: &str,
) -> Result<(), String> {
    let stderr = String::from_utf8_lossy(&run_output.stderr);
    if run_output.status.success() {
        return Err(format!(
            "string-indexing bounds binary should exit non-zero, stdout='{}', stderr='{}'",
            String::from_utf8_lossy(&run_output.stdout),
            stderr,
        ));
    }

    let trimmed_stderr = stderr.trim();
    if trimmed_stderr != "IndexOutOfBoundsError" {
        return Err(format!(
            "string-indexing bounds stderr should surface IndexOutOfBoundsError, got: '{stderr}'"
        ));
    }

    if !run_output.stdout.is_empty() {
        return Err(format!(
            "string-indexing bounds binary should not print stdout on failure for {source_path} at '{source_line}' ({indexed_expression}), got: '{}'",
            String::from_utf8_lossy(&run_output.stdout),
        ));
    }

    Ok(())
}
