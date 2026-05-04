#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, fs_project_root, unique_probe_target_dir,
};
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
        "opalescent-t21-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn bytes_hex_literal(bytes: &[u8]) -> String {
    let capacity = bytes.len().checked_mul(2).unwrap_or_default();
    let mut out = String::with_capacity(capacity);
    for b in bytes {
        use std::fmt::Write;
        assert!(
            write!(&mut out, "{b:02x}").is_ok(),
            "writing hex into String should not fail"
        );
    }
    out
}

fn build_write_bytes_success_source(path: &str, payload_hex: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, write_contents_sync, read_contents_sync, bytes_from_hex, bytes_to_hex from standard\n\n##\n  Description: Integration probe that writes bytes and reads them back for hash verification.\n##\nentry main = f(args: string[]): void errors HexDecodeError, FileNotFoundError, PermissionDeniedError, IsADirectoryError, WriteFailureError, ReadFailureError, InvalidPathError, FilesystemFullError =>\n    let payload = propagate bytes_from_hex('{payload_hex}')\n    propagate write_contents_sync(path_from('{escaped_path}'), payload)\n    let content = propagate read_contents_sync(path_from('{escaped_path}'))\n    print('HEX_START')\n    print(bytes_to_hex(content))\n    print('HEX_END')\n    return void\n"
    )
}

fn build_write_bytes_error_source(path: &str, payload_hex: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, write_contents_sync, bytes_from_hex from standard\n\n##\n  Description: Integration probe that captures write_contents_sync errors via guard.\n##\nentry main = f(args: string[]): void errors HexDecodeError =>\n    let payload = propagate bytes_from_hex('{payload_hex}')\n    guard write_contents_sync(path_from('{escaped_path}'), payload) into ok else err =>\n        print(err)\n        return void\n\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn build_write_bytes_empty_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, write_contents_sync, bytes_from_hex from standard\n\n##\n  Description: Integration probe that writes empty bytes payload.\n##\nentry main = f(args: string[]): void errors HexDecodeError, PermissionDeniedError, IsADirectoryError, WriteFailureError, InvalidPathError, FilesystemFullError =>\n    let payload = propagate bytes_from_hex('')\n    propagate write_contents_sync(path_from('{escaped_path}'), payload)\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program(
        Path::new("test-projects/_t21_write_file_bytes/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t21 write_bytes probe source should compile into a binary: {error}"
            ));
        }
    };

    std::process::Command::new(&binary_path)
        .output()
        .map_err(|error| format!("t21 write_bytes probe binary should execute: {error}"))
}

