//! Deterministic Linux terminal backend contract core.
//!
//! This module models the selected Linux backend's ordering and restoration
//! rules without depending on a host TTY. Platform syscalls can plug into the
//! same snapshot/ledger shape in a later smoke layer.

extern crate alloc;

use super::{
    TerminalBackend, TerminalDiagnostic, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalInputEventKind,
    TerminalOperation, TerminalOsCode,
};
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// Simulated descriptor mode bits captured before raw-mode acquisition.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LinuxTerminalDescriptorSnapshot {
    /// Terminal input mode bits.
    pub input_flags: u32,
    /// Terminal output mode bits.
    pub output_flags: u32,
    /// Terminal local mode bits.
    pub local_flags: u32,
    /// Descriptor file-status bits.
    pub file_status_flags: u32,
}

/// Deterministic backend ownership state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxTerminalBackendState {
    /// Not currently owned.
    Free,
    /// Raw-mode ownership is active.
    Active,
    /// Restoration is incomplete and retryable.
    RestorePending,
}

/// Ordered inverse operation retained by the Linux ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinuxTerminalInverseStep {
    /// Restore descriptor status flags such as nonblocking.
    FileStatus,
    /// Restore termios bits.
    Termios,
    /// Release process-global ownership last.
    Ownership,
}

/// Deterministic Linux backend harness used by session runtime and tests.
#[derive(Debug, Clone)]
pub struct LinuxTerminalBackendRuntime {
    /// Original descriptor snapshot.
    snapshot: LinuxTerminalDescriptorSnapshot,
    /// Current descriptor snapshot.
    current: LinuxTerminalDescriptorSnapshot,
    /// Backend ownership state.
    state: LinuxTerminalBackendState,
    /// Reverse-order restoration ledger.
    ledger: Vec<LinuxTerminalInverseStep>,
    /// Queued decoded input events.
    input_queue: VecDeque<TerminalInputEventKind>,
    /// Queued resize events.
    resize_queue: VecDeque<TerminalInputEventKind>,
    /// Sticky EOF/HUP after queues drain.
    eof: bool,
    /// Optional inverse step to fail once.
    fail_next_inverse: Option<LinuxTerminalInverseStep>,
    /// Diagnostics from attempted inverse steps.
    diagnostics: Vec<TerminalDiagnostic>,
}

impl LinuxTerminalBackendRuntime {
    /// Construct a deterministic backend from an original snapshot.
    #[must_use]
    pub const fn new(snapshot: LinuxTerminalDescriptorSnapshot) -> Self {
        Self {
            snapshot,
            current: snapshot,
            state: LinuxTerminalBackendState::Free,
            ledger: Vec::new(),
            input_queue: VecDeque::new(),
            resize_queue: VecDeque::new(),
            eof: false,
            fail_next_inverse: None,
            diagnostics: Vec::new(),
        }
    }

    /// Acquire raw-ish Linux ownership without flushing pending input.
    pub fn acquire(&mut self, capture_control_keys: bool) {
        self.state = LinuxTerminalBackendState::Active;
        self.current.file_status_flags |= 0x800;
        self.ledger.push(LinuxTerminalInverseStep::FileStatus);
        self.current.input_flags &= !0x0001;
        if capture_control_keys {
            self.current.local_flags &= !0x0002;
        }
        self.ledger.push(LinuxTerminalInverseStep::Termios);
        self.ledger.push(LinuxTerminalInverseStep::Ownership);
    }

    /// Return current backend state.
    #[must_use]
    pub const fn state(&self) -> LinuxTerminalBackendState {
        self.state
    }

    /// Return current simulated descriptor state.
    #[must_use]
    pub const fn current_snapshot(&self) -> LinuxTerminalDescriptorSnapshot {
        self.current
    }

    /// Return diagnostics from attempted inverse restoration.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec slice dereference is not const-stable on this toolchain"
    )]
    pub fn diagnostics(&self) -> &[TerminalDiagnostic] {
        &self.diagnostics
    }

    /// Fail the next matching inverse step once.
    pub const fn fail_next_inverse(&mut self, step: LinuxTerminalInverseStep) {
        self.fail_next_inverse = Some(step);
    }

    /// Queue decoded input.
    pub fn push_input(&mut self, event: TerminalInputEventKind) {
        self.input_queue.push_back(event);
    }

    /// Queue a resize notification.
    pub fn push_resize(&mut self, event: TerminalInputEventKind) {
        self.resize_queue.push_back(event);
    }

    /// Mark HUP/EOF; readable queues still drain first.
    pub const fn mark_hup(&mut self) {
        self.eof = true;
    }

    /// Poll one backend event, preserving input-before-resize-before-EOF order.
    pub fn poll_event(&mut self) -> TerminalInputEventKind {
        if let Some(event) = self.input_queue.pop_front() {
            return event;
        }
        if let Some(event) = self.resize_queue.pop_front() {
            return event;
        }
        if self.eof {
            return TerminalInputEventKind::EndOfInput;
        }
        TerminalInputEventKind::TimedOut
    }

    /// Restore every remaining inverse in strict reverse order.
    pub fn restore(&mut self) -> Result<(), TerminalDiagnostic> {
        while let Some(step) = self.ledger.pop() {
            if self.fail_next_inverse == Some(step) {
                self.fail_next_inverse = None;
                self.ledger.push(step);
                self.state = LinuxTerminalBackendState::RestorePending;
                let diagnostic = linux_diagnostic(match step {
                    LinuxTerminalInverseStep::FileStatus | LinuxTerminalInverseStep::Termios => {
                        TerminalOperation::RestorePendingClose
                    }
                    LinuxTerminalInverseStep::Ownership => TerminalOperation::Close,
                });
                self.diagnostics.push(diagnostic.clone());
                return Err(diagnostic);
            }
            match step {
                LinuxTerminalInverseStep::FileStatus => {
                    self.current.file_status_flags = self.snapshot.file_status_flags;
                }
                LinuxTerminalInverseStep::Termios => {
                    self.current.input_flags = self.snapshot.input_flags;
                    self.current.output_flags = self.snapshot.output_flags;
                    self.current.local_flags = self.snapshot.local_flags;
                }
                LinuxTerminalInverseStep::Ownership => {
                    self.state = LinuxTerminalBackendState::Free;
                }
            }
        }
        self.state = LinuxTerminalBackendState::Free;
        Ok(())
    }
}

/// Build a deterministic Linux diagnostic.
fn linux_diagnostic(operation: TerminalOperation) -> TerminalDiagnostic {
    TerminalDiagnostic::new_runtime(
        TerminalBackend::LinuxVt,
        operation,
        TerminalDiagnosticStage::RestoreOperatingSystemState,
        super::TerminalCoordinatorState::RestorePending,
        TerminalDiagnosticSessionState::RestorePending,
        TerminalOsCode::Unavailable,
        match super::TerminalDiagnosticDetail::new_runtime("linux backend inverse failed") {
            Ok(detail) => detail,
            Err(_error) => super::TerminalDiagnosticDetail::new_runtime("")
                .unwrap_or_else(|_fallback_error| unreachable!("empty diagnostic detail is valid")),
        },
        TerminalDiagnosticRetryability::SameLiveSession,
        false,
    )
}
