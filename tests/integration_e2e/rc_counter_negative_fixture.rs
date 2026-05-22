#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;

const NEGATIVE_COUNTER_HARNESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compile_negative_counter_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let repo_root = repo_root();
    let harness_bin = temp_dir.join("rc_counter_negative_fixture_harness");
    let fixture_path =
        repo_root.join("tests/integration_e2e/fixtures/rc_counter_negative_fixture.c");

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
        .arg("-DOPAL_ENABLE_INTERNAL_TESTING")
        .arg("-I.")
        .arg("runtime/opal_rc.c")
        .arg(&fixture_path)
        .arg("-o")
        .arg(&harness_bin)
        .current_dir(&repo_root);

    let compile = run_command_output_with_timeout(
        &mut compile_command,
        NEGATIVE_COUNTER_HARNESS_TIMEOUT,
        "rc counter negative fixture harness compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "rc counter negative fixture harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(harness_bin)
}

fn run_negative_counter_harness(temp_dir: &Path) -> Result<String, String> {
    let harness_bin = compile_negative_counter_harness(temp_dir)?;

    let mut run_command = Command::new(&harness_bin);
    let output = run_command_output_with_timeout(
        &mut run_command,
        NEGATIVE_COUNTER_HARNESS_TIMEOUT,
        "rc counter negative fixture harness",
    )?;

    drop(std::fs::remove_file(&harness_bin));

    if !output.status.success() {
        return Err(format!(
            "rc counter negative fixture harness should exit 0 after detecting the imbalance, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
#[serial(fs)]
fn rc_counter_negative_fixture() {
    let temp_dir = unique_probe_target_dir("rc-counter-negative-fixture");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "rc counter negative fixture temp directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let stdout = run_negative_counter_harness(&temp_dir)?;

        if !stdout.contains("fixture_counter:rc_child_arrays alloc=1 free=0 live=1") {
            return Err(format!(
                "negative fixture should expose the imbalanced rc_child_arrays counts, got: {stdout}"
            ));
        }
        if !stdout.contains("fixture_status=imbalance-detected") {
            return Err(format!(
                "negative fixture should report imbalance-detected status, got: {stdout}"
            ));
        }
        if !stdout.contains(
            "fixture_message=rc counter imbalance detected for rc_child_arrays (alloc=1 free=0 live=1)",
        ) {
            return Err(format!(
                "negative fixture should print the deterministic imbalance message, got: {stdout}"
            ));
        }
        if stdout.contains("fixture_status=balanced") {
            return Err(format!(
                "negative fixture must not report balanced counters, got: {stdout}"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "rc counter negative fixture temp directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "rc counter negative fixture should detect and report the intentional imbalance: {failure_message}"
    );
}
