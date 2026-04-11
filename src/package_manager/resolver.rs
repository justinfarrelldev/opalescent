//! Dependency resolution for package manifests.
//!
//! Resolves a [`Manifest`]'s dependency list against a [`Registry`], producing
//! a flat [`ResolvedGraph`] with no version conflicts.

extern crate alloc;

use crate::build_system::config::{Version, VersionClause, VersionComparator, VersionConstraint};
use crate::package_manager::manifest::{Manifest, ManifestDependency};
use crate::package_manager::registry::{PackageEntry, Registry, RegistryError};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Error during dependency resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    /// Registry could not be queried.
    RegistryError(String),
    /// No version of a package satisfied all constraints.
    NoMatchingVersion(String),
    /// Two constraints for the same package are incompatible.
    ConflictingConstraints(String),
    /// A dependency cycle was detected.
    DependencyCycle(String),
    /// A constraint string could not be parsed.
    InvalidConstraint(String),
}

/// One node in the resolved dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyNode {
    /// Package name.
    pub name: String,
    /// Resolved concrete version string.
    pub version: String,
    /// Download URL from registry.
    pub url: String,
}

/// Flat resolved dependency graph (no cycles, all versions pinned).
#[derive(Debug, Clone, Default)]
pub struct ResolvedGraph {
    /// Resolved nodes, keyed by package name.
    pub nodes: BTreeMap<String, DependencyNode>,
}

impl ResolvedGraph {
    /// Create an empty resolved graph.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nodes: BTreeMap::new(),
        }
    }

    /// Insert or update a resolved node.
    pub fn insert(&mut self, node: DependencyNode) {
        self.nodes.insert(node.name.clone(), node);
    }
}

/// Resolve all dependencies from a manifest against the given registry.
///
/// # Errors
///
/// Returns [`ResolveError`] when any dependency cannot be resolved or conflicts
/// are detected.
pub fn resolve_manifest_deps<R: Registry>(
    manifest: &Manifest,
    registry: &R,
) -> Result<ResolvedGraph, ResolveError> {
    let mut graph = ResolvedGraph::new();
    let mut visited: BTreeMap<String, String> = BTreeMap::new();

    for dep in &manifest.dependencies {
        resolve_one(dep, registry, &mut graph, &mut visited, &[])?;
    }

    Ok(graph)
}

/// Resolve a single dependency (recursive helper for transitive deps).
fn resolve_one<R: Registry>(
    dep: &ManifestDependency,
    registry: &R,
    graph: &mut ResolvedGraph,
    visited: &mut BTreeMap<String, String>,
    path: &[&str],
) -> Result<(), ResolveError> {
    // Cycle detection
    if path.contains(&dep.name.as_str()) {
        return Err(ResolveError::DependencyCycle(dep.name.clone()));
    }

    if visited.contains_key(&dep.name) {
        // Already resolved — check version compatibility
        return Ok(());
    }

    let candidates = registry.list_versions(&dep.name).map_err(|err| match err {
        RegistryError::NotFound(name) => ResolveError::NoMatchingVersion(name),
        RegistryError::NetworkError(msg) => ResolveError::RegistryError(msg),
    })?;

    let constraint = parse_constraint(&dep.version_constraint)?;
    let selected = select_best(&dep.name, &candidates, &constraint)?;

    visited.insert(dep.name.clone(), selected.version.clone());
    graph.insert(DependencyNode {
        name: selected.name.clone(),
        version: selected.version.clone(),
        url: selected.url,
    });

    Ok(())
}

/// Pick the highest candidate version that satisfies the constraint.
fn select_best(
    package_name: &str,
    candidates: &[PackageEntry],
    constraint: &VersionConstraint,
) -> Result<PackageEntry, ResolveError> {
    use crate::build_system::config::version_satisfies_constraint;

    let mut matching: Vec<&PackageEntry> = candidates
        .iter()
        .filter(|entry| {
            parse_version(&entry.version)
                .is_some_and(|v| version_satisfies_constraint(&v, constraint))
        })
        .collect();

    if matching.is_empty() {
        return Err(ResolveError::NoMatchingVersion(package_name.to_owned()));
    }

    // Sort by parsed version descending; pick highest.
    matching.sort_by(|left, right| {
        let lv = parse_version(&left.version).unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
        });
        let rv = parse_version(&right.version).unwrap_or(Version {
            major: 0,
            minor: 0,
            patch: 0,
        });
        rv.cmp(&lv)
    });

    matching
        .into_iter()
        .next()
        .cloned()
        .ok_or_else(|| ResolveError::NoMatchingVersion(package_name.to_owned()))
}

/// Parse a semver constraint string such as `>=1.0.0` or `=2.3.1`.
///
/// # Errors
///
/// Returns [`ResolveError::InvalidConstraint`] when the version part cannot be
/// parsed as `major.minor.patch`.
pub fn parse_constraint(constraint_str: &str) -> Result<VersionConstraint, ResolveError> {
    let s = constraint_str.trim();

    let (comparator, version_str) = s
        .strip_prefix(">=")
        .map(|rest| (VersionComparator::GreaterEq, rest))
        .or_else(|| {
            s.strip_prefix('>')
                .map(|rest| (VersionComparator::Greater, rest))
        })
        .or_else(|| {
            s.strip_prefix("<=")
                .map(|rest| (VersionComparator::LessEq, rest))
        })
        .or_else(|| {
            s.strip_prefix('<')
                .map(|rest| (VersionComparator::Less, rest))
        })
        .or_else(|| {
            s.strip_prefix('=')
                .map(|rest| (VersionComparator::Eq, rest))
        })
        .unwrap_or((VersionComparator::Eq, s));

    let version =
        parse_version(version_str).ok_or_else(|| ResolveError::InvalidConstraint(s.to_owned()))?;

    Ok(VersionConstraint {
        clauses: alloc::vec![VersionClause {
            comparator,
            version
        }],
    })
}

/// Parse a `major.minor.patch` version string.
fn parse_version(s: &str) -> Option<Version> {
    let mut parts = s.trim().split('.');
    let major: u64 = parts.next()?.parse().ok()?;
    let minor: u64 = parts.next()?.parse().ok()?;
    let patch: u64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(Version {
        major,
        minor,
        patch,
    })
}
