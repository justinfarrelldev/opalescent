//! Test-only terminal factories, fake backend scenarios, and activation scopes.
//!
//! These helpers intentionally preserve production invariants: factories can
//! create ordinary sealed data-model values for deterministic tests, but cannot
//! mint sessions, recovery tokens, coordinator leases, host handles, or terminal
//! stop/cancel sentinel authority outside the normal lifecycle paths.

extern crate alloc;

use super::constraints::{
    TerminalCommittedText, TerminalCompositionId, TerminalCorrelatedEventLimit,
    TerminalDiagnosticCollectionByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
    TerminalEventId, TerminalKeyRepeatCount,
};
use super::diagnostics::{TerminalDiagnostic, TerminalDiagnosticCollectionLimits};
use super::model::{
    TerminalBackend, TerminalCapabilities, TerminalCapabilitySupportedEvidence,
    TerminalColorCapability, TerminalCoordinatorState, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalFeatureCapability,
    TerminalInputEvent, TerminalInputEventKind, TerminalKeyOccurrence, TerminalLogicalKey,
    TerminalModifiers, TerminalOperation, TerminalOrdinaryFeature, TerminalOsCode,
    TerminalTextInputOrigin, TerminalTrustedPasteCapability, TerminalTrustedPasteEvidence,
    next_hidden_stream_id,
};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::{Cell, RefCell};
use core::fmt;

/// Test-only factory and activation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalTestFactoryError {
    /// A hidden stream or scenario identity did not match.
    WrongScenario,
    /// A second activation tried to bind the same scenario on this task.
    AlreadyActive,
    /// A deactivation guard observed a non-LIFO activation order.
    ActivationOrderViolation,
    /// Delivery ordinals are exhausted and cannot be reused.
    DeliveryOrdinalExhausted,
    /// The requested fault is not part of the deterministic fake-backend plan.
    FaultNotPlanned,
    /// A constrained terminal value rejected factory input.
    InvalidValue {
        type_name: &'static str,
        reason: String,
    },
}

impl fmt::Display for TerminalTestFactoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongScenario => {
                f.write_str("terminal test value belongs to a different scenario")
            }
            Self::AlreadyActive => {
                f.write_str("terminal test scenario is already active on this task")
            }
            Self::ActivationOrderViolation => {
                f.write_str("terminal test activation scopes must exit in LIFO order")
            }
            Self::DeliveryOrdinalExhausted => {
                f.write_str("terminal test delivery ordinal exhausted")
            }
            Self::FaultNotPlanned => f.write_str("terminal test fault was not planned"),
            Self::InvalidValue {
                type_name,
                ref reason,
            } => write!(f, "{type_name} rejected terminal test value: {reason}"),
        }
    }
}

impl std::error::Error for TerminalTestFactoryError {}

impl From<super::constraints::TerminalConstraintError> for TerminalTestFactoryError {
    fn from(value: super::constraints::TerminalConstraintError) -> Self {
        Self::InvalidValue {
            type_name: value.type_name,
            reason: value.reason,
        }
    }
}

/// Deterministic test scenario authority for one hidden terminal stream.
#[derive(Debug)]
pub struct TerminalTestScenario {
    /// Hidden stream identity shared by values created from this scenario.
    stream_id: u64,
    /// Next delivery ordinal; zero is never published.
    next_delivery_ordinal: Cell<u64>,
    /// Planned backend fault points.
    fault_plan: RefCell<Vec<TerminalTestFault>>,
}

