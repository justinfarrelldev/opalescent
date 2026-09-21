#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

const GENERATED_TERMINAL_FIXTURE_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
struct TerminalSessionFixtureMetadata {
    name: &'static str,
    opal_toml_path: &'static str,
    source_path: &'static str,
    input_plan: &'static str,
    fault_plan: &'static str,
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
        fault_plan: "none; deterministic typed-event-session scenario uses the selected public proposal surface",
        expected_stdout_summary: "KEY_LOG_SUMMARY keys=2 last_key=named+control timeouts=1 cancellations=1 termination=cancelled",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-text-echo-safe",
        opal_toml_path: "test-projects/terminal-text-echo-safe/opal.toml",
        source_path: "test-projects/terminal-text-echo-safe/src/main.op",
        input_plan: "TextInput(complete safe text), Paste(discard), UnknownBytes(discard), InputReset(PasteFallback), Key(non-text), TimedOut, EndOfInput",
        fault_plan: "none; deterministic typed-event-session scenario uses the selected public proposal surface",
        expected_stdout_summary: "committed text\nSAFE_ECHO_SUMMARY text=1 paste=1 unknown=1 resets=1 non_text=2 raw_session_writes=0",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-size-probe",
        opal_toml_path: "test-projects/terminal-size-probe/opal.toml",
        source_path: "test-projects/terminal-size-probe/src/main.op",
        input_plan: "Open with deterministic 80x24 size and fixed capability snapshot; no input events",
        fault_plan: "none; deterministic typed-event-session scenario uses the selected public proposal surface",
        expected_stdout_summary: "SIZE_PROBE_SUMMARY columns=80 rows=24 ordinary_capability=queried trusted_paste=queried",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-pause-counter",
        opal_toml_path: "test-projects/terminal-pause-counter/opal.toml",
        source_path: "test-projects/terminal-pause-counter/src/main.op",
        input_plan: "Pre-pause Key, TextInput; pause delivery UnknownBytes, InputReset(PauseBoundary)",
        fault_plan: "none; deterministic typed-event-session scenario uses the selected public proposal surface",
        expected_stdout_summary: "PAUSE_COUNTER_SUMMARY pre_pause_reads=2 pause_events=2 input_resets=1 final_boundary=InputReset.PauseBoundary observed=1",
        expected_stderr: "",
        expected_status: 0,
    },
];

const COORDINATION_AND_DIAGNOSTIC_TERMINAL_SESSION_FIXTURES: &[TerminalSessionFixtureMetadata] = &[
    TerminalSessionFixtureMetadata {
        name: "terminal-legacy-io-rejection",
        opal_toml_path: "test-projects/terminal-legacy-io-rejection/opal.toml",
        source_path: "test-projects/terminal-legacy-io-rejection/src/main.op",
        input_plan: "Acquire stale StdoutWriter/StdoutTerminal leases before opening; while Active call take_input, stdout_writer, stdout_terminal, terminal_supports_ansi, writer_write_sync, writer_flush_sync, terminal_clear_screen_on_sync",
        fault_plan: "coordinator Active rejects legacy reads, lease creation, stale capability inspection, stale write, stale flush, and stale screen mutation before consumption or mutation; diagnostic lane marker attempted",
        expected_stdout_summary: "LEGACY_IO_SUMMARY rejections=7 unexpected_successes=0 stale_writer=true stale_terminal=true diagnostic_lane=attempted mutations=0",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-timeout-menu",
        opal_toml_path: "test-projects/terminal-timeout-menu/opal.toml",
        source_path: "test-projects/terminal-timeout-menu/src/main.op",
        input_plan: "Render three-option menu, then TerminalWait.For(75 ms) receives TimedOut; EndOfInput remains unconsumed",
        fault_plan: "deterministic TimedOut terminal event selects default option 2 without using real wall-clock duration",
        expected_stdout_summary: "TIMEOUT_MENU_SUMMARY displayed_options=3 wait_ms=75 timed_out=true selected=2 reason=default status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-cancel-demo",
        opal_toml_path: "test-projects/terminal-cancel-demo/opal.toml",
        source_path: "test-projects/terminal-cancel-demo/src/main.op",
        input_plan: "Request cancellation before the first read while backend has queued input; terminal_session_read_event_sync returns Cancelled",
        fault_plan: "sticky cancellation wins over newly ready input and the fixture performs no post-cancel read",
        expected_stdout_summary: "CANCEL_DEMO_SUMMARY read_attempts=1 cancellations=1 consumed_after_cancel=0 sticky=true cleanup=closed",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-diagnostics-inspector",
        opal_toml_path: "test-projects/terminal-diagnostics-inspector/opal.toml",
        source_path: "test-projects/terminal-diagnostics-inspector/src/main.op",
        input_plan: "No terminal input events; fixture emits generated diagnostic declarations/status coverage only",
        fault_plan: "runtime TerminalDiagnostic object construction remains outside this production fixture; focused codegen tests cover declaration readiness",
        expected_stdout_summary: "DIAGNOSTICS_INSPECTOR_SUMMARY invalid_options=1 single_diagnostics=1 collections=1 formatted=2 authority=declaration_status session_opened=false",
        expected_stderr: "",
        expected_status: 0,
    },
];

