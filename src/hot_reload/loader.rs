//! Host-controlled module loading and hot-swap orchestration.

extern crate alloc;

use crate::hot_reload::abi::{signatures_compatible, AbiSignature, ModuleVTable};
use crate::hot_reload::guard::{AbiGuard, AbiGuardResult, FallbackRestartTrigger};
use alloc::string::String;

/// Error variants produced by hot-reload module management.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotReloadError {
    /// Loader failed to open or resolve a module.
    ModuleLoadFailed {
        module_name: String,
        reason: String,
    },
    /// Loader failed to unload a module.
    ModuleUnloadFailed {
        module_name: String,
        reason: String,
    },
    /// Candidate module ABI is incompatible with currently active module.
    IncompatibleAbi {
        active_module: String,
        candidate_module: String,
    },
    RequiresFullRestart,
}

/// Runtime-loaded hot module metadata owned by the host process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedModule {
    /// File-system module identifier (e.g. `logic_v0001.so`).
    pub module_name: String,
    /// Narrow C ABI function table exported by module.
    pub vtable: ModuleVTable,
    /// Computed ABI signature used for compatibility checks.
    pub abi_signature: AbiSignature,
}

/// Mockable interface for module loading backends.
pub trait ModuleLoader {
    /// Loads a module by file name and returns its resolved metadata.
    ///
    /// # Errors
    ///
    /// Returns [`HotReloadError::ModuleLoadFailed`] when the backend cannot load
    /// or resolve the requested module.
    fn load_module(&mut self, module_name: &str) -> Result<LoadedModule, HotReloadError>;

    /// Unloads a previously loaded module by file name.
    ///
    /// # Errors
    ///
    /// Returns [`HotReloadError::ModuleUnloadFailed`] when the backend cannot
    /// unload the requested module.
    fn unload_module(&mut self, module_name: &str) -> Result<(), HotReloadError>;
}

/// Host process state owner for active hot-reload module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostProcess {
    /// Currently active loaded module owned by the host.
    active_module: Option<LoadedModule>,
}

impl HostProcess {
    /// Creates a host process with no loaded module.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            active_module: None,
        }
    }

    /// Returns the currently active module, if one is loaded.
    #[must_use]
    pub const fn active_module(&self) -> Option<&LoadedModule> {
        self.active_module.as_ref()
    }

    pub fn set_active_module(&mut self, module: LoadedModule) {
        self.active_module = Some(module);
    }
}

impl Default for HostProcess {
    fn default() -> Self {
        Self::new()
    }
}

/// Loads a new module and atomically hot-swaps it into the host.
///
/// # Errors
///
/// Returns [`HotReloadError::ModuleLoadFailed`] when loading the candidate
/// module fails, [`HotReloadError::IncompatibleAbi`] when ABI signatures differ,
/// and [`HotReloadError::ModuleUnloadFailed`] when unloading fails.
pub fn hot_swap_module(
    host_process: &mut HostProcess,
    loader: &mut dyn ModuleLoader,
    next_module_name: &str,
) -> Result<(), HotReloadError> {
    let next_module = loader.load_module(next_module_name)?;

    if let Some(active_module) = host_process.active_module.as_ref() {
        if !signatures_compatible(&active_module.abi_signature, &next_module.abi_signature)
            || AbiGuard::check(&active_module.abi_signature, &next_module.abi_signature)
                == AbiGuardResult::Reject
        {
            loader.unload_module(next_module_name)?;
            return Err(FallbackRestartTrigger::trigger());
        }

        loader.unload_module(&active_module.module_name)?;
    }

    host_process.set_active_module(next_module);
    Ok(())
}
