#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;

const TERMINAL_SESSION_RED_ENV: &str = "OPAL_TERMINAL_SESSION_RED";

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
