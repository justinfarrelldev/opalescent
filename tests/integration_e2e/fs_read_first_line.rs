#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t18-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_read_first_line_success_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, read_first_line_sync from standard\n\n##\n  Description: Integration probe that reads first line and prints marker-delimited content.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, IsADirectoryError, InvalidUtf8Error, OffsetOutOfRangeError, ReadFailureError, InvalidPathError =>\n    let line = propagate read_first_line_sync(path_from('{escaped_path}'))\n    print('LINE_START')\n    print(line)\n    print('LINE_END')\n    return void\n"
    )
}

fn build_read_first_line_error_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, read_first_line_sync from standard\n\n##\n  Description: Integration probe that captures read_first_line_sync errors via guard.\n##\nentry main = f(args: string[]): void =>\n    guard read_first_line_sync(path_from('{escaped_path}')) into line else err =>\n        print(err)\n        return void\n\n    print('UNEXPECTED_SUCCESS')\n    print(line)\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program(
        Path::new("test-projects/_t18_read_first_line/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t18 read_first_line probe source should compile into a binary: {error}"
            ));
        }
    };

    std::process::Command::new(&binary_path)
        .output()
        .map_err(|error| format!("t18 read_first_line probe binary should execute: {error}"))
}

fn extract_line_payload(stdout: &str) -> Result<String, String> {
    let start_marker = "LINE_START";
    let end_marker = "LINE_END";

    let start_idx = stdout
        .find(start_marker)
        .ok_or_else(|| format!("stdout should contain start marker, got:\n{stdout}"))?;
    let content_start = start_idx
        .checked_add(start_marker.len())
        .ok_or_else(|| format!("stdout marker offset overflowed, got:\n{stdout}"))?;

    let tail = stdout.get(content_start..).ok_or_else(|| {
        format!("stdout marker offset should be a valid UTF-8 boundary, got:\n{stdout}")
    })?;
    let end_rel = tail
        .find(end_marker)
        .ok_or_else(|| format!("stdout should contain end marker, got:\n{stdout}"))?;
    let mut extracted = tail.get(..end_rel).ok_or_else(|| {
        format!("stdout end marker should align to a UTF-8 boundary, got:\n{stdout}")
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

fn build_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("read_first_line_harness.c");
    let harness_bin = temp_dir.join("read_first_line_harness");

    let source = r#"#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <path>\n", argv[0]);
        return 64;
    }

    FsStringResult result = read_first_line_sync(argv[1]);
    if (result.error != NULL) {
        printf("ERROR:%s\n", result.error);
        free((void*)result.error);
        return 2;
    }

    const char* line = result.value ? result.value : "";
    printf("LINE:%s\n", line);
    free(result.value);
    return 0;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("t18 harness source should be written: {e}"))?;

    let compile = Command::new("cc")
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_fs.c")
        .arg(&harness_c)
        .arg("-o")
        .arg(&harness_bin)
        .output()
        .map_err(|e| format!("t18 harness compile command should execute: {e}"))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "t18 harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

#[test]
#[serial(fs)]
fn read_first_line_empty() {
    let temp_dir = unique_probe_target_dir("read-first-line-empty");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t18 empty-file temp directory should be created"
    );

    let fixture_dir = make_temp_path("empty");
    let fixture_file = fixture_dir.join("empty.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("empty fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "")
            .map_err(|e| format!("empty fixture file should be written: {e}"))?;

        let source = build_read_first_line_error_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "empty file probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("OffsetOutOfRangeError: file is empty") {
            return Err(format!(
                "empty file output should contain 'OffsetOutOfRangeError: file is empty', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t18 empty-file temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_empty should return locked empty-file error policy: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_single_line_no_lf() {
    let temp_dir = unique_probe_target_dir("read-first-line-single");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t18 single-line temp directory should be created"
    );

    let fixture_dir = make_temp_path("single");
    let fixture_file = fixture_dir.join("single.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("single-line fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "hello world")
            .map_err(|e| format!("single-line fixture file should be written: {e}"))?;

        let source = build_read_first_line_success_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "single-line probe should exit 0, stderr:\n{stderr}"
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let line = extract_line_payload(&stdout)?;
        if line != "hello world" {
            return Err(format!(
                "single-line file should return full first line, got '{line}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t18 single-line temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_single_line_no_lf should return content as-is: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_multiline_lf() {
    let temp_dir = unique_probe_target_dir("read-first-line-lf");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "t18 lf temp directory should be created");

    let fixture_dir = make_temp_path("lf");
    let fixture_file = fixture_dir.join("lf.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("lf fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "alpha\nbeta\ngamma")
            .map_err(|e| format!("lf fixture file should be written: {e}"))?;

        let source = build_read_first_line_success_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!("LF probe should exit 0, stderr:\n{stderr}"));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let line = extract_line_payload(&stdout)?;
        if line != "alpha" {
            return Err(format!(
                "LF multiline file should return first line only, got '{line}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "t18 lf temp directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_multiline_lf should return only first LF-delimited line: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_crlf() {
    let temp_dir = unique_probe_target_dir("read-first-line-crlf");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "t18 crlf temp directory should be created");

    let fixture_dir = make_temp_path("crlf");
    let fixture_file = fixture_dir.join("crlf.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("crlf fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, b"first\r\nsecond\r\n")
            .map_err(|e| format!("crlf fixture file should be written: {e}"))?;

        let source = build_read_first_line_success_source(&fixture_file.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!("CRLF probe should exit 0, stderr:\n{stderr}"));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let line = extract_line_payload(&stdout)?;
        if line != "first" {
            return Err(format!(
                "CRLF multiline file should drop trailing CR and return 'first', got '{line}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "t18 crlf temp directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_crlf should normalize CRLF to LF semantics for first line: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_is_directory() {
    let temp_dir = unique_probe_target_dir("read-first-line-is-dir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t18 is-directory temp directory should be created"
    );

    let directory_path = make_temp_path("dir");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&directory_path)
            .map_err(|e| format!("directory fixture should be created: {e}"))?;

        let source = build_read_first_line_error_source(&directory_path.to_string_lossy());
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
        "t18 is-directory temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_is_directory should map directory paths to IsADirectoryError: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_not_found() {
    let temp_dir = unique_probe_target_dir("read-first-line-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t18 not-found temp directory should be created"
    );

    let missing_path = make_temp_path("missing").join("missing.txt");

    let execution_result: Result<(), String> = (|| {
        let source = build_read_first_line_error_source(&missing_path.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");
        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "not-found probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "not-found output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
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
        "t18 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_not_found should map ENOENT to FileNotFoundError: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_first_line_streaming_bounded() {
    let fixture_dir = make_temp_path("streaming");
    let fixture_file = fixture_dir.join("large.txt");
    let harness_dir = make_temp_path("streaming-harness");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("streaming fixture directory should be created: {e}"))?;
        fs::create_dir_all(&harness_dir)
            .map_err(|e| format!("streaming harness directory should be created: {e}"))?;

        let harness_bin = build_harness(&harness_dir)?;

        let mut payload = vec![b'x'; 10 * 1024 * 1024];
        payload[100] = b'\n';
        payload[101] = b'y';
        fs::write(&fixture_file, payload)
            .map_err(|e| format!("streaming fixture file should be written: {e}"))?;

        let start = Instant::now();
        let output = Command::new(&harness_bin)
            .arg(&fixture_file)
            .output()
            .map_err(|e| format!("streaming harness binary should execute: {e}"))?;
        let elapsed = start.elapsed();

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            let line = stdout
                .lines()
                .find_map(|line| line.strip_prefix("LINE:"))
                .ok_or_else(|| format!("streaming stdout missing LINE record: {stdout}"))?
                .to_owned();

            if line.len() != 100 {
                return Err(format!(
                    "streaming fixture first line length should be 100 bytes, got {}",
                    line.len()
                ));
            }
            if elapsed.as_millis() >= 50 {
                return Err(format!(
                    "streaming first-line read should complete under 50ms, got {}ms",
                    elapsed.as_millis()
                ));
            }
        } else {
            let error_payload = stdout
                .lines()
                .find_map(|line| line.strip_prefix("ERROR:"))
                .unwrap_or("<missing error payload>");
            return Err(format!(
                "streaming fixture should succeed, got error '{error_payload}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));
    drop(fs::remove_file(harness_dir.join("read_first_line_harness")));
    drop(fs::remove_file(
        harness_dir.join("read_first_line_harness.c"),
    ));
    drop(fs::remove_dir_all(&harness_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_first_line_streaming_bounded should stop at first newline in bounded time: {failure_message}"
    );
}
