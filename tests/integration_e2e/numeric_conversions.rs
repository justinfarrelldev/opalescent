#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn numeric_conversions_project_compiles_and_runs() {
    let cwd = std::env::current_dir().expect("current working directory should be readable");
    let project_dir = cwd.join("test-projects/numeric-conversions");
    let temp_dir = unique_probe_target_dir("numeric-conversions");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "numeric conversion target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| format!("numeric conversions project should compile: {error}"))?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "numeric conversions compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let expected = "ok:2147483647:-32768:127:65535:9223372036854775807:42:1.5:1024\nrange:-1\nfractional:-1\nimprecise:0\n";
        if stdout != expected {
            return Err(format!(
                "numeric conversions stdout should equal {expected:?}, got {stdout:?}"
            ));
        }
        if !run_output.status.success() {
            return Err(format!(
                "numeric conversions binary should exit successfully, got {:?}",
                run_output.status.code()
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "numeric conversion target directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "numeric conversions project should compile, run, and print expected output: {failure_message}"
    );
}
