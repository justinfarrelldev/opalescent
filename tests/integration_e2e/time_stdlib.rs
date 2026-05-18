#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::{Duration, Instant};

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn sleep_ms_sync_rejects_negative() {
    let temp_dir = unique_probe_target_dir("sleep-ms-sync-rejects-negative");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "sleep-ms-sync-rejects-negative target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/sleep-ms-sync-rejects-negative/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("sleep-ms-sync-rejects-negative source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("sleep-ms-sync-rejects-negative source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "sleep-ms-sync-rejects-negative compiled binary",
        )?;

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "sleep-ms-sync-rejects-negative binary unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("InvalidDurationError") {
            return Err(format!(
                "sleep-ms-sync-rejects-negative output should contain InvalidDurationError, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "sleep-ms-sync-rejects-negative target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "sleep-ms-sync-rejects-negative should compile, run, and report InvalidDurationError: {failure_message}"
    );
}

#[test]
fn sleep_ms_sync_50ms_timing() {
    let temp_dir = unique_probe_target_dir("sleep-ms-sync-50ms-timing");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "sleep-ms-sync-50ms-timing target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/sleep-ms-sync-50ms-timing/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("sleep-ms-sync-50ms-timing source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("sleep-ms-sync-50ms-timing source should compile into a binary: {error}")
        })?;

        let start = Instant::now();
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "sleep-ms-sync-50ms-timing compiled binary",
        )?;
        let elapsed = start.elapsed();

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "sleep-ms-sync-50ms-timing binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let elapsed_ms = elapsed.as_millis();
        if elapsed_ms < 45 {
            return Err(format!(
                "sleep-ms-sync-50ms-timing should take at least 45ms, got {elapsed_ms}ms"
            ));
        }
        if elapsed_ms > 5000 {
            return Err(format!(
                "sleep-ms-sync-50ms-timing should take at most 5000ms, got {elapsed_ms}ms"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "sleep-ms-sync-50ms-timing target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "sleep-ms-sync-50ms-timing should compile, run, and stay within timing bounds: {failure_message}"
    );
}

#[test]
fn frame_clock_rejects_invalid_fps() {
    let temp_dir = unique_probe_target_dir("frame-clock-rejects-invalid-fps");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "frame-clock-rejects-invalid-fps target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        for (label, project_name) in [
            ("fps=0", "frame-clock-rejects-zero-fps"),
            ("fps=-1", "frame-clock-rejects-negative-fps"),
        ] {
            let source_path =
                Path::new(&format!("test-projects/{project_name}/src/main.op")).to_path_buf();
            let source_str = fs::read_to_string(&source_path).map_err(|error| {
                format!("{project_name} source file should be readable: {error}")
            })?;

            let binary_path = compile_program_for_tests(
                source_path.as_path(),
                source_str.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            )
            .map_err(|error| {
                format!("{project_name} source should compile into a binary: {error}")
            })?;

            let run_output = run_binary_output_with_timeout(
                &binary_path,
                GENERATED_BINARY_TEST_TIMEOUT,
                &format!("{project_name} compiled binary"),
            )?;

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            let combined = format!("{stdout}\n{stderr}");

            if combined.contains("UNEXPECTED_SUCCESS") {
                return Err(format!(
                    "{project_name} binary unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
            if !combined.contains("InvalidFrameRateError") {
                return Err(format!(
                    "{project_name} output should contain InvalidFrameRateError, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
            if !combined.contains(label) {
                return Err(format!(
                    "{project_name} output should contain {label}, got stdout='{stdout}', stderr='{stderr}'"
                ));
            }
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "frame-clock-rejects-invalid-fps target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "frame-clock-rejects-invalid-fps should compile, run, and report InvalidFrameRateError for 0 and -1: {failure_message}"
    );
}

#[test]
fn frame_clock_30fps_ten_waits_timing() {
    let temp_dir = unique_probe_target_dir("frame-clock-30fps-ten-waits-timing");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "frame-clock-30fps-ten-waits-timing target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/frame-clock-30fps-ten-waits-timing/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("frame-clock-30fps-ten-waits-timing source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "frame-clock-30fps-ten-waits-timing source should compile into a binary: {error}"
            )
        })?;

        let start = Instant::now();
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "frame-clock-30fps-ten-waits-timing compiled binary",
        )?;
        let elapsed = start.elapsed();

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "frame-clock-30fps-ten-waits-timing binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let elapsed_ms = elapsed.as_millis();
        if elapsed_ms < 280 {
            return Err(format!(
                "frame-clock-30fps-ten-waits-timing should take at least 280ms, got {elapsed_ms}ms"
            ));
        }
        if elapsed_ms > 5000 {
            return Err(format!(
                "frame-clock-30fps-ten-waits-timing should take at most 5000ms, got {elapsed_ms}ms"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "frame-clock-30fps-ten-waits-timing target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "frame-clock-30fps-ten-waits-timing should compile, run, and stay within timing bounds: {failure_message}"
    );
}
