#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn compiler_owned_string_semantics_project_runs_without_defensive_copies() {
    let temp_dir = unique_probe_target_dir("compiler-owned-string-semantics");
    prepare_dir(&temp_dir)
        .expect("compiler-owned-string-semantics temp directory should be created");

    let result: Result<(), String> = (|| {
        let project_dir = std::env::current_dir()
            .map_err(|error| format!("current directory should be readable: {error}"))?
            .join("test-projects/compiler-owned-string-semantics");
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("compiler-owned string fixture should compile into a binary: {error}")
            })?;

        let output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "compiler-owned string semantics binary",
        )?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(format!(
                "compiler-owned string fixture should exit successfully, status {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                output.status.code()
            ));
        }

        let expected =
            fs::read_to_string("test-projects/compiler-owned-string-semantics/expected/stdout.txt")
                .map_err(|error| format!("expected stdout fixture should be readable: {error}"))?;
        if stdout != expected {
            return Err(format!(
                "compiler-owned string stdout mismatch\nexpected:\n{expected:?}\nactual:\n{stdout:?}\nstderr:\n{stderr}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir)
        .expect("compiler-owned-string-semantics temp directory should be removed");
    assert!(
        result.is_ok(),
        "compiler-owned string semantics fixture should compile and run: {}",
        result.err().unwrap_or_default()
    );
}
