//! Authoritative runtime-readiness inventory for proposal-gated symbols.
//!
//! Task 23 opened the selected public type-checking gate once every prerequisite
//! landed, but codegen still needs a finer-grained inventory so later terminal
//! lifecycle/back-end work can stay gated while pure data-model APIs lower.

use super::terminal_proposal_symbols::{CORE_PREREQUISITE_FUNCTIONS, TERMINAL_CHORD_FUNCTIONS};

/// Selected `standard.terminal` data-model functions implemented by Task 24.
const SELECTED_DATA_MODEL_RUNTIME_NAMES: &[&str] = &[
    "terminal_session_options_default",
    "terminal_session_options_with_feature_policy",
    "terminal_session_options_with_resource_limits",
    "terminal_session_options_validate",
    "trusted_terminal_output_from_application_text",
    "terminal_session_open_sync",
    "terminal_session_state",
    "terminal_session_capabilities",
    "terminal_capabilities_feature",
    "terminal_capabilities_trusted_paste_framing",
    "terminal_capabilities_color",
    "terminal_session_size_sync",
    "terminal_session_read_event_sync",
    "terminal_session_write_sync",
    "terminal_session_write_diagnostic_sync",
    "terminal_session_flush_sync",
    "terminal_session_clear_screen_sync",
    "terminal_session_move_cursor_sync",
    "terminal_session_draw_rows_sync",
    "terminal_session_bell_sync",
    "terminal_session_set_cursor_visible_sync",
    "terminal_session_set_cursor_shape_sync",
    "terminal_session_pause_sync",
    "terminal_session_resume_sync",
    "terminal_session_close_sync",
    "safe_terminal_diagnostic_format",
    "safe_terminal_diagnostic_collection_format",
    "terminal_pause_events_length",
    "terminal_pause_events_at",
    "terminal_diagnostic_backend",
    "terminal_diagnostic_operation",
    "terminal_diagnostic_stage",
    "terminal_diagnostic_coordinator_state",
    "terminal_diagnostic_session_state",
    "terminal_diagnostic_os_code",
    "terminal_diagnostic_detail",
    "terminal_diagnostic_retryability",
    "terminal_diagnostic_was_truncated",
    "terminal_diagnostics_length",
    "terminal_diagnostics_at",
    "terminal_diagnostics_retained_count",
    "terminal_diagnostics_omitted_count",
    "terminal_diagnostics_retained_bytes",
    "terminal_diagnostics_omitted_bytes",
    "terminal_diagnostics_was_truncated",
];

/// Return whether a module/symbol pair is implemented enough to lower today.
#[must_use]
pub(super) fn contains_import(module_path: &str, symbol_name: &str) -> bool {
    (module_path == "standard.system"
        && CORE_PREREQUISITE_FUNCTIONS
            .iter()
            .any(|spec| spec.name == symbol_name))
        || (module_path == "standard.terminal"
            && SELECTED_DATA_MODEL_RUNTIME_NAMES.contains(&symbol_name))
        || (module_path == "standard.terminal.chords"
            && TERMINAL_CHORD_FUNCTIONS
                .iter()
                .any(|spec| spec.name == symbol_name))
}

/// Return whether a runtime symbol name is implemented enough to lower today.
#[must_use]
pub(super) fn contains_runtime_name(symbol_name: &str) -> bool {
    CORE_PREREQUISITE_FUNCTIONS
        .iter()
        .any(|spec| spec.name == symbol_name)
        || SELECTED_DATA_MODEL_RUNTIME_NAMES.contains(&symbol_name)
        || TERMINAL_CHORD_FUNCTIONS
            .iter()
            .any(|spec| spec.name == symbol_name)
}
