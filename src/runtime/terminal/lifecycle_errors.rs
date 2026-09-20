//! Sealed terminal session lifecycle outcomes and errors.

extern crate alloc;

use super::{
    TerminalDiagnostic, TerminalDiagnosticCollection, TerminalInvalidOptions,
    TerminalRecoveryLedgerKind, TerminalSessionState,
};
use alloc::sync::Arc;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

/// Successful close result, including discarded decoder accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCloseOutcome {
    /// No retained input was discarded.
    Clean,
    /// Retained input was discarded while ending ownership.
    DiscardedInput {
        /// Production-accounted discarded bytes.
        discarded_bytes: u64,
        /// Production-accounted discarded events.
        discarded_events: u64,
    },
}

/// Immutable independently retained pause delivery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalPauseEvents {
    /// Independently owned retained events.
    events: alloc::vec::Vec<super::TerminalInputEvent>,
}

impl TerminalPauseEvents {
    /// Construct a sealed pause delivery inside the runtime.
    pub(crate) const fn new_runtime(events: alloc::vec::Vec<super::TerminalInputEvent>) -> Self {
        Self { events }
    }

    /// Return retained event count.
    #[must_use]
    pub fn len(&self) -> i64 {
        i64::try_from(self.events.len()).unwrap_or(i64::MAX)
    }

    /// Return whether this delivery is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Return one retained event.
    #[must_use]
    pub fn at(&self, index: i64) -> Option<super::TerminalInputEvent> {
        usize::try_from(index)
            .ok()
            .and_then(|value| self.events.get(value).cloned())
    }
}

/// Successful pause result.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TerminalPauseResult {
    /// Independently retained pause events.
    pub events: TerminalPauseEvents,
}

/// Open and resume failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionOpenError {
    /// Immutable options failed final validation.
    InvalidOptions {
        /// Structured invalid-options data.
        invalid_options: TerminalInvalidOptions,
        /// Structured diagnostic.
        diagnostic: TerminalDiagnostic,
    },
    /// Another session or recovery ledger owns the coordinator.
    TerminalAlreadyOwned { diagnostic: TerminalDiagnostic },
    /// Process recovery must complete before another open.
    RecoveryPending {
        diagnostic: TerminalDiagnostic,
        recovery_token: TerminalRecoveryToken,
    },
    /// Opening failed and rollback was incomplete.
    RollbackFailed {
        diagnostics: TerminalDiagnosticCollection,
        recovery_token: TerminalRecoveryToken,
    },
    /// Resume failed before mutation or was completely compensated.
    ResumeFailed { diagnostic: TerminalDiagnostic },
    /// No nonzero never-reused recovery generation remains.
    GenerationExhausted {
        last_issued_generation: u64,
        diagnostic: TerminalDiagnostic,
    },
    /// Backend opening failed and rollback completed.
    ModeWriteFailed { diagnostic: TerminalDiagnostic },
}

/// Session output/cursor/flush failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionWriteError {
    /// Output write failed.
    WriteFailed { diagnostic: TerminalDiagnostic },
    /// Flush failed.
    FlushFailed { diagnostic: TerminalDiagnostic },
    /// Cursor shape unsupported.
    UnsupportedCursorShape { diagnostic: TerminalDiagnostic },
}

/// Read and pause-delivery failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionReadError {
    /// Host read failed.
    ReadFailed { diagnostic: TerminalDiagnostic },
    /// Allocation failed before publication.
    AllocationFailed { diagnostic: TerminalDiagnostic },
    /// Pause delivery failed before transition.
    PauseDeliveryFailed { diagnostic: TerminalDiagnostic },
    /// A required monotonic identifier was exhausted.
    IdentifierExhausted { diagnostic: TerminalDiagnostic },
}

/// Non-mutating session-state rejection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionStateError {
    SessionActive { diagnostic: TerminalDiagnostic },
    SessionPaused { diagnostic: TerminalDiagnostic },
    SessionRestorePending { diagnostic: TerminalDiagnostic },
    SessionClosed { diagnostic: TerminalDiagnostic },
}

impl TerminalSessionStateError {
    /// Return the rejected observable state.
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching by reference keeps this const accessor non-moving"
    )]
    pub const fn state(&self) -> TerminalSessionState {
        match self {
            Self::SessionActive { .. } => TerminalSessionState::Active,
            Self::SessionPaused { .. } => TerminalSessionState::Paused,
            Self::SessionRestorePending { .. } => TerminalSessionState::RestorePending,
            Self::SessionClosed { .. } => TerminalSessionState::Closed,
        }
    }
}

