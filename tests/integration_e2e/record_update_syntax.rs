#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::time::Duration;

const RECORD_UPDATE_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn block_record_update_project_compiles_and_runs() {
    let cwd = std::env::current_dir().expect("current directory should be readable");
    let project_dir = cwd.join("test-projects/record-update-block");
    let temp_dir = unique_probe_target_dir("record-update-block");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "record update temp directory should be created"
    );

    let result: Result<(), String> = (|| {
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| format!("record-update-block should compile: {error}"))?;
        let output = run_binary_output_with_timeout(
            &binary_path,
            RECORD_UPDATE_TIMEOUT,
            "record-update-block binary",
        )?;
        if !output.status.success() {
            return Err(format!(
                "record-update-block should exit successfully, status {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "ORIGINAL saved=0 dirty=0 cursor=1,2 status=initial",
            "UPDATED saved=1 dirty=1 cursor=3,4 status=moved",
            "RENAMED status=renamed cursor=3,9",
        ] {
            if !stdout.contains(expected) {
                return Err(format!(
                    "record-update-block stdout should contain {expected:?}, got:\n{stdout}"
                ));
            }
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "record update temp directory should be removed"
    );
    assert!(
        result.is_ok(),
        "record-update-block should compile and run: {}",
        result.err().unwrap_or_default()
    );
}
