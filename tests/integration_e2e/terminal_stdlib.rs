#![cfg(feature = "integration")]

use super::fs_helpers::unique_probe_target_dir;
use super::*;
use std::path::Path;
use std::time::Duration;

const GENERATED_BINARY_TEST_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn terminal_clear_screen_ansi_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-clear-screen-ansi-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-clear-screen-ansi-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-clear-screen-ansi-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-clear-screen-ansi-bytes source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal-clear-screen-ansi-bytes source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-clear-screen-ansi-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-clear-screen-ansi-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"\x1b[2J\x1b[3J\x1b[H";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-clear-screen-ansi-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-clear-screen-ansi-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-clear-screen-ansi-bytes should compile, run, and write exact ANSI clear bytes: {failure_message}"
    );
}

#[test]
fn terminal_move_cursor_zero_based_ansi_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-move-cursor-zero-based-ansi-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-move-cursor-zero-based-ansi-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path =
            Path::new("test-projects/terminal-move-cursor-zero-based-ansi-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!(
                "terminal-move-cursor-zero-based-ansi-bytes source file should be readable: {error}"
            )
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "terminal-move-cursor-zero-based-ansi-bytes source should compile into a binary: {error}"
            )
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-move-cursor-zero-based-ansi-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-move-cursor-zero-based-ansi-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"\x1b[1;1H";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-move-cursor-zero-based-ansi-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-move-cursor-zero-based-ansi-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-move-cursor-zero-based-ansi-bytes should compile, run, and write exact ANSI cursor bytes: {failure_message}"
    );
}

