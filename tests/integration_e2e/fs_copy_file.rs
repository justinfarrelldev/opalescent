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
        "opalescent-t26-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_copy_error_source(source: &str, destination: &str) -> String {
    let escaped_source = source.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_destination = destination.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, copy_file_sync from standard\n\n##\n  Description: T26 copy_file_sync error probe via guard.\n##\nentry main = f(args: string[]): void =>\n    guard copy_file_sync(path_from('{escaped_source}'), path_from('{escaped_destination}')) into ok else err =>\n        print(err)\n        return void\n\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn build_copy_success_source(source: &str, destination: &str) -> String {
    let escaped_source = source.replace('\\', "\\\\").replace('\'', "\\'");
    let escaped_destination = destination.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, copy_file_sync from standard\n\n##\n  Description: T26 copy_file_sync success probe via propagate.\n##\nentry main = f(args: string[]): void errors FileNotFoundError, PermissionDeniedError, CopyFailureError, IsADirectoryError, InvalidPathError, FilesystemFullError =>\n    propagate copy_file_sync(path_from('{escaped_source}'), path_from('{escaped_destination}'))\n    print('copy=ok')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program(
        Path::new("test-projects/_t26_copy_file/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t26 copy_file probe source should compile into a binary: {error}"
            ));
        }
    };

    std::process::Command::new(&binary_path)
        .output()
        .map_err(|error| format!("t26 copy_file probe binary should execute: {error}"))
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let capacity = digest.len().checked_mul(2).unwrap_or_default();
    let mut out = String::with_capacity(capacity);
    for b in digest {
        use std::fmt::Write;
        assert!(
            write!(&mut out, "{b:02x}").is_ok(),
            "writing sha256 hex into String should not fail"
        );
    }
    out
}

#[test]
#[serial(fs)]
fn copy_file_src_missing() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("copy-src-missing");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 src-missing temp directory should be created"
    );

    let base = make_temp_path("src-missing");
    let source_path = base.join("missing.bin");
    let destination_path = base.join("dest.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("src-missing fixture directory should be created: {e}"))?;

        let source = build_copy_error_source(
            &source_path.to_string_lossy(),
            &destination_path.to_string_lossy(),
        );
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "src-missing copy probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "src-missing copy output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&destination_path));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 src-missing temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "copy_file_src_missing should fail with FileNotFoundError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn copy_file_dest_parent_missing() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("copy-dest-parent-missing");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 dest-parent-missing temp directory should be created"
    );

    let base = make_temp_path("dest-parent-missing");
    let source_path = base.join("source.bin");
    let destination_path = base.join("missing").join("dest.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("dest-parent-missing fixture directory should be created: {e}"))?;
        fs::write(&source_path, b"abc")
            .map_err(|e| format!("dest-parent-missing source file should be created: {e}"))?;

        let source = build_copy_error_source(
            &source_path.to_string_lossy(),
            &destination_path.to_string_lossy(),
        );
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "dest-parent-missing copy probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("FileNotFoundError:") {
            return Err(format!(
                "dest-parent-missing copy output should contain 'FileNotFoundError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&source_path));
    drop(fs::remove_file(&destination_path));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 dest-parent-missing temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "copy_file_dest_parent_missing should fail with FileNotFoundError prefix: {failure_message}"
    );
}


#[test]
#[serial(fs)]
fn copy_file_src_isdir() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("copy-src-isdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 src-isdir temp directory should be created"
    );

    let base = make_temp_path("src-isdir");
    let source_dir = base.join("source-dir");
    let destination_file = base.join("dest.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&source_dir)
            .map_err(|e| format!("src-isdir source directory should be created: {e}"))?;

        let source = build_copy_error_source(
            &source_dir.to_string_lossy(),
            &destination_file.to_string_lossy(),
        );
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "src-isdir copy probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "src-isdir copy output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&destination_file));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 src-isdir temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "copy_file_src_isdir should fail with IsADirectoryError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn copy_file_dest_isdir() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("copy-dest-isdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 dest-isdir temp directory should be created"
    );

    let base = make_temp_path("dest-isdir");
    let source_file = base.join("source.bin");
    let destination_dir = base.join("dest-dir");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&destination_dir)
            .map_err(|e| format!("dest-isdir destination directory should be created: {e}"))?;
        fs::write(&source_file, b"dest-isdir")
            .map_err(|e| format!("dest-isdir source file should be created: {e}"))?;

        let source = build_copy_error_source(
            &source_file.to_string_lossy(),
            &destination_dir.to_string_lossy(),
        );
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        let stderr = String::from_utf8_lossy(&run_output.stderr);
        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let combined = format!("{stdout}\n{stderr}");

        if combined.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "dest-isdir copy probe unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }
        if !combined.contains("IsADirectoryError:") {
            return Err(format!(
                "dest-isdir copy output should contain 'IsADirectoryError:', status={:?}, stdout='{}', stderr='{}'",
                run_output.status.code(),
                stdout,
                stderr
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&source_file));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 dest-isdir temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "copy_file_dest_isdir should fail with IsADirectoryError prefix: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn copy_file_10mb() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("copy-10mb");
    let prepare = prepare_dir(&temp_dir);
    assert!(prepare.is_ok(), "t26 10mb temp directory should be created");

    let base = make_temp_path("copy-10mb");
    let source_file = base.join("source-10mb.bin");
    let destination_file = base.join("dest-10mb.bin");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("10mb fixture base directory should be created: {e}"))?;

        let payload_size = 10 * 1024 * 1024_usize;
        let mut payload = vec![0_u8; payload_size];
        let mut byte_value = 0_u8;
        for byte in &mut payload {
            *byte = byte_value;
            byte_value = if byte_value == 250 { 0 } else { byte_value + 1 };
        }

        fs::write(&source_file, &payload)
            .map_err(|e| format!("10mb source fixture should be written: {e}"))?;

        let source = build_copy_success_source(
            &source_file.to_string_lossy(),
            &destination_file.to_string_lossy(),
        );

        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;
        if !run_output.status.success() {
            return Err(format!(
                "10mb copy probe should exit 0, status={:?}, stderr='{}'",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("copy=ok") {
            return Err(format!(
                "10mb copy output should contain success marker copy=ok, got: {stdout}"
            ));
        }

        let source_bytes = fs::read(&source_file)
            .map_err(|e| format!("10mb source fixture should be readable: {e}"))?;
        let dest_bytes = fs::read(&destination_file)
            .map_err(|e| format!("10mb destination fixture should be readable: {e}"))?;

        if source_bytes.len() != payload_size {
            return Err(format!(
                "10mb source size should remain {payload_size}, got {}",
                source_bytes.len()
            ));
        }
        if dest_bytes.len() != payload_size {
            return Err(format!(
                "10mb destination size should be {payload_size}, got {}",
                dest_bytes.len()
            ));
        }

        let source_hash = sha256_hex(&source_bytes);
        let dest_hash = sha256_hex(&dest_bytes);
        if source_hash != dest_hash {
            return Err(format!(
                "10mb copy sha256 mismatch: source={source_hash}, destination={dest_hash}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&destination_file));
    drop(fs::remove_file(&source_file));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(cleanup.is_ok(), "t26 10mb temp directory should be removed");

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "copy_file_10mb should stream-copy 10MB with matching sha256: {failure_message}"
    );
}
