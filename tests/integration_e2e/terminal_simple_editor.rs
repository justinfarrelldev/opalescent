#![cfg(feature = "integration")]

use super::*;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const EDITOR_TEST_TIMEOUT: Duration = Duration::from_secs(30);

fn run_editor_fixture(
    binary_path: &Path,
    input_path: &Path,
    events: &str,
    context: &str,
) -> Result<String, String> {
    let input_dir = input_path
        .parent()
        .ok_or_else(|| format!("{context} input path should have a parent"))?;
    let input_name = input_path
        .file_name()
        .ok_or_else(|| format!("{context} input path should have a file name"))?;
    let child = Command::new(binary_path)
        .current_dir(input_dir)
        .arg(input_name)
        .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
        .env("OPAL_TERMINAL_FAKE_EVENTS", events)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("{context} binary should spawn: {error}"))?;
    let output =
        tests::fs_helpers::wait_for_child_output_with_timeout(child, EDITOR_TEST_TIMEOUT, context)?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "{context} should exit successfully, status {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
            output.status.code()
        ));
    }
    Ok(stdout)
}

fn assert_real_editor_run_omits_test_summary(
    binary_path: &Path,
    input_path: &Path,
) -> Result<(), String> {
    let input_dir = input_path
        .parent()
        .ok_or_else(|| String::from("real-run input should have parent"))?;
    let input_name = input_path
        .file_name()
        .ok_or_else(|| String::from("real-run input should have file name"))?;
    let child = Command::new(binary_path)
        .current_dir(input_dir)
        .arg(input_name)
        .env_remove("OPAL_TERMINAL_FAKE_BACKEND")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!("terminal-simple-editor real-style binary should spawn: {error}")
        })?;
    let output = tests::fs_helpers::wait_for_child_output_with_timeout(
        child,
        EDITOR_TEST_TIMEOUT,
        "terminal-simple-editor real-style no-summary binary",
    )?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() || stdout.contains("EDITOR_SUMMARY") {
        return Err(format!(
            "real-style editor run should exit without fake-backend summary, status {:?}, stdout:\n{stdout}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr),
        ));
    }
    Ok(())
}

#[test]
fn terminal_simple_editor_uses_shared_adt_manifests_and_runs() {
    let cwd = std::env::current_dir().expect("current directory should be readable");
    let project_dir = cwd.join("test-projects/terminal-simple-editor");
    let source_dir = project_dir.join("src");
    let types_path = source_dir.join("editor.types.op");
    let legacy_constants_path = source_dir.join("constants.op");

    let types_source = fs::read_to_string(&types_path)
        .expect("simple editor should declare shared public ADTs in editor.types.op");
    for expected_type in [
        "public type EditorMode",
        "public type EditorCommand",
        "public type EditorStatus",
        "public type EditorTermination",
        "public type CursorPosition",
        "public type EditorState",
        "public type EditorTransition",
    ] {
        assert!(
            types_source.contains(expected_type),
            "editor.types.op should contain {expected_type}"
        );
    }
    assert!(
        !legacy_constants_path.exists(),
        "simple editor should no longer use the legacy integer constants module"
    );

    let temp_dir = tests::fs_helpers::unique_probe_target_dir("terminal-simple-editor-adt");
    prepare_dir(&temp_dir).expect("terminal-simple-editor temp directory should be created");

    let result: Result<(), String> = (|| {
        let input_path = temp_dir.join("input.txt");
        fs::write(&input_path, "alpha\nbeta\n")
            .map_err(|error| format!("editor input file should be writable: {error}"))?;
        let invalid_path = temp_dir.join("invalid.txt");
        fs::write(&invalid_path, "alpha\nbeta\n")
            .map_err(|error| format!("invalid-command input file should be writable: {error}"))?;

        let binary_path = compile_project_for_tests(&project_dir, &temp_dir, &TargetTriple::host())
            .map_err(|error| format!("terminal-simple-editor should compile: {error}"))?;

        let stdout = run_editor_fixture(
            &binary_path,
            &input_path,
            "key:ArrowDown|key:End|text:i|text:!|key:Escape|text::|text:wq|key:Enter|eof",
            "terminal-simple-editor ADT save/quit binary",
        )?;
        for expected_fragment in ["\u{1b}[1;1H1 | alpha", "\u{1b}[2;1H2 | beta"] {
            if !stdout.contains(expected_fragment) {
                return Err(format!(
                    "editor should render rows at absolute terminal columns; missing {expected_fragment:?} in stdout:\n{stdout:?}"
                ));
            }
        }
        if !stdout.contains("EDITOR_SUMMARY")
            || !stdout.contains("saved=1")
            || !stdout.contains("termination=command-wq")
            || !stdout.contains("status=ok")
        {
            return Err(format!(
                "save/quit stdout should contain the successful summary, got:\n{stdout}"
            ));
        }
        let saved_text = fs::read_to_string(&input_path)
            .map_err(|error| format!("saved editor file should be readable: {error}"))?;
        if saved_text.trim_end() != "alpha\nbeta!" {
            return Err(format!(
                "editor should save the edited second line, got {saved_text:?}"
            ));
        }

        let invalid_stdout = run_editor_fixture(
            &binary_path,
            &invalid_path,
            "text::|text:nope|key:Enter|eof",
            "terminal-simple-editor ADT invalid-command binary",
        )?;
        if !invalid_stdout.contains("not an editor command: nope") {
            return Err(format!(
                "invalid-command run should render the payload status, got:\n{invalid_stdout}"
            ));
        }

        assert_real_editor_run_omits_test_summary(&binary_path, &input_path)?;
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-simple-editor temp directory should be removed");
    assert!(
        result.is_ok(),
        "terminal-simple-editor ADT manifest refactor should compile and run: {}",
        result.err().unwrap_or_default()
    );
}
