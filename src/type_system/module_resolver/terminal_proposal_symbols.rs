//! Gated terminal proposal function symbol tables.
//!
//! These signatures intentionally stop at type-check/module-resolution metadata.
//! They do not imply code generation, runtime lowering, or public availability.

use super::ModuleInterface;
use crate::{
    token::{Position, Span},
    type_system::{
        symbol_table::{SymbolInfo, SymbolType, Visibility},
        types::CoreType,
    },
};

/// Compact type reference used by terminal proposal signature specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ApiTypeRef {
    /// A primitive or nominal type name.
    Named(&'static str),
    /// An array of primitive or nominal values.
    Array(&'static str),
}

/// One proposal function signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TerminalApiFunctionSpec {
    /// Exported function name.
    pub name: &'static str,
    /// Parameter types in declaration order.
    pub parameters: &'static [ApiTypeRef],
    /// Single return type; `void` maps to unit.
    pub return_type: ApiTypeRef,
    /// Declared nominal error families in declaration order.
    pub errors: &'static [&'static str],
}

/// Build a primitive or nominal type reference.
const fn named(name: &'static str) -> ApiTypeRef {
    ApiTypeRef::Named(name)
}

/// Build an array type reference.
const fn array(element: &'static str) -> ApiTypeRef {
    ApiTypeRef::Array(element)
}

/// Empty parameter list shared by nullary proposal functions.
const EMPTY: &[ApiTypeRef] = &[];
/// Empty error list shared by infallible proposal functions.
const NO_ERRORS: &[&str] = &[];
/// Common allocation failure error list.
const ALLOCATION_FAILURE: &[&str] = &["AllocationFailureError"];
/// Common indexed-access failure error list.
const INDEX_OUT_OF_BOUNDS: &[&str] = &["IndexOutOfBoundsError"];
/// Common test factory constraint/factory failure list.
const CONSTRAINT_AND_FACTORY: &[&str] = &["ConstraintViolationError", "TerminalTestFactoryError"];

