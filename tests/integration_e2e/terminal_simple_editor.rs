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

fn assert_editor_sources(source_dir: &Path) {
    let types_path = source_dir.join("editor.types.op");
    let main_path = source_dir.join("main.op");
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
    let main_source = fs::read_to_string(&main_path)
        .expect("terminal-simple-editor main source should be readable");
    assert!(
        !main_source.contains("OPAL_TERMINAL_FAKE_BACKEND")
            && !main_source.contains("OPAL_TERMINAL_FAKE_EVENTS")
            && !main_source.contains("EDITOR_SUMMARY"),
        "terminal-simple-editor production source should not contain fake-backend test hooks"
    );
}

fn assert_editor_save_quit(binary_path: &Path, input_path: &Path) -> Result<(), String> {
    let stdout = run_editor_fixture(
        binary_path,
        input_path,
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
    if stdout.contains("EDITOR_SUMMARY") {
        return Err(format!(
            "save/quit stdout should not contain app-level test summaries, got:\n{stdout}"
        ));
    }
    let saved_text = fs::read_to_string(input_path)
        .map_err(|error| format!("saved editor file should be readable: {error}"))?;
    if saved_text.trim_end() != "alpha\nbeta!" {
        return Err(format!(
            "editor should save the edited second line, got {saved_text:?}"
        ));
    }
    Ok(())
}

fn assert_editor_command_statuses(
    binary_path: &Path,
    invalid_path: &Path,
    temp_dir: &Path,
) -> Result<(), String> {
    let invalid_stdout = run_editor_fixture(
        binary_path,
        invalid_path,
        "text::|text:nope|key:Enter|eof",
        "terminal-simple-editor ADT invalid-command binary",
    )?;
    if !invalid_stdout.contains("not an editor command: nope") {
        return Err(format!(
            "invalid-command run should render the payload status, got:\n{invalid_stdout}"
        ));
    }

    let blank_command_stdout = run_editor_fixture(
        binary_path,
        invalid_path,
        "text::|key:Enter|eof",
        "terminal-simple-editor ADT blank-command binary",
    )?;
    if !blank_command_stdout.contains("not an editor command: ")
        || blank_command_stdout.contains("not an editor command: invalid.txt")
    {
        return Err(format!(
            "blank-command run should not use the file path as the invalid command, got:\n{blank_command_stdout}"
        ));
    }

    let missing_path = temp_dir.join("missing.txt");
    let missing_stdout = run_editor_fixture(
        binary_path,
        &missing_path,
        "eof",
        "terminal-simple-editor ADT missing-file binary",
    )?;
    if !missing_stdout.contains("new file missing.txt") {
        return Err(format!(
            "missing-file run should open a new empty buffer, got:\n{missing_stdout}"
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn assert_editor_save_failure(binary_path: &Path, temp_dir: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let read_only_dir = temp_dir.join("read-only");
    prepare_dir(&read_only_dir)
        .map_err(|error| format!("read-only save directory should be created: {error}"))?;
    let read_only_input = read_only_dir.join("input.txt");
    fs::write(&read_only_input, "alpha\nbeta\n")
        .map_err(|error| format!("read-only input file should be writable: {error}"))?;
    let mut permissions = fs::metadata(&read_only_dir)
        .map_err(|error| format!("read-only directory metadata should be readable: {error}"))?
        .permissions();
    permissions.set_mode(0o555);
    fs::set_permissions(&read_only_dir, permissions)
        .map_err(|error| format!("read-only directory permissions should be set: {error}"))?;

    let probe_path = read_only_dir.join("write-probe.txt");
    let read_only_is_enforced = fs::write(&probe_path, "probe").is_err();
    if !read_only_is_enforced && probe_path.exists() {
        fs::remove_file(&probe_path)
            .map_err(|error| format!("write probe cleanup should succeed: {error}"))?;
    }

    let save_failure_stdout = if read_only_is_enforced {
        run_editor_fixture(
            binary_path,
            &read_only_input,
            "text:i|text:!|key:Escape|text::|text:wq|key:Enter|eof",
            "terminal-simple-editor ADT save-failure binary",
        )?
    } else {
        String::new()
    };

    let mut writable_permissions = fs::metadata(&read_only_dir)
        .map_err(|error| format!("read-only directory metadata should be restored: {error}"))?
        .permissions();
    writable_permissions.set_mode(0o755);
    fs::set_permissions(&read_only_dir, writable_permissions)
        .map_err(|error| format!("read-only directory permissions should be restored: {error}"))?;

    if read_only_is_enforced
        && (!save_failure_stdout.contains("write failed:")
            || !save_failure_stdout.contains("[dirty]"))
    {
        return Err(format!(
            "save-failure run should keep editing and render dirty write failure, got:\n{save_failure_stdout}"
        ));
    }
    Ok(())
}

fn assert_editor_long_line_cursor(binary_path: &Path, temp_dir: &Path) -> Result<(), String> {
    let long_path = temp_dir.join("long.txt");
    fs::write(&long_path, format!("{}\n", "a".repeat(140)))
        .map_err(|error| format!("long-line input file should be writable: {error}"))?;
    let long_stdout = run_editor_fixture(
        binary_path,
        &long_path,
        "key:End|eof",
        "terminal-simple-editor ADT long-line binary",
    )?;
    if long_stdout.contains("\u{1b}[1;145H") || !long_stdout.contains("\u{1b}[1;80H") {
        return Err(format!(
            "long-line run should clamp the screen cursor to column 80, got:\n{long_stdout:?}"
        ));
    }
    Ok(())
}

#[test]
fn terminal_simple_editor_uses_shared_adt_manifests_and_runs() {
    let cwd = std::env::current_dir().expect("current directory should be readable");
    let project_dir = cwd.join("test-projects/terminal-simple-editor");
    let source_dir = project_dir.join("src");
    assert_editor_sources(&source_dir);

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

        assert_editor_save_quit(&binary_path, &input_path)?;
        assert_editor_command_statuses(&binary_path, &invalid_path, &temp_dir)?;
        #[cfg(unix)]
        assert_editor_save_failure(&binary_path, &temp_dir)?;
        assert_editor_long_line_cursor(&binary_path, &temp_dir)?;
        Ok(())
    })();

    cleanup_dir(&temp_dir).expect("terminal-simple-editor temp directory should be removed");
    assert!(
        result.is_ok(),
        "terminal-simple-editor ADT manifest refactor should compile and run: {}",
        result.err().unwrap_or_default()
    );
}
