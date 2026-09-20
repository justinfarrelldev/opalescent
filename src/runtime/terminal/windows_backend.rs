//! Deterministic Windows Console/ConPTY backend contract core.
//!
//! This module models the selected Windows restoration and native-input
//! quarantine rules without requiring a Windows host in ordinary CI.

extern crate alloc;

use super::{
    TerminalBackend, TerminalDiagnostic, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalInputEventKind,
    TerminalNativeEventKind, TerminalNativeEventName, TerminalNativeMetadata, TerminalOperation,
    TerminalOsCode, TerminalUnknownBytesReason,
};
use alloc::collections::VecDeque;

/// Snapshot of Console mode, cursor, and buffer ownership state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowsConsoleSnapshot {
    /// Original input mode bits.
    pub input_mode: u32,
    /// Original output mode bits.
    pub output_mode: u32,
    /// Original cursor visibility.
    pub cursor_visible: bool,
    /// Original cursor shape token.
    pub cursor_shape: u32,
    /// Original active screen-buffer token.
    pub active_buffer: u64,
    /// Alternate screen-buffer token, if one is active.
    pub alternate_buffer: Option<u64>,
    /// Quick Edit flag state.
    pub quick_edit_enabled: bool,
}

impl Default for WindowsConsoleSnapshot {
    fn default() -> Self {
        Self {
            input_mode: 0,
            output_mode: 0,
            cursor_visible: true,
            cursor_shape: 0,
            active_buffer: 1,
            alternate_buffer: None,
            quick_edit_enabled: false,
        }
    }
}

/// Deterministic Windows backend state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsTerminalBackendState {
    /// Not currently owned.
    Free,
    /// Console or `ConPTY` ownership is active.
    Active,
    /// Restoration is incomplete and retryable.
    RestorePending,
}

/// Reverse-order Windows restoration steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowsTerminalInverseStep {
    /// Restore the original active screen buffer before closing alternate.
    ActiveBuffer,
    /// Close or detach alternate screen buffer.
    AlternateBuffer,
    /// Restore cursor state.
    Cursor,
    /// Restore console modes including Quick Edit.
    Modes,
    /// Release terminal ownership last.
    Ownership,
}

/// Deterministic Windows backend runtime.
#[derive(Debug, Clone)]
pub struct WindowsTerminalBackendRuntime {
    /// Original snapshot.
    snapshot: WindowsConsoleSnapshot,
    /// Current snapshot.
    current: WindowsConsoleSnapshot,
    /// Backend kind.
    backend: TerminalBackend,
    /// Runtime state.
    state: WindowsTerminalBackendState,
    /// Reverse restoration ledger.
    ledger: Vec<WindowsTerminalInverseStep>,
    /// Queued input events.
    input: VecDeque<TerminalInputEventKind>,
    /// Optional inverse failure.
    fail_next_inverse: Option<WindowsTerminalInverseStep>,
    /// Diagnostics from failed inverses.
    diagnostics: Vec<TerminalDiagnostic>,
}

impl WindowsTerminalBackendRuntime {
    /// Construct a deterministic Console backend.
    #[must_use]
    pub const fn console(snapshot: WindowsConsoleSnapshot) -> Self {
        Self::new(snapshot, TerminalBackend::WindowsConsole)
    }

    /// Construct a deterministic `ConPTY` backend.
    #[must_use]
    pub const fn conpty(snapshot: WindowsConsoleSnapshot) -> Self {
        Self::new(snapshot, TerminalBackend::WindowsConPty)
    }

    /// Construct a deterministic Windows backend.
    const fn new(snapshot: WindowsConsoleSnapshot, backend: TerminalBackend) -> Self {
        Self {
            snapshot,
            current: snapshot,
            backend,
            state: WindowsTerminalBackendState::Free,
            ledger: Vec::new(),
            input: VecDeque::new(),
            fail_next_inverse: None,
            diagnostics: Vec::new(),
        }
    }

    /// Acquire terminal ownership and mutate snapshot fields as Windows backends do.
    pub fn acquire(&mut self, use_alternate_buffer: bool) {
        self.state = WindowsTerminalBackendState::Active;
        self.current.input_mode |= 0x0200;
        self.current.output_mode |= 0x0004;
        self.current.quick_edit_enabled = false;
        self.ledger.push(WindowsTerminalInverseStep::Modes);
        self.current.cursor_visible = false;
        self.current.cursor_shape = 2;
        self.ledger.push(WindowsTerminalInverseStep::Cursor);
        if use_alternate_buffer {
            self.current.alternate_buffer = Some(2);
            self.current.active_buffer = 2;
            self.ledger
                .push(WindowsTerminalInverseStep::AlternateBuffer);
            self.ledger.push(WindowsTerminalInverseStep::ActiveBuffer);
        }
        self.ledger.push(WindowsTerminalInverseStep::Ownership);
    }

