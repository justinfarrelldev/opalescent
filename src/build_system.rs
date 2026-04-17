#![expect(
    clippy::pub_use,
    reason = "Task 33 requires a build-system API surface exposed from src/build_system.rs"
)]

//! Build-system surface for project configuration, dependency resolution, caching,
//! incremental compilation planning, and target selection.

pub mod cache;
pub mod config;
pub mod dependency;
pub mod incremental;
pub mod targets;

pub use cache::{hash_content, BuildCache};
pub use config::{parse_config, Dependency, ProjectConfig, Version, VersionConstraint};
pub use dependency::{resolve_dependencies, PackageVersion, ResolvedDep};
pub use incremental::modules_to_rebuild;
pub use targets::{
    dynamic_lib_extension, parse_target_triple, Architecture, Platform, TargetTriple,
};

/// Unified build-system error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Generic parse failure with context.
    ParseError(String),
    /// Required field was not present.
    MissingField(String),
    /// Invalid semantic version string.
    InvalidVersion(String),
    /// Invalid version constraint expression.
    InvalidConstraint(String),
    /// Two constraints for same package are incompatible.
    DependencyConflict(String),
    /// No available package version satisfied a dependency.
    PackageNotFound(String),
    /// Unsupported or malformed target triple.
    InvalidTarget(String),
}

#[cfg(test)]
mod tests;
