#![expect(
    clippy::pub_use,
    reason = "Task 28 requires a hot-reload API surface exposed from src/hot_reload.rs"
)]

pub mod abi;
pub mod cache;
pub mod change_detection;
pub mod classifier;
pub mod dependency_graph;
pub mod guard;
pub mod loader;
pub mod recovery;
pub mod state;
pub mod version;

pub use abi::{
    AbiSignature, ExportedFunction, FunctionSignature, ModuleVTable, PodLayout,
    generate_abi_signature, signatures_compatible,
};
pub use cache::AbiSignatureCache;
pub use change_detection::{
    ChangeDetectionError, FileChangeEvent, FileWatcher, MockFileWatcher, PollingFileWatcher,
};
pub use classifier::{ChangeClassifier, HotReloadCategory, ReloadDecision};
pub use dependency_graph::ModuleDependencyGraph;
pub use guard::{AbiGuard, AbiGuardResult, FallbackRestartTrigger};
pub use loader::{
    FsModuleLoader, HostProcess, HotReloadError, LoadedModule, ModuleLoader, hot_swap_module,
};
pub use recovery::ErrorRecovery;
pub use state::{HostState, StateError, StatePreserver};
pub use version::{ModuleVersion, versioned_module_name};

#[cfg(test)]
mod tests;
