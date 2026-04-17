#![expect(
    clippy::pub_use,
    reason = "Task 41 requires a package-manager API surface exposed from src/package_manager.rs"
)]

//! Package manager surface for project manifests, dependency resolution,
//! package registry access, installation, and publishing.

pub mod commands;
pub mod installer;
pub mod manifest;
pub mod publisher;
pub mod registry;
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
mod tests;
