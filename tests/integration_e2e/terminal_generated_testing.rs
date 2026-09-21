#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn generated_terminal_fixture_uses_injected_fake_backend() {
    let temp_dir = unique_probe_target_dir("terminal-generated-fake-backend");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-generated-fake-backend target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-generated-fake-backend/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-generated-fake-backend source should be readable: {error}")
        })?;
        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal-generated-fake-backend source should compile: {error}")
        })?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .env("OPAL_TERMINAL_FAKE_EVENTS", "text:hello")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!("terminal-generated-fake-backend binary should execute: {error}")
            })?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-generated-fake-backend compiled binary",
        )?;

        if !run_output.status.success() {
            return Err(format!(
                "terminal-generated-fake-backend should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("FAKE_FRAME_OK\nTERMINAL_FAKE_DONE\n") {
            return Err(format!(
                "terminal-generated-fake-backend stdout should include captured trusted output and summary, got {stdout:?}"
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-generated-fake-backend target directory should be removed"
    );

    assert!(
        execution_result.is_ok(),
        "terminal-generated-fake-backend should compile and run: {}",
        execution_result.err().unwrap_or_default()
    );
}
