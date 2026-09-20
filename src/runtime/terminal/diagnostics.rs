//! Terminal structured diagnostic values and diagnostic collection accounting.

extern crate alloc;

use super::constraints::{
    TerminalDiagnosticCollectionByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
};
use super::formatting::{collection_metadata_bytes, diagnostic_accounted_bytes};
use super::model::{
    TerminalBackend, TerminalCoordinatorState, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalOperation, TerminalOsCode,
};
use alloc::vec::Vec;
use core::fmt;

/// Immutable runtime-origin structured terminal diagnostic.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalDiagnostic {
    /// Stored backend classification.
    backend: TerminalBackend,
    /// Stored operation classification.
    operation: TerminalOperation,
    /// Stored lifecycle stage classification.
    stage: TerminalDiagnosticStage,
    /// Stored coordinator state classification.
    coordinator_state: TerminalCoordinatorState,
    /// Stored diagnostic session-state presence.
    session_state: TerminalDiagnosticSessionState,
    /// Stored operating-system code.
    os_code: TerminalOsCode,
    /// Stored bounded detail text.
    detail: TerminalDiagnosticDetail,
    /// Stored advisory retryability metadata.
    retryability: TerminalDiagnosticRetryability,
    /// Stored per-diagnostic truncation flag.
    was_truncated: bool,
    /// Stored production-accounting byte contribution.
    accounted_bytes: u64,
}

impl fmt::Debug for TerminalDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalDiagnostic")
            .field("backend", &self.backend)
            .field("operation", &self.operation)
            .field("stage", &self.stage)
            .field("coordinator_state", &self.coordinator_state)
            .field("session_state", &self.session_state)
            .field("os_code", &self.os_code)
            .field("detail", &self.detail)
            .field("retryability", &self.retryability)
            .field("was_truncated", &self.was_truncated)
            .finish_non_exhaustive()
    }
}

impl TerminalDiagnostic {
    /// Construct one immutable runtime-origin structured diagnostic.
    #[expect(
        clippy::too_many_arguments,
        reason = "diagnostic construction mirrors the sealed proposal field set"
    )]
    pub(crate) fn new_runtime(
        backend: TerminalBackend,
        operation: TerminalOperation,
        stage: TerminalDiagnosticStage,
        coordinator_state: TerminalCoordinatorState,
        session_state: TerminalDiagnosticSessionState,
        os_code: TerminalOsCode,
        detail: TerminalDiagnosticDetail,
        retryability: TerminalDiagnosticRetryability,
        was_truncated: bool,
    ) -> Self {
        let accounted_bytes = diagnostic_accounted_bytes(detail.as_str());
        Self {
            backend,
            operation,
            stage,
            coordinator_state,
            session_state,
            os_code,
            detail,
            retryability,
            was_truncated,
            accounted_bytes,
        }
    }

    /// Override accounted bytes in tests to exercise saturation behavior.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "test-only accounting override is used only by terminal data-model tests"
        )
    )]
    pub(crate) const fn with_accounted_bytes_for_tests(mut self, accounted_bytes: u64) -> Self {
        self.accounted_bytes = accounted_bytes;
        self
    }

    #[must_use]
    pub const fn backend(&self) -> TerminalBackend {
        self.backend
    }

    #[must_use]
    pub const fn operation(&self) -> TerminalOperation {
        self.operation
    }

    #[must_use]
    pub const fn stage(&self) -> TerminalDiagnosticStage {
        self.stage
    }

    #[must_use]
    pub const fn coordinator_state(&self) -> TerminalCoordinatorState {
        self.coordinator_state
    }

    #[must_use]
    pub const fn session_state(&self) -> TerminalDiagnosticSessionState {
        self.session_state
    }

    #[must_use]
    pub fn os_code(&self) -> TerminalOsCode {
        self.os_code.clone()
    }

    #[must_use]
    pub fn detail(&self) -> TerminalDiagnosticDetail {
        self.detail.clone()
    }

    #[must_use]
    pub const fn retryability(&self) -> TerminalDiagnosticRetryability {
        self.retryability
    }

    #[must_use]
    pub const fn was_truncated(&self) -> bool {
        self.was_truncated
    }

    /// Return this diagnostic's stored production-accounting byte contribution.
    pub(crate) const fn accounted_bytes(&self) -> u64 {
        self.accounted_bytes
    }
}

