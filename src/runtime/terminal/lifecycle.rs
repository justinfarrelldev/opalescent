//! Terminal session lifecycle and recovery-token runtime model.

extern crate alloc;

use super::diagnostics::TerminalDiagnostic;
use super::lifecycle_errors::{
    TerminalCloseOutcome, TerminalRecoveryToken, TerminalSessionOpenError,
    TerminalSessionRestoreError, TerminalSessionStateError,
};
use super::model::{
    TerminalBackend, TerminalCapabilities, TerminalCapabilityUnsupportedEvidence,
    TerminalCoordinatorState, TerminalDiagnosticRetryability, TerminalDiagnosticSessionState,
    TerminalDiagnosticStage, TerminalOperation, TerminalOsCode, TerminalSessionOptions,
};
use super::tail_types::{
    TerminalRecoveryLedgerKind, TerminalSessionOptionsError, TerminalSessionState,
};
use crate::runtime::terminal::constraints::TerminalCorrelatedEventLimit;
use alloc::collections::BTreeMap;
#[cfg(test)]
use core::cell::Cell;
use core::sync::atomic::{AtomicU64, Ordering};

/// Hidden host identity for the in-process terminal coordinator.
const PROCESS_TERMINAL_HOST_ID: u64 = 1;

/// Issued recovery generations. Zero is never issued publicly.
static NEXT_RECOVERY_GENERATION: AtomicU64 = AtomicU64::new(1);
/// Runtime session identity source.
static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
/// Runtime ledger identity source.
static NEXT_LEDGER_ID: AtomicU64 = AtomicU64::new(1);

#[cfg(test)]
thread_local! {
    /// Per-test deterministic recovery-generation override.
    static NEXT_RECOVERY_GENERATION_OVERRIDE: Cell<Option<u64>> = const { Cell::new(None) };
}

/// Affine terminal-session runtime value.
#[derive(Clone, Debug)]
pub struct TerminalSession {
    /// Current observable public state.
    state: TerminalSessionState,
    /// Hidden session attempt identity.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "session identity is consumed by recovery transfer paths in later backend work"
        )
    )]
    session_id: u64,
    /// Hidden restoration ledger identity.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "ledger identity is consumed by recovery transfer paths in later backend work"
        )
    )]
    ledger_id: u64,
    /// Last immutable capability snapshot.
    capabilities: TerminalCapabilities,
    /// Whether lexical cleanup still owns restoration responsibility.
    cleanup_obligation: bool,
    /// Exclusive unissued recovery generation reservation.
    reserved_recovery_generation: u64,
}

impl TerminalSession {
    /// Return the current public state.
    #[must_use]
    pub const fn state(&self) -> TerminalSessionState {
        self.state
    }

    /// Return the current immutable capability snapshot.
    #[must_use]
    pub fn capabilities(&self) -> TerminalCapabilities {
        self.capabilities.clone()
    }

    /// Return whether lexical cleanup is still owed by this binding.
    #[must_use]
    pub const fn cleanup_obligation(&self) -> bool {
        self.cleanup_obligation
    }

    /// Return the hidden session identity for runtime tests.
    #[cfg(test)]
    pub(crate) const fn session_id_for_tests(&self) -> u64 {
        self.session_id
    }

    /// Return the reserved recovery generation for runtime tests.
    #[cfg(test)]
    pub(crate) const fn reserved_recovery_generation_for_tests(&self) -> u64 {
        self.reserved_recovery_generation
    }

    /// Query a deterministic size while active or paused.
    ///
    /// Task 26 replaces this placeholder with backend snapshots, but Task 25 owns
    /// exact state rejection and non-mutating behavior.
    pub fn size_sync(&self) -> Result<super::TerminalSize, TerminalSessionStateError> {
        match self.state {
            TerminalSessionState::Active | TerminalSessionState::Paused => {
                let columns =
                    crate::runtime::terminal::constraints::TerminalColumnCount::new(80)
                        .map_err(|_error| state_error(self.state, TerminalOperation::QuerySize))?;
                let rows = crate::runtime::terminal::constraints::TerminalRowCount::new(24)
                    .map_err(|_error| state_error(self.state, TerminalOperation::QuerySize))?;
                Ok(super::TerminalSize { columns, rows })
            }
            TerminalSessionState::RestorePending | TerminalSessionState::Closed => {
                Err(state_error(self.state, TerminalOperation::QuerySize))
            }
        }
    }

    /// Pause the session without input delivery; Task 26 fills pause events.
    pub fn pause_sync(&mut self) -> Result<(), TerminalSessionStateError> {
        match self.state {
            TerminalSessionState::Active => {
                self.state = TerminalSessionState::Paused;
                Ok(())
            }
            TerminalSessionState::Paused => Ok(()),
            TerminalSessionState::RestorePending | TerminalSessionState::Closed => {
                Err(state_error(self.state, TerminalOperation::Pause))
            }
        }
    }

