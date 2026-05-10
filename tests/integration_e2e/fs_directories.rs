#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, fs_project_root, unique_probe_target_dir,
};
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
        "opalescent-t28-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_directories_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("directories_harness.c");
    let harness_bin = temp_dir.join("directories_harness");

    let source = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc < 3) {
        fprintf(stderr, "usage: %s <mkdir|rmdir|list> <path>\n", argv[0]);
        return 64;
    }

    if (strcmp(argv[1], "mkdir") == 0) {
        FsVoidResult result = create_directory_sync(argv[2]);
        if (result.error != NULL) {
            printf("ERR:%s\n", result.error);
            free((void*)result.error);
            return 2;
        }
        printf("OK\n");
        return 0;
    }

    if (strcmp(argv[1], "mkdirp") == 0) {
        FsVoidResult result = create_directory_recursive_sync(argv[2]);
        if (result.error != NULL) {
            printf("ERR:%s\n", result.error);
            free((void*)result.error);
            return 2;
        }
        printf("OK\n");
        return 0;
    }

    if (strcmp(argv[1], "rmdir") == 0) {
        FsVoidResult result = delete_directory_sync(argv[2]);
        if (result.error != NULL) {
            printf("ERR:%s\n", result.error);
            free((void*)result.error);
            return 2;
        }
        printf("OK\n");
        return 0;
    }

    if (strcmp(argv[1], "list") == 0) {
        FsPathArrayResult result = list_directory_sync(argv[2]);
        if (result.error != NULL) {
            printf("ERR:%s\n", result.error);
            free((void*)result.error);
            return 2;
        }

        for (int64_t i = 0; i < result.count; i++) {
            printf("%s\n", result.value[i]);
            free(result.value[i]);
        }
        free(result.value);
        return 0;
    }

    fprintf(stderr, "unknown mode: %s\n", argv[1]);
    return 64;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("t28 harness source should be written: {e}"))?;

    let mut compile_command = Command::new("cc");
    compile_command
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_fs.c")
        .arg(&harness_c)
        .arg("-o")
        .arg(&harness_bin);
    let compile = run_command_output_with_timeout(
        &mut compile_command,
        std::time::Duration::from_secs(10),
        "t28 harness compile command",
    )?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "t28 harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

