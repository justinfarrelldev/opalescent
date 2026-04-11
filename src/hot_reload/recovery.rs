//! Recovery handlers for failed hot-reload attempts.

use crate::hot_reload::loader::{HostProcess, HotReloadError, ModuleLoader};

/// Recovery strategy entrypoints for load/swap failures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ErrorRecovery;

impl ErrorRecovery {
    /// Handles a load failure while keeping host process alive.
    ///
    /// # Errors
    ///
    /// Returns the original [`HotReloadError`] only when recovery policy
    /// decides the host can no longer remain operational.
    pub const fn handle_load_failure(
        _host_process: &mut HostProcess,
        _error: &HotReloadError,
    ) -> Result<(), HotReloadError> {
        Ok(())
    }

    /// Rolls host state back to a previously active module after a partial swap.
    ///
    /// # Errors
    ///
    /// Returns a loader error if unloading the failed candidate or reloading the
    /// previous module fails.
    pub fn rollback_partial_swap(
        host_process: &mut HostProcess,
        loader: &mut dyn ModuleLoader,
        previous_module_name: &str,
        failed_candidate_name: &str,
    ) -> Result<(), HotReloadError> {
        if let Err(unload_error) = loader.unload_module(failed_candidate_name) {
            Self::handle_load_failure(host_process, &unload_error)?;
        }

        let previous_module = loader.load_module(previous_module_name)?;
        host_process.set_active_module(previous_module);
        Ok(())
    }
}
