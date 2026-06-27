#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn string_stdlib_failure_unhandled_string_join_reports_allocation_failure_error() {
    const SOURCE_PATH: &str = "test-projects/string-stdlib-failures/src/unhandled-string-join.main.op";
    const SOURCE: &str = "##\n    Description: Entry function verifies bare string_join compile failure\n##\nentry main = f(parts: string[]): string => {\n    return string_join(parts, ',')\n}\n";
    let temp_dir = unique_probe_target_dir("string-stdlib-unhandled-join");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-stdlib-unhandled-join target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let compile_error = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            SOURCE,
            &temp_dir,
            &TargetTriple::host(),
        )
        .expect_err("bare string_join should fail to compile");

        let CompileError::Report {
            report,
            normalized_source,
            ..
        } = compile_error
        else {
            return Err(format!(
                "bare string_join should fail with CompileError::Report, got: {compile_error}"
            ));
        };

        let rendered = opalescent::errors::renderer::render_report(
            SOURCE_PATH,
            &normalized_source,
            &report,
        );
        if !rendered.contains("AllocationFailureError")
            || !rendered.contains("unhandled_call_error")
        {
            return Err(format!(
                "bare string_join diagnostic should mention AllocationFailureError and unhandled_call_error, got:\n{rendered}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-stdlib-unhandled-join target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "bare string_join should be rejected with the expected diagnostic: {failure_message}"
    );
}

#[test]
fn string_stdlib_failure_unhandled_search_error_surfaces_runtime_error_name() {
    const SOURCE_PATH: &str = "test-projects/string-stdlib-failures/src/unhandled-search-error.main.op";
    const SOURCE: &str = "import string_find_last_index_of_text from standard\n\n##\n    Description: Entry function verifies propagated StringEmptySearchTextError reaches stderr\n##\nentry main = f(): void errors StringEmptySearchTextError, StringPatternNotFoundError => {\n    let _last: int64 = propagate string_find_last_index_of_text('hello', '')\n    return void\n}\n";
    let temp_dir = unique_probe_target_dir("string-stdlib-unhandled-search-error");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "string-stdlib-unhandled-search-error target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            SOURCE,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("string-stdlib-unhandled-search-error source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "string-stdlib-unhandled-search-error compiled binary",
        )?;

        if run_output.status.success() {
            return Err(format!(
                "string-stdlib-unhandled-search-error binary should exit non-zero, stdout='{}', stderr='{}'",
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if stderr.trim() != "StringEmptySearchTextError" {
            return Err(format!(
                "string-stdlib-unhandled-search-error stderr should surface StringEmptySearchTextError, got: '{stderr}'"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "string-stdlib-unhandled-search-error target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "propagated StringEmptySearchTextError should reach stderr as expected: {failure_message}"
    );
}
