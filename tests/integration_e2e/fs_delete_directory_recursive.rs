#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use serial_test::serial;
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

#[test]
#[serial(fs)]
fn fs_recursive_delete_missing_path_error_from_op_source() {
    let temp_dir = unique_probe_target_dir("recursive-delete-missing-path-op-source");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 recursive-delete missing-path temp directory should be created"
    );

    let missing_dir = temp_dir.join("missing-recursive-delete-target");
    assert!(
        !missing_dir.exists(),
        "missing recursive-delete target must not exist before the .op probe runs"
    );

    let execution_result: Result<(), String> = (|| {
        let source = build_recursive_delete_missing_path_source(&missing_dir.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            return Err(format!(
                "recursive-delete missing-path probe should exit 0 because .op handles the error, status={:?}, stderr='{}'",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        if !stdout.contains("ERR_PATH=") {
            return Err(format!(
                "recursive-delete missing-path probe should print ERR_PATH= from the guard error branch, stdout='{stdout}'"
            ));
        }
        if stdout.contains("UNEXPECTED_SUCCESS") {
            return Err(format!(
                "recursive-delete missing-path probe should not print UNEXPECTED_SUCCESS, stdout='{stdout}'"
            ));
        }
        if missing_dir.exists() {
            return Err(format!(
                "missing recursive-delete target should remain absent after handled error, missing_dir='{missing_dir:?}'"
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 recursive-delete missing-path temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "fs_recursive_delete_missing_path_error_from_op_source should handle missing recursive-delete targets in .op and leave no residue: {failure_message}"
    );
}

fn build_recursive_delete_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, delete_directory_recursive_sync, path_exists_sync, is_directory_sync from standard\n\n##\n  Description: T26 recursive delete probe from inline .op source.\n##\nentry main = f(args: string[]): void errors DirectoryNotFoundError, PermissionDeniedError, DeleteFailureError, IsNotADirectoryError, InvalidPathError =>
    let target = path_from('{escaped_path}')\n    propagate delete_directory_recursive_sync(target)\n\n    let exists_after = propagate path_exists_sync(target)\n    let dir_after = propagate is_directory_sync(target)\n    print('exists_after={{exists_after}}')\n    print('dir_after={{dir_after}}')\n    return void\n"
    )
}

fn build_recursive_delete_missing_path_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, delete_directory_recursive_sync from standard\n\n##\n  Description: T26 recursive delete missing-path error probe from inline .op source.\n##\nentry main = f(args: string[]): void =>\n    let target = path_from('{escaped_path}')\n    guard delete_directory_recursive_sync(target) into ignored else err =>\n        print('ERR_PATH={{err}}')\n        return void\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
    )
}

fn build_empty_directory_workflow_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, list_directory_sync, join_path_components, is_directory_sync, delete_directory_recursive_sync, delete_file_sync from standard\n\n##\n  Description: T26 empty-directory workflow probe from inline .op source.\n##\nentry main = f(args: string[]): void errors DirectoryNotFoundError, PermissionDeniedError, ReadFailureError, IsNotADirectoryError, InvalidPathError, DeleteFailureError, FileNotFoundError, IsADirectoryError =>\n    let base = path_from('{escaped_path}')\n\n    guard list_directory_sync(base) into entries else err =>\n        print('LIST_ERR={{err}}')\n        return void\n\n    for child_entry in entries:\n        let child = join_path_components(base, [child_entry])\n        guard is_directory_sync(child) into child_is_dir else err =>\n            print('STAT_ERR={{err}}')\n            return void\n\n        if child_is_dir:\n            guard delete_directory_recursive_sync(child) into _ else err =>\n                print('RMDIR_ERR={{err}}')\n                return void\n        else:\n            guard delete_file_sync(child) into _ else err =>\n                print('DEL_ERR={{err}}')\n                return void\n\n    guard list_directory_sync(base) into remaining else err =>\n        print('FINAL_LIST_ERR={{err}}')\n        return void\n\n    let remaining_len = remaining.length\n    print('remaining={{remaining_len}}')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program_for_tests(Path::new("test-projects/_t26_delete_directory_recursive/src/main.op"), source, temp_dir, &TargetTriple::host());

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t26 recursive-delete probe source should compile into a binary: {error:?}"
            ));
        }
    };

    let child = std::process::Command::new(&binary_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| format!("t26 recursive-delete probe binary should execute: {error}"))?;

    super::fs_helpers::wait_for_child_output_with_timeout(
        child,
        std::time::Duration::from_secs(30),
        "t26 recursive-delete probe binary",
    )
}

fn parse_bool_like(rest: &str) -> Option<bool> {
    match rest.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_post_delete(stdout: &str) -> Result<(bool, bool), String> {
    let mut exists_after: Option<bool> = None;
    let mut dir_after: Option<bool> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("exists_after=") {
            exists_after = parse_bool_like(rest);
        } else if let Some(rest) = line.strip_prefix("dir_after=") {
            dir_after = parse_bool_like(rest);
        }
    }

    let exists_after = exists_after
        .ok_or_else(|| format!("recursive-delete output missing exists_after= line: {stdout}"))?;
    let dir_after = dir_after
        .ok_or_else(|| format!("recursive-delete output missing dir_after= line: {stdout}"))?;

    Ok((exists_after, dir_after))
}

fn parse_remaining_len(stdout: &str) -> Result<u64, String> {
    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("remaining=") {
            let trimmed = rest.trim();
            let value = trimmed.parse::<u64>().map_err(|e| {
                format!("remaining= value should parse as u64, got '{trimmed}', error={e}")
            })?;
            return Ok(value);
        }
    }

    Err(format!(
        "empty-directory workflow output missing remaining= line: {stdout}"
    ))
}

