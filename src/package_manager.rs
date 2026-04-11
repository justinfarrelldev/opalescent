#![expect(
    clippy::pub_use,
    reason = "Task 41 requires a package-manager API surface exposed from src/package_manager.rs"
)]

//! Package manager surface for project manifests, dependency resolution,
//! package registry access, installation, and publishing.

#[path = "package_manager/commands.rs"]
pub mod commands;
#[path = "package_manager/installer.rs"]
pub mod installer;
#[path = "package_manager/manifest.rs"]
pub mod manifest;
#[path = "package_manager/publisher.rs"]
pub mod publisher;
#[path = "package_manager/registry.rs"]
pub mod registry;
#[path = "package_manager/resolver.rs"]
pub mod resolver;

pub use commands::{dispatch_pkg_command, PkgCommand, PkgCommandResult};
pub use installer::{InstallPlan, Installer, InstallerError};
pub use manifest::{parse_manifest, Manifest, ManifestDependency, ManifestError};
pub use publisher::{PublishError, PublishPlan, Publisher};
pub use registry::{MockRegistry, PackageEntry, Registry, RegistryError};
pub use resolver::{resolve_manifest_deps, DependencyNode, ResolveError, ResolvedGraph};

/// Unified package-manager error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkgError {
    /// Manifest parse or validation failure.
    ManifestError(String),
    /// Dependency resolution failure.
    ResolveError(String),
    /// Registry access failure.
    RegistryError(String),
    /// Installation failure.
    InstallError(String),
    /// Publish failure.
    PublishError(String),
}

#[cfg(test)]
#[path = "package_manager/tests.rs"]
mod tests;