    /// Resume a paused session.
    pub fn resume_sync(&mut self) -> Result<(), TerminalSessionStateError> {
        match self.state {
            TerminalSessionState::Paused => {
                self.state = TerminalSessionState::Active;
                Ok(())
            }
            TerminalSessionState::Active
            | TerminalSessionState::RestorePending
            | TerminalSessionState::Closed => {
                Err(state_error(self.state, TerminalOperation::Resume))
            }
        }
    }

    /// Explicit close consumes cleanup obligation on success and remains inspectable.
    pub const fn close_sync(
        &mut self,
    ) -> Result<TerminalCloseOutcome, TerminalSessionRestoreError> {
        match self.state {
            TerminalSessionState::Active
            | TerminalSessionState::Paused
            | TerminalSessionState::RestorePending => {
                self.state = TerminalSessionState::Closed;
                self.cleanup_obligation = false;
                self.reserved_recovery_generation = 0;
                Ok(TerminalCloseOutcome::Clean)
            }
            TerminalSessionState::Closed => Ok(TerminalCloseOutcome::Clean),
        }
    }

    /// Test hook: move a live binding into restore pending without producing a token.
    #[cfg(test)]
    pub(crate) const fn force_restore_pending_for_tests(&mut self) {
        self.state = TerminalSessionState::RestorePending;
    }

    /// Create a close-recovery token for lexical cleanup transfer tests.
    #[cfg(test)]
    pub(crate) fn cleanup_transfer_token_for_tests(&self) -> TerminalRecoveryToken {
        TerminalRecoveryToken::new_runtime(
            PROCESS_TERMINAL_HOST_ID,
            self.session_id,
            self.ledger_id,
            self.reserved_recovery_generation,
            TerminalRecoveryLedgerKind::CloseRestore,
        )
    }
}

/// Set the next recovery generation for deterministic boundary tests.
#[cfg(test)]
pub(crate) fn set_next_recovery_generation_for_tests(value: u64) {
    NEXT_RECOVERY_GENERATION_OVERRIDE
        .with(|override_generation| override_generation.set(Some(value)));
}

/// Clear the deterministic recovery generation override for the current test.
#[cfg(test)]
pub(crate) fn clear_next_recovery_generation_for_tests() {
    NEXT_RECOVERY_GENERATION_OVERRIDE.with(|override_generation| override_generation.set(None));
}

/// Reserve the next nonzero recovery generation.
#[cfg(test)]
fn reserve_recovery_generation() -> u64 {
    NEXT_RECOVERY_GENERATION_OVERRIDE.with(|override_generation| {
        override_generation.get().map_or_else(
            || NEXT_RECOVERY_GENERATION.fetch_add(1, Ordering::Relaxed),
            |generation| {
                override_generation.set(generation.checked_add(1));
                generation
            },
        )
    })
}

/// Reserve the next nonzero recovery generation.
#[cfg(not(test))]
fn reserve_recovery_generation() -> u64 {
    NEXT_RECOVERY_GENERATION.fetch_add(1, Ordering::Relaxed)
}

/// Open a terminal session from immutable options.
pub fn terminal_session_open_sync(
    options: &TerminalSessionOptions,
) -> Result<TerminalSession, TerminalSessionOpenError> {
    let validated = options.validate().map_err(|error| match error {
        TerminalSessionOptionsError::InvalidOptions { invalid_options } => {
            TerminalSessionOpenError::InvalidOptions {
                invalid_options,
                diagnostic: diagnostic(
                    TerminalOperation::ValidateOptions,
                    TerminalSessionState::Closed,
                ),
            }
        }
    })?;
    let session_id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let ledger_id = NEXT_LEDGER_ID.fetch_add(1, Ordering::Relaxed);
    let generation = reserve_recovery_generation();
    if generation == u64::MAX {
        #[cfg(not(test))]
        NEXT_RECOVERY_GENERATION.store(u64::MAX, Ordering::Release);
        return Err(TerminalSessionOpenError::GenerationExhausted {
            last_issued_generation: u64::MAX,
            diagnostic: diagnostic(TerminalOperation::Open, TerminalSessionState::Closed),
        });
    }
    let capabilities = default_capabilities(validated.resource_limits().maximum_correlated_events);
    Ok(TerminalSession {
        state: TerminalSessionState::Active,
        session_id,
        ledger_id,
        capabilities,
        cleanup_obligation: true,
        reserved_recovery_generation: generation,
    })
}

/// Recover a failed open rollback using an authenticated token.
pub fn terminal_session_recover_open_sync(
    token: &TerminalRecoveryToken,
) -> Result<(), TerminalSessionRestoreError> {
    validate_recovery_token(token, TerminalRecoveryLedgerKind::OpenRollback)
}

