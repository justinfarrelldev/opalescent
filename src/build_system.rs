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
pub mod linker;
pub mod targets;

pub use cache::{BuildCache, hash_content};
pub use config::{Dependency, ProjectConfig, Version, VersionConstraint, parse_config};
pub use dependency::{PackageVersion, ResolvedDep, resolve_dependencies};
pub use incremental::modules_to_rebuild;
pub use linker::{Linker, LinkerCommand, detect_mingw, detect_preferred_linker};
pub use targets::{
    Architecture, Platform, TargetTriple, TripleEnv, dynamic_lib_extension, executable_filename,
    object_file_extension, parse_target_triple,
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