    /// Return current state.
    #[must_use]
    pub const fn state(&self) -> WindowsTerminalBackendState {
        self.state
    }

    /// Return current snapshot.
    #[must_use]
    pub const fn current_snapshot(&self) -> WindowsConsoleSnapshot {
        self.current
    }

    /// Return diagnostics from failed inverse steps.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec slice dereference is not const-stable"
    )]
    pub fn diagnostics(&self) -> &[TerminalDiagnostic] {
        &self.diagnostics
    }

    /// Fail the next matching inverse step once.
    pub const fn fail_next_inverse(&mut self, step: WindowsTerminalInverseStep) {
        self.fail_next_inverse = Some(step);
    }

    /// Queue an input event.
    pub fn push_input(&mut self, event: TerminalInputEventKind) {
        self.input.push_back(event);
    }

    /// Queue an unpaired surrogate as unknown native input plus reset.
    pub fn push_unpaired_surrogate(&mut self) {
        self.input.push_back(TerminalInputEventKind::UnknownNative {
            metadata: TerminalNativeMetadata {
                kind: TerminalNativeEventKind::WindowsUnknownRecord,
                code: TerminalOsCode::WindowsError { value: 0 },
            },
        });
        self.input.push_back(TerminalInputEventKind::UnknownBytes {
            raw_bytes: crate::stdlib::bytes::Bytes::new(),
            reason: TerminalUnknownBytesReason::BackendOverflow,
        });
    }

    /// Queue an unsupported native record without synthesizing text.
    pub fn push_unsupported_native_record(&mut self, name: &str) {
        self.input.push_back(TerminalInputEventKind::UnknownNative {
            metadata: TerminalNativeMetadata {
                kind: TerminalNativeEventKind::Other {
                    name: native_event_name(name),
                },
                code: TerminalOsCode::Unavailable,
            },
        });
    }

    /// Poll one event.
    pub fn poll_event(&mut self) -> TerminalInputEventKind {
        self.input
            .pop_front()
            .unwrap_or(TerminalInputEventKind::TimedOut)
    }

    /// Restore every remaining inverse in strict reverse order.
    pub fn restore(&mut self) -> Result<(), TerminalDiagnostic> {
        while let Some(step) = self.ledger.pop() {
            if self.fail_next_inverse == Some(step) {
                self.fail_next_inverse = None;
                self.ledger.push(step);
                self.state = WindowsTerminalBackendState::RestorePending;
                let diagnostic =
                    windows_diagnostic(self.backend, TerminalOperation::RestorePendingClose);
                self.diagnostics.push(diagnostic.clone());
                return Err(diagnostic);
            }
            match step {
                WindowsTerminalInverseStep::ActiveBuffer => {
                    self.current.active_buffer = self.snapshot.active_buffer;
                }
                WindowsTerminalInverseStep::AlternateBuffer => {
                    self.current.alternate_buffer = self.snapshot.alternate_buffer;
                }
                WindowsTerminalInverseStep::Cursor => {
                    self.current.cursor_visible = self.snapshot.cursor_visible;
                    self.current.cursor_shape = self.snapshot.cursor_shape;
                }
                WindowsTerminalInverseStep::Modes => {
                    self.current.input_mode = self.snapshot.input_mode;
                    self.current.output_mode = self.snapshot.output_mode;
                    self.current.quick_edit_enabled = self.snapshot.quick_edit_enabled;
                }
                WindowsTerminalInverseStep::Ownership => {
                    self.state = WindowsTerminalBackendState::Free;
                }
            }
        }
        self.state = WindowsTerminalBackendState::Free;
        Ok(())
    }
}

/// Build a bounded native event name, falling back to a static name on invalid input.
fn native_event_name(name: &str) -> TerminalNativeEventName {
    match TerminalNativeEventName::new_runtime(name) {
        Ok(value) => value,
        Err(_error) => TerminalNativeEventName::new_runtime("UnknownNative")
            .unwrap_or_else(|_fallback_error| unreachable!("static native name is valid")),
    }
}

/// Build a deterministic Windows diagnostic.
fn windows_diagnostic(
    backend: TerminalBackend,
    operation: TerminalOperation,
) -> TerminalDiagnostic {
    TerminalDiagnostic::new_runtime(
        backend,
        operation,
        TerminalDiagnosticStage::RestoreOperatingSystemState,
        super::TerminalCoordinatorState::RestorePending,
        TerminalDiagnosticSessionState::RestorePending,
        TerminalOsCode::WindowsError { value: 0 },
        match super::TerminalDiagnosticDetail::new_runtime("windows backend inverse failed") {
            Ok(detail) => detail,
            Err(_error) => super::TerminalDiagnosticDetail::new_runtime("")
                .unwrap_or_else(|_fallback_error| unreachable!("empty diagnostic detail is valid")),
        },
        TerminalDiagnosticRetryability::SameLiveSession,
        false,
    )
}
