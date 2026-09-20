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
        fault_plan: "none; selected typed-event-session APIs are expected to be missing until implementation",
        expected_stdout_summary: "KEY_LOG_SUMMARY keys=2 last_key=named+control timeouts=1 cancellations=1 termination=cancelled",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-text-echo-safe",
        opal_toml_path: "test-projects/terminal-text-echo-safe/opal.toml",
        source_path: "test-projects/terminal-text-echo-safe/src/main.op",
        input_plan: "TextInput(complete safe text), Paste(discard), UnknownBytes(discard), InputReset(PasteFallback), Key(non-text), TimedOut, EndOfInput",
        fault_plan: "none; selected typed-event-session APIs are expected to be missing until implementation",
        expected_stdout_summary: "<committed text>\nSAFE_ECHO_SUMMARY text=1 paste=1 unknown=1 resets=1 non_text=2 raw_session_writes=0",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-size-probe",
        opal_toml_path: "test-projects/terminal-size-probe/opal.toml",
        source_path: "test-projects/terminal-size-probe/src/main.op",
        input_plan: "Open with deterministic 80x24 size and fixed capability snapshot; no input events",
        fault_plan: "none; selected typed-event-session APIs are expected to be missing until implementation",
        expected_stdout_summary: "SIZE_PROBE_SUMMARY columns=80 rows=24 ordinary_capability=queried trusted_paste=queried",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-pause-counter",
        opal_toml_path: "test-projects/terminal-pause-counter/opal.toml",
        source_path: "test-projects/terminal-pause-counter/src/main.op",
        input_plan: "Pre-pause Key, TextInput; pause delivery UnknownBytes, InputReset(PauseBoundary)",
        fault_plan: "none; selected typed-event-session APIs are expected to be missing until implementation",
        expected_stdout_summary: "PAUSE_COUNTER_SUMMARY pre_pause_reads=2 pause_events=2 input_resets=1 final_boundary=InputReset.PauseBoundary observed=true",
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
        input_plan: "No terminal input events; fixture only validates options and opens sessions against deterministic diagnostic faults",
        fault_plan: "options validation emits InvalidOptions, strict feature open emits UnsupportedFeature diagnostic, and default open emits RollbackFailed diagnostics collection",
        expected_stdout_summary: "DIAGNOSTICS_INSPECTOR_SUMMARY invalid_options=1 single_diagnostics=1 collections=1 formatted=2 authority=structured_fields session_opened=false",
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
        expected_stdout_summary: "LIFE_INTERACTIVE_SUMMARY generation=1 frames=6 paused=false speed_ms=40 pauses=1 resumes=1 speed_ups=1 speed_downs=1 reseeds=1 quit=true live_cells=3 status=ok",
        expected_stderr: "",
        expected_status: 0,
    },
    TerminalSessionFixtureMetadata {
        name: "terminal-sokoban-mini",
        opal_toml_path: "test-projects/terminal-sokoban-mini/opal.toml",
        source_path: "test-projects/terminal-sokoban-mini/src/main.op",
        input_plan: "Key(Named ArrowRight), Key(Named ArrowRight illegal push), Key(Text r reset), Key(Text q quit), EndOfInput",
        fault_plan: "fixed 5x4 board rejects wall pushes, reset restores starting coordinates, and every accepted input redraws and flushes once",
        expected_stdout_summary: "SOKOBAN_MINI_SUMMARY moves=1 pushes=1 illegal_moves=1 resets=1 redraws=4 player=1,1 box=2,1 solved=false quit=true status=ok",
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
        expected_stdout_summary: "STOPWATCH_POMODORO_SUMMARY elapsed_ticks=1 starts=2 stops=1 resets=1 stale_wakes=1 running=true quit=true status=ok",
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
                "{} unexpectedly compiled before selected terminal session support landed: {}\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}",
                fixture.name,
                binary_path.display(),
                fixture.input_plan,
                fixture.fault_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
            Err(error) => format!(
                "{} should compile only after selected typed-event-session support is implemented; current compiler rejection is expected RED evidence.\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}\ncompiler rejection:\n{error}",
                fixture.name,
                fixture.input_plan,
                fixture.fault_plan,
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

#[test]
#[ignore = "RED test: opt-in coordination and diagnostic terminal fixtures via --ignored and OPAL_TERMINAL_SESSION_RED=1"]
fn terminal_session_input_gated_coordination_and_diagnostics_fixtures_red() {
    if !should_run_terminal_session_red() {
        eprintln!(
            "skipping terminal_session_input_gated_coordination_and_diagnostics_fixtures_red: {TERMINAL_SESSION_RED_ENV} != 1"
        );
        return;
    }

    let mut setup_failures: Vec<String> = Vec::new();
    let mut red_evidence: Vec<String> = Vec::new();

    for fixture in COORDINATION_AND_DIAGNOSTIC_TERMINAL_SESSION_FIXTURES {
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
                "{} unexpectedly compiled before selected terminal session support landed: {}\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}",
                fixture.name,
                binary_path.display(),
                fixture.input_plan,
                fixture.fault_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
            Err(error) => format!(
                "{} should compile only after selected typed-event-session support is implemented; current compiler rejection is expected RED evidence.\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}\ncompiler rejection:\n{error}",
                fixture.name,
                fixture.input_plan,
                fixture.fault_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
        };
        red_evidence.push(fixture_evidence);
    }

    assert!(
        setup_failures.is_empty(),
        "coordination and diagnostic terminal fixture RED setup should use valid on-disk project layouts:\n{}",
        setup_failures.join("\n\n")
    );

    assert!(
        red_evidence.is_empty(),
        "coordination and diagnostic terminal fixtures are intentionally RED until selected terminal session support lands:\n{}",
        red_evidence.join("\n\n")
    );
}

#[test]
#[ignore = "RED test: opt-in remaining interactive terminal fixtures via --ignored and OPAL_TERMINAL_SESSION_RED=1"]
fn terminal_session_input_gated_remaining_interactive_fixtures_red() {
    if !should_run_terminal_session_red() {
        eprintln!(
            "skipping terminal_session_input_gated_remaining_interactive_fixtures_red: {TERMINAL_SESSION_RED_ENV} != 1"
        );
        return;
    }

    let mut setup_failures: Vec<String> = Vec::new();
    let mut red_evidence: Vec<String> = Vec::new();

    for fixture in REMAINING_INTERACTIVE_TERMINAL_SESSION_FIXTURES {
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
                "{} unexpectedly compiled before selected terminal session support landed: {}\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}",
                fixture.name,
                binary_path.display(),
                fixture.input_plan,
                fixture.fault_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
            Err(error) => format!(
                "{} should compile only after selected typed-event-session and chord/test support is implemented; current compiler rejection is expected RED evidence.\ninput plan: {}\nfault plan: {}\nexpected stdout summary: {}\nexpected stderr: {:?}\nexpected status: {}\ncompiler rejection:\n{error}",
                fixture.name,
                fixture.input_plan,
                fixture.fault_plan,
                fixture.expected_stdout_summary,
                fixture.expected_stderr,
                fixture.expected_status,
            ),
        };
        red_evidence.push(fixture_evidence);
    }

    assert!(
        setup_failures.is_empty(),
        "remaining interactive terminal fixture RED setup should use valid on-disk project layouts:\n{}",
        setup_failures.join("\n\n")
    );

    assert!(
        red_evidence.is_empty(),
        "remaining interactive terminal fixtures are intentionally RED until selected terminal session support lands:\n{}",
        red_evidence.join("\n\n")
    );
}