/// Live-binding restoration and process-recovery failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionRestoreError {
    RestoreInputModeFailed {
        diagnostic: TerminalDiagnostic,
    },
    MultipleRestoreStepsFailed {
        diagnostics: TerminalDiagnosticCollection,
    },
    PendingOpenRollbackFailed {
        diagnostics: TerminalDiagnosticCollection,
        recovery_token: TerminalRecoveryToken,
    },
    CloseRestorePending {
        diagnostics: TerminalDiagnosticCollection,
        recovery_token: TerminalRecoveryToken,
    },
    PendingCloseRestoreFailed {
        diagnostics: TerminalDiagnosticCollection,
        recovery_token: TerminalRecoveryToken,
    },
    WrongSession {
        recovery_token: TerminalRecoveryToken,
        diagnostic: TerminalDiagnostic,
    },
    WrongKind {
        recovery_token: TerminalRecoveryToken,
        expected_kind: TerminalRecoveryLedgerKind,
        actual_kind: TerminalRecoveryLedgerKind,
        diagnostic: TerminalDiagnostic,
    },
    Stale {
        recovery_token: TerminalRecoveryToken,
        token_generation: u64,
        diagnostic: TerminalDiagnostic,
    },
    Consumed {
        recovery_token: TerminalRecoveryToken,
        token_generation: u64,
        diagnostic: TerminalDiagnostic,
    },
    RecoveryInProgress {
        recovery_token: TerminalRecoveryToken,
        generation: u64,
        diagnostic: TerminalDiagnostic,
    },
    ResumeRestorePending {
        diagnostics: TerminalDiagnosticCollection,
    },
}

/// Hidden shared capability cell behind every immutable recovery-token alias.
pub(crate) struct RecoveryCapabilityCell {
    /// Host-process identity for validation ordering.
    pub(crate) host_id: u64,
    /// Session-attempt identity for validation ordering.
    pub(crate) attempt_id: u64,
    /// Recovery ledger identity for validation ordering.
    pub(crate) ledger_id: u64,
    /// Nonzero issuance generation.
    pub(crate) generation: u64,
    /// Token authority kind.
    pub(crate) kind: TerminalRecoveryLedgerKind,
    /// Shared one-shot consumption bit.
    pub(crate) consumed: AtomicBool,
    /// Shared retryable in-progress claim bit.
    pub(crate) claimed: AtomicBool,
}

/// Sealed immutable alias to one process-recovery capability cell.
#[derive(Clone)]
pub struct TerminalRecoveryToken {
    /// Shared sealed recovery authority cell.
    pub(crate) cell: Arc<RecoveryCapabilityCell>,
}

impl TerminalRecoveryToken {
    /// Construct a sealed recovery-token alias inside the runtime.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "runtime recovery tokens are issued by failure paths added after the lifecycle state model"
        )
    )]
    pub(crate) fn new_runtime(
        host_id: u64,
        attempt_id: u64,
        ledger_id: u64,
        generation: u64,
        kind: TerminalRecoveryLedgerKind,
    ) -> Self {
        Self {
            cell: Arc::new(RecoveryCapabilityCell {
                host_id,
                attempt_id,
                ledger_id,
                generation,
                kind,
                consumed: AtomicBool::new(false),
                claimed: AtomicBool::new(false),
            }),
        }
    }

    /// Return the only public authority classification.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub fn kind(&self) -> TerminalRecoveryLedgerKind {
        self.cell.kind
    }

    /// Return the issued nonzero generation.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub fn generation(&self) -> u64 {
        self.cell.generation
    }

    /// Return whether every alias has observed consumed authority.
    pub(crate) fn is_consumed(&self) -> bool {
        self.cell.consumed.load(Ordering::Acquire)
    }

    /// Return hidden host identity for runtime validation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub(crate) fn host_id(&self) -> u64 {
        self.cell.host_id
    }

    /// Return hidden attempt identity for runtime validation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub(crate) fn attempt_id(&self) -> u64 {
        self.cell.attempt_id
    }

    /// Return hidden ledger identity for runtime validation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub(crate) fn ledger_id(&self) -> u64 {
        self.cell.ledger_id
    }

    /// Return consumed storage for runtime validation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub(crate) fn consumed(&self) -> &AtomicBool {
        &self.cell.consumed
    }

    /// Return in-progress claim storage for runtime validation.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc dereference cannot be const-stabilized here"
    )]
    pub(crate) fn claimed(&self) -> &AtomicBool {
        &self.cell.claimed
    }
}

impl PartialEq for TerminalRecoveryToken {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.cell, &other.cell)
    }
}

impl Eq for TerminalRecoveryToken {}

impl fmt::Debug for TerminalRecoveryToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalRecoveryToken")
            .field("kind", &self.kind())
            .field("generation", &self.generation())
            .finish_non_exhaustive()
    }
}
