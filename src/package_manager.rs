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

pub use commands::{PkgCommand, PkgCommandResult, dispatch_pkg_command};
pub use installer::{InstallPlan, Installer, InstallerError};
pub use manifest::{Manifest, ManifestDependency, ManifestError, parse_manifest};
pub use publisher::{PublishError, PublishPlan, Publisher};
pub use registry::{MockRegistry, PackageEntry, Registry, RegistryError};
pub use resolver::{DependencyNode, ResolveError, ResolvedGraph, resolve_manifest_deps};

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
