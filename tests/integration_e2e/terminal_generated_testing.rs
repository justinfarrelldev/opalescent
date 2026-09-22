#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::fd::{AsRawFd, FromRawFd};

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn run_terminal_real_utf8_probe(label: &str, input_bytes: &[u8]) -> Result<String, String> {
    let temp_dir = unique_probe_target_dir(label);
    prepare_dir(&temp_dir)
        .map_err(|error| format!("{label} target directory setup failed: {error}"))?;

    let result: Result<String, String> = (|| {
        let source_path = Path::new("test-projects/terminal-real-utf8-probe/src/main.op");
        let source = "import terminal_session_options_default, terminal_session_open_sync, terminal_session_read_event_sync, terminal_session_write_sync, terminal_session_flush_sync, terminal_session_close_sync, trusted_terminal_output_from_application_text, cancellation_source_new, cancellation_token from standard\nimport type TerminalWait, TerminalInputEvent, TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError from standard\n\n##\n  Description: Generated terminal fixture verifies real terminal UTF-8 text vs unknown-byte classification.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError =>\n    let source = propagate cancellation_source_new()\n    let token = cancellation_token(ref source)\n    let options = terminal_session_options_default()\n    let mutable session = propagate terminal_session_open_sync(options)\n    let event = propagate terminal_session_read_event_sync(mutable ref session, new TerminalWait.Poll, token)\n    if event is TerminalInputEvent.UnknownBytes:\n        let marker = propagate trusted_terminal_output_from_application_text('UTF8_UNKNOWN\\n')\n        propagate terminal_session_write_sync(ref session, marker)\n    if event is TerminalInputEvent.TextInput:\n        let marker = propagate trusted_terminal_output_from_application_text('UTF8_TEXT\\n')\n        propagate terminal_session_write_sync(ref session, marker)\n    propagate terminal_session_flush_sync(ref session)\n    let _close = propagate terminal_session_close_sync(mutable ref session)\n    return void\n";
        let binary_path =
            compile_program_for_tests(source_path, source, &temp_dir, &TargetTriple::host())
                .map_err(|error| format!("{label} fixture should compile: {error}"))?;
        let mut child = Command::new(&binary_path)
            .env_remove("OPAL_TERMINAL_FAKE_BACKEND")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{label} binary should spawn: {error}"))?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(input_bytes)
                .map_err(|error| format!("{label} bytes should write to stdin: {error}"))?;
        } else {
            return Err(format!("{label} stdin should be piped"));
        }
        let output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            format!("{label} compiled binary").as_str(),
        )?;
        if !output.status.success() {
            return Err(format!(
                "{label} binary should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    })();

    cleanup_dir(&temp_dir)
        .map_err(|error| format!("{label} target directory cleanup failed: {error}"))?;
    result
}

#[cfg(unix)]
fn duplicate_stdio_from_fd(fd: std::os::fd::RawFd) -> Result<Stdio, String> {
    // SAFETY: `fd` is an open pty slave descriptor owned by this test process.
    // `dup` returns a new descriptor or -1 without taking ownership of `fd`.
    let duplicated = unsafe { libc::dup(fd) };
    if duplicated < 0_i32 {
        return Err(format!(
            "dup failed for pty slave fd: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: `duplicated` is a fresh descriptor from `dup`, so transferring it
    // into `Stdio` gives exactly one Rust owner for that duplicated descriptor.
    Ok(unsafe { Stdio::from_raw_fd(duplicated) })
}

#[cfg(unix)]
fn set_fd_nonblocking(fd: std::os::fd::RawFd) -> Result<(), String> {
    // SAFETY: `fd` is an open pty master descriptor. `F_GETFL` only reads the
    // descriptor flags and does not require additional aliasing guarantees.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0_i32 {
        return Err(format!(
            "fcntl F_GETFL failed for pty master: {}",
            std::io::Error::last_os_error()
        ));
    }
    // SAFETY: `fd` remains open and owned by this test. `F_SETFL` updates only
    // descriptor status flags; preserving `flags` keeps existing settings.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0_i32 {
        return Err(format!(
            "fcntl F_SETFL O_NONBLOCK failed for pty master: {}",
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn drain_pty_output(master: &mut std::fs::File, output: &mut Vec<u8>) -> Result<(), String> {
    let mut buffer = [0_u8; 512];
    loop {
        match master.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => output.extend_from_slice(&buffer[..count]),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) if error.raw_os_error() == Some(libc::EIO) => return Ok(()),
            Err(error) => return Err(format!("failed to read pty output: {error}")),
        }
    }
}

#[cfg(unix)]
#[test]
fn generated_terminal_real_tty_delayed_arrow_sequence_stays_one_key_event() {
    let temp_dir = unique_probe_target_dir("terminal-real-tty-delayed-arrow");
    prepare_dir(&temp_dir).expect("terminal delayed arrow target directory should be created");

    let result: Result<(), String> = (|| {
        let source = "import terminal_session_options_default, terminal_session_open_sync, terminal_session_read_event_sync, terminal_session_close_sync, cancellation_source_new, cancellation_token from standard\nimport type TerminalWait, TerminalInputEvent, TerminalLogicalKey, TerminalNamedKey, TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionStateError, TerminalSessionRestoreError from standard\n\n##\n  Description: Generated terminal fixture verifies delayed real-tty arrow escape sequences remain atomic.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError =>\n    let source = propagate cancellation_source_new()\n    let token = cancellation_token(ref source)\n    let options = terminal_session_options_default()\n    let mutable session = propagate terminal_session_open_sync(options)\n    let event = propagate terminal_session_read_event_sync(mutable ref session, new TerminalWait.Forever, token)\n    let mutable summary_code: int64 = 0\n    if event is TerminalInputEvent.Key into key_event:\n        let logical = key_event.key\n        if logical is TerminalLogicalKey.Named into named:\n            if named.key is TerminalNamedKey.ArrowDown:\n                summary_code = 1\n            if named.key is TerminalNamedKey.Escape:\n                summary_code = 2\n        if logical is TerminalLogicalKey.Text:\n            summary_code = 3\n    let _closed = propagate terminal_session_close_sync(mutable ref session)\n    if summary_code is 1:\n        print('ARROW_DOWN')\n        return void\n    if summary_code is 2:\n        print('ESCAPE')\n        return void\n    if summary_code is 3:\n        print('TEXT')\n        return void\n    print('OTHER')\n    return void\n";
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-real-tty-delayed-arrow/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal delayed arrow fixture should compile: {error}"))?;

        let mut master_fd: libc::c_int = -1_i32;
        let mut slave_fd: libc::c_int = -1_i32;
        // SAFETY: all output pointers are valid for writes for the duration of
        // the call, and null termios/winsize pointers request default settings.
        let open_result = unsafe {
            libc::openpty(
                &mut master_fd,
                &mut slave_fd,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            )
        };
        if open_result != 0_i32 {
            return Err(format!(
                "openpty failed for delayed arrow test: {}",
                std::io::Error::last_os_error()
            ));
        }
        // SAFETY: `master_fd` is returned by `openpty` and has not yet been
        // transferred to any Rust owner; `File` now owns and closes it.
        let mut master = unsafe { std::fs::File::from_raw_fd(master_fd) };
        // SAFETY: `slave_fd` is returned by `openpty` and has not yet been
        // transferred to any Rust owner; `File` now owns and closes it.
        let slave = unsafe { std::fs::File::from_raw_fd(slave_fd) };
        set_fd_nonblocking(master.as_raw_fd())?;
        let slave_raw = slave.as_raw_fd();
        let mut child = Command::new(&binary_path)
            .env_remove("OPAL_TERMINAL_FAKE_BACKEND")
            .stdin(duplicate_stdio_from_fd(slave_raw)?)
            .stdout(duplicate_stdio_from_fd(slave_raw)?)
            .stderr(duplicate_stdio_from_fd(slave_raw)?)
            .spawn()
            .map_err(|error| format!("terminal delayed arrow binary should spawn: {error}"))?;
        drop(slave);

        std::thread::sleep(Duration::from_millis(50));
        master
            .write_all(b"\x1b")
            .map_err(|error| format!("delayed arrow ESC byte should write to pty: {error}"))?;
        std::thread::sleep(Duration::from_millis(20));
        master
            .write_all(b"[B")
            .map_err(|error| format!("delayed arrow suffix should write to pty: {error}"))?;
        master
            .flush()
            .map_err(|error| format!("delayed arrow pty input should flush: {error}"))?;

        let started = Instant::now();
        let mut output = Vec::new();
        let status = loop {
            drain_pty_output(&mut master, &mut output)?;
            if let Some(status) = child
                .try_wait()
                .map_err(|error| format!("failed to poll delayed arrow child: {error}"))?
            {
                std::thread::sleep(Duration::from_millis(25));
                drain_pty_output(&mut master, &mut output)?;
                break status;
            }
            if started.elapsed() > GENERATED_BINARY_TEST_TIMEOUT {
                if let Err(error) = child.kill() {
                    return Err(format!(
                        "terminal delayed arrow binary timed out and kill failed: {error}; output so far: {:?}",
                        String::from_utf8_lossy(&output)
                    ));
                }
                return Err(format!(
                    "terminal delayed arrow binary timed out, output so far: {:?}",
                    String::from_utf8_lossy(&output)
                ));
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if !status.success() {
            return Err(format!(
                "terminal delayed arrow binary should exit cleanly, status {:?}, output {:?}",
                status.code(),
                String::from_utf8_lossy(&output)
            ));
        }
        let output_text = String::from_utf8_lossy(&output);
        if !output_text.contains("ARROW_DOWN") || output_text.contains("ESCAPE") {
            return Err(format!(
                "delayed ESC [ B should be one ArrowDown key event, got pty output {output_text:?}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal delayed arrow target directory should be removed");
    assert!(
        result.is_ok(),
        "terminal delayed arrow sequence should remain atomic: {}",
        result.err().unwrap_or_default()
    );
}

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
            "\u{1b}[2J\u{1b}[3J\u{1b}[H\u{1b}[2;3Halpha\r\nbeta\r\n\u{7}TERMINAL_RENDER_DONE\n",
        ) {
            return Err(format!(
                "terminal-session-rendering stdout should include clear, cursor move, CRLF-delimited rows, bell, and summary, got {stdout:?}"
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
fn generated_terminal_real_utf8_accepts_multibyte_text() {
    let stdout = run_terminal_real_utf8_probe("terminal-real-utf8-valid", &[0xC3, 0xA9])
        .expect("valid UTF-8 probe should compile and run");
    assert!(
        stdout.contains("UTF8_TEXT") && !stdout.contains("UTF8_UNKNOWN"),
        "valid UTF-8 bytes should produce TextInput, got stdout {stdout:?}"
    );
}

#[test]
fn generated_terminal_real_utf8_rejects_malformed_sequences() {
    for (label, bytes) in [
        ("terminal-real-utf8-surrogate", &[0xED, 0xA0, 0x80][..]),
        ("terminal-real-utf8-overlong", &[0xE0, 0x80, 0x80][..]),
        (
            "terminal-real-utf8-out-of-range",
            &[0xF4, 0x90, 0x80, 0x80][..],
        ),
    ] {
        let stdout = run_terminal_real_utf8_probe(label, bytes)
            .expect("malformed UTF-8 probe should compile and run");
        assert!(
            stdout.contains("UTF8_UNKNOWN") && !stdout.contains("UTF8_TEXT"),
            "malformed UTF-8 bytes for {label} should be quarantined as UnknownBytes, got stdout {stdout:?}"
        );
    }
}

#[test]
fn generated_terminal_session_open_without_fake_backend_uses_production_path() {
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
        if !run_output.status.success() {
            return Err(format!(
                "terminal no-fake binary should use the production session path, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stderr = String::from_utf8_lossy(&run_output.stderr);
        if stderr.contains("FakeBackendNotInjected") {
            return Err(format!(
                "terminal no-fake stderr should not mention FakeBackendNotInjected after production open support, got: {stderr}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal open without fake target directory should be removed");
    assert!(
        execution_result.is_ok(),
        "terminal no-fake run should succeed through production open support: {}",
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

#[test]
fn generated_terminal_from_standard_reexports_size_and_capabilities() {
    let temp_dir = unique_probe_target_dir("terminal-from-standard-size-caps");
    prepare_dir(&temp_dir).expect("terminal-from-standard-size-caps target directory should exist");

    let execution_result: Result<(), String> = (|| {
        let source = "import terminal_session_options_default, terminal_session_open_sync, terminal_session_size_sync, terminal_session_capabilities, terminal_capabilities_feature, terminal_capabilities_trusted_paste_framing, terminal_capabilities_color, terminal_session_write_sync, terminal_session_flush_sync, terminal_session_close_sync, trusted_terminal_output_from_application_text from standard\nimport type TerminalOrdinaryFeature, TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError from standard\n\n##\n  Description: Generated terminal fixture verifies standard re-exports plus size/capability lowerings.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError =>\n    let options = terminal_session_options_default()\n    let mutable session = propagate terminal_session_open_sync(options)\n    let _size = propagate terminal_session_size_sync(ref session)\n    let caps = terminal_session_capabilities(ref session)\n    let _feature = terminal_capabilities_feature(caps, new TerminalOrdinaryFeature.AlternateScreen)\n    let _paste = terminal_capabilities_trusted_paste_framing(caps)\n    let _color = terminal_capabilities_color(caps)\n    let marker = propagate trusted_terminal_output_from_application_text('STANDARD_SIZE_CAPS_OK\\n')\n    propagate terminal_session_write_sync(ref session, marker)\n    propagate terminal_session_flush_sync(ref session)\n    let _close = propagate terminal_session_close_sync(mutable ref session)\n    return void\n";
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-from-standard-size-caps/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("from-standard size/caps fixture should compile: {error}"))?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("from-standard size/caps binary should execute: {error}"))?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-from-standard-size-caps compiled binary",
        )?;
        if !run_output.status.success() {
            return Err(format!(
                "from-standard size/caps binary should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("STANDARD_SIZE_CAPS_OK\n") {
            return Err(format!(
                "from-standard size/caps stdout should include marker, got {stdout:?}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir)
        .expect("terminal-from-standard-size-caps target directory should be removed");
    assert!(
        execution_result.is_ok(),
        "from-standard size/caps fixture should compile and run: {}",
        execution_result.err().unwrap_or_default()
    );
}

#[test]
fn generated_terminal_typed_fake_events_expose_variants_and_payloads() {
    let temp_dir = unique_probe_target_dir("terminal-typed-fake-events");
    prepare_dir(&temp_dir).expect("terminal-typed-fake-events target directory should exist");

    let execution_result: Result<(), String> = (|| {
        let source = "import terminal_session_options_default, terminal_session_open_sync, terminal_session_read_event_sync, terminal_session_write_sync, terminal_session_flush_sync, terminal_session_close_sync, trusted_terminal_output_from_application_text, cancellation_source_new, cancellation_token from standard\nimport type TerminalWait, TerminalInputEvent, TerminalLogicalKey, TerminalNamedKey, TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError from standard\n\n##\n  Description: Generated terminal fixture verifies typed input event tags, refinements, and payload access.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError =>\n    let source = propagate cancellation_source_new()\n    let token = cancellation_token(ref source)\n    let options = terminal_session_options_default()\n    let mutable session = propagate terminal_session_open_sync(options)\n    let mutable reads: int64 = 0\n    while reads < 4:\n        let event = propagate terminal_session_read_event_sync(mutable ref session, new TerminalWait.Poll, token)\n        reads = reads + 1\n        if event is TerminalInputEvent.TextInput:\n            let echo = propagate trusted_terminal_output_from_application_text(event.text)\n            propagate terminal_session_write_sync(ref session, echo)\n            let marker = propagate trusted_terminal_output_from_application_text('|TEXT\\n')\n            propagate terminal_session_write_sync(ref session, marker)\n        if event is TerminalInputEvent.Key:\n            let logical = event.key\n            if logical is TerminalLogicalKey.Named into named:\n                if named.key is TerminalNamedKey.Escape:\n                    let marker = propagate trusted_terminal_output_from_application_text('ESC\\n')\n                    propagate terminal_session_write_sync(ref session, marker)\n        if event is TerminalInputEvent.Resize into resized:\n            let _size = resized.size\n            let marker = propagate trusted_terminal_output_from_application_text('RESIZE\\n')\n            propagate terminal_session_write_sync(ref session, marker)\n        if event is TerminalInputEvent.EndOfInput:\n            let marker = propagate trusted_terminal_output_from_application_text('EOF\\n')\n            propagate terminal_session_write_sync(ref session, marker)\n    propagate terminal_session_flush_sync(ref session)\n    let _close = propagate terminal_session_close_sync(mutable ref session)\n    print('DONE')\n    return void\n";
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-typed-fake-events/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("typed fake events fixture should compile: {error}"))?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .env(
                "OPAL_TERMINAL_FAKE_EVENTS",
                "text:hello|key:Escape|resize:24x80|eof",
            )
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("typed fake events binary should execute: {error}"))?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-typed-fake-events compiled binary",
        )?;
        if !run_output.status.success() {
            return Err(format!(
                "typed fake events binary should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        for expected in ["hello|TEXT\n", "ESC\n", "RESIZE\n", "EOF\n", "DONE\n"] {
            if !stdout.contains(expected) {
                return Err(format!(
                    "typed fake events stdout should include {expected:?}, got {stdout:?}"
                ));
            }
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-typed-fake-events target directory should be removed");
    assert!(
        execution_result.is_ok(),
        "typed fake events fixture should compile and run: {}",
        execution_result.err().unwrap_or_default()
    );
}

#[test]
fn generated_terminal_using_cleanup_scaffold_links() {
    let temp_dir = unique_probe_target_dir("terminal-using-cleanup-link");
    prepare_dir(&temp_dir).expect("terminal-using-cleanup-link target directory should exist");

    let execution_result: Result<(), String> = (|| {
        let source = "import terminal_session_options_default, terminal_session_open_sync, terminal_session_write_sync, trusted_terminal_output_from_application_text from standard\nimport type TerminalSessionOpenError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError from standard\n\n##\n  Description: Generated terminal fixture verifies lexical using cleanup links terminal close scaffold.\n##\nentry main = f(args: string[]): void errors TerminalSessionOpenError, TerminalSessionWriteError, TerminalSessionStateError, TerminalSessionRestoreError, AllocationFailureError =>\n    let options = terminal_session_options_default()\n    using session = propagate terminal_session_open_sync(options):\n        let marker = propagate trusted_terminal_output_from_application_text('USING_CLEANUP_OK\\n')\n        propagate terminal_session_write_sync(ref session, marker)\n    return void\n";
        let binary_path = compile_program_for_tests(
            Path::new("test-projects/terminal-using-cleanup-link/src/main.op"),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| format!("terminal using-cleanup fixture should compile/link: {error}"))?;

        let child = Command::new(&binary_path)
            .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("terminal using-cleanup binary should execute: {error}"))?;
        let run_output = fs_helpers::wait_for_child_output_with_timeout(
            child,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-using-cleanup-link compiled binary",
        )?;
        if !run_output.status.success() {
            return Err(format!(
                "terminal using-cleanup binary should exit cleanly, status {:?}\nstdout:\n{}\nstderr:\n{}",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stdout),
                String::from_utf8_lossy(&run_output.stderr),
            ));
        }
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("USING_CLEANUP_OK\n") {
            return Err(format!(
                "terminal using-cleanup stdout should include marker, got {stdout:?}"
            ));
        }
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-using-cleanup-link target directory should be removed");
    assert!(
        execution_result.is_ok(),
        "terminal using-cleanup fixture should compile, link, and run: {}",
        execution_result.err().unwrap_or_default()
    );
}
