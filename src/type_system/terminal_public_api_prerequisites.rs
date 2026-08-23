extern crate alloc;

use alloc::{collections::BTreeSet, format, string::String, vec::Vec};

/// Authoritative Task 13-22 prerequisite capabilities required before the
/// selected terminal public API may be imported outside the narrow inspector
/// exception path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TerminalPublicApiPrerequisite {
    /// Task 13 affine ownership and second-class borrow enforcement.
    AffineOwnership,
    /// Task 14 `using` cleanup and cleanup-authority transfer.
    CleanupTransfer,
    /// Task 15 nominal refinement and `constrain` support.
    RefinementAndConstrain,
    /// Task 16 immutable error attachments and inspectors.
    ImmutableErrors,
    /// Task 17 test-only availability and sealed runner authority.
    TestOnlyAuthority,
    /// Task 18 transactional affine aggregate support.
    TransactionalAffineAggregates,
    /// Task 19 wait-set and cancellation support.
    WaitCancellation,
    /// Task 20 monotonic timer support.
    Timers,
    /// Task 21 process-control support.
    ProcessControl,
    /// Task 22 terminal coordinator and legacy I/O coordination.
    TerminalCoordinatorAndLegacyIo,
}

impl TerminalPublicApiPrerequisite {
    /// Deterministically ordered authoritative Task 13-22 prerequisite list.
    pub const ALL: [Self; 10] = [
        Self::AffineOwnership,
        Self::CleanupTransfer,
        Self::RefinementAndConstrain,
        Self::ImmutableErrors,
        Self::TestOnlyAuthority,
        Self::TransactionalAffineAggregates,
        Self::WaitCancellation,
        Self::Timers,
        Self::ProcessControl,
        Self::TerminalCoordinatorAndLegacyIo,
    ];

    /// Human-readable prerequisite name used in Task 23 diagnostics.
    #[must_use]
    pub const fn diagnostic_name(self) -> &'static str {
        match self {
            Self::AffineOwnership => "affine ownership",
            Self::CleanupTransfer => "cleanup/transfer",
            Self::RefinementAndConstrain => "refinement/constrain",
            Self::ImmutableErrors => "immutable errors",
            Self::TestOnlyAuthority => "test-only authority",
            Self::TransactionalAffineAggregates => "transactional affine aggregates",
            Self::WaitCancellation => "wait/cancellation",
            Self::Timers => "timers",
            Self::ProcessControl => "process control",
            Self::TerminalCoordinatorAndLegacyIo => "terminal coordinator/legacy I/O coordination",
        }
    }
}

/// Mutable test/config view of which Task 13-22 prerequisite capabilities are enabled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalPublicApiPrerequisites {
    /// Set of prerequisite capabilities currently enabled for the gate.
    enabled: BTreeSet<TerminalPublicApiPrerequisite>,
}

impl Default for TerminalPublicApiPrerequisites {
    fn default() -> Self {
        Self {
            enabled: TerminalPublicApiPrerequisite::ALL.into_iter().collect(),
        }
    }
}

impl TerminalPublicApiPrerequisites {
    /// Return whether the complete selected-terminal prerequisite set is enabled.
    #[must_use]
    pub(crate) fn allows_selected_public_api(&self) -> bool {
        self.missing().is_empty()
    }

    /// Disable one prerequisite for focused gate validation tests.
    pub(crate) fn disable_for_tests(&mut self, prerequisite: TerminalPublicApiPrerequisite) {
        self.enabled.remove(&prerequisite);
    }

    /// Disable every prerequisite for focused bypass tests.
    pub(crate) fn clear_for_tests(&mut self) {
        self.enabled.clear();
    }

    /// Return the precise missing prerequisite list in deterministic order.
    #[must_use]
    pub(crate) fn missing(&self) -> Vec<TerminalPublicApiPrerequisite> {
        TerminalPublicApiPrerequisite::ALL
            .into_iter()
            .filter(|prerequisite| !self.enabled.contains(prerequisite))
            .collect()
    }

    /// Build the explicit import diagnostic reason naming each missing prerequisite.
    #[must_use]
    pub(crate) fn unavailable_reason(&self) -> String {
        let missing = self.missing_diagnostic_names();
        if missing.is_empty() {
            return String::from("terminal public API prerequisites are satisfied");
        }
        format!(
            "terminal public API prerequisite validation is missing: {}",
            missing.join(", ")
        )
    }

    /// Build the matching import diagnostic help text.
    #[must_use]
    pub(crate) fn unavailable_help(&self) -> String {
        let missing = self.missing_diagnostic_names();
        if missing.is_empty() {
            return String::from("No prerequisite action is required.");
        }
        format!(
            "Enable every Task 13-22 prerequisite before importing selected terminal/chord/core/test APIs. Missing prerequisite(s): {}.",
            missing.join(", ")
        )
    }

    /// Translate the current missing prerequisite set into user-facing labels.
    #[must_use]
    fn missing_diagnostic_names(&self) -> Vec<&'static str> {
        self.missing()
            .into_iter()
            .map(TerminalPublicApiPrerequisite::diagnostic_name)
            .collect()
    }
}
