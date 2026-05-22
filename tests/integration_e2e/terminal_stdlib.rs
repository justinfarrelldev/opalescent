#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn terminal_clear_screen_ansi_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-clear-screen-ansi-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-clear-screen-ansi-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-clear-screen-ansi-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-clear-screen-ansi-bytes source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal-clear-screen-ansi-bytes source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-clear-screen-ansi-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-clear-screen-ansi-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"\x1b[2J\x1b[3J\x1b[H";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-clear-screen-ansi-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-clear-screen-ansi-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-clear-screen-ansi-bytes should compile, run, and write exact ANSI clear bytes: {failure_message}"
    );
}

#[test]
fn terminal_move_cursor_zero_based_ansi_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-move-cursor-zero-based-ansi-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-move-cursor-zero-based-ansi-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path =
            Path::new("test-projects/terminal-move-cursor-zero-based-ansi-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!(
                "terminal-move-cursor-zero-based-ansi-bytes source file should be readable: {error}"
            )
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "terminal-move-cursor-zero-based-ansi-bytes source should compile into a binary: {error}"
            )
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-move-cursor-zero-based-ansi-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-move-cursor-zero-based-ansi-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"\x1b[1;1H";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-move-cursor-zero-based-ansi-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-move-cursor-zero-based-ansi-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-move-cursor-zero-based-ansi-bytes should compile, run, and write exact ANSI cursor bytes: {failure_message}"
    );
}

#[test]
fn terminal_move_cursor_rejects_negative() {
    let temp_dir = unique_probe_target_dir("terminal-move-cursor-rejects-negative");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-move-cursor-rejects-negative target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        for (label, project_name) in [
            ("row=-1", "terminal-move-cursor-rejects-negative-row"),
            ("column=-1", "terminal-move-cursor-rejects-negative-column"),
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
            if !combined.contains("InvalidCursorPositionError") {
                return Err(format!(
                    "{project_name} output should contain InvalidCursorPositionError, status={:?}, stdout='{}', stderr='{}'",
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
        "terminal-move-cursor-rejects-negative target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-move-cursor-rejects-negative should compile, run, and report InvalidCursorPositionError: {failure_message}"
    );
}

#[test]
fn terminal_draw_rows_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-draw-rows-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-draw-rows-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-draw-rows-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-draw-rows-bytes source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal-draw-rows-bytes source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-draw-rows-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-draw-rows-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"##\n..";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-draw-rows-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-draw-rows-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-draw-rows-bytes should compile, run, and write exact row bytes: {failure_message}"
    );
}