#[test]
fn terminal_move_cursor_rejects_negative() {
    let temp_dir = unique_probe_target_dir("terminal-move-cursor-rejects-negative");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-move-cursor-rejects-negative target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        for (label, project_name) in [
            ("row=-1", "terminal-move-cursor-rejects-negative-row"),
            ("column=-1", "terminal-move-cursor-rejects-negative-column"),
        ] {
            let source_path =
                Path::new(&format!("test-projects/{project_name}/src/main.op")).to_path_buf();
            let source_str = fs::read_to_string(&source_path).map_err(|error| {
                format!("{project_name} source file should be readable: {error}")
            })?;

            let binary_path = compile_program_for_tests(
                source_path.as_path(),
                source_str.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            )
            .map_err(|error| {
                format!("{project_name} source should compile into a binary: {error}")
            })?;

            let run_output = run_binary_output_with_timeout(
                &binary_path,
                GENERATED_BINARY_TEST_TIMEOUT,
                &format!("{project_name} compiled binary"),
            )?;

            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            let combined = format!("{stdout}\n{stderr}");

            if combined.contains("UNEXPECTED_SUCCESS") {
                return Err(format!(
                    "{project_name} binary unexpectedly succeeded, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
            if !combined.contains("InvalidCursorPositionError") {
                return Err(format!(
                    "{project_name} output should contain InvalidCursorPositionError, status={:?}, stdout='{}', stderr='{}'",
                    run_output.status.code(),
                    stdout,
                    stderr
                ));
            }
            if !combined.contains(label) {
                return Err(format!(
                    "{project_name} output should contain {label}, got stdout='{stdout}', stderr='{stderr}'"
                ));
            }
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-move-cursor-rejects-negative target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-move-cursor-rejects-negative should compile, run, and report InvalidCursorPositionError: {failure_message}"
    );
}

#[test]
fn terminal_task24_data_model_runtime_links_and_runs() {
    let temp_dir = unique_probe_target_dir("terminal-task24-data-model-runtime-links-and-runs");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-task24-data-model-runtime-links-and-runs target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = temp_dir.join("terminal_task24_runtime.op");
        let source = "
import print from standard
import type AllocationFailureError from standard
import terminal_session_options_default, terminal_session_options_with_feature_policy, terminal_session_options_with_resource_limits, terminal_session_options_validate, trusted_terminal_output_from_application_text from 'standard.terminal'
import type TerminalInputSequenceTimeoutMilliseconds, TerminalCommittedTextByteLimit, TerminalCompositionPreeditByteLimit, TerminalPasteChunkByteLimit, TerminalUnknownByteChunkLimit, TerminalPendingSequenceByteLimit, TerminalRetainedEventLimit, TerminalRetainedByteLimit, TerminalCorrelatedEventLimit, TerminalCorrelatedByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticCollectionByteLimit, TerminalMouseTracking, TerminalSessionFeaturePolicy, TerminalSessionOptionsError, TerminalSessionResourceLimits from 'standard.terminal'

##
    Description: Generated runtime smoke test for Task 24 terminal option setters and trust conversion
##
entry main = f(): void errors AllocationFailureError, ConstraintViolationError, TerminalSessionOptionsError =>
    let defaults = terminal_session_options_default()
    let policy = new TerminalSessionFeaturePolicy:
        use_alternate_screen: true
        hide_cursor: false
        enable_bracketed_paste: true
        require_trusted_paste_framing: false
        enable_enhanced_key_identity: true
        enable_focus_events: true
        mouse_tracking: new TerminalMouseTracking.ButtonsAndDrag
        capture_control_keys: true
        require_requested_features: false
    let limits = new TerminalSessionResourceLimits:
        input_sequence_timeout: propagate constrain TerminalInputSequenceTimeoutMilliseconds from 25 as int32
        maximum_committed_text_bytes: propagate constrain TerminalCommittedTextByteLimit from 4096 as int32
        maximum_composition_preedit_bytes: propagate constrain TerminalCompositionPreeditByteLimit from 4096 as int32
        maximum_paste_chunk_bytes: propagate constrain TerminalPasteChunkByteLimit from 4096 as int32
        maximum_unknown_chunk_bytes: propagate constrain TerminalUnknownByteChunkLimit from 1024 as int32
        maximum_pending_sequence_bytes: propagate constrain TerminalPendingSequenceByteLimit from 1024 as int32
        maximum_retained_events: propagate constrain TerminalRetainedEventLimit from 1024 as int32
        maximum_retained_bytes: propagate constrain TerminalRetainedByteLimit from 1048576 as int32
        maximum_correlated_events: propagate constrain TerminalCorrelatedEventLimit from 64 as int32
        maximum_correlated_bytes: propagate constrain TerminalCorrelatedByteLimit from 65536 as int32
        maximum_diagnostics: propagate constrain TerminalDiagnosticCountLimit from 16 as int32
        maximum_diagnostic_bytes: propagate constrain TerminalDiagnosticCollectionByteLimit from 65536 as int32
    let updated_a = propagate terminal_session_options_with_resource_limits(propagate terminal_session_options_with_feature_policy(defaults, policy), limits)
    let updated_b = propagate terminal_session_options_with_feature_policy(propagate terminal_session_options_with_resource_limits(defaults, limits), policy)
    let _checked_a = propagate terminal_session_options_validate(updated_a)
    let _checked_b = propagate terminal_session_options_validate(updated_b)
    let _trusted = propagate trusted_terminal_output_from_application_text('safe terminal output')
    print('TASK24_SETTERS_OK')
    return void
";

        let binary_path = compile_program_for_tests(
            source_path.as_path(),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "terminal-task24-data-model-runtime-links-and-runs source should compile into a binary: {error}"
            )
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-task24-data-model-runtime-links-and-runs compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-task24-data-model-runtime-links-and-runs binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        if run_output.stdout.as_slice() != b"TASK24_SETTERS_OK\n" {
            return Err(format!(
                "terminal-task24-data-model-runtime-links-and-runs stdout should equal TASK24_SETTERS_OK\\n, got {:?}",
                run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-task24-data-model-runtime-links-and-runs target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-task24-data-model-runtime-links-and-runs should compile, link, and run: {failure_message}"
    );
}

#[test]
fn terminal_task24_options_setters_link_and_run() {
    let temp_dir = unique_probe_target_dir("terminal-task24-options-setters-link-and-run");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-task24-options-setters-link-and-run target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = temp_dir.join("terminal_task24_setters.op");
        let source = "
import terminal_session_options_default, terminal_session_options_with_feature_policy, terminal_session_options_with_resource_limits, terminal_session_options_validate, trusted_terminal_output_from_application_text from 'standard.terminal'
import type TerminalMouseTracking, TerminalSessionFeaturePolicy, TerminalSessionOptions, TerminalSessionOptionsError, TerminalSessionResourceLimits from 'standard.terminal'

##
    Description: Generated runtime proof for both Task 24 options setters
##
entry main = f(): void errors AllocationFailureError, ConstraintViolationError, TerminalSessionOptionsError =>
    let defaults: TerminalSessionOptions = terminal_session_options_default()
    let policy: TerminalSessionFeaturePolicy = new TerminalSessionFeaturePolicy:
        use_alternate_screen: true
        hide_cursor: false
        enable_bracketed_paste: true
        require_trusted_paste_framing: false
        enable_enhanced_key_identity: false
        enable_focus_events: false
        mouse_tracking: new TerminalMouseTracking.Disabled
        capture_control_keys: false
        require_requested_features: true
    let limits: TerminalSessionResourceLimits = new TerminalSessionResourceLimits:
        input_sequence_timeout: propagate constrain TerminalInputSequenceTimeoutMilliseconds from 25 as int32
        maximum_committed_text_bytes: propagate constrain TerminalCommittedTextByteLimit from 4096 as int32
        maximum_composition_preedit_bytes: propagate constrain TerminalCompositionPreeditByteLimit from 4096 as int32
        maximum_paste_chunk_bytes: propagate constrain TerminalPasteChunkByteLimit from 4096 as int32
        maximum_unknown_chunk_bytes: propagate constrain TerminalUnknownByteChunkLimit from 1024 as int32
        maximum_pending_sequence_bytes: propagate constrain TerminalPendingSequenceByteLimit from 1024 as int32
        maximum_retained_events: propagate constrain TerminalRetainedEventLimit from 1024 as int32
        maximum_retained_bytes: propagate constrain TerminalRetainedByteLimit from 1048576 as int32
        maximum_correlated_events: propagate constrain TerminalCorrelatedEventLimit from 64 as int32
        maximum_correlated_bytes: propagate constrain TerminalCorrelatedByteLimit from 65536 as int32
        maximum_diagnostics: propagate constrain TerminalDiagnosticCountLimit from 16 as int32
        maximum_diagnostic_bytes: propagate constrain TerminalDiagnosticCollectionByteLimit from 65536 as int32
    let from_policy = propagate terminal_session_options_with_feature_policy(defaults, policy)
    let from_limits = propagate terminal_session_options_with_resource_limits(defaults, limits)
    let combined_a = propagate terminal_session_options_with_feature_policy(from_limits, policy)
    let combined_b = propagate terminal_session_options_with_resource_limits(from_policy, limits)
    let _validated_a = propagate terminal_session_options_validate(combined_a)
    let _validated_b = propagate terminal_session_options_validate(combined_b)
    let _trusted = propagate trusted_terminal_output_from_application_text('task24 setters ok')
    print('TASK24_SETTERS_OK')
    return void
";

        let binary_path = compile_program_for_tests(
            source_path.as_path(),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!(
                "terminal-task24-options-setters-link-and-run source should compile into a binary: {error}"
            )
        })?;
        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-task24-options-setters-link-and-run compiled binary",
        )?;
        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-task24-options-setters-link-and-run binary should exit cleanly but exited with status {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }
        if run_output.stdout.as_slice() != b"TASK24_SETTERS_OK\n" {
            return Err(format!(
                "terminal-task24-options-setters-link-and-run stdout should equal TASK24_SETTERS_OK\\n, got {:?}",
                run_output.stdout,
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-task24-options-setters-link-and-run target directory should be removed"
    );
    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-task24-options-setters-link-and-run should compile, link, and run: {failure_message}"
    );
}

#[test]
fn terminal_task24_validation_failures_execute() {
    let temp_dir = unique_probe_target_dir("terminal-task24-validation-failures-execute");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-task24-validation-failures-execute target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        for (project_name, retained_bytes, correlated_bytes, retained_events, correlated_events) in [
            ("bytes", 4096_i32, 8192_i32, 1024_i32, 64_i32),
            ("events", 0x0010_0000_i32, 0x0001_0000_i32, 8_i32, 64_i32),
        ] {
            let source_path = temp_dir.join(format!("terminal_task24_validate_{project_name}.op"));
            let source = format!(
                "import terminal_session_options_default, terminal_session_options_with_resource_limits, terminal_session_options_validate from 'standard.terminal'\nimport type TerminalSessionOptions, TerminalSessionOptionsError, TerminalSessionResourceLimits from 'standard.terminal'\n\n##\n    Description: Generated runtime proof for one Task 24 validation failure path\n##\nentry main = f(): void errors AllocationFailureError, ConstraintViolationError, TerminalSessionOptionsError =>\n    let defaults: TerminalSessionOptions = terminal_session_options_default()\n    let limits: TerminalSessionResourceLimits = new TerminalSessionResourceLimits:\n        input_sequence_timeout: propagate constrain TerminalInputSequenceTimeoutMilliseconds from 25 as int32\n        maximum_committed_text_bytes: propagate constrain TerminalCommittedTextByteLimit from 4096 as int32\n        maximum_composition_preedit_bytes: propagate constrain TerminalCompositionPreeditByteLimit from 4096 as int32\n        maximum_paste_chunk_bytes: propagate constrain TerminalPasteChunkByteLimit from 4096 as int32\n        maximum_unknown_chunk_bytes: propagate constrain TerminalUnknownByteChunkLimit from 1024 as int32\n        maximum_pending_sequence_bytes: propagate constrain TerminalPendingSequenceByteLimit from 1024 as int32\n        maximum_retained_events: propagate constrain TerminalRetainedEventLimit from {retained_events} as int32\n        maximum_retained_bytes: propagate constrain TerminalRetainedByteLimit from {retained_bytes} as int32\n        maximum_correlated_events: propagate constrain TerminalCorrelatedEventLimit from {correlated_events} as int32\n        maximum_correlated_bytes: propagate constrain TerminalCorrelatedByteLimit from {correlated_bytes} as int32\n        maximum_diagnostics: propagate constrain TerminalDiagnosticCountLimit from 16 as int32\n        maximum_diagnostic_bytes: propagate constrain TerminalDiagnosticCollectionByteLimit from 65536 as int32\n    let invalid = propagate terminal_session_options_with_resource_limits(defaults, limits)\n    let _validated = propagate terminal_session_options_validate(invalid)\n    print('UNEXPECTED_SUCCESS')\n    return void\n"
            );

            let binary_path = compile_program_for_tests(
                source_path.as_path(),
                source.as_str(),
                &temp_dir,
                &TargetTriple::host(),
            )
            .map_err(|error| {
                format!(
                    "terminal-task24-validation-failures-execute {project_name} source should compile into a binary: {error}"
                )
            })?;
            let run_output = run_binary_output_with_timeout(
                &binary_path,
                GENERATED_BINARY_TEST_TIMEOUT,
                &format!(
                    "terminal-task24-validation-failures-execute {project_name} compiled binary"
                ),
            )?;
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            let combined = format!("{stdout}\n{stderr}");
            if combined.contains("UNEXPECTED_SUCCESS") {
                return Err(format!(
                    "terminal-task24-validation-failures-execute {project_name} unexpectedly succeeded, stdout={stdout:?}, stderr={stderr:?}"
                ));
            }
            if !combined.contains("TerminalSessionOptionsError.InvalidOptions") {
                return Err(format!(
                    "terminal-task24-validation-failures-execute {project_name} should surface TerminalSessionOptionsError.InvalidOptions, got stdout={stdout:?}, stderr={stderr:?}"
                ));
            }
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-task24-validation-failures-execute target directory should be removed"
    );
    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-task24-validation-failures-execute should compile, link, and run: {failure_message}"
    );
}

#[test]
fn terminal_task24_unimplemented_runtime_api_remains_codegen_gated() {
    let temp_dir =
        unique_probe_target_dir("terminal-task24-unimplemented-runtime-api-remains-gated");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-task24-unimplemented-runtime-api-remains-gated target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = temp_dir.join("terminal_task24_gate.op");
        let source = "
import terminal_session_open_sync, terminal_session_options_default from 'standard.terminal'
import type TerminalSessionOpenError from 'standard.terminal'

##
    Description: Generated compile failure proving unimplemented lifecycle API stays gated
##
entry main = f(): void errors TerminalSessionOpenError =>
    let session = propagate terminal_session_open_sync(terminal_session_options_default())
    return void
";

        let error = compile_program_for_tests(
            source_path.as_path(),
            source,
            &temp_dir,
            &TargetTriple::host(),
        )
        .expect_err("unimplemented Task 25+ lifecycle API must remain codegen gated");
        let rendered = error.to_string();
        if !rendered.contains("runtime lowering")
            || !rendered.contains("terminal_session_open_sync")
        {
            return Err(format!(
                "expected runtime-readiness diagnostic mentioning terminal_session_open_sync, got: {rendered}"
            ));
        }
        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-task24-unimplemented-runtime-api-remains-gated target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-task24-unimplemented-runtime-api-remains-gated should preserve the gate: {failure_message}"
    );
}

#[test]
fn terminal_draw_rows_bytes() {
    let temp_dir = unique_probe_target_dir("terminal-draw-rows-bytes");
    let prepare = prepare_dir(&temp_dir);
    assert!(
        prepare.is_ok(),
        "terminal-draw-rows-bytes target directory should be created"
    );

    let execution_result: Result<(), String> = (|| {
        let source_path = Path::new("test-projects/terminal-draw-rows-bytes/src/main.op");
        let source_str = fs::read_to_string(source_path).map_err(|error| {
            format!("terminal-draw-rows-bytes source file should be readable: {error}")
        })?;

        let binary_path = compile_program_for_tests(
            source_path,
            source_str.as_str(),
            &temp_dir,
            &TargetTriple::host(),
        )
        .map_err(|error| {
            format!("terminal-draw-rows-bytes source should compile into a binary: {error}")
        })?;

        let run_output = run_binary_output_with_timeout(
            &binary_path,
            GENERATED_BINARY_TEST_TIMEOUT,
            "terminal-draw-rows-bytes compiled binary",
        )?;

        if !run_output.status.success() {
            let stdout = String::from_utf8_lossy(&run_output.stdout);
            let stderr = String::from_utf8_lossy(&run_output.stderr);
            return Err(format!(
                "terminal-draw-rows-bytes binary should exit cleanly but exited with status \
                 {status:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
                status = run_output.status.code(),
            ));
        }

        let expected = b"##\n..";
        if run_output.stdout.as_slice() != expected {
            return Err(format!(
                "terminal-draw-rows-bytes stdout should equal {:?}, got {:?}",
                expected, run_output.stdout,
            ));
        }

        Ok(())
    })();

    let cleanup = cleanup_dir(&temp_dir);
    assert!(
        cleanup.is_ok(),
        "terminal-draw-rows-bytes target directory should be removed"
    );

    let failure_message = match execution_result {
        Ok(()) => String::new(),
        Err(message) => message,
    };
    assert!(
        failure_message.is_empty(),
        "terminal-draw-rows-bytes should compile, run, and write exact row bytes: {failure_message}"
    );
}