impl TerminalTestScenario {
    /// Create a new deterministic scenario with fresh hidden stream identity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            stream_id: next_hidden_stream_id(),
            next_delivery_ordinal: Cell::new(1),
            fault_plan: RefCell::new(Vec::new()),
        }
    }

    /// Create a scenario with the next ordinal forced near exhaustion for tests.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn with_next_delivery_ordinal_for_tests(next_delivery_ordinal: u64) -> Self {
        Self {
            stream_id: next_hidden_stream_id(),
            next_delivery_ordinal: Cell::new(next_delivery_ordinal),
            fault_plan: RefCell::new(Vec::new()),
        }
    }

    /// Hidden stream identity, visible only to runtime/test infrastructure.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "hidden scenario identity is asserted by task-31 tests"
        )
    )]
    pub(crate) const fn hidden_stream_id(&self) -> u64 {
        self.stream_id
    }

    /// Construct a key press event with scenario provenance and unique ordinal.
    pub fn key_press(
        &self,
        event_id: i32,
        text: &str,
    ) -> Result<TerminalInputEvent, TerminalTestFactoryError> {
        let event_id = TerminalEventId::new_runtime(u64::try_from(event_id).map_err(|_err| {
            TerminalTestFactoryError::InvalidValue {
                type_name: "TerminalEventId",
                reason: "expected value >= 1".to_owned(),
            }
        })?)?;
        let text = TerminalCommittedText::new_runtime(text.to_owned())?;
        let repeat = TerminalKeyRepeatCount::new(1)?;
        self.next_event(TerminalInputEventKind::Key {
            event_id,
            key: TerminalLogicalKey::Text { logical_text: text },
            occurrence: TerminalKeyOccurrence::Press { count: repeat },
            modifiers: TerminalModifiers {
                shift: false,
                control: false,
                alt: false,
                super_key: false,
                caps_lock: false,
                num_lock: false,
            },
        })
    }

    /// Construct text input linked to a key event from the same stream.
    pub fn linked_text(
        &self,
        source: &TerminalInputEvent,
        text: &str,
    ) -> Result<TerminalInputEvent, TerminalTestFactoryError> {
        if source.hidden_stream_id() != self.stream_id {
            return Err(TerminalTestFactoryError::WrongScenario);
        }
        let &TerminalInputEventKind::Key { event_id, .. } = source.kind() else {
            return Err(TerminalTestFactoryError::WrongScenario);
        };
        let text = TerminalCommittedText::new_runtime(text.to_owned())?;
        self.next_event(TerminalInputEventKind::TextInput {
            text,
            origin: TerminalTextInputOrigin::Key { event_id },
            linked_phase: super::model::TerminalLinkedTextPhase::Complete,
        })
    }

    /// Construct a composition-start event with hidden scenario provenance.
    pub fn composition_started(
        &self,
        composition_id: i32,
    ) -> Result<TerminalInputEvent, TerminalTestFactoryError> {
        let composition_id =
            TerminalCompositionId::new_runtime(u64::try_from(composition_id).map_err(|_err| {
                TerminalTestFactoryError::InvalidValue {
                    type_name: "TerminalCompositionId",
                    reason: "expected value >= 1".to_owned(),
                }
            })?)?;
        self.next_event(TerminalInputEventKind::CompositionStarted { composition_id })
    }

    /// Construct a deterministic capability snapshot for this scenario.
    pub fn capabilities(
        &self,
        features: &[TerminalOrdinaryFeature],
    ) -> Result<TerminalCapabilities, TerminalTestFactoryError> {
        let mut ordinary = BTreeMap::new();
        for feature in features {
            if ordinary
                .insert(
                    *feature,
                    TerminalFeatureCapability::Enabled {
                        evidence: TerminalCapabilitySupportedEvidence::ProtocolAssumed,
                    },
                )
                .is_some()
            {
                return Err(TerminalTestFactoryError::InvalidValue {
                    type_name: "TerminalCapabilities",
                    reason: "duplicate ordinary feature".to_owned(),
                });
            }
        }
        Ok(TerminalCapabilities::new_runtime(
            ordinary,
            TerminalTrustedPasteCapability::Enabled {
                evidence: TerminalTrustedPasteEvidence::SanitizedProtocolBoundary,
            },
            TerminalColorCapability::TrueColor {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolAssumed,
            },
            self.stream_id,
            TerminalCorrelatedEventLimit::new(64)?,
        ))
    }

    /// Construct a deterministic diagnostic from public enum payloads.
    pub fn diagnostic(&self, detail: &str) -> Result<TerminalDiagnostic, TerminalTestFactoryError> {
        let detail = TerminalDiagnosticDetail::new_runtime(detail.to_owned())?;
        Ok(TerminalDiagnostic::new_runtime(
            TerminalBackend::VtStream,
            TerminalOperation::Read,
            TerminalDiagnosticStage::Decode,
            TerminalCoordinatorState::Active,
            TerminalDiagnosticSessionState::Active,
            TerminalOsCode::Unavailable,
            detail,
            TerminalDiagnosticRetryability::NonRetryable,
            false,
        ))
    }

    /// Construct production-accounted diagnostic collection limits.
    pub fn diagnostic_collection_limits(
        maximum_diagnostics: i32,
        maximum_bytes: i32,
    ) -> Result<TerminalDiagnosticCollectionLimits, TerminalTestFactoryError> {
        Ok(TerminalDiagnosticCollectionLimits {
            maximum_diagnostics: TerminalDiagnosticCountLimit::new(maximum_diagnostics)?,
            maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit::new(maximum_bytes)?,
        })
    }

    /// Add one deterministic backend fault to this scenario's plan.
    pub fn plan_fault(&self, fault: TerminalTestFault) {
        self.fault_plan.borrow_mut().push(fault);
    }

    /// Consume a matching deterministic backend fault.
    pub fn consume_fault(&self, fault: TerminalTestFault) -> Result<(), TerminalTestFactoryError> {
        let mut faults = self.fault_plan.borrow_mut();
        let Some(position) = faults.iter().position(|planned| *planned == fault) else {
            return Err(TerminalTestFactoryError::FaultNotPlanned);
        };
        faults.remove(position);
        Ok(())
    }

    /// Build an input event with a fresh hidden delivery ordinal.
    fn next_event(
        &self,
        kind: TerminalInputEventKind,
    ) -> Result<TerminalInputEvent, TerminalTestFactoryError> {
        let ordinal = self.next_delivery_ordinal.get();
        if ordinal == 0 {
            return Err(TerminalTestFactoryError::DeliveryOrdinalExhausted);
        }
        let next = ordinal
            .checked_add(1)
            .ok_or(TerminalTestFactoryError::DeliveryOrdinalExhausted)?;
        let event = TerminalInputEvent::new_runtime(self.stream_id, ordinal, kind)?;
        self.next_delivery_ordinal.set(next);
        Ok(event)
    }
}

