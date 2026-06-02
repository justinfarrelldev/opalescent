#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

#[test]
fn string_indexing_project_compiles_and_runs() {
    let cwd = std::env::current_dir().expect("current working directory should be readable");
    let project_dir = cwd.join("test-projects/string-indexing");
    let temp_dir = unique_probe_target_dir("string-indexing");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "string-indexing target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("string-indexing project should compile into a binary: {error}")
            })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            Duration::from_secs(30),
            "string-indexing compiled binary",
        )?;
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if stdout.trim() != "oa" {
            return Err(format!(
                "string-indexing binary stdout should be 'oa', got: '{stdout}'"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "string-indexing binary should exit with status code 0, got: {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "string-indexing target directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "string-indexing project should compile, run, and print the indexed string characters: {failure_message}"
    );
}
