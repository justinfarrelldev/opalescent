//! Authoritative runtime-readiness inventory for proposal-gated symbols.
//!
//! Task 23 opens the selected public gate once every prerequisite is implemented,
//! but codegen must still distinguish already-lowered core prerequisites from
//! later selected terminal/chord/test surfaces that remain runtime-blocked.

use super::terminal_proposal_symbols::CORE_PREREQUISITE_FUNCTIONS;

/// Return whether a module/symbol pair is implemented enough to lower today.
#[must_use]
pub(super) fn contains_import(module_path: &str, symbol_name: &str) -> bool {
    module_path == "standard.system"
        && CORE_PREREQUISITE_FUNCTIONS
            .iter()
            .any(|spec| spec.name == symbol_name)
}

/// Return whether a runtime symbol name is implemented enough to lower today.
#[must_use]
pub(super) fn contains_runtime_name(symbol_name: &str) -> bool {
    CORE_PREREQUISITE_FUNCTIONS
        .iter()
        .any(|spec| spec.name == symbol_name)
}