/// Future core/system prerequisite functions required by the terminal proposal.
pub(super) const CORE_PREREQUISITE_FUNCTIONS: &[TerminalApiFunctionSpec] = &[
    TerminalApiFunctionSpec {
        name: "system_wait_set_new",
        parameters: EMPTY,
        return_type: named("SystemWaitSet"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "system_wait_set_register",
        parameters: &[named("SystemWaitSet"), named("SystemReadinessSource")],
        return_type: named("SystemWaitRegistration"),
        errors: &["SystemWaitSetError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "system_wait_set_remove",
        parameters: &[named("SystemWaitSet"), named("SystemWaitRegistration")],
        return_type: named("void"),
        errors: &["SystemWaitSetError"],
    },
    TerminalApiFunctionSpec {
        name: "system_wait_set_register_owned",
        parameters: &[named("SystemWaitSet"), named("SystemReadinessSource")],
        return_type: named("SystemOwnedWaitRegistration"),
        errors: &["SystemWaitSetError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "system_owned_wait_registration_retarget",
        parameters: &[
            named("SystemOwnedWaitRegistration"),
            named("SystemReadinessSource"),
        ],
        return_type: named("void"),
        errors: &["SystemWaitSetError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "system_owned_wait_registration_remove",
        parameters: &[named("SystemOwnedWaitRegistration")],
        return_type: named("void"),
        errors: &["SystemWaitSetError"],
    },
    TerminalApiFunctionSpec {
        name: "system_wait_set_wait_sync",
        parameters: &[named("SystemWaitSet"), named("CancellationToken")],
        return_type: named("SystemWaitWake"),
        errors: &["SystemWaitSetError"],
    },
    TerminalApiFunctionSpec {
        name: "cancellation_source_new",
        parameters: EMPTY,
        return_type: named("CancellationSource"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "cancellation_token",
        parameters: &[named("CancellationSource")],
        return_type: named("CancellationToken"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "cancellation_request",
        parameters: &[named("CancellationSource")],
        return_type: named("void"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_new",
        parameters: EMPTY,
        return_type: named("MonotonicTimer"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_readiness_source",
        parameters: &[named("MonotonicTimer")],
        return_type: named("SystemReadinessSource"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_arm",
        parameters: &[named("MonotonicTimer"), named("MonotonicDeadline")],
        return_type: named("uint64"),
        errors: &["MonotonicTimerError"],
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_disarm",
        parameters: &[named("MonotonicTimer")],
        return_type: named("uint64"),
        errors: &["MonotonicTimerError"],
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_generation",
        parameters: &[named("MonotonicTimer")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "monotonic_timer_deadline",
        parameters: &[named("MonotonicTimer")],
        return_type: named("MonotonicDeadline"),
        errors: &["MonotonicTimerNotArmedError"],
    },
    TerminalApiFunctionSpec {
        name: "monotonic_clock_now",
        parameters: EMPTY,
        return_type: named("MonotonicDeadline"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "process_control_source_new",
        parameters: EMPTY,
        return_type: named("ProcessControlSource"),
        errors: &["ProcessControlUnavailableError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "process_control_readiness_source",
        parameters: &[named("ProcessControlSource")],
        return_type: named("SystemReadinessSource"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "process_control_poll",
        parameters: &[named("ProcessControlSource")],
        return_type: named("ProcessControlPollResult"),
        errors: &["ProcessControlError"],
    },
    TerminalApiFunctionSpec {
        name: "process_control_acknowledge_suspend",
        parameters: &[named("ProcessControlSource"), named("uint64")],
        return_type: named("void"),
        errors: &["ProcessControlAcknowledgementError"],
    },
    TerminalApiFunctionSpec {
        name: "process_control_resume_application",
        parameters: &[named("ProcessControlSource"), named("uint64")],
        return_type: named("void"),
        errors: &["ProcessControlResumeError"],
    },
    TerminalApiFunctionSpec {
        name: "error_cause",
        parameters: &[named("Error")],
        return_type: named("Error"),
        errors: &["ErrorAttachmentAbsentError"],
    },
    TerminalApiFunctionSpec {
        name: "error_suppressed_length",
        parameters: &[named("Error")],
        return_type: named("int64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "error_suppressed_at",
        parameters: &[named("Error"), named("int64")],
        return_type: named("Error"),
        errors: INDEX_OUT_OF_BOUNDS,
    },
    TerminalApiFunctionSpec {
        name: "error_attachment_truncation",
        parameters: &[named("Error")],
        return_type: named("ErrorAttachmentTruncation"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "error_attachment_truncation_cause_depth",
        parameters: &[named("ErrorAttachmentTruncation")],
        return_type: named("boolean"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "error_attachment_truncation_suppressed_count",
        parameters: &[named("ErrorAttachmentTruncation")],
        return_type: named("boolean"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "error_attachment_truncation_bytes",
        parameters: &[named("ErrorAttachmentTruncation")],
        return_type: named("boolean"),
        errors: NO_ERRORS,
    },
];

/// Selected `standard.terminal` proposal public functions.
pub(super) const SELECTED_TERMINAL_FUNCTIONS: &[TerminalApiFunctionSpec] = &[
    TerminalApiFunctionSpec {
        name: "terminal_session_options_default",
        parameters: EMPTY,
        return_type: named("TerminalSessionOptions"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_options_with_feature_policy",
        parameters: &[
            named("TerminalSessionOptions"),
            named("TerminalSessionFeaturePolicy"),
        ],
        return_type: named("TerminalSessionOptions"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_options_with_resource_limits",
        parameters: &[
            named("TerminalSessionOptions"),
            named("TerminalSessionResourceLimits"),
        ],
        return_type: named("TerminalSessionOptions"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_options_validate",
        parameters: &[named("TerminalSessionOptions")],
        return_type: named("TerminalSessionOptions"),
        errors: &["TerminalSessionOptionsError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_open_sync",
        parameters: &[named("TerminalSessionOptions")],
        return_type: named("TerminalSession"),
        errors: &["TerminalSessionOpenError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_recover_open_sync",
        parameters: &[named("TerminalRecoveryToken")],
        return_type: named("void"),
        errors: &["TerminalSessionRestoreError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_recover_close_sync",
        parameters: &[named("TerminalRecoveryToken")],
        return_type: named("void"),
        errors: &["TerminalSessionRestoreError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_recovery_token_kind",
        parameters: &[named("TerminalRecoveryToken")],
        return_type: named("TerminalRecoveryLedgerKind"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_recovery_token_generation",
        parameters: &[named("TerminalRecoveryToken")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_state",
        parameters: &[named("TerminalSession")],
        return_type: named("TerminalSessionState"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_capabilities",
        parameters: &[named("TerminalSession")],
        return_type: named("TerminalCapabilities"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_capabilities_feature",
        parameters: &[
            named("TerminalCapabilities"),
            named("TerminalOrdinaryFeature"),
        ],
        return_type: named("TerminalFeatureCapability"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_capabilities_trusted_paste_framing",
        parameters: &[named("TerminalCapabilities")],
        return_type: named("TerminalTrustedPasteCapability"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_capabilities_color",
        parameters: &[named("TerminalCapabilities")],
        return_type: named("TerminalColorCapability"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_readiness_source",
        parameters: &[named("TerminalSession")],
        return_type: named("SystemReadinessSource"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_size_sync",
        parameters: &[named("TerminalSession")],
        return_type: named("TerminalSize"),
        errors: &["TerminalSessionReadError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_read_event_sync",
        parameters: &[
            named("TerminalSession"),
            named("TerminalWait"),
            named("CancellationToken"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalSessionReadError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_write_sync",
        parameters: &[named("TerminalSession"), named("TrustedTerminalOutput")],
        return_type: named("void"),
        errors: &["TerminalSessionWriteError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_write_diagnostic_sync",
        parameters: &[
            named("TerminalSession"),
            named("SafeTerminalDiagnosticOutput"),
        ],
        return_type: named("void"),
        errors: &["TerminalSessionWriteError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_flush_sync",
        parameters: &[named("TerminalSession")],
        return_type: named("void"),
        errors: &["TerminalSessionWriteError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_set_cursor_visible_sync",
        parameters: &[named("TerminalSession"), named("boolean")],
        return_type: named("void"),
        errors: &["TerminalSessionWriteError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_set_cursor_shape_sync",
        parameters: &[named("TerminalSession"), named("TerminalCursorShape")],
        return_type: named("void"),
        errors: &["TerminalSessionWriteError", "TerminalSessionStateError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_pause_sync",
        parameters: &[named("TerminalSession")],
        return_type: named("TerminalPauseResult"),
        errors: &[
            "TerminalSessionReadError",
            "TerminalSessionRestoreError",
            "TerminalSessionStateError",
        ],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_resume_sync",
        parameters: &[named("TerminalSession")],
        return_type: named("void"),
        errors: &[
            "TerminalSessionOpenError",
            "TerminalSessionRestoreError",
            "TerminalSessionStateError",
        ],
    },
    TerminalApiFunctionSpec {
        name: "terminal_session_close_sync",
        parameters: &[named("TerminalSession")],
        return_type: named("TerminalCloseOutcome"),
        errors: &["TerminalSessionRestoreError"],
    },
    TerminalApiFunctionSpec {
        name: "trusted_terminal_output_from_application_text",
        parameters: &[named("string")],
        return_type: named("TrustedTerminalOutput"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "safe_terminal_diagnostic_format",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("SafeTerminalDiagnosticOutput"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "safe_terminal_diagnostic_collection_format",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("SafeTerminalDiagnosticOutput"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_pause_events_length",
        parameters: &[named("TerminalPauseEvents")],
        return_type: named("int64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_pause_events_at",
        parameters: &[named("TerminalPauseEvents"), named("int64")],
        return_type: named("TerminalInputEvent"),
        errors: INDEX_OUT_OF_BOUNDS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_backend",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalBackend"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_operation",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalOperation"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_stage",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalDiagnosticStage"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_coordinator_state",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalCoordinatorState"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_session_state",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalDiagnosticSessionState"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_os_code",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalOsCode"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_detail",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalDiagnosticDetail"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_retryability",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("TerminalDiagnosticRetryability"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostic_was_truncated",
        parameters: &[named("TerminalDiagnostic")],
        return_type: named("boolean"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_length",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("int64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_at",
        parameters: &[named("TerminalDiagnosticCollection"), named("int64")],
        return_type: named("TerminalDiagnostic"),
        errors: INDEX_OUT_OF_BOUNDS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_retained_count",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_omitted_count",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_retained_bytes",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_omitted_bytes",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_diagnostics_was_truncated",
        parameters: &[named("TerminalDiagnosticCollection")],
        return_type: named("boolean"),
        errors: NO_ERRORS,
    },
];

/// Companion `standard.terminal.chords` proposal functions.
pub(super) const TERMINAL_CHORD_FUNCTIONS: &[TerminalApiFunctionSpec] = &[
    TerminalApiFunctionSpec {
        name: "terminal_chord_modifiers",
        parameters: &[
            named("boolean"),
            named("boolean"),
            named("boolean"),
            named("boolean"),
        ],
        return_type: named("TerminalChordModifiers"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_new",
        parameters: &[
            named("TerminalChordKey"),
            named("TerminalChordModifiers"),
            named("TerminalChordTrigger"),
        ],
        return_type: named("TerminalChord"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_with_lock_modifier_mask",
        parameters: &[named("TerminalChord"), named("TerminalLockModifierMask")],
        return_type: named("TerminalChord"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_sequence_single",
        parameters: &[named("TerminalChord")],
        return_type: named("TerminalChordSequence"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_sequence_append",
        parameters: &[named("TerminalChordSequence"), named("TerminalChord")],
        return_type: named("TerminalChordSequence"),
        errors: &["TerminalChordValidationError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_new",
        parameters: &[
            named("TerminalCapabilities"),
            named("TerminalChordRouterPolicy"),
        ],
        return_type: named("TerminalChordRouter"),
        errors: &["TerminalChordValidationError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_register",
        parameters: &[
            named("TerminalChordRouter"),
            named("TerminalChordSequence"),
            named("TerminalChordPriority"),
            named("TerminalChordTextPolicy"),
        ],
        return_type: named("TerminalChordBindingId"),
        errors: &["TerminalChordValidationError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_unregister",
        parameters: &[
            named("TerminalChordRouter"),
            named("TerminalChordBindingId"),
        ],
        return_type: named("TerminalChordMutationResult"),
        errors: &["TerminalChordMutationError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_replace",
        parameters: &[
            named("TerminalChordRouter"),
            named("TerminalChordBindingId"),
            named("TerminalChordSequence"),
            named("TerminalChordPriority"),
            named("TerminalChordTextPolicy"),
        ],
        return_type: named("TerminalChordMutationResult"),
        errors: &[
            "TerminalChordValidationError",
            "TerminalChordMutationError",
            "AllocationFailureError",
        ],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_binding_id_ordinal",
        parameters: &[named("TerminalChordBindingId")],
        return_type: named("uint64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_process",
        parameters: &[named("TerminalChordRouter"), named("TerminalInputEvent")],
        return_type: named("TerminalChordRouterOutput"),
        errors: &["TerminalChordProcessError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_expire_sync",
        parameters: &[named("TerminalChordRouter")],
        return_type: named("TerminalChordRouterOutput"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_router_reset",
        parameters: &[
            named("TerminalChordRouter"),
            named("TerminalChordResetReason"),
        ],
        return_type: named("TerminalChordReleasedInput"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_released_input_length",
        parameters: &[named("TerminalChordReleasedInput")],
        return_type: named("int64"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_chord_released_input_at",
        parameters: &[named("TerminalChordReleasedInput"), named("int64")],
        return_type: named("TerminalInputEvent"),
        errors: INDEX_OUT_OF_BOUNDS,
    },
];

/// Test-runner-only `standard.testing.terminal` proposal functions.
pub(super) const TERMINAL_TESTING_FUNCTIONS: &[TerminalApiFunctionSpec] = &[
    TerminalApiFunctionSpec {
        name: "test_runner_terminal_authority",
        parameters: EMPTY,
        return_type: named("TerminalTestAuthority"),
        errors: NO_ERRORS,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_scenario_new",
        parameters: &[
            named("TerminalTestAuthority"),
            named("TerminalTestScenarioLimits"),
        ],
        return_type: named("TerminalTestScenario"),
        errors: &["TerminalSessionOptionsError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_event_id_new",
        parameters: &[named("TerminalTestScenario")],
        return_type: named("TerminalEventId"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_composition_id_new",
        parameters: &[named("TerminalTestScenario")],
        return_type: named("TerminalCompositionId"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_trusted_paste_evidence",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestTrustedPasteBoundary"),
        ],
        return_type: named("TerminalTrustedPasteEvidence"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_key_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalEventId"),
            named("TerminalTestLogicalKey"),
            named("TerminalTestKeyOccurrence"),
            named("TerminalTestModifiers"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_text_input_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("string"),
            named("TerminalTestTextInputOrigin"),
            named("TerminalTestLinkedTextPhase"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_composition_started_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalCompositionId"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_composition_updated_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalCompositionId"),
            named("string"),
            named("int64"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_composition_ended_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalCompositionId"),
            named("TerminalTestCompositionEnd"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_paste_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("string"),
            named("TerminalTestPastePhase"),
            named("TerminalTrustedPasteEvidence"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_mouse_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestMouseAction"),
            named("TerminalTestModifiers"),
            named("int32"),
            named("int32"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_resize_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("int32"),
            named("int32"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_focus_gained_event",
        parameters: &[named("TerminalTestScenario")],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_focus_lost_event",
        parameters: &[named("TerminalTestScenario")],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_unknown_bytes_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("Bytes"),
            named("TerminalTestUnknownBytesReason"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_unknown_native_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestNativeMetadata"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: CONSTRAINT_AND_FACTORY,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_input_reset_event",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestInputResetReason"),
        ],
        return_type: named("TerminalInputEvent"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_capabilities",
        parameters: &[
            named("TerminalTestScenario"),
            array("TerminalTestOrdinaryCapabilityEntry"),
            named("TerminalTestTrustedPasteCapabilitySpec"),
            named("TerminalTestColorCapabilitySpec"),
        ],
        return_type: named("TerminalCapabilities"),
        errors: &["TerminalTestFactoryError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_diagnostic",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestDiagnosticSpec"),
        ],
        return_type: named("TerminalDiagnostic"),
        errors: &[
            "ConstraintViolationError",
            "TerminalTestFactoryError",
            "AllocationFailureError",
        ],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_diagnostic_collection",
        parameters: &[
            named("TerminalTestAuthority"),
            array("TerminalDiagnostic"),
            named("TerminalTestDiagnosticCollectionLimits"),
        ],
        return_type: named("TerminalDiagnosticCollection"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_fake_backend",
        parameters: &[named("TerminalTestAuthority")],
        return_type: named("TerminalTestFakeBackend"),
        errors: ALLOCATION_FAILURE,
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_fake_backend_with_fault",
        parameters: &[
            named("TerminalTestFakeBackend"),
            named("TerminalTestFakeBackendFault"),
        ],
        return_type: named("TerminalTestFakeBackend"),
        errors: &["TerminalTestFactoryError", "AllocationFailureError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_bind_fake_backend",
        parameters: &[
            named("TerminalTestScenario"),
            named("TerminalTestFakeBackend"),
        ],
        return_type: named("void"),
        errors: &["TerminalTestFactoryError"],
    },
    TerminalApiFunctionSpec {
        name: "terminal_test_activate_backend",
        parameters: &[named("TerminalTestScenario")],
        return_type: named("TerminalTestBackendActivation"),
        errors: &["TerminalTestFactoryError"],
    },
];

/// Add gated proposal function symbols to a parsed terminal proposal interface.
pub(super) fn register_terminal_proposal_symbols(interface: &mut ModuleInterface) {
    let specs = match interface.module_path.as_str() {
        "standard.system" => CORE_PREREQUISITE_FUNCTIONS,
        "standard.terminal" => SELECTED_TERMINAL_FUNCTIONS,
        "standard.terminal.chords" => TERMINAL_CHORD_FUNCTIONS,
        "standard.testing.terminal" => TERMINAL_TESTING_FUNCTIONS,
        _ => return,
    };

    for spec in specs {
        let result = interface.register_symbol(function_symbol(spec));
        assert!(
            result.is_ok(),
            "duplicate terminal proposal function export in {}: {}",
            interface.module_path,
            spec.name
        );
    }
}

/// Convert one compact proposal spec into an importable module symbol.
fn function_symbol(spec: &TerminalApiFunctionSpec) -> SymbolInfo {
    SymbolInfo {
        name: String::from(spec.name),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: spec.parameters.iter().copied().map(core_type).collect(),
            return_types: vec![core_type(spec.return_type)],
            error_types: spec.errors.iter().copied().map(nominal_type).collect(),
        },
        visibility: Visibility::Public,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    }
}

/// Convert a compact type reference into the checker core type representation.
fn core_type(type_ref: ApiTypeRef) -> CoreType {
    match type_ref {
        ApiTypeRef::Named(name) => named_core_type(name),
        ApiTypeRef::Array(element) => CoreType::Array(Box::new(named_core_type(element))),
    }
}

/// Convert a primitive or nominal type name into core type metadata.
fn named_core_type(name: &str) -> CoreType {
    match name {
        "int8" => CoreType::Int8,
        "int16" => CoreType::Int16,
        "int32" => CoreType::Int32,
        "int64" => CoreType::Int64,
        "uint8" => CoreType::UInt8,
        "uint16" => CoreType::UInt16,
        "uint32" => CoreType::UInt32,
        "uint64" => CoreType::UInt64,
        "float32" => CoreType::Float32,
        "float64" => CoreType::Float64,
        "string" => CoreType::String,
        "boolean" => CoreType::Boolean,
        "void" => CoreType::Unit,
        other => nominal_type(other),
    }
}

/// Construct a nominal generic core type with no type arguments.
fn nominal_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: String::from(name),
        type_args: Vec::new(),
    }
}
