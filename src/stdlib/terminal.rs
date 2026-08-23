//! Stdlib façade for the selected terminal data-model APIs.
//!
//! This module exposes the stable Task 24 options, capability inspectors,
//! diagnostic inspectors/formatters, explicit trust conversion, and test-only
//! synthetic factories while leaving session lifecycle and backend behavior for
//! later tasks.

use crate::runtime::terminal::{
    SafeTerminalDiagnosticOutput, TerminalBackend, TerminalCapabilities,
    TerminalColorCapability, TerminalCoordinatorState, TerminalDiagnostic,
    TerminalDiagnosticCollection, TerminalDiagnosticRetryability,
    TerminalDiagnosticSessionState, TerminalDiagnosticStage,
    TerminalFeatureCapability, TerminalOperation,
    TerminalOrdinaryFeature, TerminalSessionFeaturePolicy, TerminalSessionOptions,
    TerminalSessionOptionsError, TerminalSessionResourceLimits,
    TerminalTrustedPasteCapability, TrustedTerminalOutput,
    safe_terminal_diagnostic_collection_format as runtime_safe_collection_format,
    safe_terminal_diagnostic_format as runtime_safe_format,
};

/// Return the ABI-stable default terminal options snapshot.
#[must_use]
pub fn terminal_session_options_default() -> TerminalSessionOptions {
    TerminalSessionOptions::default()
}

/// Replace the feature-policy category in `options`.
#[must_use]
pub const fn terminal_session_options_with_feature_policy(
    options: &TerminalSessionOptions,
    policy: TerminalSessionFeaturePolicy,
) -> TerminalSessionOptions {
    options.with_feature_policy(policy)
}

/// Replace the resource-limits category in `options`.
#[must_use]
pub const fn terminal_session_options_with_resource_limits(
    options: &TerminalSessionOptions,
    limits: TerminalSessionResourceLimits,
) -> TerminalSessionOptions {
    options.with_resource_limits(limits)
}

/// Validate one immutable options snapshot.
///
/// # Errors
///
/// Returns [`TerminalSessionOptionsError`] when cross-field validation fails.
pub fn terminal_session_options_validate(
    options: &TerminalSessionOptions,
) -> Result<TerminalSessionOptions, TerminalSessionOptionsError> {
    options.validate()
}

/// Inspect one ordinary capability field.
#[must_use]
pub fn terminal_capabilities_feature(
    capabilities: &TerminalCapabilities,
    feature: TerminalOrdinaryFeature,
) -> TerminalFeatureCapability {
    capabilities.feature(feature)
}

/// Inspect trusted-paste framing capability.
#[must_use]
pub const fn terminal_capabilities_trusted_paste_framing(
    capabilities: &TerminalCapabilities,
) -> TerminalTrustedPasteCapability {
    capabilities.trusted_paste_framing()
}

/// Inspect color capability.
#[must_use]
pub const fn terminal_capabilities_color(capabilities: &TerminalCapabilities) -> TerminalColorCapability {
    capabilities.color()
}

/// Explicitly convert reviewed application text into trusted terminal output.
#[must_use]
pub fn trusted_terminal_output_from_application_text(
    text: impl Into<String>,
) -> TrustedTerminalOutput {
    TrustedTerminalOutput::from_application_text(text)
}

/// Safely format one diagnostic for terminal display.
#[must_use]
pub fn safe_terminal_diagnostic_format(
    diagnostic: &TerminalDiagnostic,
) -> SafeTerminalDiagnosticOutput {
    runtime_safe_format(diagnostic)
}

/// Safely format one diagnostic collection for terminal display.
#[must_use]
pub fn safe_terminal_diagnostic_collection_format(
    diagnostics: &TerminalDiagnosticCollection,
) -> SafeTerminalDiagnosticOutput {
    runtime_safe_collection_format(diagnostics)
}

/// Stable diagnostic inspector for backend.
#[must_use]
pub const fn terminal_diagnostic_backend(diagnostic: &TerminalDiagnostic) -> TerminalBackend {
    diagnostic.backend()
}

/// Stable diagnostic inspector for operation.
#[must_use]
pub const fn terminal_diagnostic_operation(diagnostic: &TerminalDiagnostic) -> TerminalOperation {
    diagnostic.operation()
}

/// Stable diagnostic inspector for stage.
#[must_use]
pub const fn terminal_diagnostic_stage(diagnostic: &TerminalDiagnostic) -> TerminalDiagnosticStage {
    diagnostic.stage()
}

/// Stable diagnostic inspector for coordinator state.
#[must_use]
pub const fn terminal_diagnostic_coordinator_state(
    diagnostic: &TerminalDiagnostic,
) -> TerminalCoordinatorState {
    diagnostic.coordinator_state()
}

/// Stable diagnostic inspector for session state.
#[must_use]
pub const fn terminal_diagnostic_session_state(
    diagnostic: &TerminalDiagnostic,
) -> TerminalDiagnosticSessionState {
    diagnostic.session_state()
}

/// Stable diagnostic inspector for OS code.
#[must_use]
pub fn terminal_diagnostic_os_code(
    diagnostic: &TerminalDiagnostic,
) -> crate::runtime::terminal::TerminalOsCode {
    diagnostic.os_code()
}

/// Stable diagnostic inspector for bounded detail text.
#[must_use]
pub fn terminal_diagnostic_detail(
    diagnostic: &TerminalDiagnostic,
) -> crate::runtime::terminal::TerminalDiagnosticDetail {
    diagnostic.detail()
}

/// Stable diagnostic inspector for retryability metadata.
#[must_use]
pub const fn terminal_diagnostic_retryability(
    diagnostic: &TerminalDiagnostic,
) -> TerminalDiagnosticRetryability {
    diagnostic.retryability()
}

/// Stable diagnostic inspector for per-diagnostic truncation.
#[must_use]
pub const fn terminal_diagnostic_was_truncated(diagnostic: &TerminalDiagnostic) -> bool {
    diagnostic.was_truncated()
}

/// Stable collection inspector for retained indexed length.
#[must_use]
pub fn terminal_diagnostics_length(diagnostics: &TerminalDiagnosticCollection) -> i64 {
    diagnostics.len()
}

/// Stable collection inspector for retained diagnostic access.
#[must_use]
pub fn terminal_diagnostics_at(
    diagnostics: &TerminalDiagnosticCollection,
    index: i64,
) -> Option<TerminalDiagnostic> {
    diagnostics.at(index)
}

/// Stable collection inspector for retained count.
#[must_use]
pub const fn terminal_diagnostics_retained_count(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.retained_count()
}

/// Stable collection inspector for omitted count.
#[must_use]
pub const fn terminal_diagnostics_omitted_count(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.omitted_count()
}

/// Stable collection inspector for retained bytes.
#[must_use]
pub const fn terminal_diagnostics_retained_bytes(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.retained_bytes()
}

/// Stable collection inspector for omitted bytes.
#[must_use]
pub const fn terminal_diagnostics_omitted_bytes(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.omitted_bytes()
}

/// Stable collection inspector for truncation.
#[must_use]
pub const fn terminal_diagnostics_was_truncated(diagnostics: &TerminalDiagnosticCollection) -> bool {
    diagnostics.was_truncated()
}
