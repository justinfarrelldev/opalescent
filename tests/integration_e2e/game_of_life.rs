#![cfg(feature = "integration")]

use super::fs_helpers::{FsStateGuard, fs_project_root};
use super::*;
use serial_test::serial;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(10);

#[test]
#[serial(fs)]
fn game_of_life_ten_frames() {
    let project_name = "game-of-life";
    let project_dir = fs_project_root(project_name);
    let expected_path = project_dir.join("fixtures/expected_10_frames.txt");

    let execution_result: Result<(), String> = (|| {
        let _guard = FsStateGuard::new("test-projects/game-of-life")
            .map_err(|error| format!("game-of-life guard should initialize: {error}"))?;

        let temp_dir = super::fs_helpers::unique_probe_target_dir("game-of-life-ten-frames");
        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| {
                format!("game-of-life fixture should compile into a binary: {error}")
            })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "game-of-life compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "game-of-life binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = fs::read(&expected_path).map_err(|error| {
            format!("game-of-life expected fixture should be readable: {error}")
        })?;
        if run_output.stdout != expected {
            return Err(format!(
                "game-of-life stdout should match expected_10_frames.txt byte-for-byte\nexpected: {:?}\nactual: {:?}",
                expected, run_output.stdout,
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        for frame in 0_i32..10_i32 {
            let header = format!("Frame {frame}");
            if !stdout.contains(&header) {
                return Err(format!(
                    "game-of-life output should contain {header:?}, got: {stdout:?}"
                ));
            }
        }
        if stdout.contains("Frame 10") {
            return Err(format!(
                "game-of-life output must stop at Frame 9 and must not contain \"Frame 10\": {stdout:?}"
            ));
        }

        Ok(())
    })();

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "game-of-life should compile, run within the bounded timeout, emit Frame 0..Frame 9, and match the golden fixture: {failure_message}"
    );
}