/// Recover a failed close restoration using an authenticated token.
pub fn terminal_session_recover_close_sync(
    token: &TerminalRecoveryToken,
) -> Result<(), TerminalSessionRestoreError> {
    validate_recovery_token(token, TerminalRecoveryLedgerKind::CloseRestore)
}

/// Validate and consume one recovery capability token.
pub fn validate_recovery_token(
    token: &TerminalRecoveryToken,
    expected_kind: TerminalRecoveryLedgerKind,
) -> Result<(), TerminalSessionRestoreError> {
    if token.host_id() != PROCESS_TERMINAL_HOST_ID
        || token.attempt_id() == 0
        || token.ledger_id() == 0
    {
        return Err(TerminalSessionRestoreError::WrongSession {
            recovery_token: token.clone(),
            diagnostic: diagnostic(
                TerminalOperation::RestorePendingClose,
                TerminalSessionState::Closed,
            ),
        });
    }
    if token.kind() != expected_kind {
        return Err(TerminalSessionRestoreError::WrongKind {
            recovery_token: token.clone(),
            expected_kind,
            actual_kind: token.kind(),
            diagnostic: diagnostic(
                TerminalOperation::RestorePendingClose,
                TerminalSessionState::Closed,
            ),
        });
    }
    if token.is_consumed() {
        return Err(TerminalSessionRestoreError::Consumed {
            recovery_token: token.clone(),
            token_generation: token.generation(),
            diagnostic: diagnostic(
                TerminalOperation::RestorePendingClose,
                TerminalSessionState::Closed,
            ),
        });
    }
    if token.generation() == 0 {
        return Err(TerminalSessionRestoreError::Stale {
            recovery_token: token.clone(),
            token_generation: token.generation(),
            diagnostic: diagnostic(
                TerminalOperation::RestorePendingClose,
                TerminalSessionState::Closed,
            ),
        });
    }
    if token.claimed().swap(true, Ordering::AcqRel) {
        return Err(TerminalSessionRestoreError::RecoveryInProgress {
            recovery_token: token.clone(),
            generation: token.generation(),
            diagnostic: diagnostic(
                TerminalOperation::RestorePendingClose,
                TerminalSessionState::Closed,
            ),
        });
    }
    token.consumed().store(true, Ordering::Release);
    token.claimed().store(false, Ordering::Release);
    Ok(())
}

/// Construct a deterministic default capability snapshot.
fn default_capabilities(
    correlated_event_limit: TerminalCorrelatedEventLimit,
) -> TerminalCapabilities {
    TerminalCapabilities::new_runtime(
        BTreeMap::new(),
        super::TerminalTrustedPasteCapability::Unsupported {
            evidence: TerminalCapabilityUnsupportedEvidence::EnvironmentMissing,
        },
        super::TerminalColorCapability::Unsupported {
            evidence: TerminalCapabilityUnsupportedEvidence::EnvironmentMissing,
        },
        1,
        correlated_event_limit,
    )
}

/// Build a state rejection error with a diagnostic.
fn state_error(
    state: TerminalSessionState,
    operation: TerminalOperation,
) -> TerminalSessionStateError {
    let diag = diagnostic(operation, state);
    match state {
        TerminalSessionState::Active => {
            TerminalSessionStateError::SessionActive { diagnostic: diag }
        }
        TerminalSessionState::Paused => {
            TerminalSessionStateError::SessionPaused { diagnostic: diag }
        }
        TerminalSessionState::RestorePending => {
            TerminalSessionStateError::SessionRestorePending { diagnostic: diag }
        }
        TerminalSessionState::Closed => {
            TerminalSessionStateError::SessionClosed { diagnostic: diag }
        }
    }
}

/// Build a structured lifecycle diagnostic.
fn diagnostic(operation: TerminalOperation, state: TerminalSessionState) -> TerminalDiagnostic {
    TerminalDiagnostic::new_runtime(
        TerminalBackend::UnsupportedPlatform,
        operation,
        TerminalDiagnosticStage::ValidateState,
        TerminalCoordinatorState::Free,
        match state {
            TerminalSessionState::Active => TerminalDiagnosticSessionState::Active,
            TerminalSessionState::Paused => TerminalDiagnosticSessionState::Paused,
            TerminalSessionState::RestorePending => TerminalDiagnosticSessionState::RestorePending,
            TerminalSessionState::Closed => TerminalDiagnosticSessionState::Closed,
        },
        TerminalOsCode::Unavailable,
        crate::runtime::terminal::constraints::TerminalDiagnosticDetail::new_runtime("")
            .expect("empty diagnostic detail is valid"),
        TerminalDiagnosticRetryability::NonRetryable,
        false,
    )
}
