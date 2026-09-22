use crate::runtime::terminal::{
    TerminalCursorShape, TerminalInputEventKind, TerminalSessionState, TerminalWriteOperationError,
};
use crate::stdlib::terminal::{
    safe_terminal_diagnostic_format, terminal_session_bell_sync,
    terminal_session_clear_screen_sync, terminal_session_draw_rows_sync,
    terminal_session_flush_sync, terminal_session_move_cursor_sync, terminal_session_open_sync,
    terminal_session_options_default, terminal_session_set_cursor_shape_sync,
    terminal_session_set_cursor_visible_sync, terminal_session_write_diagnostic_sync,
    terminal_session_write_sync, trusted_terminal_output_from_application_text,
};

#[test]
fn terminal_session_output_accepts_only_trusted_or_safe_values_and_preserves_text() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    let trusted = trusted_terminal_output_from_application_text("hello");
    terminal_session_write_sync(&mut session, &trusted).expect("trusted write succeeds");
    assert_eq!(session.output_log_for_tests(), &["hello".to_owned()]);

    let diagnostic = crate::runtime::terminal::TerminalDiagnostic::new_runtime(
        crate::runtime::terminal::TerminalBackend::UnsupportedPlatform,
        crate::runtime::terminal::TerminalOperation::Write,
        crate::runtime::terminal::TerminalDiagnosticStage::Snapshot,
        crate::runtime::terminal::TerminalCoordinatorState::Free,
        crate::runtime::terminal::TerminalDiagnosticSessionState::Active,
        crate::runtime::terminal::TerminalOsCode::Unavailable,
        crate::runtime::terminal::TerminalDiagnosticDetail::new_runtime("bad\u{1b}[31m")
            .expect("detail valid"),
        crate::runtime::terminal::TerminalDiagnosticRetryability::NonRetryable,
        false,
    );
    let safe = safe_terminal_diagnostic_format(&diagnostic);
    assert!(!safe.as_str().contains('\u{1b}'));
    terminal_session_write_diagnostic_sync(&mut session, &safe)
        .expect("safe diagnostic write succeeds");
    assert_eq!(session.output_log_for_tests().len(), 2);
}

#[test]
fn terminal_session_rendering_helpers_record_trusted_sequences() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    let rows = [
        trusted_terminal_output_from_application_text("alpha"),
        trusted_terminal_output_from_application_text("beta"),
    ];

    terminal_session_clear_screen_sync(&mut session).expect("clear succeeds");
    terminal_session_move_cursor_sync(&mut session, 2, 3).expect("move succeeds");
    terminal_session_draw_rows_sync(&mut session, &rows).expect("draw succeeds");
    terminal_session_bell_sync(&mut session).expect("bell succeeds");

    assert_eq!(
        session.output_log_for_tests(),
        &[
            "\u{1b}[2J\u{1b}[3J\u{1b}[H".to_owned(),
            "\u{1b}[2;3H".to_owned(),
            "alpha\r\n".to_owned(),
            "beta\r\n".to_owned(),
            "\u{7}".to_owned(),
        ]
    );
    let invalid_cursor = terminal_session_move_cursor_sync(&mut session, 0, 1)
        .expect_err("invalid cursor position should reject");
    assert!(matches!(
        invalid_cursor,
        TerminalWriteOperationError::InvalidCursorPosition { .. }
    ));
}

#[test]
fn terminal_session_output_operations_reject_paused_before_mutation() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open succeeds");
    session
        .enqueue_input_event_for_tests(TerminalInputEventKind::FocusGained)
        .expect("queue event");
    session.pause_sync().expect("pause succeeds");
    let trusted = trusted_terminal_output_from_application_text("blocked");
    let error =
        terminal_session_write_sync(&mut session, &trusted).expect_err("paused write rejects");
    assert_eq!(error.state(), TerminalSessionState::Paused);
    assert!(session.output_log_for_tests().is_empty());
    assert!(terminal_session_flush_sync(&session).is_err());
    assert!(terminal_session_set_cursor_visible_sync(&session, true).is_err());
    assert!(
        terminal_session_set_cursor_shape_sync(&session, TerminalCursorShape::BlinkingBlock)
            .is_err()
    );
}
