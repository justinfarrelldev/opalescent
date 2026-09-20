//! Authoritative runtime-readiness inventory for proposal-gated symbols.
//!
//! Task 23 opened the selected public type-checking gate once every prerequisite
//! landed, but codegen still needs a finer-grained inventory so later terminal
//! lifecycle/back-end work can stay gated while pure data-model APIs lower.

use super::terminal_proposal_symbols::CORE_PREREQUISITE_FUNCTIONS;

/// Selected `standard.terminal` data-model functions implemented by Task 24.
const SELECTED_DATA_MODEL_RUNTIME_NAMES: &[&str] = &[
    "terminal_session_options_default",
    "terminal_session_options_with_feature_policy",
    "terminal_session_options_with_resource_limits",
    "terminal_session_options_validate",
    "trusted_terminal_output_from_application_text",
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
}

/// Return whether a runtime symbol name is implemented enough to lower today.
#[must_use]
pub(super) fn contains_runtime_name(symbol_name: &str) -> bool {
    CORE_PREREQUISITE_FUNCTIONS
        .iter()
        .any(|spec| spec.name == symbol_name)
        || SELECTED_DATA_MODEL_RUNTIME_NAMES.contains(&symbol_name)
}
