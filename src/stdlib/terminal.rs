//! Stdlib façade for the selected terminal data-model APIs.
//!
//! This module exposes the stable Task 24 options, capability inspectors,
//! diagnostic inspectors/formatters, explicit trust conversion, and test-only
//! synthetic factories while leaving session lifecycle and backend behavior for
//! later tasks.

use crate::runtime::terminal::{
    SafeTerminalDiagnosticOutput, TerminalBackend, TerminalCapabilities, TerminalCloseOutcome,
    TerminalColorCapability, TerminalCoordinatorState, TerminalCursorShape, TerminalDiagnostic,
    TerminalDiagnosticCollection, TerminalDiagnosticRetryability, TerminalDiagnosticSessionState,
    TerminalDiagnosticStage, TerminalFeatureCapability, TerminalInputEvent, TerminalOperation,
    TerminalOrdinaryFeature, TerminalPauseError, TerminalPauseEvents, TerminalPauseResult,
    TerminalReadEventError, TerminalRecoveryToken, TerminalSession, TerminalSessionFeaturePolicy,
    TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionOptionsError,
    TerminalSessionResourceLimits, TerminalSessionRestoreError, TerminalSessionState,
    TerminalSessionStateError, TerminalTrustedPasteCapability, TerminalWait,
    TerminalWriteOperationError, TrustedTerminalOutput,
    safe_terminal_diagnostic_collection_format as runtime_safe_collection_format,
    safe_terminal_diagnostic_format as runtime_safe_format,
    terminal_session_open_sync as runtime_session_open,
    terminal_session_recover_close_sync as runtime_recover_close,
    terminal_session_recover_open_sync as runtime_recover_open,
};
use crate::runtime::wait::{CancellationToken, SystemReadinessSource};

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

/// Open a terminal session.
pub fn terminal_session_open_sync(
    options: &TerminalSessionOptions,
) -> Result<TerminalSession, TerminalSessionOpenError> {
    runtime_session_open(options)
}

/// Recover a failed open rollback using a sealed token.
pub fn terminal_session_recover_open_sync(
    recovery_token: &TerminalRecoveryToken,
) -> Result<(), TerminalSessionRestoreError> {
    runtime_recover_open(recovery_token)
}

/// Recover a failed close restore using a sealed token.
pub fn terminal_session_recover_close_sync(
    recovery_token: &TerminalRecoveryToken,
) -> Result<(), TerminalSessionRestoreError> {
    runtime_recover_close(recovery_token)
}

/// Inspect a terminal recovery-token kind.
#[must_use]
pub fn terminal_recovery_token_kind(
    recovery_token: &TerminalRecoveryToken,
) -> crate::runtime::terminal::TerminalRecoveryLedgerKind {
    recovery_token.kind()
}

/// Inspect a terminal recovery-token generation.
#[must_use]
pub fn terminal_recovery_token_generation(recovery_token: &TerminalRecoveryToken) -> u64 {
    recovery_token.generation()
}

/// Inspect a terminal session state.
#[must_use]
pub const fn terminal_session_state(session: &TerminalSession) -> TerminalSessionState {
    session.state()
}

/// Inspect terminal session capabilities.
#[must_use]
pub fn terminal_session_capabilities(session: &TerminalSession) -> TerminalCapabilities {
    session.capabilities()
}

/// Return a session's stable readiness source identity.
#[must_use]
pub fn terminal_session_readiness_source(session: &TerminalSession) -> SystemReadinessSource {
    session.readiness_source()
}

/// Query terminal size through the session state matrix.
pub fn terminal_session_size_sync(
    session: &TerminalSession,
) -> Result<crate::runtime::terminal::TerminalSize, TerminalSessionStateError> {
    session.size_sync()
}

/// Read exactly one terminal input event.
pub fn terminal_session_read_event_sync(
    session: &mut TerminalSession,
    wait: TerminalWait,
    cancellation: &CancellationToken,
) -> Result<TerminalInputEvent, TerminalReadEventError> {
    session.read_event_sync(wait, cancellation)
}

/// Pause input delivery and return independently retained events.
pub fn terminal_session_pause_sync(
    session: &mut TerminalSession,
) -> Result<TerminalPauseResult, TerminalPauseError> {
    session.pause_sync()
}