/// Immutable bounded ordered terminal diagnostic collection.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalDiagnosticCollection {
    /// Retained longest-prefix diagnostics.
    retained: Vec<TerminalDiagnostic>,
    /// Exact retained diagnostic count.
    retained_count: u64,
    /// Saturating omitted diagnostic count.
    omitted_count: u64,
    /// Exact retained production-accounted bytes.
    retained_bytes: u64,
    /// Saturating omitted production-accounted bytes.
    omitted_bytes: u64,
    /// Whether at least one complete diagnostic was omitted.
    was_truncated: bool,
}

impl fmt::Debug for TerminalDiagnosticCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalDiagnosticCollection")
            .field("retained", &self.retained)
            .field("retained_count", &self.retained_count)
            .field("omitted_count", &self.omitted_count)
            .field("retained_bytes", &self.retained_bytes)
            .field("omitted_bytes", &self.omitted_bytes)
            .field("was_truncated", &self.was_truncated)
            .finish()
    }
}

impl TerminalDiagnosticCollection {
    /// Retain the longest prefix of complete diagnostics fitting the configured limits.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "production backends construct diagnostic collections after lifecycle/backend tasks land"
        )
    )]
    pub(crate) fn new_runtime(
        diagnostics: Vec<TerminalDiagnostic>,
        limits: TerminalDiagnosticCollectionLimits,
    ) -> Self {
        let metadata_bytes = collection_metadata_bytes();
        let count_limit =
            u64::try_from(limits.maximum_diagnostics.get()).expect("positive i32 always fits u64");
        let byte_limit = u64::try_from(limits.maximum_diagnostic_bytes.get())
            .expect("positive i32 always fits u64");
        let mut retained = Vec::new();
        let mut retained_count = 0_u64;
        let mut retained_bytes = metadata_bytes;
        let mut omitted_count = 0_u64;
        let mut omitted_bytes = 0_u64;
        let mut omitting = false;
        let mut was_truncated = false;

        for diagnostic in diagnostics {
            let diagnostic_bytes = diagnostic.accounted_bytes();
            let prospective_count = retained_count.checked_add(1);
            let prospective_bytes = retained_bytes.checked_add(diagnostic_bytes);
            let fits = !omitting
                && prospective_count.is_some_and(|value| value <= count_limit)
                && prospective_bytes.is_some_and(|value| value <= byte_limit);
            if fits {
                retained_count = prospective_count.expect("checked above");
                retained_bytes = prospective_bytes.expect("checked above");
                retained.push(diagnostic);
                continue;
            }
            omitting = true;
            was_truncated = true;
            omitted_count = omitted_count.saturating_add(1);
            omitted_bytes = omitted_bytes.saturating_add(diagnostic_bytes);
        }

        Self {
            retained,
            retained_count,
            omitted_count,
            retained_bytes,
            omitted_bytes,
            was_truncated,
        }
    }

    #[must_use]
    pub fn len(&self) -> i64 {
        i64::try_from(self.retained.len()).unwrap_or(i64::MAX)
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.retained_count == 0
    }

    #[must_use]
    pub fn at(&self, index: i64) -> Option<TerminalDiagnostic> {
        usize::try_from(index)
            .ok()
            .and_then(|value| self.retained.get(value).cloned())
    }

    #[must_use]
    pub const fn retained_count(&self) -> u64 {
        self.retained_count
    }

    #[must_use]
    pub const fn omitted_count(&self) -> u64 {
        self.omitted_count
    }

    #[must_use]
    pub const fn retained_bytes(&self) -> u64 {
        self.retained_bytes
    }

    #[must_use]
    pub const fn omitted_bytes(&self) -> u64 {
        self.omitted_bytes
    }

    #[must_use]
    pub const fn was_truncated(&self) -> bool {
        self.was_truncated
    }

    #[must_use]
    pub fn retained(&self) -> &[TerminalDiagnostic] {
        self.retained.as_slice()
    }
}

/// Production-valid collection limits used by runtime and test-only factories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalDiagnosticCollectionLimits {
    pub maximum_diagnostics: TerminalDiagnosticCountLimit,
    pub maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit,
}
