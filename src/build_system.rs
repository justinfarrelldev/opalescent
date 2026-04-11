#![expect(
    clippy::pub_use,
    reason = "Task 33 requires a build-system API surface exposed from src/build_system.rs"
)]

//! Build-system surface for project configuration, dependency resolution, caching,
//! incremental compilation planning, and target selection.

#[path = "build_system/cache.rs"]
pub mod cache;
#[path = "build_system/config.rs"]
pub mod config;
#[path = "build_system/dependency.rs"]
pub mod dependency;
#[path = "build_system/incremental.rs"]
pub mod incremental;
#[path = "build_system/targets.rs"]
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
#[path = "build_system/tests.rs"]
mod tests;
