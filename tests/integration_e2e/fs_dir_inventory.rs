#![cfg(feature = "integration")]

use super::*;
use super::fs_helpers::{FsStateGuard, assert_workspace_empty, strip_crlf, unique_probe_target_dir};
use serial_test::serial;
use std::path::Path;

fn seed_inventory_files(base_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(base_dir.join("workspace").join("inventory"))
        .map_err(|e| format!("fs_dir_inventory harness precreate inventory dir should succeed: {e}"))?;

    fs::write(base_dir.join("workspace").join("inventory").join("a.txt"), "alpha")
        .map_err(|e| format!("fs_dir_inventory harness seed a.txt should succeed: {e}"))?;
    fs::write(base_dir.join("workspace").join("inventory").join("b.txt"), "beta")
        .map_err(|e| format!("fs_dir_inventory harness seed b.txt should succeed: {e}"))?;
    fs::write(base_dir.join("workspace").join("inventory").join("c.txt"), "gamma")
        .map_err(|e| format!("fs_dir_inventory harness seed c.txt should succeed: {e}"))?;
    Ok(())
}

fn harness_c_source() -> &'static str {
    r#"#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <inventory_dir>\n", argv[0]);
        return 64;
    }

    FsPathArrayResult listed = list_directory_sync(argv[1]);
    if (listed.error != NULL) {
        fprintf(stderr, "ERR:%s\n", listed.error);
        free((void*)listed.error);
        return 2;
    }

    if (listed.count != 3) {
        fprintf(stderr, "ERR:count=%lld\n", (long long)listed.count);
        for (int64_t i = 0; i < listed.count; i++) {
            free(listed.value[i]);
        }
        free(listed.value);
        return 3;
    }

    const char* expected[] = {"a.txt", "b.txt", "c.txt"};
    for (int64_t i = 0; i < listed.count; i++) {
        if (strcmp(listed.value[i], expected[i]) != 0) {
            fprintf(stderr, "ERR:order[%lld]=%s\n", (long long)i, listed.value[i]);
            for (int64_t j = 0; j < listed.count; j++) {
                free(listed.value[j]);
            }
            free(listed.value);
            return 4;
        }
    }

    for (int64_t i = 0; i < listed.count; i++) {
        free(listed.value[i]);
    }
    free(listed.value);

    char path_a[1024];
    char path_b[1024];
    char path_c[1024];

    snprintf(path_a, sizeof(path_a), "%s/a.txt", argv[1]);
    snprintf(path_b, sizeof(path_b), "%s/b.txt", argv[1]);
    snprintf(path_c, sizeof(path_c), "%s/c.txt", argv[1]);

    if (remove(path_a) != 0) return 5;
    if (remove(path_b) != 0) return 6;
    if (remove(path_c) != 0) return 7;
    if (rmdir(argv[1]) != 0) return 8;

    return 0;
}
"#
}

fn compile_list_harness(harness_c: &std::path::Path, harness_bin: &std::path::Path) -> Result<(), String> {
    let compile = std::process::Command::new("cc")
        .arg("-std=gnu11")
        .arg("-I.")
        .arg("runtime/opal_fs.c")
        .arg(harness_c)
        .arg("-o")
        .arg(harness_bin)
        .output()
        .map_err(|e| format!("fs_dir_inventory harness compile command should execute: {e}"))?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "fs_dir_inventory harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(), stdout, stderr
        ));
    }
    Ok(())
}

fn run_harness_list_sorted(base_dir: &Path) -> Result<(), String> {
    seed_inventory_files(base_dir)?;
    let temp_dir = unique_probe_target_dir("dir-inventory-harness");
    let prepare_target = prepare_dir(&temp_dir);
    if prepare_target.is_err() {
        return Err(format!(
            "fs_dir_inventory harness target prepare should succeed: {:?}",
            prepare_target.err()
        ));
    }

    let harness_c = temp_dir.join("fs_dir_inventory_harness.c");
    let harness_bin = temp_dir.join("fs_dir_inventory_harness");
    let inventory_dir = base_dir.join("workspace").join("inventory");

    fs::write(&harness_c, harness_c_source())
        .map_err(|e| format!("fs_dir_inventory harness source should be written: {e}"))?;

    compile_list_harness(&harness_c, &harness_bin)?;

    let run = std::process::Command::new(&harness_bin)
        .arg(inventory_dir.to_string_lossy().into_owned())
        .current_dir(".")
        .output()
        .map_err(|e| format!("fs_dir_inventory harness should execute: {e}"))?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(&harness_c));
    drop(fs::remove_dir_all(&inventory_dir));

    let cleanup_target = cleanup_dir(&temp_dir);
    if cleanup_target.is_err() {
        return Err(format!(
            "fs_dir_inventory harness target cleanup should succeed: {:?}",
            cleanup_target.err()
        ));
    }

    if !run.status.success() {
        return Err(format!(
            "fs_dir_inventory harness should validate sorted list/count and cleanup, status={:?}, stdout='{}', stderr='{}'",
            run.status.code(),
            String::from_utf8_lossy(&run.stdout),
            String::from_utf8_lossy(&run.stderr)
        ));
    }

    Ok(())
}

#[cfg(feature = "integration")]
#[test]
#[serial(fs)]
fn fs_dir_inventory() {
    {
        let _guard = FsStateGuard::new("test-projects/_fs_dir_inventory")
            .expect("_fs_dir_inventory guard should initialize and reset target/workspace");

        assert_workspace_empty("_fs_dir_inventory");

        let cwd = std::env::current_dir();
        assert!(
            cwd.is_ok(),
            "current working directory should be readable for _fs_dir_inventory fixture test"
        );
        let Ok(cwd_path) = cwd else {
            return;
        };

        let project_dir = cwd_path.join("test-projects/_fs_dir_inventory");
        let temp_dir = unique_probe_target_dir("dir-inventory-fixture");
        let prepare = prepare_dir(&temp_dir);
        assert!(
            prepare.is_ok(),
            "_fs_dir_inventory target directory should be created before compile"
        );

        let binary_result = opalescent::compiler::compile_project(
            &project_dir,
            &temp_dir,
            &TargetTriple::host(),
        );
        assert!(
            binary_result.is_ok(),
            "_fs_dir_inventory fixture should compile into a binary: {}",
            binary_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown compile error"), |error| format!("{error}"))
        );
        let Ok(binary_path) = binary_result else {
            return;
        };

        let output_result = std::process::Command::new(&binary_path).output();
        assert!(
            output_result.is_ok(),
            "_fs_dir_inventory compiled binary should execute: {}",
            output_result
                .as_ref()
                .err()
                .map_or_else(|| String::from("unknown execution error"), |error| format!("{error}"))
        );
        let Ok(run_output) = output_result else {
            return;
        };

        let stdout = strip_crlf(&String::from_utf8_lossy(&run_output.stdout));
        assert!(
            run_output.status.success(),
            "_fs_dir_inventory binary should exit with status code 0, got: {:?}, stderr={}",
            run_output.status.code(),
            String::from_utf8_lossy(&run_output.stderr)
        );
        assert!(
            stdout.contains("inventory: 3 files; cleanup ok"),
            "_fs_dir_inventory output should contain success line, got: {stdout:?}"
        );

        let harness_check = run_harness_list_sorted(&project_dir);
        assert!(
            harness_check.is_ok(),
            "_fs_dir_inventory harness should verify sorted list/count and cleanup: {}",
            harness_check.err().unwrap_or_default()
        );

        assert_workspace_empty("_fs_dir_inventory");
    }

    assert_workspace_empty("_fs_dir_inventory");
}
