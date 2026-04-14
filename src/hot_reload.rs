#![expect(
    clippy::pub_use,
    reason = "Task 28 requires a hot-reload API surface exposed from src/hot_reload.rs"
)]

#[path = "hot_reload/abi.rs"]
pub mod abi;
#[path = "hot_reload/cache.rs"]
pub mod cache;
#[path = "hot_reload/change_detection.rs"]
pub mod change_detection;
#[path = "hot_reload/classifier.rs"]
pub mod classifier;
#[path = "hot_reload/dependency_graph.rs"]
pub mod dependency_graph;
#[path = "hot_reload/guard.rs"]
pub mod guard;
#[path = "hot_reload/loader.rs"]
pub mod loader;
#[path = "hot_reload/recovery.rs"]
pub mod recovery;
#[path = "hot_reload/state.rs"]
pub mod state;
#[path = "hot_reload/version.rs"]
pub mod version;

pub use abi::{
    generate_abi_signature, signatures_compatible, AbiSignature, ExportedFunction,
    FunctionSignature, ModuleVTable, PodLayout,
};
pub use cache::AbiSignatureCache;
pub use change_detection::{
    ChangeDetectionError, FileChangeEvent, FileWatcher, MockFileWatcher, PollingFileWatcher,
};
pub use classifier::{ChangeClassifier, HotReloadCategory, ReloadDecision};
pub use dependency_graph::ModuleDependencyGraph;
pub use guard::{AbiGuard, AbiGuardResult, FallbackRestartTrigger};
pub use loader::{
    hot_swap_module, FsModuleLoader, HostProcess, HotReloadError, LoadedModule, ModuleLoader,
};
pub use recovery::ErrorRecovery;
pub use state::{HostState, StateError, StatePreserver};
pub use version::{versioned_module_name, ModuleVersion};

#[cfg(test)]
#[path = "hot_reload/tests.rs"]
mod tests;
