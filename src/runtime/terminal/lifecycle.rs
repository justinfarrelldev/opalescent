//! Terminal session lifecycle and recovery-token runtime model.

extern crate alloc;

use super::diagnostics::TerminalDiagnostic;
use super::formatting::{SafeTerminalDiagnosticOutput, TrustedTerminalOutput};
use super::lifecycle_errors::{
    TerminalCloseOutcome, TerminalPauseEvents, TerminalPauseResult, TerminalRecoveryToken,
    TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionRestoreError,
    TerminalSessionStateError, TerminalSessionWriteError,
};
use super::model::{
    TerminalBackend, TerminalCapabilities, TerminalCapabilityUnsupportedEvidence,
    TerminalCoordinatorState, TerminalCursorShape, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalInputEvent,
    TerminalInputEventKind, TerminalInputResetReason, TerminalOperation, TerminalOsCode,
    TerminalSessionOptions,
};
use super::tail_types::{
    TerminalRecoveryLedgerKind, TerminalSessionOptionsError, TerminalSessionState,
};
use crate::runtime::terminal::constraints::TerminalCorrelatedEventLimit;
use crate::runtime::wait::{CancellationToken, SourceAvailability, SystemReadinessSource};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::format;
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

/// Combined read-event failure preserving independent proposal families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalReadEventError {
    /// Read-family failure.
    Read(TerminalSessionReadError),
    /// State-family failure.
    State(TerminalSessionStateError),
}

impl TerminalReadEventError {
    /// Return the rejected state for state-family failures.
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching by reference keeps this const accessor non-moving"
    )]
    pub const fn state(&self) -> TerminalSessionState {
        match self {
            Self::State(error) => error.state(),
            Self::Read(_error) => TerminalSessionState::Active,
        }
    }
}

/// Combined pause failure preserving independent proposal families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalPauseError {
    /// Read-family failure.
    Read(TerminalSessionReadError),
    /// State-family failure.
    State(TerminalSessionStateError),
}

impl TerminalPauseError {
    /// Return the rejected state for state-family failures.
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching by reference keeps this const accessor non-moving"
    )]
    pub const fn state(&self) -> TerminalSessionState {
        match self {
            Self::State(error) => error.state(),
            Self::Read(_error) => TerminalSessionState::Active,
        }
    }
}

/// Combined write failure preserving proposal families.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalWriteOperationError {
    /// Write-family failure.
    Write(TerminalSessionWriteError),
    /// State-family failure.
    State(TerminalSessionStateError),
    /// Cursor position was outside terminal-control invariants.
    InvalidCursorPosition { diagnostic: TerminalDiagnostic },
}

impl TerminalWriteOperationError {
    /// Return the rejected state for state-family failures.
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "matching by reference keeps this const accessor non-moving"
    )]
    pub const fn state(&self) -> TerminalSessionState {
        match self {
            Self::State(error) => error.state(),
            Self::Write(_) | Self::InvalidCursorPosition { .. } => TerminalSessionState::Active,
        }
    }
}

impl From<TerminalPauseError> for TerminalReadEventError {
    fn from(error: TerminalPauseError) -> Self {
        match error {
            TerminalPauseError::Read(read_error) => Self::Read(read_error),
            TerminalPauseError::State(state_error) => Self::State(state_error),
        }
    }
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
    /// Stable readiness source identity for this session.
    readiness_source: SystemReadinessSource,
    /// Decoded events awaiting one-event reads.
    event_queue: VecDeque<TerminalInputEvent>,
    /// Next hidden event delivery ordinal.
    next_delivery_ordinal: u64,
    /// Sticky EOF state for this active parser generation.
    end_of_input: bool,
    /// Sticky identifier-exhausted read state.
    identifier_exhausted: bool,
    /// Trusted output captured by deterministic backend tests.
    output_log: alloc::vec::Vec<alloc::string::String>,
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
    /// The deterministic backend contract currently exposes an 80x24 snapshot for
    /// in-process tests while preserving exact state rejection and non-mutating
    /// behavior.
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

