//! Error-family compatibility helpers for the type checker.

use crate::type_system::{error_families::error_type_is_covered_by_declared_type, types::CoreType};

/// Return whether a declared error type covers an emitted error type.
pub(super) fn declared_error_type_covers(emitted: &CoreType, declared: &CoreType) -> bool {
    if emitted == declared {
        return true;
    }

    let emitted_name = emitted.to_string();
    let declared_name = declared.to_string();
    error_type_is_covered_by_declared_type(&emitted_name, &declared_name)
}

/// Return whether root `Error` may inspect a nominal error value.
pub(super) fn root_error_accepts_nominal_error(
    left_name: &str,
    left_args: &[CoreType],
    right_name: &str,
    right_args: &[CoreType],
) -> bool {
    left_name == "Error"
        && left_args.is_empty()
        && (right_name == "GuardErrorContext"
            || (right_args.is_empty()
                && (right_name.ends_with("Error") || right_name.contains("Error."))))
}
