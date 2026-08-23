//! Selected terminal runtime data-model types, inspectors, and formatting.

extern crate alloc;

use super::constraints::{
    TerminalColorCount, TerminalColumnCount, TerminalColumnIndex, TerminalCommittedText,
    TerminalCommittedTextByteLimit, TerminalCompositionId, TerminalCompositionPreeditByteLimit,
    TerminalCompositionPreeditText, TerminalCompositionScalarIndex, TerminalControlCode,
    TerminalCorrelatedByteLimit, TerminalCorrelatedEventLimit,
    TerminalDiagnosticCollectionByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
    TerminalEventId, TerminalFunctionKeyNumber, TerminalInputSequenceTimeoutMilliseconds,
    TerminalKeyRepeatCount, TerminalNativeEventName, TerminalPasteChunkByteLimit,
    TerminalPasteText, TerminalPendingSequenceByteLimit, TerminalRetainedByteLimit,
    TerminalRetainedEventLimit, TerminalRowCount, TerminalRowIndex, TerminalUnknownByteChunkLimit,
};
#[cfg(test)]
use super::formatting::{collection_metadata_bytes, diagnostic_accounted_bytes};
use super::tail_types::{TerminalInvalidOptions, TerminalSessionOptionsError};
#[cfg(test)]
use crate::runtime::terminal::TerminalConstraintError;
use crate::stdlib::bytes::Bytes;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::fmt;
#[cfg(test)]
use core::sync::atomic::{AtomicU64, Ordering};

/// Stable hidden stream identity source for terminal capability and event values.
#[cfg(test)]
static NEXT_HIDDEN_STREAM_ID: AtomicU64 = AtomicU64::new(1);

/// Allocate a never-reused hidden stream identity.
#[cfg(test)]
pub(crate) fn next_hidden_stream_id() -> u64 {
    NEXT_HIDDEN_STREAM_ID.fetch_add(1, Ordering::Relaxed)
}

/// Convert one positive proposal-bounded `i32` into `u64` without panicking.
fn positive_i32_to_u64(value: i32) -> u64 {
    u64::try_from(value).unwrap_or_default()
}

/// Public v1 terminal feature policy record.
#[expect(
    clippy::struct_excessive_bools,
    reason = "authoritative terminal feature policy uses explicit booleans per proposal"
)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSessionFeaturePolicy {
    pub use_alternate_screen: bool,
    pub hide_cursor: bool,
    pub enable_bracketed_paste: bool,
    pub require_trusted_paste_framing: bool,
    pub enable_enhanced_key_identity: bool,
    pub enable_focus_events: bool,
    pub mouse_tracking: TerminalMouseTracking,
    pub capture_control_keys: bool,
    pub require_requested_features: bool,
}

impl Default for TerminalSessionFeaturePolicy {
    fn default() -> Self {
        Self {
            use_alternate_screen: false,
            hide_cursor: false,
            enable_bracketed_paste: false,
            require_trusted_paste_framing: false,
            enable_enhanced_key_identity: false,
            enable_focus_events: false,
            mouse_tracking: TerminalMouseTracking::Disabled,
            capture_control_keys: false,
            require_requested_features: false,
        }
    }
}

/// Public v1 terminal resource-limits record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSessionResourceLimits {
    pub input_sequence_timeout: TerminalInputSequenceTimeoutMilliseconds,
    pub maximum_committed_text_bytes: TerminalCommittedTextByteLimit,
    pub maximum_composition_preedit_bytes: TerminalCompositionPreeditByteLimit,
    pub maximum_paste_chunk_bytes: TerminalPasteChunkByteLimit,
    pub maximum_unknown_chunk_bytes: TerminalUnknownByteChunkLimit,
    pub maximum_pending_sequence_bytes: TerminalPendingSequenceByteLimit,
    pub maximum_retained_events: TerminalRetainedEventLimit,
    pub maximum_retained_bytes: TerminalRetainedByteLimit,
    pub maximum_correlated_events: TerminalCorrelatedEventLimit,
    pub maximum_correlated_bytes: TerminalCorrelatedByteLimit,
    pub maximum_diagnostics: TerminalDiagnosticCountLimit,
    pub maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit,
}

