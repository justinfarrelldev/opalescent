//! Process stdlib built-in nominal error registration.
//!
//! Registers the four process-module-specific nominal error types used by the
//! language-level `process` module signatures.

extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::error_families::stdlib_error_core_type;
use alloc::borrow::ToOwned;

/// Process-module-specific nominal error type names.
const PROCESS_ERROR_NAMES: &[&str] = &[
    "CurrentWorkingDirectoryUnavailableError",
    "CurrentExecutablePathUnavailableError",
    "EnvironmentVariableNotFoundError",
    "InvalidEnvironmentVariableNameError",
];

impl TypeChecker {
    /// Register process-module-specific nominal error types.
    ///
    /// Shared core errors, including `InvalidUtf8Error`, are registered by other
    /// checker builtin paths; this only adds the process-specific error set.
    pub(super) fn register_process_builtins(&mut self) {
        for name in PROCESS_ERROR_NAMES {
            self.environment
                .register_type((*name).to_owned(), stdlib_error_core_type(name));
        }
    }
}
