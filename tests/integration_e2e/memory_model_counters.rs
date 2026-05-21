#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const COUNTER_HARNESS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn make_workspace_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn compile_counter_harness(
    temp_dir: &Path,
    fixture_name: &str,
    binary_name: &str,
) -> Result<PathBuf, String> {
    let repo_root = repo_root();
    let harness_bin = temp_dir.join(binary_name);
    let fixture_path = repo_root
        .join("tests/integration_e2e/fixtures")
        .join(fixture_name);

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
        .arg("-DOPAL_ENABLE_INTERNAL_TESTING")
        .arg("-I.")
        .arg("runtime/opal_rc.c")
        .arg("runtime/opal_error.c")
        .arg("runtime/opal_string.c")
        .arg("runtime/opal_bytes.c")
        .arg("runtime/opal_fs.c")
        .arg(&fixture_path)
        .arg("-o")
        .arg(&harness_bin)
        .current_dir(&repo_root);

    let compile = run_command_output_with_timeout(
        &mut compile_command,
        COUNTER_HARNESS_TIMEOUT,
        "memory model counters harness compile command",
    )?;

    if !compile.status.success() {
        return Err(format!(
            "memory model counters harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            String::from_utf8_lossy(&compile.stdout),
            String::from_utf8_lossy(&compile.stderr)
        ));
    }

    Ok(harness_bin)
}

fn run_positive_counter_harness(temp_dir: &Path, workspace_root: &Path) -> Result<String, String> {
    let harness_bin = compile_counter_harness(
        temp_dir,
        "memory_model_counters.c",
        "memory_model_counters_harness",
    )?;

    let mut run_command = Command::new(&harness_bin);
    run_command.arg(workspace_root);
    let output = run_command_output_with_timeout(
        &mut run_command,
        COUNTER_HARNESS_TIMEOUT,
        "memory model counters harness",
    )?;

    drop(std::fs::remove_file(&harness_bin));

    if !output.status.success() {
        return Err(format!(
            "memory model counters harness should exit 0, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[test]
#[serial(fs)]
fn memory_model_counters() {
    let temp_dir = unique_probe_target_dir("memory-model-counters");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "memory model counters temp directory should be created"
    );

    let workspace_root = make_workspace_root("memory-model-counters-workspace");
    let execution_result: Result<(), String> = (|| {
        let stdout = run_positive_counter_harness(&temp_dir, &workspace_root)?;

        let expected_categories = [
            "counter:strings alloc=",
            "counter:arrays alloc=",
            "counter:bytes alloc=",
            "counter:builders alloc=",
            "counter:filesystem_objects alloc=",
            "counter:metadata_permissions alloc=",
            "counter:error_payloads alloc=",
            "counter:rc_child_arrays alloc=",
        ];

        for expected in expected_categories {
            if !stdout.contains(expected) {
                return Err(format!(
                    "memory model counters output should include '{expected}', got: {stdout}"
                ));
            }
        }

        if !stdout.contains("counter_status=balanced") {
            return Err(format!(
                "memory model counters output should report balanced counters, got: {stdout}"
            ));
        }

        for line in stdout.lines().filter(|line| line.starts_with("counter:")) {
            if line.contains(" alloc=0") {
                return Err(format!(
                    "memory model counters line should show an exercised category, got: {line}"
                ));
            }
            if !line.contains(" live=0") {
                return Err(format!(
                    "memory model counters line should end with zero live objects, got: {line}"
                ));
            }
        }

        Ok(())
    })();

    drop(std::fs::remove_dir_all(&workspace_root));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "memory model counters temp directory should be removed"
    );

    let failure_message = execution_result.err().unwrap_or_default();
    assert!(
        failure_message.is_empty(),
        "memory model counters should surface all tracked categories with balanced counts: {failure_message}"
    );
}