impl Default for TerminalTestScenario {
    fn default() -> Self {
        Self::new()
    }
}

/// Fake backend lifecycle fault point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTestFault {
    /// Fail after open mutated backend state and trigger ordinary rollback path.
    OpenAfterMutation,
    /// Fail one inverse restore step during close.
    CloseInverseRestore,
}

thread_local! {
    /// Task-local activation stack; tests use one Rust thread as one task.
    static ACTIVE_SCENARIOS: RefCell<Vec<u64>> = const { RefCell::new(Vec::new()) };
}

/// Guard for a task-local fake backend activation scope.
#[derive(Debug)]
pub struct TerminalTestActivation {
    /// Activated hidden stream identity.
    stream_id: u64,
    /// Whether drop has already deactivated this guard.
    active: bool,
}

impl TerminalTestActivation {
    /// Activate one scenario for this task, preserving LIFO nesting.
    pub fn activate(scenario: &TerminalTestScenario) -> Result<Self, TerminalTestFactoryError> {
        ACTIVE_SCENARIOS.with(|stack| {
            let mut stack = stack.borrow_mut();
            if stack.contains(&scenario.stream_id) {
                return Err(TerminalTestFactoryError::AlreadyActive);
            }
            stack.push(scenario.stream_id);
            Ok(Self {
                stream_id: scenario.stream_id,
                active: true,
            })
        })
    }

    /// Return whether the given scenario is currently top-of-stack active.
    #[must_use]
    pub fn is_current(scenario: &TerminalTestScenario) -> bool {
        ACTIVE_SCENARIOS.with(|stack| {
            stack
                .borrow()
                .last()
                .is_some_and(|active| *active == scenario.stream_id)
        })
    }

    /// Explicitly deactivate this scope and validate LIFO order.
    pub fn deactivate(mut self) -> Result<(), TerminalTestFactoryError> {
        self.deactivate_inner()
    }

    /// Shared deactivation implementation used by explicit close and Drop.
    fn deactivate_inner(&mut self) -> Result<(), TerminalTestFactoryError> {
        if !self.active {
            return Ok(());
        }
        ACTIVE_SCENARIOS.with(|stack| {
            let mut stack = stack.borrow_mut();
            if stack.pop() != Some(self.stream_id) {
                return Err(TerminalTestFactoryError::ActivationOrderViolation);
            }
            self.active = false;
            Ok(())
        })
    }
}

impl Drop for TerminalTestActivation {
    fn drop(&mut self) {
        drop(self.deactivate_inner());
    }
}
