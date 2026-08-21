//! Implemented Task 16 error-inspector inventory.

/// Core prerequisite error inspector names implemented before the terminal API gate opens.
pub(super) const IMPLEMENTED_ERROR_INSPECTORS: &[&str] = &[
    "error_cause",
    "error_suppressed_length",
    "error_suppressed_at",
    "error_attachment_truncation",
    "error_attachment_truncation_cause_depth",
    "error_attachment_truncation_suppressed_count",
    "error_attachment_truncation_bytes",
];

/// Return whether a module/symbol pair belongs to the implemented Task 16 error surface.
pub(super) fn contains_implemented_error_inspector(module_path: &str, symbol_name: &str) -> bool {
    module_path == "standard.system" && contains_implemented_error_inspector_name(symbol_name)
}

/// Return whether a symbol name belongs to the implemented Task 16 error surface.
pub(super) fn contains_implemented_error_inspector_name(symbol_name: &str) -> bool {
    IMPLEMENTED_ERROR_INSPECTORS.contains(&symbol_name)
}
