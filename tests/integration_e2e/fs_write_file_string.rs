#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};


fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t20-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_write_text_success_source(path: &str, text: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, write_text_sync, read_text_sync from standard\n\n##\n  Description: Integration probe that writes text and reads it back for hash verification.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, WriteFailureError, InvalidPathError, FilesystemFullError, ReadFailureError, InvalidUtf8Error =>\n    propagate write_text_sync(path_from('{escaped_path}'), '{escaped_text}')\n    let content = propagate read_text_sync(path_from('{escaped_path}'))\n    print('HASH_INPUT_START')\n    print(content)\n    print('HASH_INPUT_END')\n    return void\n"
    )
}

fn build_write_text_error_source(path: &str, text: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_text = text.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, write_text_sync from standard\n\n##\n  Description: Integration probe that captures write_text_sync errors via guard.\n##\nentry main = f(args: string[]): void =>\n    guard write_text_sync(path_from('{escaped_path}'), '{escaped_text}') into ok else err =>\n        print(err)\n        return void\n\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program(
        Path::new("test-projects/_t20_write_file_string/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t20 write_text probe source should compile into a binary: {error}"
            ));
        }
    };

    std::process::Command::new(&binary_path)
        .output()
        .map_err(|error| format!("t20 write_text probe binary should execute: {error}"))
}

#[test]
#[serial(fs)]
fn write_file_string_not_found() {
    let temp_dir = unique_probe_target_dir("write-string-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t20 not-found temp directory should be created"
    );

    let missing_dir = make_temp_path("missing-parent");
    let missing_file = missing_dir.join("nested").join("file.txt");

    let source = build_write_text_error_source(&missing_file.to_string_lossy(), "x");

    let execution_result: Result<(), String> = (|| {
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "missing-parent probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "missing-parent output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t20 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_string_not_found should fail with FileNotFoundError prefix: {failure_message}"
    );
}


#[test]
#[serial(fs)]
fn write_file_string_isdir() {
    let temp_dir = unique_probe_target_dir("write-string-isdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t20 is-directory temp directory should be created"
    );

    let directory_path = make_temp_path("dir-target");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&directory_path)
            .map_err(|e| format!("is-directory fixture folder should be created: {e}"))?;

        let source = build_write_text_error_source(&directory_path.to_string_lossy(), "x");
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "is-directory probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "is-directory output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&directory_path));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t20 is-directory temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_string_isdir should fail with IsADirectoryError prefix: {failure_message}"
    );
}

#[cfg(target_os = "linux")]
#[test]
#[serial(fs)]
fn write_file_string_disk_full() {
    let temp_dir = unique_probe_target_dir("write-string-disk-full");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t20 disk-full temp directory should be created"
    );

    let source = build_write_text_error_source("/dev/full", "x");

    let execution_result: Result<(), String> = (|| {
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "disk-full probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !(combined.contains("Io:")
            && (combined.contains("No space left")
                || combined.contains("space left")
                || combined.contains("ENOSPC")))
        {
            return Err(format!(
                "disk-full output should contain IO prefix and ENOSPC text, status={:?}, stdout='{}', stderr='{}'",
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
        "t20 disk-full temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_string_disk_full should fail with IO ENOSPC text: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn write_file_string_success() {
    let temp_dir = unique_probe_target_dir("write-string-success");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t20 success temp directory should be created"
    );

    let fixture_dir = make_temp_path("success");
    let fixture_file = fixture_dir.join("out.txt");
    let expected_text = "hello\n";

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("success fixture directory should be created: {e}"))?;

        let source =
            build_write_text_success_source(&fixture_file.to_string_lossy(), expected_text);
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "write_text success probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let start_marker = "HASH_INPUT_START";
        let end_marker = "HASH_INPUT_END";

        let start_idx = stdout
            .find(start_marker)
            .ok_or_else(|| format!("success stdout should contain start marker, got:\n{stdout}"))?;
        let content_start = start_idx
            .checked_add(start_marker.len())
            .ok_or_else(|| format!("success stdout marker offset overflowed, got:\n{stdout}"))?;

        let tail = stdout.get(content_start..).ok_or_else(|| {
            format!("success stdout marker offset should be a valid UTF-8 boundary, got:\n{stdout}")
        })?;
        let end_rel = tail
            .find(end_marker)
            .ok_or_else(|| format!("success stdout should contain end marker, got:\n{stdout}"))?;
        let mut extracted = tail.get(..end_rel).ok_or_else(|| {
            format!("success stdout end marker should align to a UTF-8 boundary, got:\n{stdout}")
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

        let mut expected_hasher = Sha256::new();
        expected_hasher.update(expected_text.as_bytes());
        let expected_hash = expected_hasher.finalize();

        let mut actual_hasher = Sha256::new();
        actual_hasher.update(extracted.as_bytes());
        let actual_hash = actual_hasher.finalize();

        if actual_hash != expected_hash {
            return Err(format!(
                "success stdout content sha256 should match fixture sha256; extracted='{extracted}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t20 success temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_string_success should write and read back exact content hash: {failure_message}"
    );
}
