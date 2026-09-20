use crate::runtime::terminal::{
    TerminalInputEventKind, TerminalNativeEventKind, TerminalOperation, WindowsConsoleSnapshot,
    WindowsTerminalBackendRuntime, WindowsTerminalBackendState, WindowsTerminalInverseStep,
};

#[test]
fn windows_console_backend_restores_modes_cursor_and_buffers_in_order() {
    let snapshot = WindowsConsoleSnapshot {
        input_mode: 0x0001,
        output_mode: 0x0002,
        cursor_visible: true,
        cursor_shape: 7,
        active_buffer: 11,
        alternate_buffer: None,
        quick_edit_enabled: true,
    };
    let mut backend = WindowsTerminalBackendRuntime::console(snapshot);
    backend.acquire(true);

    assert_eq!(backend.state(), WindowsTerminalBackendState::Active);
    assert_eq!(backend.current_snapshot().active_buffer, 2);
    assert_eq!(backend.current_snapshot().alternate_buffer, Some(2));
    assert!(!backend.current_snapshot().quick_edit_enabled);
    assert!(!backend.current_snapshot().cursor_visible);

    backend.restore().expect("restore succeeds");
    assert_eq!(backend.state(), WindowsTerminalBackendState::Free);
    assert_eq!(backend.current_snapshot(), snapshot);
}

#[test]
fn windows_invalid_native_input_is_quarantined_without_replacement_text() {
    let snapshot = WindowsConsoleSnapshot::default();
    let mut backend = WindowsTerminalBackendRuntime::console(snapshot);
    backend.acquire(false);
    backend.push_unpaired_surrogate();
    backend.push_unsupported_native_record("MENU_EVENT");

    assert!(matches!(
        backend.poll_event(),
        TerminalInputEventKind::UnknownNative { .. }
    ));
    assert!(matches!(
        backend.poll_event(),
        TerminalInputEventKind::UnknownBytes { .. }
    ));
    let unsupported = backend.poll_event();
    assert!(matches!(
        unsupported,
        TerminalInputEventKind::UnknownNative {
            metadata: crate::runtime::terminal::TerminalNativeMetadata {
                kind: TerminalNativeEventKind::Other { .. },
                ..
            }
        }
    ));
}

#[test]
fn windows_restoration_failure_enters_retryable_restore_pending() {
    let snapshot = WindowsConsoleSnapshot::default();
    let mut backend = WindowsTerminalBackendRuntime::conpty(snapshot);
    backend.acquire(true);
    backend.fail_next_inverse(WindowsTerminalInverseStep::ActiveBuffer);

    let diagnostic = backend
        .restore()
        .expect_err("active-buffer restore fails once");
    assert_eq!(backend.state(), WindowsTerminalBackendState::RestorePending);
    assert_eq!(
        diagnostic.operation(),
        TerminalOperation::RestorePendingClose
    );
    assert_eq!(backend.diagnostics().len(), 1);

    backend.restore().expect("retry succeeds");
    assert_eq!(backend.state(), WindowsTerminalBackendState::Free);
    assert_eq!(backend.current_snapshot(), snapshot);
}