fn run_harness(mode: &str, path: &Path, temp_dir: &Path) -> Result<(i32, String, String), String> {
    let harness_bin = build_directories_harness(temp_dir)?;

    let mut run_command = Command::new(&harness_bin);
    run_command.arg(mode).arg(path);
    let output = run_command_output_with_timeout(
        &mut run_command,
        std::time::Duration::from_secs(10),
        "t28 harness",
    )?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(temp_dir.join("directories_harness.c")));

    Ok((
        output.status.code().unwrap_or(-1_i32),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

#[test]
#[serial(fs)]
fn list_directory_sorted() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("directories-list-sorted");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t28 list-directory-sorted temp directory should be created"
    );

    let fixture_dir = make_temp_path("list-sorted");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("t28 list fixture directory should be created: {e}"))?;

        fs::write(fixture_dir.join("c.txt"), "c")
            .map_err(|e| format!("t28 list fixture c.txt should be written: {e}"))?;
        fs::write(fixture_dir.join("a.txt"), "a")
            .map_err(|e| format!("t28 list fixture a.txt should be written: {e}"))?;
        fs::write(fixture_dir.join("b.txt"), "b")
            .map_err(|e| format!("t28 list fixture b.txt should be written: {e}"))?;

        let (code, stdout, stderr) = run_harness("list", &fixture_dir, &temp_dir)?;
        if code != 0_i32 {
            return Err(format!(
                "list_directory_sorted harness should exit 0, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        let mut lines = stdout
            .lines()
            .map(|line| line.trim().to_owned())
            .filter(|line| !line.is_empty())
            .collect::<Vec<String>>();

        if lines != ["a.txt", "b.txt", "c.txt"] {
            lines.sort();
            return Err(format!(
                "list_directory_sorted should return lexicographically sorted entries [a.txt, b.txt, c.txt], got {lines:?} (stderr='{stderr}')"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&fixture_dir));
    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t28 list-directory-sorted temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "list_directory_sorted should return sorted entries: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn mkdir_rmdir_roundtrip() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("directories-mkdir-rmdir");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t28 mkdir-rmdir-roundtrip temp directory should be created"
    );

    let fixture_dir = make_temp_path("mkdir-rmdir-roundtrip");

    let execution_result: Result<(), String> = (|| {
        let (mkdir_code, mkdir_stdout, mkdir_stderr) =
            run_harness("mkdir", &fixture_dir, &temp_dir)?;
        if mkdir_code != 0_i32 || !mkdir_stdout.contains("OK") {
            return Err(format!(
                "mkdir_rmdir_roundtrip mkdir should succeed, code={mkdir_code}, stdout='{mkdir_stdout}', stderr='{mkdir_stderr}'"
            ));
        }

        if !fixture_dir.exists() {
            return Err(String::from(
                "mkdir_rmdir_roundtrip should create the fixture directory",
            ));
        }

        let (rmdir_code, rmdir_stdout, rmdir_stderr) =
            run_harness("rmdir", &fixture_dir, &temp_dir)?;
        if rmdir_code != 0_i32 || !rmdir_stdout.contains("OK") {
            return Err(format!(
                "mkdir_rmdir_roundtrip rmdir should succeed, code={rmdir_code}, stdout='{rmdir_stdout}', stderr='{rmdir_stderr}'"
            ));
        }

        if fixture_dir.exists() {
            return Err(String::from(
                "mkdir_rmdir_roundtrip should remove the fixture directory",
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&fixture_dir));
    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t28 mkdir-rmdir-roundtrip temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "mkdir_rmdir_roundtrip should create and remove directory cleanly: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn mkdirp_accepts_existing_ancestor_directories() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("directories-mkdirp-existing-ancestors");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t28 mkdirp-existing-ancestors temp directory should be created"
    );

    let fixture_root = make_temp_path("mkdirp-existing-ancestors");
    let nested_dir = fixture_root
        .join("existing-parent")
        .join("child")
        .join("grandchild");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(fixture_root.join("existing-parent"))
            .map_err(|e| format!("t28 mkdirp existing parent should be created: {e}"))?;

        let (code, stdout, stderr) = run_harness("mkdirp", &nested_dir, &temp_dir)?;
        if code != 0_i32 || !stdout.contains("OK") {
            return Err(format!(
                "mkdirp_accepts_existing_ancestor_directories should succeed, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        if !nested_dir.exists() {
            return Err(format!(
                "mkdirp_accepts_existing_ancestor_directories should create nested directory {}",
                nested_dir.display()
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&fixture_root));
    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t28 mkdirp-existing-ancestors temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "mkdirp_accepts_existing_ancestor_directories should allow pre-existing ancestor directories: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn rmdir_not_empty() {
    let _guard = FsStateGuard::new(fs_project_root("_fs_path_from"))
        .expect("_fs_path_from guard should initialize and reset target/workspace");
    assert_workspace_empty("_fs_path_from");

    let temp_dir = unique_probe_target_dir("directories-rmdir-not-empty");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t28 rmdir-not-empty temp directory should be created"
    );

    let fixture_dir = make_temp_path("rmdir-not-empty");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("t28 not-empty fixture directory should be created: {e}"))?;
        fs::write(fixture_dir.join("child.txt"), "child")
            .map_err(|e| format!("t28 not-empty fixture child should be written: {e}"))?;

        let (code, stdout, stderr) = run_harness("rmdir", &fixture_dir, &temp_dir)?;
        if code == 0_i32 || stdout.contains("OK") {
            return Err(format!(
                "rmdir_not_empty should fail, code={code}, stdout='{stdout}', stderr='{stderr}'"
            ));
        }
        if !stdout.contains("ERR:Io: directory not empty") {
            return Err(format!(
                "rmdir_not_empty should contain exact not-empty message, stdout='{stdout}', stderr='{stderr}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(fixture_dir.join("child.txt")));
    drop(fs::remove_dir_all(&fixture_dir));
    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t28 rmdir-not-empty temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "rmdir_not_empty should fail with Io: directory not empty: {failure_message}"
    );
}
