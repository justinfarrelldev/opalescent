#![cfg(feature = "integration")]

use super::fs_helpers::{
    FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir,
};
use super::*;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn stringify_error<E: core::fmt::Display>(error: E) -> String {
    format!("{error}")
}

fn make_temp_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    std::env::temp_dir().join(format!(
        "opalescent-task6-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("absolute_path_sync_harness.c");
    let harness_bin = temp_dir.join("absolute_path_sync_harness");

    let source = r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "runtime/opal_runtime.h"

FsPathResult absolute_path_sync(const char* path);

int main(void) {
    FsPathResult empty = absolute_path_sync("");
    if (empty.error == NULL) {
        fprintf(stderr, "ERR:missing-empty-error\n");
        return 2;
    }
    if (strstr(empty.error, "InvalidPathError:") == NULL) {
        fprintf(stderr, "ERR:bad-empty-error=%s\n", empty.error);
        free((void*)empty.error);
        return 3;
    }
    free((void*)empty.error);

    char long_input[512];
    for (size_t i = 0; i < sizeof(long_input) - 1; i++) {
        long_input[i] = 'a';
    }
    long_input[sizeof(long_input) - 1] = '\0';

    FsPathResult invalid = absolute_path_sync(long_input);
    if (invalid.error == NULL) {
        fprintf(stderr, "ERR:missing-invalid-error\n");
        return 4;
    }
    if (strstr(invalid.error, "InvalidPathError:") == NULL) {
        fprintf(stderr, "ERR:bad-invalid-error=%s\n", invalid.error);
        free((void*)invalid.error);
        return 5;
    }
    free((void*)invalid.error);

    FsPathResult linux_abs = absolute_path_sync("/tmp");
    if (linux_abs.error != NULL) {
        fprintf(stderr, "ERR:linux-abs-failed=%s\n", linux_abs.error);
        free((void*)linux_abs.error);
        return 6;
    }
    if (linux_abs.value == NULL) {
        fprintf(stderr, "ERR:linux-abs-null\n");
        return 7;
    }
    printf("linux-abs=%s\n", linux_abs.value);
    free(linux_abs.value);

    FsPathResult drive_abs = absolute_path_sync("C:\\Users\\foo");
    if (drive_abs.error != NULL) {
        fprintf(stderr, "ERR:drive-abs-failed=%s\n", drive_abs.error);
        free((void*)drive_abs.error);
        return 8;
    }
    if (drive_abs.value == NULL) {
        fprintf(stderr, "ERR:drive-abs-null\n");
        return 9;
    }
    printf("drive-abs=%s\n", drive_abs.value);
    free(drive_abs.value);

    FsPathResult unc_abs = absolute_path_sync("\\\\server\\share\\dir\\file.ext");
    if (unc_abs.error != NULL) {
        fprintf(stderr, "ERR:unc-abs-failed=%s\n", unc_abs.error);
        free((void*)unc_abs.error);
        return 10;
    }
    if (unc_abs.value == NULL) {
        fprintf(stderr, "ERR:unc-abs-null\n");
        return 11;
    }
    printf("unc-abs=%s\n", unc_abs.value);
    free(unc_abs.value);

    return 0;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("task6 absolute_path_sync harness source should be written: {e}"))?;

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
        "task6 absolute_path_sync harness compile command",
    )?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "task6 absolute_path_sync harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

fn run_harness() -> Result<String, String> {
    let temp_dir = make_temp_path("absolute-path-sync-harness");
    fs::create_dir_all(&temp_dir).map_err(|e| {
        format!("task6 absolute_path_sync harness temp directory should be created: {e}")
    })?;

    let harness_bin = build_harness(&temp_dir)?;

    let output = run_binary_output_with_timeout(
        &harness_bin,
        std::time::Duration::from_secs(10),
        "task6 absolute_path_sync harness binary",
    )
    .map_err(|e| format!("task6 absolute_path_sync harness binary should execute: {e}"))?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(
        temp_dir.join("absolute_path_sync_harness.c"),
    ));
    drop(fs::remove_dir_all(&temp_dir));

    if !output.status.success() {
        return Err(format!(
            "task6 absolute_path_sync harness should exit 0, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(strip_crlf(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_absolute_path_sync_fixture_showcase() {
    {
        let _guard = FsStateGuard::new("test-projects/_absolute_path_sync")
            .expect("_absolute_path_sync guard should initialize and reset target/workspace");

        assert_workspace_empty("_absolute_path_sync");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _absolute_path_sync fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_absolute_path_sync");
        let temp_dir = unique_probe_target_dir("absolute-path-sync-fixture");

        let binary_result =
            compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host());
        assert!(
            binary_result.is_ok(),
            "_absolute_path_sync fixture should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), stringify_error)
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = run_binary_output_with_timeout(
            &binary_path,
            std::time::Duration::from_secs(10),
            "compiled binary",
        );
        assert!(
            output_result.is_ok(),
            "_absolute_path_sync compiled binary should execute: {}",
            output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), stringify_error)
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        let lines: Vec<&str> = stdout.lines().map(str::trim).collect();

        assert_eq!(
            lines.len(),
            4,
            "_absolute_path_sync fixture should print exactly 4 lines"
        );

        let existing_line = lines.first().copied().unwrap_or_default();
        assert!(
            existing_line.starts_with("./test-projects/_absolute_path_sync/src/main.op -> ")
                && existing_line.contains("main.op"),
            "existing relative path should resolve to an absolute main.op path, got: {existing_line}"
        );

        let missing_line = lines.get(1).copied().unwrap_or_default();
        assert!(
            missing_line.starts_with("./test-projects/_absolute_path_sync/does_not_exist.txt -> ")
                && missing_line.contains("does_not_exist.txt"),
            "non-existing relative path should resolve lexically to an absolute path, got: {missing_line}"
        );

        let normalized_line = lines.get(2).copied().unwrap_or_default();
        assert!(
            normalized_line.starts_with("./test-projects/_absolute_path_sync/src/../README.md -> ")
                && normalized_line.contains("README.md"),
            "path containing '..' should collapse to README absolute path, got: {normalized_line}"
        );

        let root_line = lines.get(3).copied().unwrap_or_default();
        assert_eq!(
            root_line, "/ -> /",
            "already absolute root path should remain root"
        );

        assert!(
            run_output.status.success(),
            "_absolute_path_sync binary should exit with status code 0, got: {:?}",
            run_output.status.code()
        );
    }

    assert_workspace_empty("_absolute_path_sync");
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn absolute_path_sync_allocates_errors_and_keeps_absolute_inputs() {
    let output = run_harness();
    assert!(
        output.is_ok(),
        "task6 absolute_path_sync harness should validate error allocation + absolute behavior: {}",
        output.err().unwrap_or_default()
    );

    let stdout = output.unwrap_or_default();
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();

    assert_eq!(
        lines.len(),
        3,
        "task6 harness should print linux/drive/unc absolute lines, got: {lines:?}"
    );

    let linux_line = lines.first().copied().unwrap_or_default();
    assert_eq!(
        linux_line, "linux-abs=/tmp",
        "Linux absolute path /tmp should remain unchanged"
    );

    let drive_line = lines.get(1).copied().unwrap_or_default();
    if cfg!(windows) {
        assert!(
            drive_line.starts_with("drive-abs=C:/Users/foo")
                || drive_line.starts_with("drive-abs=C:\\Users\\foo"),
            "Drive-letter input should be treated as absolute on Windows, got: {drive_line}"
        );
    } else {
        assert!(
            drive_line.contains("C:/Users/foo") || drive_line.contains("C:\\Users\\foo"),
            "Drive-letter input should survive absolute path resolution without error, got: {drive_line}"
        );
    }

    let unc_line = lines.get(2).copied().unwrap_or_default();
    if cfg!(windows) {
        assert!(
            unc_line.starts_with("unc-abs=//server/share/dir/file.ext")
                || unc_line.starts_with("unc-abs=\\\\server\\share\\dir\\file.ext"),
            "UNC input should be treated as absolute on Windows, got: {unc_line}"
        );
    } else {
        assert!(
            unc_line.contains("server/share/dir/file.ext")
                || unc_line.contains("server\\share\\dir\\file.ext"),
            "UNC-like input should survive absolute path resolution without error, got: {unc_line}"
        );
    }
}
