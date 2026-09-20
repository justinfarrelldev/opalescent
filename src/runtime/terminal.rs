//! Selected terminal runtime/stdlib data-model support for Task 24.
//!
//! This module intentionally implements only the option, capability, event,
//! diagnostic, and trust data-model slices. Session lifecycle, test factories,
//! backend activation, and later terminal behavior stay out of scope.

#[path = "terminal/constraints.rs"]
pub mod constraints;
#[path = "terminal/diagnostics.rs"]
pub mod diagnostics;
#[path = "terminal/formatting.rs"]
pub mod formatting;
#[path = "terminal/lifecycle.rs"]
pub mod lifecycle;
#[path = "terminal/lifecycle_errors.rs"]
pub mod lifecycle_errors;
#[path = "terminal/linux_backend.rs"]
pub mod linux_backend;
#[path = "terminal/model.rs"]
pub mod model;
#[path = "terminal/process_workflow.rs"]
pub mod process_workflow;
#[path = "terminal/tail_types.rs"]
pub mod tail_types;
#[path = "terminal/test_backend.rs"]
pub mod test_backend;
#[path = "terminal/windows_backend.rs"]
pub mod windows_backend;

pub use constraints::{
    TerminalColorCount, TerminalColumnCount, TerminalColumnIndex, TerminalCommittedText,
    TerminalCommittedTextByteLimit, TerminalCompositionId, TerminalCompositionPreeditByteLimit,
    TerminalCompositionPreeditText, TerminalCompositionScalarIndex, TerminalConstraintError,
    TerminalControlCode, TerminalCorrelatedByteLimit, TerminalCorrelatedEventLimit,
    TerminalDiagnosticCollectionByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
    TerminalEventId, TerminalFunctionKeyNumber, TerminalInputSequenceTimeoutMilliseconds,
    TerminalKeyRepeatCount, TerminalNativeEventName, TerminalPasteChunkByteLimit,
    TerminalPasteText, TerminalPendingSequenceByteLimit, TerminalRetainedByteLimit,
    TerminalRetainedEventLimit, TerminalRowCount, TerminalRowIndex, TerminalUnknownByteChunkLimit,
    TerminalWaitMilliseconds,
};
pub use diagnostics::{
    TerminalDiagnostic, TerminalDiagnosticCollection, TerminalDiagnosticCollectionLimits,
};
pub use formatting::{
    SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES, SafeTerminalDiagnosticOutput, TrustedTerminalOutput,
    safe_terminal_diagnostic_collection_format, safe_terminal_diagnostic_format,
};
pub use lifecycle::{
    TerminalPauseError, TerminalReadEventError, TerminalSession, TerminalWriteOperationError,
    terminal_session_open_sync, terminal_session_recover_close_sync,
    terminal_session_recover_open_sync,
};
pub use lifecycle_errors::{
    TerminalCloseOutcome, TerminalPauseEvents, TerminalPauseResult, TerminalRecoveryToken,
    TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionRestoreError,
    TerminalSessionStateError, TerminalSessionWriteError,
};
pub use linux_backend::{
    LinuxTerminalBackendRuntime, LinuxTerminalBackendState, LinuxTerminalDescriptorSnapshot,
    LinuxTerminalInverseStep,
};
pub use model::{
    TerminalBackend, TerminalCapabilities, TerminalCapabilitySupportedEvidence,
    TerminalCapabilityUnsupportedEvidence, TerminalColorCapability, TerminalCoordinatorState,
    TerminalCursorShape, TerminalDiagnosticRetryability, TerminalDiagnosticSessionState,
    TerminalDiagnosticStage, TerminalFeatureCapability, TerminalInputEvent, TerminalInputEventKind,
    TerminalInputResetReason, TerminalKeyOccurrence, TerminalLinkedTextPhase, TerminalLogicalKey,
    TerminalModifiers, TerminalMouseAction, TerminalMouseButton, TerminalMouseTracking,
    TerminalNamedKey, TerminalNativeEventKind, TerminalNativeMetadata, TerminalOperation,
    TerminalOrdinaryFeature, TerminalOsCode, TerminalPastePhase, TerminalScrollDirection,
    TerminalSessionFeaturePolicy, TerminalSessionOptions, TerminalSessionResourceLimits,
    TerminalSize, TerminalTextInputOrigin, TerminalTrustedPasteCapability,
    TerminalTrustedPasteEvidence, TerminalUnknownBytesReason,
};
pub use process_workflow::{
    TerminalProcessWorkflowError, TerminalProcessWorkflowStep,
    terminal_process_suspend_continue_sync,
};
pub use tail_types::{
    TerminalFeature, TerminalInvalidOptions, TerminalRecoveryLedgerKind,
    TerminalSessionOptionsError, TerminalSessionState, TerminalWait, required_ordinary_features,
};
pub use test_backend::{
    TerminalTestActivation, TerminalTestFactoryError, TerminalTestFault, TerminalTestScenario,
};
pub use windows_backend::{
    WindowsConsoleSnapshot, WindowsTerminalBackendRuntime, WindowsTerminalBackendState,
    WindowsTerminalInverseStep,
};
