//! Test-runner-only terminal proposal function specs.

use super::{
    ALLOCATION_FAILURE, CONSTRAINT_AND_FACTORY, EMPTY, NO_ERRORS, TerminalApiFunctionSpec, array,
    named,
};

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
