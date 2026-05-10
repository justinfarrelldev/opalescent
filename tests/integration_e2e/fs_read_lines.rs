#![cfg(feature = "integration")]

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
        "opalescent-t17-{label}-{}-{nanos}",
        std::process::id()
    ))
}

fn build_harness(temp_dir: &Path) -> Result<PathBuf, String> {
    let harness_c = temp_dir.join("read_lines_harness.c");
    let harness_bin = temp_dir.join("read_lines_harness");

    let source = r#"#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include "runtime/opal_runtime.h"

int main(int argc, char** argv) {
    if (argc != 2) {
        fprintf(stderr, "usage: %s <path>\n", argv[0]);
        return 64;
    }

    FsStringArrayResult result = read_lines_sync(argv[1]);
    if (result.error != NULL) {
        printf("ERROR:%s\n", result.error);
        free((void*)result.error);
        return 2;
    }

    printf("COUNT:%lld\n", (long long)result.count);
    for (long long i = 0; i < result.count; i++) {
        const char* line = result.value[i] ? result.value[i] : "";
        printf("LINE:%lld:%s\n", i, line);
        free(result.value[i]);
    }
    free(result.value);
    return 0;
}
"#;

    fs::write(&harness_c, source)
        .map_err(|e| format!("t17 harness source should be written: {e}"))?;

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
        "t17 harness compile command",
    )?;

    if !compile.status.success() {
        let stderr = String::from_utf8_lossy(&compile.stderr);
        let stdout = String::from_utf8_lossy(&compile.stdout);
        return Err(format!(
            "t17 harness compile should succeed, status={:?}, stdout='{}', stderr='{}'",
            compile.status.code(),
            stdout,
            stderr
        ));
    }

    Ok(harness_bin)
}

fn run_harness(path: &Path) -> Result<(i64, Vec<String>), String> {
    let temp_dir = make_temp_path("harness");
    fs::create_dir_all(&temp_dir)
        .map_err(|e| format!("t17 harness temp directory should be created: {e}"))?;

    let harness_bin = build_harness(&temp_dir)?;

    let mut run_command = Command::new(&harness_bin);
    run_command.arg(path);
    let output = run_command_output_with_timeout(
        &mut run_command,
        std::time::Duration::from_secs(10),
        "t17 harness binary",
    )?;

    drop(fs::remove_file(&harness_bin));
    drop(fs::remove_file(temp_dir.join("read_lines_harness.c")));
    drop(fs::remove_dir_all(&temp_dir));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "t17 harness should exit 0, status={:?}, stdout='{}', stderr='{}'",
            output.status.code(),
            stdout,
            stderr
        ));
    }

    parse_harness_stdout(&String::from_utf8_lossy(&output.stdout))
}

fn parse_harness_stdout(stdout: &str) -> Result<(i64, Vec<String>), String> {
    let mut count: Option<i64> = None;
    let mut lines: Vec<String> = Vec::new();

    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("COUNT:") {
            let parsed = rest
                .trim()
                .parse::<i64>()
                .map_err(|e| format!("COUNT field should parse as i64, got '{rest}': {e}"))?;
            count = Some(parsed);
            continue;
        }

        if let Some(rest) = line.strip_prefix("LINE:") {
            let mut parts = rest.splitn(2, ':');
            let _index = parts
                .next()
                .ok_or_else(|| format!("LINE record missing index in '{line}'"))?;
            let payload = parts
                .next()
                .ok_or_else(|| format!("LINE record missing payload in '{line}'"))?;
            lines.push(payload.to_owned());
        }
    }

    let count = count.ok_or_else(|| format!("stdout missing COUNT record: {stdout}"))?;
    Ok((count, lines))
}

#[test]
#[serial(fs)]
fn read_file_to_lines_lf() {
    let fixture_dir = make_temp_path("lf");
    let fixture_file = fixture_dir.join("lf.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("LF fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "a\nb\nc")
            .map_err(|e| format!("LF fixture file should be written: {e}"))?;

        let (count, lines) = run_harness(&fixture_file)?;

        if count != 3 {
            return Err(format!("LF input should produce count 3, got {count}"));
        }

        let expected = ["a", "b", "c"];
        if lines != expected {
            return Err(format!(
                "LF input should produce a,b,c lines, got {lines:?}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_lines_lf should split LF-separated lines correctly: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_lines_crlf() {
    let fixture_dir = make_temp_path("crlf");
    let fixture_file = fixture_dir.join("crlf.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("CRLF fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, b"a\r\nb\r\n")
            .map_err(|e| format!("CRLF fixture file should be written: {e}"))?;

        let (count, lines) = run_harness(&fixture_file)?;

        if count != 2 {
            return Err(format!("CRLF input should produce count 2, got {count}"));
        }

        let expected = ["a", "b"];
        if lines != expected {
            return Err(format!(
                "CRLF input should normalize and split to a,b, got {lines:?}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_lines_crlf should normalize CRLF and drop trailing empty line: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_lines_mixed() {
    let fixture_dir = make_temp_path("mixed");
    let fixture_file = fixture_dir.join("mixed.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("mixed fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, b"a\r\nb\nc\r\n")
            .map_err(|e| format!("mixed fixture file should be written: {e}"))?;

        let (count, lines) = run_harness(&fixture_file)?;

        if count != 3 {
            return Err(format!("mixed input should produce count 3, got {count}"));
        }

        let expected = ["a", "b", "c"];
        if lines != expected {
            return Err(format!(
                "mixed line endings should produce a,b,c, got {lines:?}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_lines_mixed should normalize CRLF and split on LF only: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_lines_trailing_newline() {
    let fixture_dir = make_temp_path("trailing");
    let fixture_file = fixture_dir.join("trailing.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("trailing fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "a\nb\nc\n")
            .map_err(|e| format!("trailing fixture file should be written: {e}"))?;

        let (count, lines) = run_harness(&fixture_file)?;

        if count != 3 {
            return Err(format!(
                "trailing-newline input should produce count 3 (no extra empty line), got {count}"
            ));
        }

        let expected = ["a", "b", "c"];
        if lines != expected {
            return Err(format!(
                "trailing newline should not append empty element, got {lines:?}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_lines_trailing_newline should not return trailing empty element: {failure_message}"
    );
}

#[test]
#[serial(fs)]
fn read_file_to_lines_empty() {
    let fixture_dir = make_temp_path("empty");
    let fixture_file = fixture_dir.join("empty.txt");

    let execution_result: Result<(), String> = (|| {
        fs::create_dir_all(&fixture_dir)
            .map_err(|e| format!("empty fixture directory should be created: {e}"))?;
        fs::write(&fixture_file, "")
            .map_err(|e| format!("empty fixture file should be written: {e}"))?;

        let (count, lines) = run_harness(&fixture_file)?;

        if count != 0 {
            return Err(format!("empty file should produce count 0, got {count}"));
        }

        if !lines.is_empty() {
            return Err(format!(
                "empty file should produce no line entries, got {lines:?}"
            ));
        }

        Ok(())
    })();

    drop(fs::remove_file(&fixture_file));
    drop(fs::remove_dir_all(&fixture_dir));

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "read_file_to_lines_empty should return count=0 with no lines: {failure_message}"
    );
}
