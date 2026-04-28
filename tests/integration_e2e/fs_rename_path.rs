#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::fs_state_guard::FsStateGuard;
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use std::os::unix::fs::MetadataExt;

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-t27-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_rename_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("rename_harness.c");
    let harness_bin = temp_dir.join("rename_harness");

    let source = r#"#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc != 3) {
        fprintf(stderr, "usage: %s <source> <dest>\n", argv[0]);
        return 64;
    }

    FsVoidResult result = move_path_sync(argv[1], argv[2]);
    if (result.error != NULL) {
        printf("ERR:%s\n", result.error);
        free((void*)result.error);
        return 2;
    }

    printf("OK\n");
    return 0;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("t27 harness source should be written: {e}"))?;

    let compile = Command::new("cc")
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_fs.c")
        .arg(&harness_c)
        .arg("-o")
        .arg(&harness_bin)
        .output()
        .map_err(|e| format!("t27 harness compile command should execute: {e}"))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "t27 harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

fn run_rename_harness(
    src: &Path,
    dst: &Path,
    temp_dir: &Path,
) -> Result<(i32, String, String), String> {
    let harness_bin = build_rename_harness(temp_dir)?;

    let output = Command::new(&harness_bin)
        .arg(src)
        .arg(dst)
        .output()
        .map_err(|e| format!("t27 harness should execute: {e}"))?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(temp_dir.join("rename_harness.c")));

    let code = output.status.code().unwrap_or(-1_i32);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    Ok((code, stdout, stderr))
}

#[test]
#[serial(fs)]
fn rename_within_fs() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for rename_within_fs");

    let temp_dir = unique_probe_target_dir("rename-within-fs");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t27 within-fs temp directory should be created"
    );

    let fixture_dir = make_temp_path("within-fs");
    let src = fixture_dir.join("from.txt");
    let dst = fixture_dir.join("to.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("within-fs fixture directory should be created: {e}"))?;
        fs::write(&src, "alpha")
            .map_err(|e| format!("within-fs source fixture should be written: {e}"))?;

        let (code, stdout, stderr) = run_rename_harness(&src, &dst, &temp_dir)?;
        if code != 0_i32 || !stdout.contains("OK") {
            return Err(format!(
                "rename within-fs should succeed with OK, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        if src.exists() {
            return Err(String::from("rename within-fs should remove source path"));
        }

        let on_disk = fs::read_to_string(&dst)
            .map_err(|e| format!("rename within-fs destination should be readable: {e}"))?;
        if on_disk != "alpha" {
            return Err(format!(
                "rename within-fs destination content should be 'alpha', got '{on_disk}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&src));
    drop(fs::remove_file(&dst));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t27 within-fs temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "rename_within_fs should move content and remove source: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn rename_not_found() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for rename_not_found");

    let temp_dir = unique_probe_target_dir("rename-not-found");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t27 not-found temp directory should be created"
    );

    let fixture_dir = make_temp_path("not-found");
    let src = fixture_dir.join("missing.txt");
    let dst = fixture_dir.join("to.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("not-found fixture directory should be created: {e}"))?;

        let (code, stdout, stderr) = run_rename_harness(&src, &dst, &temp_dir)?;
        if code == 0_i32 || stdout.contains("OK") {
            return Err(format!(
                "rename-not-found should fail, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }
        if !stdout.contains("ERR:FileNotFoundError:") {
            return Err(format!(
                "rename-not-found output should contain FileNotFoundError prefix, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&dst));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t27 not-found temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "rename_not_found should fail with FileNotFoundError prefix: {failure_message}"
    );
}


#[test]
#[serial(fs)]
fn rename_overwrite() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for rename_overwrite");

    let temp_dir = unique_probe_target_dir("rename-overwrite");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t27 overwrite temp directory should be created"
    );

    let fixture_dir = make_temp_path("overwrite");
    let src = fixture_dir.join("from.txt");
    let dst = fixture_dir.join("to.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("overwrite fixture directory should be created: {e}"))?;
        fs::write(&src, "new")
            .map_err(|e| format!("overwrite source fixture should be written: {e}"))?;
        fs::write(&dst, "old")
            .map_err(|e| format!("overwrite destination fixture should be seeded: {e}"))?;

        let (code, stdout, stderr) = run_rename_harness(&src, &dst, &temp_dir)?;
        if code != 0_i32 || !stdout.contains("OK") {
            return Err(format!(
                "rename-overwrite should succeed with OK, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        if src.exists() {
            return Err(String::from("rename-overwrite should remove source path"));
        }

        let on_disk = fs::read_to_string(&dst)
            .map_err(|e| format!("rename-overwrite destination should be readable: {e}"))?;
        if on_disk != "new" {
            return Err(format!(
                "rename-overwrite destination content should be 'new', got '{on_disk}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&src));
    drop(fs::remove_file(&dst));
    drop(fs::remove_dir_all(&fixture_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t27 overwrite temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "rename_overwrite should overwrite destination and remove source: {failure_message}"
    );
}

#[cfg(target_os = "linux")]
#[test]
#[serial(fs)]
fn rename_cross_device() {
    let _guard = FsStateGuard::new("test-projects/_fs_path_from")
        .expect("fs state guard should initialize for rename_cross_device");

    let temp_dir = unique_probe_target_dir("rename-cross-device");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t27 cross-device temp directory should be created"
    );

    let src = make_temp_path("cross-device-src");
    let dst = PathBuf::from(format!(
        "/dev/shm/opal-t27-cross-device-{}",
        std::process::id()
    ));

    let execution_result: Result<(), String> = (|| {
        let dst_parent = Path::new("/dev/shm");
        if !dst_parent.exists() {
            eprintln!("SKIP rename_cross_device: /dev/shm is unavailable on this host");
            return Ok(());
        }

        let src_parent = src
            .parent()
            .ok_or_else(|| String::from("cross-device source should have a parent directory"))?;

        let src_dev = fs::metadata(src_parent)
            .map_err(|e| format!("cross-device source parent metadata should be readable: {e}"))?
            .dev();
        let dst_dev = fs::metadata(dst_parent)
            .map_err(|e| {
                format!("cross-device destination parent metadata should be readable: {e}")
            })?
            .dev();

        if src_dev == dst_dev {
            eprintln!(
                "SKIP rename_cross_device: source parent and /dev/shm are same device ({src_dev})"
            );
            return Ok(());
        }

        fs::write(&src, "cross-device")
            .map_err(|e| format!("cross-device source fixture should be written: {e}"))?;

        let (code, stdout, stderr) = run_rename_harness(&src, &dst, &temp_dir)?;
        if code == 0_i32 || stdout.contains("OK") {
            return Err(format!(
                "rename-cross-device should fail with EXDEV, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        let expected =
            "ERR:Io: EXDEV: cross-device rename not supported (caller should copy+delete)";
        if !stdout.contains(expected) {
            return Err(format!(
                "rename-cross-device should contain explicit EXDEV message '{expected}', stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&src));
    drop(fs::remove_file(&dst));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t27 cross-device temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "rename_cross_device should assert EXDEV message or deterministic skip path: {failure_message}"
    );
}
