#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn build_read_bytes_success_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, read_contents_sync, bytes_length, bytes_to_hex, int32_to_string from standard\n\n##\n  Description: Integration probe that reads bytes and prints marker-delimited metadata.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, ReadFailureError, InvalidPathError =>\n    let bytes = propagate read_contents_sync(path_from('{escaped_path}'))\n    print('COUNT_START')\n    print(int32_to_string(bytes_length(bytes)))\n    print('COUNT_END')\n    print('HEX_START')\n    print(bytes_to_hex(bytes))\n    print('HEX_END')\n    return void\n"
    )
}

fn build_read_bytes_error_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, read_contents_sync, bytes_length, int32_to_string from standard\n\n##\n  Description: Integration probe that captures read_contents_sync errors via guard.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidPathError, ReadFailureError, InvalidUtf8Error, OffsetOutOfRangeError, WriteFailureError, FilesystemFullError, CopyFailureError, DeleteFailureError, DirectoryNotFoundError, IsNotADirectoryError =>\n    guard read_contents_sync(path_from('{escaped_path}')) into bytes else err =>\n        print(err)\n        propagate err\n\n    print('UNEXPECTED_SUCCESS')\n    print(int32_to_string(bytes_length(bytes)))\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program_for_tests(
        Path::new("test-projects/_t16_read_bytes/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t16 read_bytes probe source should compile into a binary: {error}"
            ));
        }
    };

    run_binary_output_with_timeout(
        &binary_path,
        std::time::Duration::from_secs(10),
        "compiled binary",
    )
    .map_err(|error| format!("t16 read_bytes probe binary should execute: {error}"))
}

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t16-{label}-{}-{nanos}",
        std::process::id()
    ))
}

