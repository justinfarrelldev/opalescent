#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t25-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_metadata_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("metadata_harness.c");
    let harness_bin = temp_dir.join("metadata_harness");

    let source = r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

typedef struct {
    int64_t size_bytes;
    int8_t is_directory;
    int8_t is_symlink;
    int64_t modified_unix_seconds;
} OpalFileMetadata;

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <path>\n", argv[0]);
        return 64;
    }

    FsMetadataResult result = read_metadata_sync(argv[1]);
    if (result.error != NULL) {
        printf("ERROR:%s\n", result.error);
        free((void*)result.error);
        return 2;
    }

    OpalFileMetadata* metadata = (OpalFileMetadata*)result.value;
    if (!metadata) {
        printf("ERROR:MetadataUnavailableError: null metadata pointer\n");
        return 3;
    }

    printf("size=%lld\n", (long long)metadata->size_bytes);
    printf("mtime=%lld\n", (long long)metadata->modified_unix_seconds);
    printf("is_directory=%d\n", (int)metadata->is_directory);
    printf("is_file=%d\n", metadata->is_directory ? 0 : 1);

    free(metadata);
    return 0;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("t25 metadata harness source should be written: {e}"))?;

    let compile = Command::new("cc")
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_fs.c")
        .arg(&harness_c)
        .arg("-o")
        .arg(&harness_bin)
        .output()
        .map_err(|e| format!("t25 metadata harness compile command should execute: {e}"))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "t25 metadata harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

fn run_metadata_harness(path: &Path, temp_dir: &Path) -> Result<String, String> {
    let harness_bin = build_metadata_harness(temp_dir)?;

    let output = Command::new(&harness_bin)
        .arg(path)
        .output()
        .map_err(|e| format!("t25 metadata harness should execute: {e}"))?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(temp_dir.join("metadata_harness.c")));

    if !output.status.success() {
        return Err(format!(
            "t25 metadata harness should exit 0, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn build_remove_file_error_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, delete_file_sync from standard\n\n##\n  Description: T25 delete_file error probe.\n##\nentry main = f(args: string[]): void =>\n    guard delete_file_sync(path_from('{escaped_path}')) into ok else err =>\n        print(err)\n        return void\n\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn build_remove_file_success_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, delete_file_sync from standard\n\n##\n  Description: T25 delete_file success probe.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, DeleteFailureError, IsADirectoryError, InvalidPathError =>\n    propagate delete_file_sync(path_from('{escaped_path}'))\n    print('removed=true')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program(
        Path::new("test-projects/_t25_fs_metadata/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t25 metadata probe source should compile into a binary: {error}"
            ));
        }
    };

    std::process::Command::new(&binary_path)
        .output()
        .map_err(|error| format!("t25 metadata probe binary should execute: {error}"))
}

#[test]
#[serial(fs)]
fn metadata_size_mtime() {
    let temp_dir = unique_probe_target_dir("metadata-size-mtime");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t25 metadata temp directory should be created"
    );

    let base = make_temp_path("metadata");
    let file_path = base.join("payload.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("t25 metadata base directory should be created: {e}"))?;

        let payload = b"hello";
        fs::write(&file_path, payload)
            .map_err(|e| format!("t25 metadata fixture should be written: {e}"))?;

        let stdout = run_metadata_harness(&file_path, &temp_dir)?;

        let mut size: Option<i64> = None;
        let mut mtime: Option<i64> = None;
        let mut is_directory: Option<bool> = None;

        for line in stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("size=") {
                size = rest.parse::<i64>().ok();
            } else if let Some(rest) = line.strip_prefix("mtime=") {
                mtime = rest.parse::<i64>().ok();
            } else if let Some(rest) = line.strip_prefix("is_directory=") {
                is_directory = match rest.trim() {
                    "true" | "1" => Some(true),
                    "false" | "0" => Some(false),
                    _ => None,
                };
            }
        }

        let size = size.ok_or_else(|| format!("metadata output missing size line: {stdout}"))?;
        let mtime = mtime.ok_or_else(|| format!("metadata output missing mtime line: {stdout}"))?;
        let is_directory = is_directory
            .ok_or_else(|| format!("metadata output missing is_directory line: {stdout}"))?;

        let mut is_file: Option<bool> = None;
        for line in stdout.lines() {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("is_file=") {
                is_file = match rest.trim() {
                    "1" | "true" => Some(true),
                    "0" | "false" => Some(false),
                    _ => None,
                };
            }
        }
        let is_file =
            is_file.ok_or_else(|| format!("metadata output missing is_file line: {stdout}"))?;

        if size != 5 {
            return Err(format!("metadata size should be 5 bytes, got {size}"));
        }
        if mtime <= 0 {
            return Err(format!(
                "metadata mtime should be positive unix seconds, got {mtime}"
            ));
        }
        if is_directory {
            return Err(String::from(
                "metadata is_directory should be false for regular file",
            ));
        }
        if !is_file {
            return Err(String::from(
                "metadata is_file should be true for regular file",
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&file_path));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t25 metadata temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "metadata_size_mtime should return size + mtime + directory flag for regular file: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn remove_file_not_found() {
    let temp_dir = unique_probe_target_dir("metadata-remove-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t25 remove-file-not-found temp directory should be created"
    );

    let missing_path = make_temp_path("remove-missing").join("missing.txt");

    let execution_result: Result<(), String> = (|| {
        let source = build_remove_file_error_source(&missing_path.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "remove-file-not-found probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "remove-file-not-found output should contain FileNotFoundError prefix, status={:?}, stdout='{}', stderr='{}'",
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
        "t25 remove-file-not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "remove_file_not_found should report FileNotFoundError: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn remove_file_isdir() {
    let temp_dir = unique_probe_target_dir("metadata-remove-isdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t25 remove-file-isdir temp directory should be created"
    );

    let dir_path = make_temp_path("remove-isdir");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&dir_path)
            .map_err(|e| format!("remove-file-isdir fixture directory should be created: {e}"))?;

        let source = build_remove_file_error_source(&dir_path.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "remove-file-isdir probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "remove-file-isdir output should contain IsADirectoryError prefix, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&dir_path));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t25 remove-file-isdir temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "remove_file_isdir should report IsADirectoryError: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn remove_file_success() {
    let temp_dir = unique_probe_target_dir("metadata-remove-success");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t25 remove-file-success temp directory should be created"
    );

    let base = make_temp_path("remove-success");
    let file_path = base.join("victim.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("remove-file-success base directory should be created: {e}"))?;
        fs::write(&file_path, "x")
            .map_err(|e| format!("remove-file-success fixture file should be created: {e}"))?;

        let source = build_remove_file_success_source(&file_path.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            return Err(format!(
                "remove-file-success probe should exit 0, status={:?}, stderr='{}'",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("removed=true") {
            return Err(format!(
                "remove-file-success output should contain removed=true marker, got: {stdout}"
            ));
        }

        if file_path.exists() {
            return Err(String::from(
                "remove-file-success fixture should be deleted by delete_file_sync",
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&file_path));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t25 remove-file-success temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "remove_file_success should delete regular file and return success sentinel: {failure_message}"
    );
}
