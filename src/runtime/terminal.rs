//! Selected terminal runtime/stdlib data-model support for Task 24.
//!
//! This module intentionally implements only the option, capability, event,
//! diagnostic, and trust data-model slices. Session lifecycle, test factories,
//! backend activation, and later terminal behavior stay out of scope.

#[path = "terminal/constraints.rs"]
pub mod constraints;
#[path = "terminal/formatting.rs"]
pub mod formatting;
#[path = "terminal/model.rs"]
pub mod model;
#[path = "terminal/tail_types.rs"]
pub mod tail_types;

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
pub use formatting::{
    SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES, SafeTerminalDiagnosticOutput, TrustedTerminalOutput,
    safe_terminal_diagnostic_collection_format, safe_terminal_diagnostic_format,
};
pub use model::{
    TerminalBackend, TerminalCapabilities, TerminalCapabilitySupportedEvidence,
    TerminalCapabilityUnsupportedEvidence, TerminalColorCapability, TerminalCoordinatorState,
    TerminalCursorShape, TerminalDiagnostic, TerminalDiagnosticCollection,
    TerminalDiagnosticCollectionLimits, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalFeatureCapability,
    TerminalInputEvent, TerminalInputEventKind, TerminalInputResetReason,
    TerminalKeyOccurrence, TerminalLinkedTextPhase, TerminalLogicalKey, TerminalModifiers,
    TerminalMouseAction, TerminalMouseButton, TerminalMouseTracking, TerminalNamedKey,
    TerminalNativeEventKind, TerminalNativeMetadata, TerminalOperation, TerminalOrdinaryFeature,
    TerminalOsCode, TerminalPastePhase, TerminalScrollDirection, TerminalSessionFeaturePolicy,
    TerminalSessionOptions, TerminalSessionResourceLimits, TerminalSize,
    TerminalTextInputOrigin, TerminalTrustedPasteCapability,
    TerminalTrustedPasteEvidence, TerminalUnknownBytesReason,
};
pub use tail_types::{
    TerminalFeature, TerminalInvalidOptions, TerminalRecoveryLedgerKind,
    TerminalSessionOptionsError, TerminalSessionState, TerminalWait,
    required_ordinary_features,
};