impl Default for TerminalSessionResourceLimits {
    fn default() -> Self {
        Self {
            input_sequence_timeout: TerminalInputSequenceTimeoutMilliseconds::new(25)
                .expect("proposal default must be valid"),
            maximum_committed_text_bytes: TerminalCommittedTextByteLimit::new(4_096)
                .expect("proposal default must be valid"),
            maximum_composition_preedit_bytes: TerminalCompositionPreeditByteLimit::new(4_096)
                .expect("proposal default must be valid"),
            maximum_paste_chunk_bytes: TerminalPasteChunkByteLimit::new(4_096)
                .expect("proposal default must be valid"),
            maximum_unknown_chunk_bytes: TerminalUnknownByteChunkLimit::new(1_024)
                .expect("proposal default must be valid"),
            maximum_pending_sequence_bytes: TerminalPendingSequenceByteLimit::new(1_024)
                .expect("proposal default must be valid"),
            maximum_retained_events: TerminalRetainedEventLimit::new(1_024)
                .expect("proposal default must be valid"),
            maximum_retained_bytes: TerminalRetainedByteLimit::new(0x0010_0000)
                .expect("proposal default must be valid"),
            maximum_correlated_events: TerminalCorrelatedEventLimit::new(64)
                .expect("proposal default must be valid"),
            maximum_correlated_bytes: TerminalCorrelatedByteLimit::new(0x0001_0000)
                .expect("proposal default must be valid"),
            maximum_diagnostics: TerminalDiagnosticCountLimit::new(16)
                .expect("proposal default must be valid"),
            maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit::new(0x0001_0000)
                .expect("proposal default must be valid"),
        }
    }
}

/// Immutable options snapshot built from stable defaults and functional updates.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct TerminalSessionOptions {
    /// Immutable requested feature-policy category.
    feature_policy: TerminalSessionFeaturePolicy,
    /// Immutable parser/resource-limit category.
    resource_limits: TerminalSessionResourceLimits,
}

impl fmt::Debug for TerminalSessionOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalSessionOptions")
            .field("feature_policy", &self.feature_policy)
            .field("resource_limits", &self.resource_limits)
            .finish()
    }
}

impl TerminalSessionOptions {
    #[must_use]
    pub const fn with_feature_policy(&self, feature_policy: TerminalSessionFeaturePolicy) -> Self {
        Self {
            feature_policy,
            resource_limits: self.resource_limits,
        }
    }

    #[must_use]
    pub const fn with_resource_limits(&self, resource_limits: TerminalSessionResourceLimits) -> Self {
        Self {
            feature_policy: self.feature_policy,
            resource_limits,
        }
    }

    pub fn validate(&self) -> Result<Self, TerminalSessionOptionsError> {
        let retained_bytes = positive_i32_to_u64(self.resource_limits.maximum_retained_bytes.get());
        let correlated_bytes = positive_i32_to_u64(self.resource_limits.maximum_correlated_bytes.get());
        if retained_bytes < correlated_bytes {
            return Err(TerminalSessionOptionsError::InvalidOptions {
                invalid_options: TerminalInvalidOptions::RetainedCapacityTooSmall {
                    required_bytes: correlated_bytes,
                    configured_bytes: retained_bytes,
                },
            });
        }
        let retained_events = positive_i32_to_u64(self.resource_limits.maximum_retained_events.get());
        let correlated_events = positive_i32_to_u64(self.resource_limits.maximum_correlated_events.get());
        if correlated_events > retained_events {
            return Err(TerminalSessionOptionsError::InvalidOptions {
                invalid_options: TerminalInvalidOptions::CorrelatedGroupTooLarge {
                    required_events: correlated_events,
                    configured_events: retained_events,
                },
            });
        }
        Ok(self.clone())
    }

