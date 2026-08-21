//! Process-global coordination state for legacy terminal and standard I/O.
//!
//! This module models ownership and validation only. It deliberately performs no
//! standard-input reads or standard-output writes.

#![expect(
    dead_code,
    reason = "Task 22 coordinator scaffold is consumed by later I/O slices"
)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use core::fmt;
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Process-global terminal ownership state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerminalCoordinatorState {
    /// No terminal session owns the process terminal.
    Free,
    /// A session is reserving terminal ownership.
    Opening,
    /// A session owns the terminal.
    Active,
    /// A session temporarily relinquished terminal interaction.
    Paused,
    /// A session is restoring terminal state during close.
    RestorePending,
    /// Opening failed and recovery is required.
    FailedOpenRecovery,
    /// Closing failed and recovery is required.
    FailedCloseRecovery,
}

/// Classification for a legacy terminal or standard I/O operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TerminalOperation {
    /// Read standard input.
    TakeInput,
    /// Write diagnostic text.
    PrintText,
    /// Flush standard output.
    FlushStandardOutput,
    /// Acquire a standard-output writer.
    StdoutWriter,
    /// Write through a standard-output writer.
    WriterWrite,
    /// Flush a standard-output writer.
    WriterFlush,
    /// Acquire a terminal handle.
    StdoutTerminal,
    /// Inspect ANSI support.
    TerminalSupportsAnsi,
    /// Enable or disable clear-screen behavior.
    TerminalClearScreenOn,
    /// Enable or disable cursor movement behavior.
    TerminalMoveCursorOn,
    /// Draw terminal rows.
    TerminalDrawRows,
    /// Clear the terminal screen.
    TerminalClearScreen,
    /// Move the terminal cursor.
    TerminalMoveCursor,
}

/// Rejection returned before a legacy operation can touch host I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct TerminalCoordinatorUnavailable {
    /// State observed when the operation was rejected.
    state: TerminalCoordinatorState,
    /// Operation that was rejected.
    operation: TerminalOperation,
}

/// Private failures from attempting to reserve terminal ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalCoordinatorReservationError {
    /// The hidden epoch has no representable successor.
    EpochExhausted,
    /// The coordinator is not available for reservation.
    Unavailable(TerminalCoordinatorUnavailable),
}

/// Opaque lease bound to one coordinator epoch.
#[derive(Clone, Copy, PartialEq, Eq)]
struct TerminalLease {
    /// Hidden epoch used to validate this lease.
    epoch: LeaseEpoch,
}

impl fmt::Debug for TerminalLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TerminalLease { epoch: <redacted> }")
    }
}

/// Hidden, never-reused lease identity.
#[derive(Clone, Copy, PartialEq, Eq)]
struct LeaseEpoch(u64);

impl fmt::Debug for LeaseEpoch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// Mutable process-global coordinator model.
#[derive(Debug)]
struct TerminalCoordinator {
    /// Current process-global ownership state.
    state: TerminalCoordinatorState,
    /// Current hidden, never-reused lease epoch.
    epoch: LeaseEpoch,
}

impl Default for TerminalCoordinator {
    fn default() -> Self {
        Self {
            state: TerminalCoordinatorState::Free,
            epoch: LeaseEpoch(0),
        }
    }
}

impl TerminalCoordinator {
    /// Acquire a lease for a legacy operation while the coordinator is free.
    fn acquire_lease(
        &self,
        operation: TerminalOperation,
    ) -> Result<TerminalLease, TerminalCoordinatorUnavailable> {
        if self.state == TerminalCoordinatorState::Free {
            Ok(TerminalLease { epoch: self.epoch })
        } else {
            Err(self.unavailable(operation))
        }
    }

    /// Acquire an opaque lease for a standard-output writer.
    fn acquire_stdout_writer(&self) -> Result<TerminalLease, TerminalCoordinatorUnavailable> {
        self.acquire_lease(TerminalOperation::StdoutWriter)
    }

    /// Acquire an opaque lease for a standard-output terminal.
    fn acquire_stdout_terminal(&self) -> Result<TerminalLease, TerminalCoordinatorUnavailable> {
        self.acquire_lease(TerminalOperation::StdoutTerminal)
    }

    /// Acquire a lease for the current free epoch in unit tests.
    #[cfg(test)]
    fn acquire_free_lease(&self) -> Result<TerminalLease, TerminalCoordinatorUnavailable> {
        self.acquire_stdout_terminal()
    }

