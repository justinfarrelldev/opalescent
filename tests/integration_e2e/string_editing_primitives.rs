#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn string_editing_primitives_project_compiles_and_runs() {
    const SOURCE_PATH: &str = "test-projects/string-editing-primitives/src/main.op";
    let source = std::fs::read_to_string(SOURCE_PATH)
        .expect("string-editing-primitives fixture should be readable");

    let temp_dir = unique_probe_target_dir("string-editing-primitives");
    prepare_dir(&temp_dir).expect("string-editing-primitives target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_program_for_tests(
            Path::new(SOURCE_PATH),
            &source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("string-editing-primitives source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "string-editing-primitives compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "string-editing-primitives binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "INSERT_ASCII=[abc]\nINSERT_UNICODE=[hé🙂z]\nDELETE_ASCII=[abef]\nDELETE_UNICODE=[héz]\nREPLACE_ASCII=[aXf]\nREPLACE_UNICODE=[hiz]\n";
        if stdout != expected {
            return Err(format!(
                "string-editing-primitives stdout should equal {expected:?}, got {stdout:?}"
            ));
        }

        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("string-editing-primitives target directory should be removed");

    assert!(
        execution_result.is_ok(),
        "string-editing-primitives should compile, run, and match expected output: {}",
        execution_result.err().unwrap_or_default()
    );
}
