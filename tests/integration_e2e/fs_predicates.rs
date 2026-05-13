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
        "opalescent-t25-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_predicate_source(path: &str) -> String {
    let escaped_path = path.replace('\\', "\\\\").replace('\'', "\\'");

    format!(
        "import path_from, path_exists_sync, is_file_sync, is_directory_sync from standard\n\n##\n  Description: T25 predicate matrix probe.\n##\nentry main = f(args: string[]): void errors PermissionDeniedError, InvalidPathError =>\n    let target = path_from('{escaped_path}')\n    let exists = propagate path_exists_sync(target)\n    let file = propagate is_file_sync(target)\n    let dir = propagate is_directory_sync(target)\n\n    print('exists={{exists}}')\n    print('file={{file}}')\n    print('dir={{dir}}')\n    return void\n"
    )
}

fn compile_and_run_inline_program(
    source: &str,
    temp_dir: &Path,
) -> Result<std::process::Output, String> {
    let binary_result = compile_program_for_tests(
        Path::new("test-projects/_t25_fs_predicates/src/main.op"),
        source,
        temp_dir,
        &TargetTriple::host(),
    );

    let binary_path = match binary_result {
        Ok(path) => path,
        Err(error) => {
            return Err(format!(
                "t25 predicate probe source should compile into a binary: {error}"
            ));
        }
    };

    run_binary_output_with_timeout(
        &binary_path,
        std::time::Duration::from_secs(10),
        "compiled binary",
    )
    .map_err(|error| format!("t25 predicate probe binary should execute: {error}"))
}

fn parse_bool_like(rest: &str) -> Option<bool> {
    match rest.trim() {
        "true" | "1" => Some(true),
        "false" | "0" => Some(false),
        _ => None,
    }
}

fn parse_matrix(stdout: &str) -> Result<(bool, bool, bool), String> {
    let mut exists: Option<bool> = None;
    let mut file: Option<bool> = None;
    let mut dir: Option<bool> = None;

    for line in stdout.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("exists=") {
            exists = parse_bool_like(rest);
        } else if let Some(rest) = line.strip_prefix("file=") {
            file = parse_bool_like(rest);
        } else if let Some(rest) = line.strip_prefix("dir=") {
            dir = parse_bool_like(rest);
        }
    }

    let exists =
        exists.ok_or_else(|| format!("predicate output missing exists= line: {stdout}"))?;
    let file = file.ok_or_else(|| format!("predicate output missing file= line: {stdout}"))?;
    let dir = dir.ok_or_else(|| format!("predicate output missing dir= line: {stdout}"))?;

    Ok((exists, file, dir))
}

#[test]
#[serial(fs)]
fn fs_predicates_matrix() {
    let temp_dir = unique_probe_target_dir("predicates-matrix");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "t25 predicate temp directory should be created"
    );

    let base = make_temp_path("predicates");
    let file_path = base.join("file.txt");
    let dir_path = base.join("folder");
    let missing_path = base.join("missing.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&base)
            .map_err(|e| format!("t25 predicate base directory should be created: {e}"))?;
        fs::write(&file_path, "x")
            .map_err(|e| format!("t25 predicate file fixture should be written: {e}"))?;
        fs::create_dir_all(&dir_path)
            .map_err(|e| format!("t25 predicate dir fixture should be created: {e}"))?;

        let file_source = build_predicate_source(&file_path.to_string_lossy());
        let file_out = compile_and_run_inline_program(&file_source, &temp_dir)?;
        if !file_out.status.success() {
            return Err(format!(
                "file predicate probe should exit 0, status={:?}, stderr='{}'",
                file_out.status.code(),
                String::from_utf8_lossy(&file_out.stderr)
            ));
        }
        let (file_exists, file_is_file, file_is_dir) =
            parse_matrix(&String::from_utf8_lossy(&file_out.stdout))?;
        if (file_exists, file_is_file, file_is_dir) != (true, true, false) {
            return Err(format!(
                "file predicate tuple should be (true,true,false), got ({file_exists},{file_is_file},{file_is_dir})"
            ));
        }

        let dir_source = build_predicate_source(&dir_path.to_string_lossy());
        let dir_out = compile_and_run_inline_program(&dir_source, &temp_dir)?;
        if !dir_out.status.success() {
            return Err(format!(
                "dir predicate probe should exit 0, status={:?}, stderr='{}'",
                dir_out.status.code(),
                String::from_utf8_lossy(&dir_out.stderr)
            ));
        }
        let (dir_exists, dir_is_file, dir_is_dir) =
            parse_matrix(&String::from_utf8_lossy(&dir_out.stdout))?;
        if (dir_exists, dir_is_file, dir_is_dir) != (true, false, true) {
            return Err(format!(
                "dir predicate tuple should be (true,false,true), got ({dir_exists},{dir_is_file},{dir_is_dir})"
            ));
        }

        let missing_source = build_predicate_source(&missing_path.to_string_lossy());
        let missing_out = compile_and_run_inline_program(&missing_source, &temp_dir)?;
        if !missing_out.status.success() {
            return Err(format!(
                "missing predicate probe should exit 0, status={:?}, stderr='{}'",
                missing_out.status.code(),
                String::from_utf8_lossy(&missing_out.stderr)
            ));
        }
        let (missing_exists, missing_is_file, missing_is_dir) =
            parse_matrix(&String::from_utf8_lossy(&missing_out.stdout))?;
        if (missing_exists, missing_is_file, missing_is_dir) != (false, false, false) {
            return Err(format!(
                "missing predicate tuple should be (false,false,false), got ({missing_exists},{missing_is_file},{missing_is_dir})"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&file_path));
    drop(fs::remove_dir_all(&dir_path));
    drop(fs::remove_dir_all(&base));

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "t25 predicate temp directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "fs_predicates_matrix should lock exists/is_file/is_directory behavior for file, dir, and missing paths: {failure_message}"
    );
}