#[test]
#[serial(fs)]
fn fs_recursive_delete_from_op_source() {
    let temp_dir = unique_probe_target_dir("recursive-delete-op-source");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 recursive-delete temp directory should be created"
    );

    let fixture_root = make_temp_path("recursive-fixture-root");
    let nested_dir = fixture_root.join("nested").join("deeper").join("leaf");
    let nested_file = nested_dir.join("payload.txt");
    let sibling_file = fixture_root.join("sibling.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&nested_dir)
            .map_err(|e| format!("t26 nested fixture directories should be created: {e}"))?;
        fs::write(&nested_file, "recursive delete fixture payload")
            .map_err(|e| format!("t26 nested fixture file should be written: {e}"))?;
        fs::write(&sibling_file, "sibling payload")
            .map_err(|e| format!("t26 sibling fixture file should be written: {e}"))?;

        let temp_root = std::env::temp_dir();
        if !fixture_root.starts_with(&temp_root) {
            return Err(format!(
                "fixture root must stay under temp dir for sandbox safety, fixture='{fixture_root:?}', temp='{temp_root:?}'"
            ));
        }
        if !fixture_root.exists() {
            return Err(format!(
                "fixture root must exist before recursive delete execution, fixture='{fixture_root:?}'"
            ));
        }

        let source = build_recursive_delete_source(&fixture_root.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            return Err(format!(
                "recursive-delete probe should exit 0, status={:?}, stderr='{}'",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let (exists_after, dir_after) = parse_post_delete(&stdout)?;
        if exists_after || dir_after {
            return Err(format!(
                "post-delete probe should report exists_after=false and dir_after=false/0, got exists_after={exists_after}, dir_after={dir_after}, stdout='{stdout}'"
            ));
        }

        if fixture_root.exists() {
            return Err(format!(
                "fixture root should be removed by delete_directory_recursive_sync, fixture='{fixture_root:?}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&fixture_root));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 recursive-delete temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "fs_recursive_delete_from_op_source should remove nested fixture root and report both exists_after/dir_after false: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn fs_empty_directory_workflow_from_op_source() {
    let temp_dir = unique_probe_target_dir("empty-directory-workflow-op-source");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t26 empty-directory workflow temp directory should be created"
    );

    let target_dir = make_temp_path("empty-workflow");
    let sub_dir = target_dir.join("sub");
    let nested_dir = sub_dir.join("nested");
    let a_txt = target_dir.join("a.txt");
    let b_txt = target_dir.join("b.txt");
    let c_txt = sub_dir.join("c.txt");
    let d_txt = nested_dir.join("d.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&nested_dir)
            .map_err(|e| format!("t26 nested workflow directories should be created: {e}"))?;
        fs::write(&a_txt, "a")
            .map_err(|e| format!("t26 fixture file a.txt should be written: {e}"))?;
        fs::write(&b_txt, "b")
            .map_err(|e| format!("t26 fixture file b.txt should be written: {e}"))?;
        fs::write(&c_txt, "c")
            .map_err(|e| format!("t26 fixture file sub/c.txt should be written: {e}"))?;
        fs::write(&d_txt, "d")
            .map_err(|e| format!("t26 fixture file sub/nested/d.txt should be written: {e}"))?;

        let temp_root = std::env::temp_dir();
        if !target_dir.starts_with(&temp_root) {
            return Err(format!(
                "target_dir must stay under temp dir for sandbox safety, target='{target_dir:?}', temp='{temp_root:?}'"
            ));
        }
        if !target_dir.exists() {
            return Err(format!(
                "target_dir must exist before empty-directory workflow execution, target='{target_dir:?}'"
            ));
        }

        let source = build_empty_directory_workflow_source(&target_dir.to_string_lossy());
        let run_output = compile_and_run_inline_program(&source, &temp_dir)?;

        if !run_output.status.success() {
            return Err(format!(
                "empty-directory workflow probe should exit 0, status={:?}, stderr='{}'",
                run_output.status.code(),
                String::from_utf8_lossy(&run_output.stderr)
            ));
        }

        let stdout = String::from_utf8_lossy(&run_output.stdout);
        let remaining_len = parse_remaining_len(&stdout)?;
        if remaining_len != 0 {
            return Err(format!(
                "empty-directory workflow should report remaining=0, got remaining={remaining_len}, stdout='{stdout}'"
            ));
        }

        if stdout.contains("LIST_ERR=")
            || stdout.contains("STAT_ERR=")
            || stdout.contains("RMDIR_ERR=")
            || stdout.contains("DEL_ERR=")
            || stdout.contains("FINAL_LIST_ERR=")
        {
            return Err(format!(
                "empty-directory workflow should not print guard error markers, stdout='{stdout}'"
            ));
        }

        if !target_dir.exists() {
            return Err(format!(
                "empty-directory workflow must preserve target root directory, target='{target_dir:?}'"
            ));
        }

        let remaining_children = std::fs::read_dir(&target_dir)
            .map_err(|e| {
                format!(
                    "empty-directory workflow should allow post-run read_dir on target root: {e}"
                )
            })?
            .count();
        if remaining_children != 0 {
            return Err(format!(
                "target root directory should be empty after workflow, remaining child count={remaining_children}, target='{target_dir:?}'"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_dir_all(&target_dir));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t26 empty-directory workflow temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "fs_empty_directory_workflow_from_op_source should delete top-level children via .op workflow while preserving target root: {failure_message}"
    );
}
