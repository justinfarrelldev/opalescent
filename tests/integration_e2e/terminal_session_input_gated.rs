#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::fs;
use std::path::Path;

const TERMINAL_SESSION_RED_ENV: &str = "OPAL_TERMINAL_SESSION_RED";

#[derive(Clone, Copy)]
struct TerminalSessionFixtureMetadata {
    name: &'static str,
    opal_toml_path: &'static str,
    source_path: &'static str,
    input_plan: &'static str,
    expected_stdout_summary: &'static str,
    expected_stderr: &'static str,
    expected_status: i32,
}

const CORE_TERMINAL_SESSION_FIXTURES: &[TerminalSessionFixtureMetadata] = &[
    TerminalSessionFixtureMetadata {
        name: "terminal-key-log",
        opal_toml_path: "test-projects/terminal-key-log/opal.toml",
        source_path: "test-projects/terminal-key-log/src/main.op",
        input_plan: "Key(Text x), TimedOut, Key(Named Escape), Cancelled, EndOfInput",
        expected_stdout_summary: "KEY_LOG_SUMMARY keys=2 last_key=named+control timeouts=1 cancellations=1 termination=cancelled",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-text-echo-safe",
        opal_toml_path: "test-projects/terminal-text-echo-safe/opal.toml",
        source_path: "test-projects/terminal-text-echo-safe/src/main.op",
        input_plan: "TextInput(complete safe text), Paste(discard), UnknownBytes(discard), InputReset(PasteFallback), Key(non-text), TimedOut, EndOfInput",
        expected_stdout_summary: "<committed text>\nSAFE_ECHO_SUMMARY text=1 paste=1 unknown=1 resets=1 non_text=2 raw_session_writes=0",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-size-probe",
        opal_toml_path: "test-projects/terminal-size-probe/opal.toml",
        source_path: "test-projects/terminal-size-probe/src/main.op",
        input_plan: "Open with deterministic 80x24 size and fixed capability snapshot; no input events",
        expected_stdout_summary: "SIZE_PROBE_SUMMARY columns=80 rows=24 ordinary_capability=queried trusted_paste=queried",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-pause-counter",
        opal_toml_path: "test-projects/terminal-pause-counter/opal.toml",
        source_path: "test-projects/terminal-pause-counter/src/main.op",
        input_plan: "Pre-pause Key, TextInput; pause delivery UnknownBytes, InputReset(PauseBoundary)",
        expected_stdout_summary: "PAUSE_COUNTER_SUMMARY pre_pause_reads=2 pause_events=2 input_resets=1 final_boundary=InputReset.PauseBoundary observed=true",
        expected_stderr: "",
        expected_status: 0,
    },
];

fn should_run_terminal_session_red() -> bool {
    std::env::var(TERMINAL_SESSION_RED_ENV)
        .map(|value| value.trim() == "1")
        .unwrap_or(false)
}

#[test]
#[ignore = "RED test: opt-in via --ignored and OPAL_TERMINAL_SESSION_RED=1"]
fn terminal_session_input_gated_selected_api_red() {
    if !should_run_terminal_session_red() {
        eprintln!(
            "skipping terminal_session_input_gated_selected_api_red: {TERMINAL_SESSION_RED_ENV} != 1"
        );
        return;
    }

    let temp_dir = unique_probe_target_dir("terminal-session-input-gated-red");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-session-input-gated-red target directory should be created"
    );

    let source = "
import terminal_session_options_default, terminal_session_open_sync from standard

entry main = f(args: string[]): void =>
    let options = terminal_session_options_default()
    guard terminal_session_open_sync(options) into session else open_error =>
        print('terminal session open failed as expected before implementation')
        return void
    print('terminal session unexpectedly opened before implementation')
    return void
";

    let compile_result = compile_program_for_tests(
        Path::new("test-projects/terminal-session-input-gated-red/src/main.op"),
        source,
        &temp_dir,
        &TargetTriple::host(),
    );

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-session-input-gated-red target directory should be removed"
    );

    let failure_message = match compile_result {
        Ok(binary_path) => format!(
            "selected terminal session RED fixture unexpectedly compiled before terminal session support landed: {}",
            binary_path.display()
        ),
        Err(error) => format!(
            "selected terminal session RED fixture should compile only after the selected typed-event-session API is implemented; current compiler rejection is expected RED evidence:\n{error}"
        ),
    };

    assert!(
        failure_message.is_empty(),
        "terminal_session_input_gated selected API probe is intentionally RED until implementation: {failure_message}"
    );
}

#[test]
#[ignore = "RED test: opt-in core terminal fixtures via --ignored and OPAL_TERMINAL_SESSION_RED=1"]
fn terminal_session_input_gated_core_fixtures_red() {
    if !should_run_terminal_session_red() {
        eprintln!(
            "skipping terminal_session_input_gated_core_fixtures_red: {TERMINAL_SESSION_RED_ENV} != 1"
        );
        return;
    }

    let mut setup_failures: Vec<String> = Vec::new();
    let mut red_evidence: Vec<String> = Vec::new();

    for fixture in CORE_TERMINAL_SESSION_FIXTURES {
        if !Path::new(fixture.opal_toml_path).is_file() {
            setup_failures.push(format!(
                "{} fixture opal.toml should exist at {}",
                fixture.name, fixture.opal_toml_path
            ));
            continue;
        }

        let source_path = Path::new(fixture.source_path);
        let source = match fs::read_to_string(source_path) {
            Ok(contents) => contents,
            Err(error) => {
                setup_failures.push(format!(
                    "{} fixture source should be readable at {}: {error}",
                    fixture.name, fixture.source_path
                ));
                continue;
            }
        };

        let temp_label = format!("{}-red", fixture.name);
        let temp_dir = unique_probe_target_dir(&temp_label);
        let prepare = prepare_dir(&temp_dir);
        if let Err(error) = prepare {
            setup_failures.push(format!(
                "{} target directory should be created before RED compile: {error}",
                fixture.name
            ));
            continue;
        }

        let compile_result = compile_program_for_tests(
            source_path,
            source.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        );

        let cleanup = cleanup_dir(&temp_dir);
        if let Err(error) = cleanup {
            setup_failures.push(format!(
                "{} target directory should be removed after RED compile: {error}",
                fixture.name
            ));
        }

        let fixture_evidence = match compile_result {
            Ok(binary_path) => format!(
                "{} unexpectedly compiled before selected terminal session support landed: {}\ninput plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}",
                fixture.name,
                binary_path.display(),
                fixture.input_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
            Err(error) => format!(
                "{} should compile only after selected typed-event-session support is implemented; current compiler rejection is expected RED evidence.\ninput plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}\ncompiler rejection:\n{error}",
                fixture.name,
                fixture.input_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
        };
        red_evidence.push(fixture_evidence);
    }

    assert!(
        setup_failures.is_empty(),
        "core terminal fixture RED setup should use valid on-disk project layouts:\n{}",
        setup_failures.join("\n\n")
    );

    assert!(
        red_evidence.is_empty(),
        "core terminal fixtures are intentionally RED until selected terminal session support lands:\n{}",
        red_evidence.join("\n\n")
    );
}