const REMAINING_INTERACTIVE_TERMINAL_SESSION_FIXTURES: &[TerminalSessionFixtureMetadata] = &[
    TerminalSessionFixtureMetadata {
        name: "terminal-chord-quit",
        opal_toml_path: "test-projects/terminal-chord-quit/opal.toml",
        source_path: "test-projects/terminal-chord-quit/src/main.op",
        input_plan: "TextInput(draft), TimedOut(expire), Key(Control Ctrl-Q), then unconsumed Key(Named Escape) and EndOfInput",
        fault_plan: "Ctrl-Q and Escape bindings registered; exactly one Ctrl-Q activation, one released non-command input, and non-activating expiration evidence",
        expected_stdout_summary: "CHORD_QUIT_SUMMARY ctrl_q_binding=1 escape_binding=2 activations=1 released_non_command=1 expirations=1 termination=ctrl-q status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-paste-quarantine",
        opal_toml_path: "test-projects/terminal-paste-quarantine/opal.toml",
        source_path: "test-projects/terminal-paste-quarantine/src/main.op",
        input_plan: "TextInput(direct), Paste(trusted complete), UnknownBytes(PasteInvalidUtf8), UnknownBytes(UnrecognizedSequence), InputReset(PasteFallback), EndOfInput",
        fault_plan: "malformed trusted paste fallback emits bounded UnknownBytes before exactly one PasteFallback reset; unknown bytes remain quarantined and non-command",
        expected_stdout_summary: "PASTE_QUARANTINE_SUMMARY direct_text=1 trusted_paste=1 quarantined_chunks=2 malformed_chunks=1 unknown_chunks=1 paste_fallback_resets=1 command_activations=0 status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-game-of-life-interactive",
        opal_toml_path: "test-projects/terminal-game-of-life-interactive/opal.toml",
        source_path: "test-projects/terminal-game-of-life-interactive/src/main.op",
        input_plan: "Key(Text space pause), Key(Text +), Key(Text -), Key(Text space resume), Key(Text r reseed), Key(Text q quit)",
        fault_plan: "deterministic 3x3 blinker boards, bounded TerminalWait.For drains, no randomness, and no wall-clock duration assertion",
        expected_stdout_summary: "LIFE_INTERACTIVE_SUMMARY generation=1 frames=6 paused=0 speed_ms=40 pauses=1 resumes=1 speed_ups=1 speed_downs=1 reseeds=1 quit=1 live_cells=3 status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-sokoban-mini",
        opal_toml_path: "test-projects/terminal-sokoban-mini/opal.toml",
        source_path: "test-projects/terminal-sokoban-mini/src/main.op",
        input_plan: "Key(Named ArrowRight), Key(Named ArrowRight illegal push), Key(Text r reset), Key(Text q quit), EndOfInput",
        fault_plan: "fixed 5x4 board rejects wall pushes, reset restores starting coordinates, and every accepted input redraws and flushes once",
        expected_stdout_summary: "SOKOBAN_MINI_SUMMARY moves=1 pushes=1 illegal_moves=1 resets=1 redraws=4 player=1,1 box=2,1 solved=0 quit=1 status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-file-picker",
        opal_toml_path: "test-projects/terminal-file-picker/opal.toml",
        source_path: "test-projects/terminal-file-picker/src/main.op",
        input_plan: "Fixed manifest [fixtures/alpha.txt, fixtures/beta.txt, fixtures/gamma.txt], ArrowDown, ArrowDown, ArrowUp, Enter",
        fault_plan: "in-source manifest defines ordering; no host directory listing order or sorting API participates in selection",
        expected_stdout_summary: "FILE_PICKER_SUMMARY manifest_count=3 navigations=3 selected_index=1 selected=fixtures/beta.txt cancelled=false status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-stopwatch-pomodoro",
        opal_toml_path: "test-projects/terminal-stopwatch-pomodoro/opal.toml",
        source_path: "test-projects/terminal-stopwatch-pomodoro/src/main.op",
        input_plan: "Key(Text s), TimedOut, TimedOut, Key(Text p), TimedOut(stale), Key(Text r), Key(Text s), TimedOut, Key(Text q)",
        fault_plan: "scripted TimedOut events are the only ticks; stopped timer wake is stale and no wall-clock duration or long Pomodoro interval is asserted",
        expected_stdout_summary: "STOPWATCH_POMODORO_SUMMARY elapsed_ticks=1 starts=2 stops=1 resets=1 stale_wakes=1 running=1 quit=1 status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
];

fn all_terminal_session_fixtures() -> Vec<TerminalSessionFixtureMetadata> {
    CORE_TERMINAL_SESSION_FIXTURES
        .iter()
        .chain(COORDINATION_AND_DIAGNOSTIC_TERMINAL_SESSION_FIXTURES.iter())
        .chain(REMAINING_INTERACTIVE_TERMINAL_SESSION_FIXTURES.iter())
        .copied()
        .collect()
}

fn assert_fixture_group_active(fixtures: &[TerminalSessionFixtureMetadata]) {
    let mut failures = Vec::new();
    for fixture in fixtures {
        let source_path = Path::new(fixture.source_path);
        if !Path::new(fixture.opal_toml_path).is_file() {
            failures.push(format!("{} is missing opal.toml", fixture.name));
            continue;
        }
        let source = match fs::read_to_string(source_path) {
            Ok(source) => source,
            Err(error) => {
                failures.push(format!("{} source is unreadable: {error}", fixture.name));
                continue;
            }
        };
        let summary_marker = fixture
            .expected_stdout_summary
            .split_whitespace()
            .find(|part| part.ends_with("_SUMMARY"))
            .expect("fixture summaries include a stable summary marker");
        if !source.contains(summary_marker) {
            failures.push(format!(
                "{} source does not contain final summary marker {summary_marker}",
                fixture.name
            ));
        }
        if source.contains("terminal_session_output_terminal")
            || source.contains("AcquireOutputTerminal")
            || source.contains("terminal_input_packet")
            || source.contains("terminal_event_batch")
        {
            failures.push(format!(
                "{} source contains forbidden historical/session-output API",
                fixture.name
            ));
        }
        if fixture.name == "terminal-diagnostics-inspector"
            && !source.contains("generated diagnostic declarations/status coverage")
        {
            failures.push(String::from(
                "terminal-diagnostics-inspector must state that it is generated diagnostic declarations/status coverage, not runtime diagnostic-object inspection",
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "terminal session fixtures should be active proposal fixtures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn terminal_session_core_fixtures_1_8_are_active() {
    assert_fixture_group_active(CORE_TERMINAL_SESSION_FIXTURES);
    assert_fixture_group_active(COORDINATION_AND_DIAGNOSTIC_TERMINAL_SESSION_FIXTURES);
}

#[test]
fn terminal_session_interactive_fixtures_9_14_are_active() {
    assert_fixture_group_active(REMAINING_INTERACTIVE_TERMINAL_SESSION_FIXTURES);
}

#[test]
fn terminal_session_all_fixture_summaries_are_unique_and_deterministic() {
    let fixtures = all_terminal_session_fixtures();
    assert_eq!(
        fixtures.len(),
        14,
        "all required terminal fixtures are listed"
    );
    let mut summaries = fixtures
        .iter()
        .map(|fixture| fixture.expected_stdout_summary)
        .collect::<Vec<_>>();
    summaries.sort_unstable();
    summaries.dedup();
    assert_eq!(summaries.len(), 14, "fixture summaries must be unique");
    for fixture in fixtures {
        assert_eq!(
            fixture.expected_status, 0_i32,
            "{} exits successfully",
            fixture.name
        );
        assert_eq!(
            fixture.expected_stderr, "",
            "{} has deterministic empty stderr",
            fixture.name
        );
        assert!(
            !fixture.input_plan.is_empty() && !fixture.fault_plan.is_empty(),
            "{} documents deterministic input and fault plans",
            fixture.name
        );
    }
}

#[test]
fn terminal_session_fixture_sources_avoid_nondeterministic_dependencies() {
    let forbidden_needles = [
        "random(",
        "sleep(",
        "read_directory",
        "list_directory",
        "host_directory",
        "callback",
        "editor_buffer",
        "Date.now",
    ];
    let mut failures = Vec::new();
    for fixture in all_terminal_session_fixtures() {
        let source = fs::read_to_string(fixture.source_path)
            .expect("terminal fixture source should be readable");
        for needle in forbidden_needles {
            if source.contains(needle) {
                failures.push(format!(
                    "{} contains nondeterministic dependency {needle}",
                    fixture.name
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "terminal fixtures should be deterministic:\n{}",
        failures.join("\n")
    );
}

fn fixture_fake_events(name: &str) -> &'static str {
    match name {
        "terminal-text-echo-safe" => {
            "text:committed|paste:trusted|unknown:PasteInvalidUtf8|reset:PasteFallback|key:Escape|timeout|eof"
        }
        "terminal-timeout-menu" => "timeout|eof",
        "terminal-cancel-demo" => "text:x|eof",
        "terminal-paste-quarantine" => {
            "text:typed|paste:trusted|unknown:PasteInvalidUtf8|unknown:UnrecognizedSequence|reset:PasteFallback|eof"
        }
        "terminal-chord-quit" => "text:draft|key:Control:17+control|key:Escape|eof",
        "terminal-game-of-life-interactive" => {
            "key:Text:space|key:Text:+|key:Text:-|key:Text:space|key:Text:r|key:Text:q"
        }
        "terminal-sokoban-mini" => "key:ArrowRight|key:ArrowRight|key:Escape|key:Escape|eof",
        "terminal-stopwatch-pomodoro" => {
            "key:Text:s|timeout|timeout|key:Text:p|timeout|key:Text:r|key:Text:s|timeout|key:Text:q"
        }
        _ => "",
    }
}

#[test]
fn terminal_session_all_fixtures_compile_and_run_with_fake_backend() {
    let mut failures = Vec::new();
    for fixture in all_terminal_session_fixtures() {
        let temp_dir = unique_probe_target_dir(&format!("{}-active-run", fixture.name));
        if let Err(error) = prepare_dir(&temp_dir) {
            failures.push(format!("{} target setup failed: {error}", fixture.name));
            continue;
        }
        let result: Result<(), String> = (|| {
            let source_path = Path::new(fixture.source_path);
            let source = fs::read_to_string(source_path)
                .map_err(|error| format!("{} source read failed: {error}", fixture.name))?;
            let binary_path = compile_program_for_tests(
                source_path,
                source.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            )
            .map_err(|error| format!("{} compile failed: {error}", fixture.name))?;
            let child = Command::new(&binary_path)
                .env("OPAL_TERMINAL_FAKE_BACKEND", "1")
                .env(
                    "OPAL_TERMINAL_FAKE_EVENTS",
                    fixture_fake_events(fixture.name),
                )
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|error| format!("{} run spawn failed: {error}", fixture.name))?;
            let output = fs_helpers::wait_for_child_output_with_timeout(
                child,
                GENERATED_TERMINAL_FIXTURE_TIMEOUT,
                fixture.name,
            )?;
            if output.status.code().unwrap_or(-1_i32) != fixture.expected_status {
                return Err(format!(
                    "{} status mismatch {:?}\nstdout:\n{}\nstderr:\n{}",
                    fixture.name,
                    output.status.code(),
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let stdout = String::from_utf8_lossy(&output.stdout);
            if !stdout.contains(fixture.expected_stdout_summary) {
                return Err(format!(
                    "{} stdout did not contain expected summary {:?}; got {:?}",
                    fixture.name, fixture.expected_stdout_summary, stdout
                ));
            }
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.trim() != fixture.expected_stderr {
                return Err(format!(
                    "{} stderr mismatch expected {:?}, got {:?}",
                    fixture.name, fixture.expected_stderr, stderr
                ));
            }
            Ok(())
        })();
        if let Err(error) = cleanup_dir(&temp_dir) {
            failures.push(format!("{} cleanup failed: {error}", fixture.name));
        }
        if let Err(error) = result {
            failures.push(error);
        }
    }
    assert!(
        failures.is_empty(),
        "terminal fixtures should compile/run deterministically:\n{}",
        failures.join("\n\n")
    );
}