    /// Return captured trusted output for deterministic tests.
    #[cfg(test)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec slice dereference is not const-stable"
    )]
    pub(crate) fn output_log_for_tests(&self) -> &[alloc::string::String] {
        &self.output_log
    }

    /// Return this session's stable readiness-source identity.
    #[must_use]
    pub fn readiness_source(&self) -> SystemReadinessSource {
        self.readiness_source.clone()
    }

    /// Read one normalized event or terminal status marker.
    ///
    /// # Errors
    ///
    /// Returns [`TerminalSessionReadError`] for sticky identifier exhaustion and
    /// [`TerminalSessionStateError`] before mutation for invalid states.
    pub fn read_event_sync(
        &mut self,
        wait: super::TerminalWait,
        cancellation: &CancellationToken,
    ) -> Result<TerminalInputEvent, TerminalReadEventError> {
        if self.identifier_exhausted {
            return Err(TerminalReadEventError::Read(identifier_exhausted_error()));
        }
        if self.state != TerminalSessionState::Active {
            return Err(TerminalReadEventError::State(state_error(
                self.state,
                TerminalOperation::Read,
            )));
        }
        if let Some(event) = self.event_queue.pop_front() {
            return Ok(event);
        }
        if self.end_of_input {
            return self.status_event(TerminalInputEventKind::EndOfInput);
        }
        if cancellation.is_cancelled() {
            return self.status_event(TerminalInputEventKind::Cancelled);
        }
        match wait {
            super::TerminalWait::Poll
            | super::TerminalWait::For { .. }
            | super::TerminalWait::Forever => self.status_event(TerminalInputEventKind::TimedOut),
        }
    }

    /// Write explicit trusted output.
    pub fn write_sync(
        &mut self,
        output: &TrustedTerminalOutput,
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::Write)?;
        self.output_log.push(output.as_str().to_owned());
        Ok(())
    }

    /// Write safe diagnostic output.
    pub fn write_diagnostic_sync(
        &mut self,
        output: &SafeTerminalDiagnosticOutput,
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::Write)?;
        self.output_log.push(output.as_str().to_owned());
        Ok(())
    }

    /// Flush session output.
    pub fn flush_sync(&self) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::Flush)
    }

    /// Clear the terminal screen through a trusted runtime-controlled sequence.
    pub fn clear_screen_sync(&mut self) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::TerminalClearScreen)?;
        self.output_log
            .push("\u{1b}[2J\u{1b}[3J\u{1b}[H".to_owned());
        Ok(())
    }

    /// Move the terminal cursor through a trusted runtime-controlled sequence.
    pub fn move_cursor_sync(
        &mut self,
        row: i32,
        column: i32,
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::TerminalMoveCursor)?;
        if row < 1_i32 || column < 1_i32 {
            return Err(TerminalWriteOperationError::InvalidCursorPosition {
                diagnostic: diagnostic(
                    TerminalOperation::TerminalMoveCursor,
                    TerminalSessionState::Active,
                ),
            });
        }
        self.output_log.push(format!("\u{1b}[{row};{column}H"));
        Ok(())
    }

    /// Draw trusted rows through the session output log.
    pub fn draw_rows_sync(
        &mut self,
        rows: &[TrustedTerminalOutput],
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::TerminalDrawRows)?;
        for row in rows {
            self.output_log.push(format!("{}\n", row.as_str()));
        }
        Ok(())
    }

    /// Ring the terminal bell through a trusted runtime-controlled sequence.
    pub fn bell_sync(&mut self) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::Write)?;
        self.output_log.push("\u{7}".to_owned());
        Ok(())
    }

    /// Set cursor visibility.
    pub fn set_cursor_visible_sync(
        &self,
        _visible: bool,
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::SetCursorVisibility)
    }

    /// Set cursor shape.
    pub fn set_cursor_shape_sync(
        &self,
        _shape: TerminalCursorShape,
    ) -> Result<(), TerminalWriteOperationError> {
        self.ensure_output_state(TerminalOperation::SetCursorShape)
    }

    /// Validate output operation state before mutation.
    fn ensure_output_state(
        &self,
        operation: TerminalOperation,
    ) -> Result<(), TerminalWriteOperationError> {
        match self.state {
            TerminalSessionState::Active => Ok(()),
            TerminalSessionState::Paused
            | TerminalSessionState::RestorePending
            | TerminalSessionState::Closed => Err(TerminalWriteOperationError::State(state_error(
                self.state, operation,
            ))),
        }
    }

    /// Pause the session and independently retain queued input plus a boundary.
    pub fn pause_sync(&mut self) -> Result<TerminalPauseResult, TerminalPauseError> {
        match self.state {
            TerminalSessionState::Active => {
                let mut events = self.event_queue.drain(..).collect::<alloc::vec::Vec<_>>();
                events.push(self.make_event(TerminalInputEventKind::InputReset {
                    reason: TerminalInputResetReason::PauseBoundary,
                })?);
                self.state = TerminalSessionState::Paused;
                self.readiness_source
                    .publish_transition(SourceAvailability::Idle);
                Ok(TerminalPauseResult {
                    events: TerminalPauseEvents::new_runtime(events),
                })
            }
            TerminalSessionState::Paused => Ok(TerminalPauseResult::default()),
            TerminalSessionState::RestorePending | TerminalSessionState::Closed => Err(
                TerminalPauseError::State(state_error(self.state, TerminalOperation::Pause)),
            ),
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
    pub fn close_sync(&mut self) -> Result<TerminalCloseOutcome, TerminalSessionRestoreError> {
        match self.state {
            TerminalSessionState::Active
            | TerminalSessionState::Paused
            | TerminalSessionState::RestorePending => {
                self.state = TerminalSessionState::Closed;
                self.cleanup_obligation = false;
                self.reserved_recovery_generation = 0;
                self.readiness_source
                    .publish_transition(SourceAvailability::Ready);
                let discarded_events = u64::try_from(self.event_queue.len()).unwrap_or(u64::MAX);
                self.event_queue.clear();
                if discarded_events == 0 {
                    Ok(TerminalCloseOutcome::Clean)
                } else {
                    Ok(TerminalCloseOutcome::DiscardedInput {
                        discarded_bytes: discarded_events,
                        discarded_events,
                    })
                }
            }
            TerminalSessionState::Closed => Ok(TerminalCloseOutcome::Clean),
        }
    }

    /// Build one runtime event and advance the sticky ordinal state.
    fn make_event(
        &mut self,
        kind: TerminalInputEventKind,
    ) -> Result<TerminalInputEvent, TerminalPauseError> {
        let ordinal = self.next_delivery_ordinal;
        if ordinal == u64::MAX {
            self.identifier_exhausted = true;
            return Err(TerminalPauseError::Read(identifier_exhausted_error()));
        }
        self.next_delivery_ordinal = self.next_delivery_ordinal.saturating_add(1);
        TerminalInputEvent::new_runtime(self.capabilities.hidden_stream_id(), ordinal, kind)
            .map_err(|_error| TerminalPauseError::Read(identifier_exhausted_error()))
    }

    /// Build one runtime status event.
    fn status_event(
        &mut self,
        kind: TerminalInputEventKind,
    ) -> Result<TerminalInputEvent, TerminalReadEventError> {
        self.make_event(kind).map_err(|error| match error {
            TerminalPauseError::Read(read_error) => TerminalReadEventError::Read(read_error),
            TerminalPauseError::State(state_error) => TerminalReadEventError::State(state_error),
        })
    }

    /// Queue a synthetic decoded event for deterministic tests.
    #[cfg(test)]
    pub(crate) fn enqueue_input_event_for_tests(
        &mut self,
        kind: TerminalInputEventKind,
    ) -> Result<(), TerminalSessionReadError> {
        let event = self.make_event(kind).map_err(|error| match error {
            TerminalPauseError::Read(read_error) => read_error,
            TerminalPauseError::State(_state_error) => identifier_exhausted_error(),
        })?;
        self.event_queue.push_back(event);
        self.readiness_source
            .publish_transition(SourceAvailability::Ready);
        Ok(())
    }

    /// Mark sticky end-of-input for deterministic tests.
    #[cfg(test)]
    pub(crate) fn mark_end_of_input_for_tests(&mut self) {
        self.end_of_input = true;
        self.readiness_source
            .publish_transition(SourceAvailability::Ready);
    }

    /// Force the next hidden delivery ordinal for deterministic tests.
    #[cfg(test)]
    pub(crate) const fn force_next_delivery_ordinal_for_tests(&mut self, ordinal: u64) {
        self.next_delivery_ordinal = ordinal;
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
        readiness_source: SystemReadinessSource::new(),
        event_queue: VecDeque::new(),
        next_delivery_ordinal: 1,
        end_of_input: false,
        identifier_exhausted: false,
        output_log: alloc::vec::Vec::new(),
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

/// Build a sticky identifier-exhausted error.
fn identifier_exhausted_error() -> TerminalSessionReadError {
    TerminalSessionReadError::IdentifierExhausted {
        diagnostic: diagnostic(TerminalOperation::Read, TerminalSessionState::Active),
    }
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
