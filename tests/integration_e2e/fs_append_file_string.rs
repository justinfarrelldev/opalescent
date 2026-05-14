#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::fs_state_guard::FsStateGuard;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t22-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_append_error_source(path: &str, text: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, append_text_sync from standard\n\n##\n  Description: Integration probe that captures append_text_sync errors via guard.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidPathError, ReadFailureError, InvalidUtf8Error, OffsetOutOfRangeError, WriteFailureError, FilesystemFullError, CopyFailureError, DeleteFailureError, DirectoryNotFoundError, IsNotADirectoryError =>\n    guard append_text_sync(path_from('{escaped_path}'), '{escaped_text}') into ok else err =>\n        print(err)\n        propagate err\n\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn build_append_success_source(path: &str, text: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, append_text_sync, read_text_sync from standard\n\n##\n  Description: Integration probe that appends text and reads it back.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidPathError, WriteFailureError, FilesystemFullError, ReadFailureError, InvalidUtf8Error =>\n    propagate append_text_sync(path_from('{escaped_path}'), '{escaped_text}')\n    let content = propagate read_text_sync(path_from('{escaped_path}'))\n    print('APPEND_OUTPUT_START')\n    print(content)\n    print('APPEND_OUTPUT_END')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program_for_tests(
        Path::new("test-projects/_t22_append_file_string/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t22 append_text probe source should compile into a binary: {error}"
            ));
        }
    };

    run_binary_output_with_timeout(
        &binary_path,
        std::time::Duration::from_secs(10),
        "compiled binary",
    )
    .map_err(|error| format!("t22 append_text probe binary should execute: {error}"))
}

fn extract_between_markers(
    stdout: &str,
    start_marker: &str,
    end_marker: &str,
) -> Result<String, String> {
    let start_idx = stdout.find(start_marker).ok_or_else(|| {
        format!("stdout should contain start marker '{start_marker}', got:\n{stdout}")
    })?;
    let content_start = start_idx.checked_add(start_marker.len()).ok_or_else(|| {
        format!("stdout marker offset overflowed for '{start_marker}', got:\n{stdout}")
    })?;

    let tail = stdout
        .get(content_start..)
        .ok_or_else(|| format!("stdout marker offset should be a valid UTF-8 boundary for '{start_marker}', got:\n{stdout}"))?;
    let end_rel = tail.find(end_marker).ok_or_else(|| {
        format!("stdout should contain end marker '{end_marker}', got:\n{stdout}")
    })?;
    let mut extracted = tail.get(..end_rel).ok_or_else(|| {
        format!(
            "stdout end marker should align to a UTF-8 boundary for '{end_marker}', got:\n{stdout}"
        )
    })?;

    if let Some(rest) = extracted.strip_prefix("\r\n") {
        extracted = rest;
    } else if let Some(rest) = extracted.strip_prefix('\n') {
        extracted = rest;
    }

    if let Some(rest) = extracted.strip_suffix("\r\n") {
        extracted = rest;
    } else if let Some(rest) = extracted.strip_suffix('\n') {
        extracted = rest;
    }

    Ok(extracted.to_owned())
}

#[test]
#[serial(fs)]
fn append_not_found() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for append_not_found");

    let compile_dir = unique_probe_target_dir("append-not-found");
    let prepare = prepare_dir(&compile_dir);
    assert!(
        prepare.is_ok(),
        "t22 not-found temp directory should be created"
    );

    let missing_dir = make_temp_path("missing-parent");
    let missing_file = missing_dir.join("nested").join("file.txt");
    let source = build_append_error_source(&missing_file.to_string_lossy(), "B");

    let execution_result: Result<(), String> = (|| {
        let run_output = compile_and_run_inline_program(&source, &compile_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "missing-parent append probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "missing-parent append output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&compile_dir);
    assert!(
        cleanup.is_ok(),
        "t22 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "append_not_found should fail with FileNotFoundError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn append_isdir() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for append_isdir");

    let compile_dir = unique_probe_target_dir("append-isdir");
    let prepare = prepare_dir(&compile_dir);
    assert!(
        prepare.is_ok(),
        "t22 is-directory temp directory should be created"
    );

    let directory_path = make_temp_path("dir-target");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&directory_path)
            .map_err(|e| format!("is-directory fixture folder should be created: {e}"))?;

        let source = build_append_error_source(&directory_path.to_string_lossy(), "B");
        let run_output = compile_and_run_inline_program(&source, &compile_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "is-directory append probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "is-directory append output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&directory_path));

    let cleanup = cleanup_dir(&compile_dir);
    assert!(
        cleanup.is_ok(),
        "t22 is-directory temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "append_isdir should fail with IsADirectoryError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn append_creates_new() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for append_creates_new");

    let compile_dir = unique_probe_target_dir("append-creates-new");
    let prepare = prepare_dir(&compile_dir);
    assert!(
        prepare.is_ok(),
        "t22 create-new temp directory should be created"
    );

    let fixture_dir = make_temp_path("create-new");
    let fixture_file = fixture_dir.join("new.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("create-new fixture directory should be created: {e}"))?;

        let source = build_append_success_source(&fixture_file.to_string_lossy(), "B");
        let run_output = compile_and_run_inline_program(&source, &compile_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "append create-new probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let extracted =
            extract_between_markers(&stdout, "APPEND_OUTPUT_START", "APPEND_OUTPUT_END")?;
        if extracted != "B" {
            return Err(format!(
                "append create-new output should be exactly 'B', got '{extracted}'"
            ));
        }

        let on_disk = fs::read_to_string(&fixture_file)
            .map_err(|e| format!("append create-new fixture file should be readable: {e}"))?;
        if on_disk != "B" {
            return Err(format!(
                "append create-new on-disk file should be exactly 'B', got '{on_disk}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&compile_dir);
    assert!(
        cleanup.is_ok(),
        "t22 create-new temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "append_creates_new should create file and append exact text: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn append_existing() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for append_existing");

    let compile_dir = unique_probe_target_dir("append-existing");
    let prepare = prepare_dir(&compile_dir);
    assert!(
        prepare.is_ok(),
        "t22 append-existing temp directory should be created"
    );

    let fixture_dir = make_temp_path("append-existing");
    let fixture_file = fixture_dir.join("existing.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("append-existing fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "A")
            .map_err(|e| format!("append-existing fixture file should be seeded: {e}"))?;

        let source = build_append_success_source(&fixture_file.to_string_lossy(), "B");
        let run_output = compile_and_run_inline_program(&source, &compile_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "append-existing probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let extracted =
            extract_between_markers(&stdout, "APPEND_OUTPUT_START", "APPEND_OUTPUT_END")?;
        if extracted != "AB" {
            return Err(format!(
                "append-existing output should be exactly 'AB', got '{extracted}'"
            ));
        }

        let on_disk = fs::read_to_string(&fixture_file)
            .map_err(|e| format!("append-existing fixture file should be readable: {e}"))?;
        if on_disk != "AB" {
            return Err(format!(
                "append-existing on-disk file should be exactly 'AB', got '{on_disk}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&compile_dir);
    assert!(
        cleanup.is_ok(),
        "t22 append-existing temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "append_existing should append to existing file and produce AB: {failure_message}"
    );
}
