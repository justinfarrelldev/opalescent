//! Host-controlled module loading and hot-swap orchestration.

extern crate alloc;

use crate::hot_reload::abi::{AbiSignature, ModuleVTable, signatures_compatible};
use crate::hot_reload::guard::{AbiGuard, AbiGuardResult, FallbackRestartTrigger};
use alloc::string::String;
use core::sync::atomic::{AtomicU64, Ordering};
use libloading::Library;
use std::ffi::OsStr;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

/// Filesystem-backed module loader used in production code paths.
#[derive(Debug, Default)]
pub struct FsModuleLoader {
    loaded_libraries: HashMap<String, Library>,
    loaded_library_paths: HashMap<String, PathBuf>,
}

static TEMP_COPY_COUNTER: AtomicU64 = AtomicU64::new(0);

impl FsModuleLoader {
    /// Create a new filesystem-backed module loader.
    #[must_use]
    pub fn new() -> Self {
        Self {
            loaded_libraries: HashMap::new(),
            loaded_library_paths: HashMap::new(),
        }
    }

    fn temp_copy_path_for(module_name: &str) -> PathBuf {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0_u128, |duration| duration.as_nanos());
        let unique = TEMP_COPY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let extension = Path::new(module_name)
            .extension()
            .and_then(OsStr::to_str)
            .map_or("so", |value| value);
        std::env::temp_dir().join(format!(
            "opalescent_hot_reload_{}_{}_{}.{}",
            std::process::id(),
            now,
            unique,
            extension
        ))
    }

    fn load_library(module_name: &str) -> Result<(Library, PathBuf), HotReloadError> {
        if fs::metadata(module_name).is_err() {
            return Err(HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module file not found"),
            });
        }

        let copy_path = Self::temp_copy_path_for(module_name);
        fs::copy(module_name, &copy_path).map_err(|error| HotReloadError::ModuleLoadFailed {
            module_name: module_name.to_owned(),
            reason: format!("failed to create temp module copy: {error}"),
        })?;

        // SAFETY: Loading a dynamic library is inherently unsafe. The path is an owned
        // temporary copy created specifically for hot reload and lives long enough
        // for the returned Library handle.
        let library = unsafe { Library::new(&copy_path) }.map_err(|error| {
            HotReloadError::ModuleLoadFailed {
                module_name: module_name.to_owned(),
                reason: format!("failed to open shared library: {error}"),
            }
        })?;

        Ok((library, copy_path))
    }

    fn resolve_module_entry(
        library: &Library,
        module_name: &str,
    ) -> Result<extern "C" fn(), HotReloadError> {
        // SAFETY: The symbol name is null-terminated and expected to resolve to
        // the module's exported entrypoint with C ABI.
        let module_entry = unsafe {
            library.get::<unsafe extern "C" fn()>(b"module_entry\0")
        }
        .map_err(|error| HotReloadError::ModuleLoadFailed {
            module_name: module_name.to_owned(),
            reason: format!("failed to resolve module_entry symbol: {error}"),
        })?;
        let unsafe_entry = *module_entry;
        // SAFETY: `module_entry` is stored and later invoked through the existing
        // ModuleVTable contract (`extern "C" fn()`).
        let safe_entry: extern "C" fn() = unsafe { core::mem::transmute(unsafe_entry) };
        Ok(safe_entry)
    }
}

impl ModuleLoader for FsModuleLoader {
    fn load_module(&mut self, module_name: &str) -> Result<LoadedModule, HotReloadError> {
        let (library, copied_path) = Self::load_library(module_name)?;
        let module_entry = Self::resolve_module_entry(&library, module_name)?;

        self.loaded_libraries.insert(module_name.to_owned(), library);
        self.loaded_library_paths
            .insert(module_name.to_owned(), copied_path);

        Ok(LoadedModule {
            module_name: module_name.to_owned(),
            vtable: ModuleVTable {
                module_entry,
            },
            abi_signature: AbiSignature::new(),
        })
    }

    fn unload_module(&mut self, module_name: &str) -> Result<(), HotReloadError> {
        let Some(library) = self.loaded_libraries.remove(module_name) else {
            return Err(HotReloadError::ModuleUnloadFailed {
                module_name: module_name.to_owned(),
                reason: String::from("module is not currently loaded"),
            });
        };
        drop(library);

        if let Some(temp_path) = self.loaded_library_paths.remove(module_name) {
            let _ = fs::remove_file(temp_path);
        }

        Ok(())
    }
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
