use crate::runtime::terminal::{
    LinuxTerminalBackendRuntime, LinuxTerminalBackendState, LinuxTerminalDescriptorSnapshot,
    LinuxTerminalInverseStep, TerminalInputEventKind, TerminalOperation, TerminalSize,
};

#[test]
fn linux_backend_snapshot_restore_and_input_before_resize_ordering_are_exact() {
    let snapshot = LinuxTerminalDescriptorSnapshot {
        input_flags: 0xffff,
        output_flags: 0x00ff,
        local_flags: 0xffff,
        file_status_flags: 0,
    };
    let mut backend = LinuxTerminalBackendRuntime::new(snapshot);
    backend.acquire(false);
    assert_eq!(backend.state(), LinuxTerminalBackendState::Active);
    assert_ne!(backend.current_snapshot(), snapshot);
    assert_ne!(backend.current_snapshot().file_status_flags & 0x800, 0);
    assert_ne!(backend.current_snapshot().local_flags & 0x0002, 0);

    backend.push_resize(TerminalInputEventKind::Resize {
        size: TerminalSize {
            columns: crate::runtime::terminal::TerminalColumnCount::new(100).unwrap(),
            rows: crate::runtime::terminal::TerminalRowCount::new(40).unwrap(),
        },
    });
    backend.push_input(TerminalInputEventKind::FocusGained);
    backend.mark_hup();

    assert!(matches!(
        backend.poll_event(),
        TerminalInputEventKind::FocusGained
    ));
    assert!(matches!(
        backend.poll_event(),
        TerminalInputEventKind::Resize { .. }
    ));
    assert!(matches!(
        backend.poll_event(),
        TerminalInputEventKind::EndOfInput
    ));

    backend.restore().expect("restore succeeds");
    assert_eq!(backend.state(), LinuxTerminalBackendState::Free);
    assert_eq!(backend.current_snapshot(), snapshot);
}

#[test]
fn linux_backend_restoration_failure_is_retryable_and_ordered() {
    let snapshot = LinuxTerminalDescriptorSnapshot {
        input_flags: 0x00f0,
        output_flags: 0x000f,
        local_flags: 0x0f0f,
        file_status_flags: 0x0100,
    };
    let mut backend = LinuxTerminalBackendRuntime::new(snapshot);
    backend.acquire(true);
    assert_eq!(backend.current_snapshot().local_flags & 0x0002, 0);
    backend.fail_next_inverse(LinuxTerminalInverseStep::Termios);

    let diagnostic = backend.restore().expect_err("termios inverse fails once");
    assert_eq!(backend.state(), LinuxTerminalBackendState::RestorePending);
    assert_eq!(
        diagnostic.operation(),
        TerminalOperation::RestorePendingClose
    );
    assert_eq!(backend.diagnostics().len(), 1);

    backend
        .restore()
        .expect("retry completes remaining inverses");
    assert_eq!(backend.state(), LinuxTerminalBackendState::Free);
    assert_eq!(backend.current_snapshot(), snapshot);
}
