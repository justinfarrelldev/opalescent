#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn generated_terminal_rendering_fixture_uses_high_level_session_operations() {
    let temp_dir = unique_probe_target_dir("terminal-session-rendering");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-session-rendering target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-session-rendering/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-session-rendering source should be readable: {error}")
        })?;
        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal-session-rendering source should compile: {error}"))?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                format!("terminal-session-rendering binary should execute: {error}")
            })?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-session-rendering compiled binary",
        )?;
        if !run_output.status.success() {
            return Err(format!(
                "terminal-session-rendering should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains(
            "\u{1b}[2J\u{1b}[3J\u{1b}[H\u{1b}[2;3Halpha\nbeta\n\u{7}TERMINAL_RENDER_DONE\n",
        ) {
            return Err(format!(
                "terminal-session-rendering stdout should include clear, cursor move, rows, bell, and summary, got {stdout:?}"
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-session-rendering target directory should be removed"
    );
    assert!(
        execution_result.is_ok(),
        "terminal-session-rendering should compile and run: {}",
        execution_result.err().unwrap_or_default()
    );
}

#[test]
fn generated_terminal_session_open_without_fake_backend_reports_gated_scope() {
    let temp_dir = unique_probe_target_dir("terminal-session-open-without-fake");
    prepare_dir(&temp_dir).expect("terminal open without fake target directory should be created");

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-session-rendering/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-session-rendering source should be readable: {error}")
        })?;
        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal-session-rendering source should compile: {error}"))?;

        let child = Command::new(&binary_path)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("terminal no-fake binary should execute: {error}"))?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-session-open-without-fake compiled binary",
        )?;
        if run_output.status.success() {
            return Err(String::from(
                "terminal no-fake binary should fail while production generated sessions are gated",
            ));
        }
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if !stderr.contains("FakeBackendNotInjected") {
            return Err(format!(
                "terminal no-fake stderr should mention FakeBackendNotInjected, got: {stderr}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal open without fake target directory should be removed");
    assert!(
        execution_result.is_ok(),
        "terminal no-fake run should document generated production gate: {}",
        execution_result.err().unwrap_or_default()
    );
}

#[test]
fn generated_terminal_invalid_cursor_reports_named_error() {
    let temp_dir = unique_probe_target_dir("terminal-session-invalid-cursor");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-session-invalid-cursor target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source = "import terminal_session_options_default, terminal_session_open_sync from 'standard.terminal'\nimport terminal_session_move_cursor_sync from 'standard.terminal'\nimport type TerminalSessionOpenError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError from 'standard.terminal'\n\n##\n  Description: Generated terminal fixture verifies invalid cursor error naming.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, InvalidCursorPositionError =>\n    let options = terminal_session_options_default()\n    let mutable session = propagate terminal_session_open_sync(options)\n    propagate terminal_session_move_cursor_sync(mutable ref session, 0 as int32, 1 as int32)\n    print('UNEXPECTED_CURSOR_SUCCESS')\n    return void\n";
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-session-invalid-cursor/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal invalid cursor source should compile: {error}"))?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("terminal invalid cursor binary should execute: {error}"))?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-session-invalid-cursor compiled binary",
        )?;
        if run_output.status.success() {
            return Err(format!(
                "terminal invalid cursor binary should fail, stdout:\n{}",
                String::from_utf8_lossy(&run_output.stdout),
            ));
        }
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if !stderr.contains("InvalidCursorPositionError") {
            return Err(format!(
                "terminal invalid cursor stderr should mention InvalidCursorPositionError, got {stderr}"
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-session-invalid-cursor target directory should be removed"
    );
    assert!(
        execution_result.is_ok(),
        "terminal invalid cursor should expose named error: {}",
        execution_result.err().unwrap_or_default()
    );
}

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
            .env("OPAL_TERMINAL_FAKE_EVENTS", "text:hello|text:world")
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