/// Resume a paused session.
pub fn terminal_session_resume_sync(
    session: &mut TerminalSession,
) -> Result<(), TerminalSessionStateError> {
    session.resume_sync()
}

/// Write trusted terminal output.
pub fn terminal_session_write_sync(
    session: &mut TerminalSession,
    output: &TrustedTerminalOutput,
) -> Result<(), TerminalWriteOperationError> {
    session.write_sync(output)
}

/// Write safe diagnostic output.
pub fn terminal_session_write_diagnostic_sync(
    session: &mut TerminalSession,
    output: &SafeTerminalDiagnosticOutput,
) -> Result<(), TerminalWriteOperationError> {
    session.write_diagnostic_sync(output)
}

/// Flush trusted terminal output.
pub fn terminal_session_flush_sync(
    session: &TerminalSession,
) -> Result<(), TerminalWriteOperationError> {
    session.flush_sync()
}

/// Clear the session terminal screen.
pub fn terminal_session_clear_screen_sync(
    session: &mut TerminalSession,
) -> Result<(), TerminalWriteOperationError> {
    session.clear_screen_sync()
}

/// Move the session terminal cursor.
pub fn terminal_session_move_cursor_sync(
    session: &mut TerminalSession,
    row: i32,
    column: i32,
) -> Result<(), TerminalWriteOperationError> {
    session.move_cursor_sync(row, column)
}

/// Draw trusted rows through the session terminal.
pub fn terminal_session_draw_rows_sync(
    session: &mut TerminalSession,
    rows: &[TrustedTerminalOutput],
) -> Result<(), TerminalWriteOperationError> {
    session.draw_rows_sync(rows)
}

/// Ring the session terminal bell.
pub fn terminal_session_bell_sync(
    session: &mut TerminalSession,
) -> Result<(), TerminalWriteOperationError> {
    session.bell_sync()
}

/// Set cursor visibility.
pub fn terminal_session_set_cursor_visible_sync(
    session: &TerminalSession,
    visible: bool,
) -> Result<(), TerminalWriteOperationError> {
    session.set_cursor_visible_sync(visible)
}

/// Set cursor shape.
pub fn terminal_session_set_cursor_shape_sync(
    session: &TerminalSession,
    shape: TerminalCursorShape,
) -> Result<(), TerminalWriteOperationError> {
    session.set_cursor_shape_sync(shape)
}

/// Close a terminal session explicitly.
pub fn terminal_session_close_sync(
    session: &mut TerminalSession,
) -> Result<TerminalCloseOutcome, TerminalSessionRestoreError> {
    session.close_sync()
}

/// Return retained pause event count.
#[must_use]
pub fn terminal_pause_events_length(events: &TerminalPauseEvents) -> i64 {
    events.len()
}

/// Return one retained pause event.
#[must_use]
pub fn terminal_pause_events_at(
    events: &TerminalPauseEvents,
    index: i64,
) -> Option<TerminalInputEvent> {
    events.at(index)
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
pub const fn terminal_capabilities_color(
    capabilities: &TerminalCapabilities,
) -> TerminalColorCapability {
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
pub const fn terminal_diagnostics_retained_count(
    diagnostics: &TerminalDiagnosticCollection,
) -> u64 {
    diagnostics.retained_count()
}

/// Stable collection inspector for omitted count.
#[must_use]
pub const fn terminal_diagnostics_omitted_count(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.omitted_count()
}

/// Stable collection inspector for retained bytes.
#[must_use]
pub const fn terminal_diagnostics_retained_bytes(
    diagnostics: &TerminalDiagnosticCollection,
) -> u64 {
    diagnostics.retained_bytes()
}

/// Stable collection inspector for omitted bytes.
#[must_use]
pub const fn terminal_diagnostics_omitted_bytes(diagnostics: &TerminalDiagnosticCollection) -> u64 {
    diagnostics.omitted_bytes()
}

/// Stable collection inspector for truncation.
#[must_use]
pub const fn terminal_diagnostics_was_truncated(
    diagnostics: &TerminalDiagnosticCollection,
) -> bool {
    diagnostics.was_truncated()
}
