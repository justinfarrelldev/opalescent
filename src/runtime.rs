#![expect(
    clippy::pub_use,
    reason = "Task 24 requires a runtime API surface exposed from src/runtime.rs"
)]

pub mod arrays;
pub mod errors;
pub mod io;
pub mod memory;
pub mod process_control;
pub mod reporting;
pub mod stdlib;
pub mod strings;
pub mod terminal;
pub mod timer;
pub mod wait;

#[path = "runtime/terminal_coordinator.rs"]
pub(crate) mod terminal_coordinator;

pub use arrays::{allocate_array, array_index, array_length};
pub use errors::{RuntimeError, RuntimeResult, RuntimeResultExt};
pub use io::{DefaultIoHandler, IoHandler, print, take_input};
pub use memory::{DefaultRuntimeAllocator, OpalArray, OpalString, RuntimeAllocator};
pub use process_control::{
    ProcessControlAcknowledgementError, ProcessControlError, ProcessControlNotification,
    ProcessControlPollResult, ProcessControlResumeError, ProcessControlSource,
    ProcessControlUnavailableError,
};
pub use reporting::format_runtime_error;
pub use stdlib::{
    DefaultRandomIntSource, RandomIntSource, format_interpolated_string, opal_array_slice,
    random_int32, random_int32_with_source, string_to_int32,
};
pub use strings::{
    string_compare, string_concat, string_equals, string_extract_range, string_find_index_or,
    string_find_last_index_of_text, string_index, string_is_blank, string_length,
    string_split_lines, string_take_prefix, string_take_suffix, string_trim_whitespace,
};
pub use terminal::{
    SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES, SafeTerminalDiagnosticOutput, TerminalBackend,
    TerminalCapabilities, TerminalCapabilitySupportedEvidence,
    TerminalCapabilityUnsupportedEvidence, TerminalChord, TerminalChordBindingId, TerminalChordKey,
    TerminalChordModifiers, TerminalChordMutationError, TerminalChordMutationResult,
    TerminalChordPrefixPolicy, TerminalChordProcessError, TerminalChordReleasedInput,
    TerminalChordResetReason, TerminalChordRouter, TerminalChordRouterOutput,
    TerminalChordRouterPolicy, TerminalChordSequence, TerminalChordTextPolicy,
    TerminalChordTrigger, TerminalChordValidationError, TerminalColorCapability,
    TerminalColorCount, TerminalColumnCount, TerminalColumnIndex, TerminalCommittedText,
    TerminalCommittedTextByteLimit, TerminalCompositionId, TerminalCompositionPreeditByteLimit,
    TerminalCompositionPreeditText, TerminalCompositionScalarIndex, TerminalConstraintError,
    TerminalControlCode, TerminalCoordinatorState, TerminalCorrelatedByteLimit,
    TerminalCorrelatedEventLimit, TerminalCursorShape, TerminalDiagnostic,
    TerminalDiagnosticCollection, TerminalDiagnosticCollectionByteLimit,
    TerminalDiagnosticCollectionLimits, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
    TerminalDiagnosticRetryability, TerminalDiagnosticSessionState, TerminalDiagnosticStage,
    TerminalEventId, TerminalFeature, TerminalFeatureCapability, TerminalFunctionKeyNumber,
    TerminalInputEvent, TerminalInputEventKind, TerminalInputResetReason,
    TerminalInputSequenceTimeoutMilliseconds, TerminalInvalidOptions, TerminalKeyOccurrence,
    TerminalKeyRepeatCount, TerminalLinkedTextPhase, TerminalLockModifierMask, TerminalLogicalKey,
    TerminalModifiers, TerminalMouseAction, TerminalMouseButton, TerminalMouseTracking,
    TerminalNamedKey, TerminalNativeEventKind, TerminalNativeEventName, TerminalNativeMetadata,
    TerminalOperation, TerminalOrdinaryFeature, TerminalOsCode, TerminalPasteChunkByteLimit,
    TerminalPastePhase, TerminalPasteText, TerminalPendingSequenceByteLimit,
    TerminalRecoveryLedgerKind, TerminalRetainedByteLimit, TerminalRetainedEventLimit,
    TerminalRowCount, TerminalRowIndex, TerminalScrollDirection, TerminalSessionFeaturePolicy,
    TerminalSessionOptions, TerminalSessionOptionsError, TerminalSessionResourceLimits,
    TerminalSessionState, TerminalSize, TerminalTestActivation, TerminalTestFactoryError,
    TerminalTestFault, TerminalTestScenario, TerminalTextInputOrigin,
    TerminalTrustedPasteCapability, TerminalTrustedPasteEvidence, TerminalUnknownByteChunkLimit,
    TerminalUnknownBytesReason, TerminalWait, TerminalWaitMilliseconds, TrustedTerminalOutput,
    required_ordinary_features, safe_terminal_diagnostic_collection_format,
    safe_terminal_diagnostic_format,
};
pub use timer::{
    MonotonicDeadline, MonotonicTimer, MonotonicTimerError, MonotonicTimerNotArmedError,
    monotonic_clock_now,
};
pub use wait::{
    CancellationSource, CancellationToken, SystemOwnedWaitRegistration, SystemReadinessSource,
    SystemReadyWakeStatus, SystemWaitRegistration, SystemWaitSet, SystemWaitSetError,
    SystemWaitWake,
};

#[cfg(test)]
mod terminal_chord_tests;
#[cfg(test)]
mod terminal_linux_tests;
#[cfg(test)]
mod terminal_output_tests;
#[cfg(test)]
mod terminal_process_workflow_tests;
#[cfg(test)]
mod terminal_read_tests;
#[cfg(test)]
mod terminal_test_backend_tests;
#[cfg(test)]
mod terminal_tests;
#[cfg(test)]
mod terminal_windows_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod timer_tests;
#[cfg(test)]
pub(crate) mod wait_test_support;
#[cfg(test)]
mod wait_tests;