#[test]
#[serial(fs)]
fn read_file_to_bytes_256_success() {
    let temp_dir = unique_probe_target_dir("read-bytes-256");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t16 256-byte temp directory should be created"
    );

    let fixture_dir = make_temp_path("256");
    let fixture_file = fixture_dir.join("fixture_256.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("256-byte fixture directory should be created: {e}"))?;

        let expected_bytes: Vec<u8> = (0_u8..=255_u8).collect();
        fs::write(&fixture_file, &expected_bytes)
            .map_err(|e| format!("256-byte fixture file should be written: {e}"))?;

        let source = build_read_bytes_success_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "read_contents_sync 256-byte probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);

        let count_start = stdout
            .find("COUNT_START")
            .ok_or_else(|| format!("stdout should include COUNT_START marker, got:\n{stdout}"))?;
        let count_offset = count_start
            .checked_add("COUNT_START".len())
            .ok_or_else(|| format!("COUNT_START marker offset overflowed, got:\n{stdout}"))?;
        let count_tail = stdout.get(count_offset..).ok_or_else(|| {
            format!("COUNT_START marker should align to a UTF-8 boundary, got:\n{stdout}")
        })?;
        let count_end_rel = count_tail
            .find("COUNT_END")
            .ok_or_else(|| format!("stdout should include COUNT_END marker, got:\n{stdout}"))?;
        let count_payload = count_tail
            .get(..count_end_rel)
            .ok_or_else(|| {
                format!("COUNT_END marker should align to a UTF-8 boundary, got:\n{stdout}")
            })?
            .trim_matches(|c| c == '\n' || c == '\r' || c == ' ' || c == '\t');

        if count_payload != "256" {
            return Err(format!(
                "byte count should be exactly 256, got '{count_payload}'"
            ));
        }

        let hex_start = stdout
            .find("HEX_START")
            .ok_or_else(|| format!("stdout should include HEX_START marker, got:\n{stdout}"))?;
        let hex_offset = hex_start
            .checked_add("HEX_START".len())
            .ok_or_else(|| format!("HEX_START marker offset overflowed, got:\n{stdout}"))?;
        let hex_tail = stdout.get(hex_offset..).ok_or_else(|| {
            format!("HEX_START marker should align to a UTF-8 boundary, got:\n{stdout}")
        })?;
        let hex_end_rel = hex_tail
            .find("HEX_END")
            .ok_or_else(|| format!("stdout should include HEX_END marker, got:\n{stdout}"))?;
        let hex_payload = hex_tail
            .get(..hex_end_rel)
            .ok_or_else(|| {
                format!("HEX_END marker should align to a UTF-8 boundary, got:\n{stdout}")
            })?
            .trim_matches(|c| c == '\n' || c == '\r' || c == ' ' || c == '\t');

        if hex_payload.len() != 512 {
            return Err(format!(
                "hex payload should be 512 chars for 256 bytes, got {}",
                hex_payload.len()
            ));
        }

        let actual_bytes = decode_hex(hex_payload)
            .map_err(|e| format!("hex payload should decode back to bytes: {e}"))?;

        let mut expected_hasher = Sha256::new();
        expected_hasher.update((0_u8..=255_u8).collect::<Vec<u8>>());
        let expected_hash = expected_hasher.finalize();

        let mut actual_hasher = Sha256::new();
        actual_hasher.update(&actual_bytes);
        let actual_hash = actual_hasher.finalize();

        if expected_hash != actual_hash {
            return Err(String::from(
                "read_contents_sync bytes sha256 should match expected 0x00..0xFF sequence",
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t16 256-byte temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_bytes_256_success should return 256 bytes and matching hash: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_bytes_empty() {
    let temp_dir = unique_probe_target_dir("read-bytes-empty");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t16 empty-file temp directory should be created"
    );

    let fixture_dir = make_temp_path("empty");
    let fixture_file = fixture_dir.join("empty.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("empty fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, [])
            .map_err(|e| format!("empty fixture file should be written: {e}"))?;

        let source = build_read_bytes_success_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "read_contents_sync empty-file probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);

        let count_start = stdout
            .find("COUNT_START")
            .ok_or_else(|| format!("stdout should include COUNT_START marker, got:\n{stdout}"))?;
        let count_offset = count_start
            .checked_add("COUNT_START".len())
            .ok_or_else(|| format!("COUNT_START marker offset overflowed, got:\n{stdout}"))?;
        let count_tail = stdout.get(count_offset..).ok_or_else(|| {
            format!("COUNT_START marker should align to a UTF-8 boundary, got:\n{stdout}")
        })?;
        let count_end_rel = count_tail
            .find("COUNT_END")
            .ok_or_else(|| format!("stdout should include COUNT_END marker, got:\n{stdout}"))?;
        let count_payload = count_tail
            .get(..count_end_rel)
            .ok_or_else(|| {
                format!("COUNT_END marker should align to a UTF-8 boundary, got:\n{stdout}")
            })?
            .trim_matches(|c| c == '\n' || c == '\r' || c == ' ' || c == '\t');

        if count_payload != "0" {
            return Err(format!(
                "empty file should return count 0, got '{count_payload}'"
            ));
        }

        let hex_start = stdout
            .find("HEX_START")
            .ok_or_else(|| format!("stdout should include HEX_START marker, got:\n{stdout}"))?;
        let hex_offset = hex_start
            .checked_add("HEX_START".len())
            .ok_or_else(|| format!("HEX_START marker offset overflowed, got:\n{stdout}"))?;
        let hex_tail = stdout.get(hex_offset..).ok_or_else(|| {
            format!("HEX_START marker should align to a UTF-8 boundary, got:\n{stdout}")
        })?;
        let hex_end_rel = hex_tail
            .find("HEX_END")
            .ok_or_else(|| format!("stdout should include HEX_END marker, got:\n{stdout}"))?;
        let hex_payload = hex_tail
            .get(..hex_end_rel)
            .ok_or_else(|| {
                format!("HEX_END marker should align to a UTF-8 boundary, got:\n{stdout}")
            })?
            .trim_matches(|c| c == '\n' || c == '\r' || c == ' ' || c == '\t');

        if !hex_payload.is_empty() {
            return Err(format!(
                "empty file should produce empty hex payload, got '{hex_payload}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t16 empty-file temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_bytes_empty should return count=0 with no error"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_bytes_not_found() {
    let temp_dir = unique_probe_target_dir("read-bytes-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t16 not-found temp directory should be created"
    );

    let source =
        build_read_bytes_error_source("/tmp/opalescent-t16-definitely-missing-read-bytes.bin");

    let execution_result: Result<(), String> = (|| {
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "missing-path bytes probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "missing-path bytes output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t16 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_bytes_not_found should fail with FileNotFoundError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_bytes_isdir() {
    let temp_dir = unique_probe_target_dir("read-bytes-is-dir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t16 is-directory temp directory should be created"
    );

    let directory_path = make_temp_path("dir");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&directory_path)
            .map_err(|e| format!("is-directory bytes probe folder should be created: {e}"))?;

        let source = build_read_bytes_error_source(&directory_path.to_string_lossy());
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
                "directory-path bytes output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t16 is-directory temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_bytes_isdir should fail with IsADirectoryError prefix: {failure_message}"
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
        let byte = u8::try_from(combined).map_err(|error| {
            format!("hex byte value should fit into u8, got {combined}: {error}")
        })?;
        out.push(byte);
    }

    Ok(out)
}
