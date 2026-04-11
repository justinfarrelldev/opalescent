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
#[path = "hot_reload/loader.rs"]
pub mod loader;
#[path = "hot_reload/version.rs"]
pub mod version;

pub use abi::{
    generate_abi_signature, signatures_compatible, AbiSignature, ExportedFunction,
    FunctionSignature, ModuleVTable, PodLayout,
};
pub use cache::AbiSignatureCache;
pub use change_detection::{ChangeDetectionError, FileChangeEvent, FileWatcher, MockFileWatcher};
pub use classifier::{ChangeClassifier, HotReloadCategory, ReloadDecision};
pub use dependency_graph::ModuleDependencyGraph;
pub use loader::{hot_swap_module, HostProcess, HotReloadError, LoadedModule, ModuleLoader};
pub use version::{versioned_module_name, ModuleVersion};

#[cfg(test)]
#[path = "hot_reload/tests.rs"]
mod tests;