    /// Reserve the current free epoch and invalidate all earlier leases.
    fn reserve_opening(&mut self) -> Result<TerminalLease, TerminalCoordinatorReservationError> {
        if self.state != TerminalCoordinatorState::Free {
            return Err(TerminalCoordinatorReservationError::Unavailable(
                self.unavailable(TerminalOperation::StdoutTerminal),
            ));
        }
        let next_epoch = self
            .epoch
            .0
            .checked_add(1)
            .ok_or(TerminalCoordinatorReservationError::EpochExhausted)?;
        self.epoch = LeaseEpoch(next_epoch);
        self.state = TerminalCoordinatorState::Opening;
        Ok(TerminalLease { epoch: self.epoch })
    }

    /// Return the coordinator to free without restoring an old epoch.
    const fn return_free(&mut self) {
        self.state = TerminalCoordinatorState::Free;
    }

    /// Set a state for deterministic state-machine tests.
    #[cfg(test)]
    const fn set_state(&mut self, state: TerminalCoordinatorState) {
        self.state = state;
    }

    /// Validate a legacy operation against the coordinator state.
    fn validate_operation(
        &self,
        operation: TerminalOperation,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        if self.state == TerminalCoordinatorState::Free {
            Ok(())
        } else {
            Err(self.unavailable(operation))
        }
    }

    /// Validate a lease and operation without touching host I/O.
    fn validate_lease(
        &self,
        lease: TerminalLease,
        operation: TerminalOperation,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        if self.state == TerminalCoordinatorState::Free && lease.epoch == self.epoch {
            Ok(())
        } else {
            Err(self.unavailable(operation))
        }
    }

    /// Validate a writer lease before a writer write operation.
    fn validate_writer_write(
        &self,
        lease: TerminalLease,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        self.validate_lease(lease, TerminalOperation::WriterWrite)
    }

    /// Validate a writer lease before a writer flush operation.
    fn validate_writer_flush(
        &self,
        lease: TerminalLease,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        self.validate_lease(lease, TerminalOperation::WriterFlush)
    }

    /// Validate a terminal lease before a terminal capability inspection.
    fn validate_terminal_supports_ansi(
        &self,
        lease: TerminalLease,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        self.validate_lease(lease, TerminalOperation::TerminalSupportsAnsi)
    }

    /// Validate a terminal lease before a terminal mutation operation.
    fn validate_terminal_operation(
        &self,
        lease: TerminalLease,
        operation: TerminalOperation,
    ) -> Result<(), TerminalCoordinatorUnavailable> {
        self.validate_lease(lease, operation)
    }

    /// Build the immutable state/operation rejection payload.
    const fn unavailable(&self, operation: TerminalOperation) -> TerminalCoordinatorUnavailable {
        TerminalCoordinatorUnavailable {
            state: self.state,
            operation,
        }
    }
}

/// Process-global terminal coordinator storage.
static COORDINATOR: OnceLock<Mutex<TerminalCoordinator>> = OnceLock::new();