    #[must_use]
    pub const fn feature_policy(&self) -> TerminalSessionFeaturePolicy {
        self.feature_policy
    }

    #[must_use]
    pub const fn resource_limits(&self) -> TerminalSessionResourceLimits {
        self.resource_limits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TerminalOrdinaryFeature {
    AlternateScreen,
    CursorShape,
    BracketedPaste,
    FocusEvents,
    MouseButtons,
    MouseMotion,
    KeyReleaseEvents,
    CompositionEvents,
    EnhancedKeyIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMouseTracking {
    Disabled,
    Buttons,
    ButtonsAndDrag,
    AllMotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCursorShape {
    Default,
    BlinkingBlock,
    SteadyBlock,
    BlinkingUnderline,
    SteadyUnderline,
    BlinkingBar,
    SteadyBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCapabilitySupportedEvidence {
    NativeConfirmed,
    ProtocolQueried,
    EnvironmentInferred,
    ProtocolAssumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTrustedPasteEvidence {
    NativeRecordBoundary,
    SanitizedProtocolBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCapabilityUnsupportedEvidence {
    NativeUnavailable,
    ProtocolRejected,
    EnvironmentMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFeatureCapability {
    Unsupported {
        evidence: TerminalCapabilityUnsupportedEvidence,
    },
    Available {
        evidence: TerminalCapabilitySupportedEvidence,
    },
    Enabled {
        evidence: TerminalCapabilitySupportedEvidence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTrustedPasteCapability {
    Unsupported {
        evidence: TerminalCapabilityUnsupportedEvidence,
    },
    Available {
        evidence: TerminalTrustedPasteEvidence,
    },
    Enabled {
        evidence: TerminalTrustedPasteEvidence,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColorCapability {
    Unsupported {
        evidence: TerminalCapabilityUnsupportedEvidence,
    },
    Monochrome {
        evidence: TerminalCapabilitySupportedEvidence,
    },
    Indexed {
        count: TerminalColorCount,
        evidence: TerminalCapabilitySupportedEvidence,
    },
    TrueColor {
        evidence: TerminalCapabilitySupportedEvidence,
    },
}

/// Immutable negotiated terminal capability snapshot.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalCapabilities {
    /// Immutable ordinary feature capability map.
    ordinary: BTreeMap<TerminalOrdinaryFeature, TerminalFeatureCapability>,
    /// Immutable trusted-paste framing capability.
    trusted_paste: TerminalTrustedPasteCapability,
    /// Immutable color capability.
    color: TerminalColorCapability,
    /// Hidden never-public stream identity bound to this snapshot.
    hidden_stream_id: u64,
    /// Hidden correlated-event limit bound to the same stream identity.
    hidden_correlated_event_limit: TerminalCorrelatedEventLimit,
}

impl fmt::Debug for TerminalCapabilities {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalCapabilities")
            .field("ordinary", &self.ordinary)
            .field("trusted_paste", &self.trusted_paste)
            .field("color", &self.color)
            .finish_non_exhaustive()
    }
}

impl TerminalCapabilities {
    #[cfg(test)]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "BTreeMap-backed test constructor cannot be const"
    )]
    pub(crate) fn new_runtime(
        ordinary: BTreeMap<TerminalOrdinaryFeature, TerminalFeatureCapability>,
        trusted_paste: TerminalTrustedPasteCapability,
        color: TerminalColorCapability,
        hidden_stream_id: u64,
        hidden_correlated_event_limit: TerminalCorrelatedEventLimit,
    ) -> Self {
        Self {
            ordinary,
            trusted_paste,
            color,
            hidden_stream_id,
            hidden_correlated_event_limit,
        }
    }

    #[must_use]
    pub fn feature(&self, feature: TerminalOrdinaryFeature) -> TerminalFeatureCapability {
        self.ordinary
            .get(&feature)
            .copied()
            .unwrap_or(TerminalFeatureCapability::Unsupported {
                evidence: TerminalCapabilityUnsupportedEvidence::EnvironmentMissing,
            })
    }

    #[must_use]
    pub const fn trusted_paste_framing(&self) -> TerminalTrustedPasteCapability {
        self.trusted_paste
    }

    #[must_use]
    pub const fn color(&self) -> TerminalColorCapability {
        self.color
    }

    #[cfg(test)]
    pub(crate) const fn hidden_stream_id(&self) -> u64 {
        self.hidden_stream_id
    }

    #[cfg(test)]
    pub(crate) const fn hidden_correlated_event_limit(&self) -> TerminalCorrelatedEventLimit {
        self.hidden_correlated_event_limit
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub columns: TerminalColumnCount,
    pub rows: TerminalRowCount,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "terminal modifier state is an authoritative fixed-field record"
)]
pub struct TerminalModifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub super_key: bool,
    pub caps_lock: bool,
    pub num_lock: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalNamedKey {
    Enter,
    Escape,
    Backspace,
    Tab,
    BackTab,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalLogicalKey {
    Text { logical_text: TerminalCommittedText },
    Control { code: TerminalControlCode },
    Named { key: TerminalNamedKey },
    Function { number: TerminalFunctionKeyNumber },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKeyOccurrence {
    Press { count: TerminalKeyRepeatCount },
    Repeat { count: TerminalKeyRepeatCount },
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTextInputOrigin {
    Direct,
    Key {
        event_id: TerminalEventId,
    },
    Composition {
        composition_id: TerminalCompositionId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalLinkedTextPhase {
    Complete,
    Start,
    Continue,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCompositionEnd {
    Committed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalPastePhase {
    Complete,
    Start,
    Continue,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalUnknownBytesReason {
    UnrecognizedSequence,
    SequenceLimitExceeded,
    PasteContainsNul,
    PasteInvalidUtf8,
    BackendOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalNativeEventKind {
    WindowsMenu,
    WindowsUnknownRecord,
    VtPrivateSequence,
    BackendSpecific,
    Other { name: TerminalNativeEventName },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalOsCode {
    Unavailable,
    PosixErrno { value: i32 },
    WindowsError { value: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalNativeMetadata {
    pub kind: TerminalNativeEventKind,
    pub code: TerminalOsCode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalInputResetReason {
    PauseBoundary,
    PasteFallback,
    SequenceLimitExceeded,
    BackendReset,
    BackendOverflow,
    CompositionInterrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMouseButton {
    Left,
    Middle,
    Right,
    AuxiliaryOne,
    AuxiliaryTwo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMouseAction {
    Press { button: TerminalMouseButton },
    Release { button: TerminalMouseButton },
    Move,
    Drag { button: TerminalMouseButton },
    Scroll { direction: TerminalScrollDirection },
}

#[derive(Clone, PartialEq, Eq)]
/// Hidden provenance and ordering metadata retained on every input event.
struct HiddenInputMetadata {
    /// Hidden never-public stream identity.
    stream_id: u64,
    /// Hidden nonzero delivery ordinal.
    delivery_ordinal: u64,
}

/// Public normalized input event with hidden stream provenance and delivery order.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminalInputEvent {
    /// Hidden runtime-only provenance and ordering metadata.
    hidden: HiddenInputMetadata,
    /// Public inspectable event payload variant.
    kind: TerminalInputEventKind,
}

impl fmt::Debug for TerminalInputEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TerminalInputEvent")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalInputEventKind {
    Key {
        event_id: TerminalEventId,
        key: TerminalLogicalKey,
        occurrence: TerminalKeyOccurrence,
        modifiers: TerminalModifiers,
    },
    TextInput {
        text: TerminalCommittedText,
        origin: TerminalTextInputOrigin,
        linked_phase: TerminalLinkedTextPhase,
    },
    CompositionStarted {
        composition_id: TerminalCompositionId,
    },
    CompositionUpdated {
        composition_id: TerminalCompositionId,
        preedit_text: TerminalCompositionPreeditText,
        cursor: TerminalCompositionScalarIndex,
    },
    CompositionEnded {
        composition_id: TerminalCompositionId,
        outcome: TerminalCompositionEnd,
    },
    Paste {
        text: TerminalPasteText,
        phase: TerminalPastePhase,
        evidence: TerminalTrustedPasteEvidence,
    },
    Mouse {
        action: TerminalMouseAction,
        modifiers: TerminalModifiers,
        row: TerminalRowIndex,
        column: TerminalColumnIndex,
    },
    Resize {
        size: TerminalSize,
    },
    FocusGained,
    FocusLost,
    TimedOut,
    Cancelled,
    EndOfInput,
    UnknownBytes {
        raw_bytes: Bytes,
        reason: TerminalUnknownBytesReason,
    },
    UnknownNative {
        metadata: TerminalNativeMetadata,
    },
    InputReset {
        reason: TerminalInputResetReason,
    },
}

impl TerminalInputEvent {
    #[cfg(test)]
    pub(crate) fn new_runtime(
        hidden_stream_id: u64,
        delivery_ordinal: u64,
        kind: TerminalInputEventKind,
    ) -> Result<Self, TerminalConstraintError> {
        if hidden_stream_id == 0 {
            return Err(TerminalConstraintError::new(
                "TerminalInputEvent",
                "hidden stream identity must be nonzero",
            ));
        }
        if delivery_ordinal == 0 {
            return Err(TerminalConstraintError::new(
                "TerminalInputEvent",
                "delivery ordinal must be nonzero",
            ));
        }
        Ok(Self {
            hidden: HiddenInputMetadata {
                stream_id: hidden_stream_id,
                delivery_ordinal,
            },
            kind,
        })
    }

    #[must_use]
    pub const fn kind(&self) -> &TerminalInputEventKind {
        &self.kind
    }

    #[cfg(test)]
    pub(crate) const fn hidden_stream_id(&self) -> u64 {
        self.hidden.stream_id
    }

    #[cfg(test)]
    pub(crate) const fn hidden_delivery_ordinal(&self) -> u64 {
        self.hidden.delivery_ordinal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalBackend {
    LinuxVt,
    WindowsConsole,
    WindowsConPty,
    VtStream,
    UnsupportedPlatform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalOperation {
    Open,
    Read,
    Write,
    Flush,
    QuerySize,
    Pause,
    Resume,
    Close,
    RestorePendingOpen,
    RestorePendingClose,
    SetCursorVisibility,
    SetCursorShape,
    NegotiateCapability,
    Allocate,
    ValidateOptions,
    TakeInput,
    PrintText,
    FlushStandardOutput,
    StdoutWriter,
    WriterWrite,
    WriterFlush,
    StdoutTerminal,
    TerminalSupportsAnsi,
    TerminalClearScreenOn,
    TerminalMoveCursorOn,
    TerminalDrawRows,
    TerminalClearScreen,
    TerminalMoveCursor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDiagnosticStage {
    Snapshot,
    AcquireOwnership,
    ConfigureInput,
    ConfigureOutput,
    EnableProtocol,
    Wait,
    Decode,
    Allocate,
    ReverseProtocol,
    RestoreOperatingSystemState,
    ReleaseOwnership,
    ValidateState,
    ValidateOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCoordinatorState {
    Free,
    Opening,
    Active,
    Paused,
    RestorePending,
    FailedOpenRecovery,
    FailedCloseRecovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDiagnosticSessionState {
    Unavailable,
    Active,
    Paused,
    RestorePending,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDiagnosticRetryability {
    NonRetryable,
    SameLiveSession,
    RecoveryToken,
}

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
    #[cfg(test)]
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

    #[cfg(test)]
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

    #[cfg(test)]
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
    #[cfg(test)]
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