#[test]
#[serial(fs)]
fn write_file_bytes_not_found() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");

    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("write-bytes-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t21 not-found temp directory should be created"
    );

    let missing_dir = make_temp_path("missing-parent");
    let missing_file = missing_dir.join("nested").join("file.bin");

    let payload_hex = bytes_hex_literal(&[0x11, 0x22, 0x33]);
    let source = build_write_bytes_error_source(&missing_file.to_string_lossy(), &payload_hex);

    let execution_result: Result<(), String> = (|| {
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "missing-parent bytes probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "missing-parent bytes output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t21 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_bytes_not_found should fail with FileNotFoundError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn write_file_bytes_isdir() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");

    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("write-bytes-isdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t21 is-directory temp directory should be created"
    );

    let directory_path = make_temp_path("dir-target");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&directory_path)
            .map_err(|e| format!("is-directory fixture folder should be created: {e}"))?;

        let payload_hex = bytes_hex_literal(&[0x0A, 0x0B, 0x0C]);
        let source =
            build_write_bytes_error_source(&directory_path.to_string_lossy(), &payload_hex);
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "is-directory bytes probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "is-directory bytes output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t21 is-directory temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_bytes_isdir should fail with IsADirectoryError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn write_file_bytes_256_roundtrip() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");

    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("write-bytes-256-roundtrip");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t21 256-byte temp directory should be created"
    );

    let fixture_dir = make_temp_path("roundtrip");
    let fixture_file = fixture_dir.join("out.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("roundtrip fixture directory should be created: {e}"))?;

        let expected_bytes: Vec<u8> = (0_u8..=255_u8).collect();
        let payload_hex = bytes_hex_literal(&expected_bytes);
        let source =
            build_write_bytes_success_source(&fixture_file.to_string_lossy(), &payload_hex);
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "write_bytes 256-byte probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let start_marker = "HEX_START";
        let end_marker = "HEX_END";

        let start_idx = stdout.find(start_marker).ok_or_else(|| {
            format!("roundtrip stdout should contain start marker, got:\n{stdout}")
        })?;
        let content_start = start_idx + start_marker.len();

        let tail = stdout.get(content_start..).ok_or_else(|| {
            format!(
                "roundtrip stdout marker offset should be a valid UTF-8 boundary, got:\n{stdout}"
            )
        })?;
        let end_rel = tail
            .find(end_marker)
            .ok_or_else(|| format!("roundtrip stdout should contain end marker, got:\n{stdout}"))?;
        let mut extracted = tail.get(..end_rel).ok_or_else(|| {
            format!("roundtrip stdout end marker should align to a UTF-8 boundary, got:\n{stdout}")
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

        let actual_bytes = decode_hex(extracted)
            .map_err(|e| format!("roundtrip hex payload should decode: {e}"))?;

        let mut expected_hasher = Sha256::new();
        expected_hasher.update(&expected_bytes);
        let expected_hash = expected_hasher.finalize();

        let mut actual_hasher = Sha256::new();
        actual_hasher.update(&actual_bytes);
        let actual_hash = actual_hasher.finalize();

        if actual_hash != expected_hash {
            return Err(String::from(
                "write_contents_sync 256-byte roundtrip sha256 should match expected bytes",
            ));
        }

        let disk_bytes = fs::read(&fixture_file)
            .map_err(|e| format!("roundtrip output file should be readable: {e}"))?;

        let mut disk_hasher = Sha256::new();
        disk_hasher.update(&disk_bytes);
        let disk_hash = disk_hasher.finalize();

        if disk_hash != expected_hash {
            return Err(String::from(
                "written file sha256 should match expected 256-byte payload",
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t21 256-byte temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_bytes_256_roundtrip should preserve exact payload hash: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn write_file_bytes_empty() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");

    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("write-bytes-empty");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t21 empty temp directory should be created"
    );

    let fixture_dir = make_temp_path("empty");
    let fixture_file = fixture_dir.join("out-empty.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("empty fixture directory should be created: {e}"))?;

        fs::write(&fixture_file, [0xAA_u8, 0xBB, 0xCC, 0xDD])
            .map_err(|e| format!("empty test should pre-seed fixture file: {e}"))?;

        let source = build_write_bytes_empty_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "write_bytes empty probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let metadata = fs::metadata(&fixture_file)
            .map_err(|e| format!("empty write output file should exist: {e}"))?;
        if metadata.len() != 0 {
            return Err(format!(
                "empty write should truncate file to size 0, got size {}",
                metadata.len()
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t21 empty temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "write_file_bytes_empty should create/truncate to zero-length file: {failure_message}"
    );
}

fn decode_hex(hex: &str) -> Result<Vec<u8>, String> {
    if (hex.len() & 1) != 0 {
        return Err(String::from("hex length must be even"));
    }

    let capacity = hex.len().checked_div(2).unwrap_or_default();
    let mut out = Vec::with_capacity(capacity);
    let mut chars = hex.chars();

    while let (Some(high), Some(low)) = (chars.next(), chars.next()) {
        let hi = high
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex character '{high}'"))?;
        let lo = low
            .to_digit(16)
            .ok_or_else(|| format!("invalid hex character '{low}'"))?;
        let combined = (hi << 4_u32) | lo;
        let byte = u8::try_from(combined)
            .map_err(|_error| format!("hex byte value should fit into u8, got {combined}"))?;
        out.push(byte);
    }

    Ok(out)
}