/// Access the process-global coordinator while recovering poisoned test locks.
fn global() -> MutexGuard<'static, TerminalCoordinator> {
    COORDINATOR
        .get_or_init(|| Mutex::new(TerminalCoordinator::default()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Validate a process-global legacy operation before host I/O is touched.
pub(super) fn validate_global_operation(
    operation: TerminalOperation,
) -> Result<(), TerminalCoordinatorUnavailable> {
    #[cfg(test)]
    if let Some(state) = TEST_STATE_OVERRIDE.with(std::cell::Cell::get) {
        return TerminalCoordinator {
            state,
            epoch: LeaseEpoch(0),
        }
        .validate_operation(operation);
    }
    global().validate_operation(operation)
}

/// Maximum encoded diagnostic-lane payload before truncation.
pub const DIAGNOSTIC_MAX_BYTES: usize = 256;

/// Return whether legacy diagnostics may use the bounded diagnostic lane.
pub fn diagnostic_lane_allows() -> bool {
    #[cfg(test)]
    if let Some(state) = TEST_STATE_OVERRIDE.with(std::cell::Cell::get) {
        return matches!(
            state,
            TerminalCoordinatorState::Free
                | TerminalCoordinatorState::Active
                | TerminalCoordinatorState::Paused
        );
    }
    matches!(
        global().state,
        TerminalCoordinatorState::Free
            | TerminalCoordinatorState::Active
            | TerminalCoordinatorState::Paused
    )
}

/// Escape hazardous scalars and bound a legacy diagnostic payload.
pub fn sanitize_diagnostic(value: &str) -> String {
    const TRUNCATION_MARKER: &str = "...[truncated]";
    let mut encoded = String::new();
    for character in value.chars() {
        let escaped = escaped_diagnostic_scalar(character);
        let Some(required_bytes) = encoded
            .len()
            .checked_add(escaped.len())
            .and_then(|length| length.checked_add(TRUNCATION_MARKER.len()))
        else {
            encoded.push_str(TRUNCATION_MARKER);
            break;
        };
        if required_bytes > DIAGNOSTIC_MAX_BYTES {
            encoded.push_str(TRUNCATION_MARKER);
            break;
        }
        encoded.push_str(&escaped);
    }
    encoded
}

/// Return an ASCII-visible representation for a hazardous scalar.
fn escaped_diagnostic_scalar(character: char) -> String {
    let code = u32::from(character);
    let hazardous_control = code <= 0x1F || (0x7F..=0x9F).contains(&code);
    let bidi_control = matches!(
        code,
        0x061C | 0x200E..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0x2066..=0x206F
    );
    let noncharacter = (0xFDD0..=0xFDEF).contains(&code) || (code & 0xFFFF) >= 0xFFFE;
    if hazardous_control {
        format!("\\x{code:02X}")
    } else if bidi_control || noncharacter {
        format!("\\u{{{code:X}}}")
    } else {
        character.to_string()
    }
}

// Thread-local override avoids cross-test mutation of the process-global state.
#[cfg(test)]
thread_local! {
    static TEST_STATE_OVERRIDE: std::cell::Cell<Option<TerminalCoordinatorState>> = const { std::cell::Cell::new(None) };
}

#[cfg(test)]
pub struct TestStateGuard {
    /// State to restore when the guard is dropped.
    prior_state: Option<TerminalCoordinatorState>,
}

#[cfg(test)]
impl Drop for TestStateGuard {
    fn drop(&mut self) {
        TEST_STATE_OVERRIDE.with(|override_state| override_state.set(self.prior_state));
    }
}

/// Temporarily override coordinator state for the current runtime test thread.
#[cfg(test)]
pub fn set_state_for_tests(state: TerminalCoordinatorState) -> TestStateGuard {
    let prior_state =
        TEST_STATE_OVERRIDE.with(|override_state| override_state.replace(Some(state)));
    TestStateGuard { prior_state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_state_operation_validates() {
        let coordinator = TerminalCoordinator::default();
        assert_eq!(
            coordinator.validate_operation(TerminalOperation::TakeInput),
            Ok(())
        );
    }

    #[test]
    fn non_free_state_rejects_with_state_and_operation() {
        let mut coordinator = TerminalCoordinator::default();
        coordinator.set_state(TerminalCoordinatorState::Active);
        assert_eq!(
            coordinator.validate_operation(TerminalOperation::WriterWrite),
            Err(TerminalCoordinatorUnavailable {
                state: TerminalCoordinatorState::Active,
                operation: TerminalOperation::WriterWrite,
            })
        );
    }

    #[test]
    fn writer_lease_validates_write_and_flush_in_free_state() {
        let coordinator = TerminalCoordinator::default();
        let lease = coordinator
            .acquire_stdout_writer()
            .expect("writer acquisition should succeed while free");
        assert_eq!(coordinator.validate_writer_write(lease), Ok(()));
        assert_eq!(coordinator.validate_writer_flush(lease), Ok(()));
    }

    #[test]
    fn terminal_lease_validates_capability_and_mutation_in_free_state() {
        let coordinator = TerminalCoordinator::default();
        let lease = coordinator
            .acquire_stdout_terminal()
            .expect("terminal acquisition should succeed while free");
        assert_eq!(coordinator.validate_terminal_supports_ansi(lease), Ok(()));
        assert_eq!(
            coordinator.validate_terminal_operation(lease, TerminalOperation::TerminalClearScreen,),
            Ok(())
        );
    }

    #[test]
    fn lease_acquisition_rejects_with_requested_operation() {
        let mut coordinator = TerminalCoordinator::default();
        coordinator.set_state(TerminalCoordinatorState::Opening);
        assert_eq!(
            coordinator.acquire_stdout_writer(),
            Err(TerminalCoordinatorUnavailable {
                state: TerminalCoordinatorState::Opening,
                operation: TerminalOperation::StdoutWriter,
            })
        );
        coordinator.set_state(TerminalCoordinatorState::Active);
        assert_eq!(
            coordinator.acquire_stdout_terminal(),
            Err(TerminalCoordinatorUnavailable {
                state: TerminalCoordinatorState::Active,
                operation: TerminalOperation::StdoutTerminal,
            })
        );
    }

    #[test]
    fn writer_and_terminal_leases_stale_after_opening_reservation() {
        let mut coordinator = TerminalCoordinator::default();
        let writer = coordinator
            .acquire_stdout_writer()
            .expect("writer acquisition should succeed");
        let terminal = coordinator
            .acquire_stdout_terminal()
            .expect("terminal acquisition should succeed");

        coordinator.reserve_opening().expect("opening reservation");
        coordinator.return_free();

        assert_eq!(
            coordinator.validate_writer_write(writer),
            Err(TerminalCoordinatorUnavailable {
                state: TerminalCoordinatorState::Free,
                operation: TerminalOperation::WriterWrite,
            })
        );
        assert_eq!(
            coordinator.validate_terminal_supports_ansi(terminal),
            Err(TerminalCoordinatorUnavailable {
                state: TerminalCoordinatorState::Free,
                operation: TerminalOperation::TerminalSupportsAnsi,
            })
        );

        let fresh_writer = coordinator
            .acquire_stdout_writer()
            .expect("fresh writer acquisition should succeed");
        let fresh_terminal = coordinator
            .acquire_stdout_terminal()
            .expect("fresh terminal acquisition should succeed");
        assert_eq!(coordinator.validate_writer_flush(fresh_writer), Ok(()));
        assert_eq!(
            coordinator.validate_terminal_operation(
                fresh_terminal,
                TerminalOperation::TerminalMoveCursor,
            ),
            Ok(())
        );
    }

    #[test]
    fn opening_invalidates_prior_lease_forever() {
        let mut coordinator = TerminalCoordinator::default();
        let old_lease = coordinator.acquire_free_lease().expect("free-state lease");
        coordinator.reserve_opening().expect("first reservation");
        coordinator.return_free();
        coordinator
            .validate_lease(old_lease, TerminalOperation::WriterFlush)
            .expect_err("old lease must be stale");
        let new_lease = coordinator.reserve_opening().expect("second reservation");
        coordinator.return_free();
        assert_eq!(
            coordinator.validate_lease(new_lease, TerminalOperation::WriterFlush),
            Ok(())
        );
    }

    #[test]
    fn epoch_exhaustion_preserves_state_and_epoch() {
        let mut coordinator = TerminalCoordinator {
            epoch: LeaseEpoch(u64::MAX - 1),
            ..TerminalCoordinator::default()
        };

        coordinator
            .reserve_opening()
            .expect("the final representable epoch increment");
        assert_eq!(coordinator.epoch, LeaseEpoch(u64::MAX));
        coordinator.return_free();

        let state_before_retry = coordinator.state;
        let epoch_before_retry = coordinator.epoch;
        assert_eq!(
            coordinator.reserve_opening(),
            Err(TerminalCoordinatorReservationError::EpochExhausted)
        );
        assert_eq!(coordinator.state, state_before_retry);
        assert_eq!(coordinator.epoch, epoch_before_retry);
    }

    #[test]
    fn debug_redacts_epoch_and_lease() {
        let mut coordinator = TerminalCoordinator::default();
        let lease = coordinator.reserve_opening().expect("reservation");
        let debug = format!("{coordinator:?} {lease:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("epoch: 1"));
        assert!(!debug.contains("LeaseEpoch(1)"));
    }

    #[test]
    fn all_proposal_operations_are_classified() {
        let operations = [
            TerminalOperation::TakeInput,
            TerminalOperation::PrintText,
            TerminalOperation::FlushStandardOutput,
            TerminalOperation::StdoutWriter,
            TerminalOperation::WriterWrite,
            TerminalOperation::WriterFlush,
            TerminalOperation::StdoutTerminal,
            TerminalOperation::TerminalSupportsAnsi,
            TerminalOperation::TerminalClearScreenOn,
            TerminalOperation::TerminalMoveCursorOn,
            TerminalOperation::TerminalDrawRows,
            TerminalOperation::TerminalClearScreen,
            TerminalOperation::TerminalMoveCursor,
        ];
        assert_eq!(operations.len(), 13);
    }

    #[test]
    fn global_coordinator_is_available_without_io() {
        let state = {
            let coordinator = global();
            coordinator.state
        };
        assert_eq!(state, TerminalCoordinatorState::Free);
    }
}
