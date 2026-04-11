//! Dependency resolution for project build planning.

extern crate alloc;

use crate::build_system::config::{version_satisfies_constraint, Dependency, Version};
use crate::build_system::BuildError;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Available package/version candidate in a dependency source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageVersion {
    /// Package name.
    pub name: String,
    /// Concrete available version.
    pub version: Version,
}

/// Dependency resolved to one concrete package version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDep {
    /// Package name.
    pub name: String,
    /// Selected package version.
    pub version: Version,
}

/// Resolve dependencies against available package versions.
///
/// # Errors
///
/// Returns [`BuildError::DependencyConflict`] when a package has multiple
/// incompatible constraints, or [`BuildError::PackageNotFound`] when no
/// available package version satisfies a dependency.
pub fn resolve_dependencies(
    dependencies: &[Dependency],
    available: &[PackageVersion],
) -> Result<Vec<ResolvedDep>, BuildError> {
    let grouped_constraints = group_constraints_by_name(dependencies);
    let mut resolved = Vec::new();

    for (name, constraints) in grouped_constraints {
        let selected =
            select_best_matching_version(name.as_str(), constraints.as_slice(), available)?;
        resolved.push(ResolvedDep {
            name,
            version: selected,
        });
    }

    resolved.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(resolved)
}

/// Group dependency constraints by package name.
fn group_constraints_by_name(dependencies: &[Dependency]) -> BTreeMap<String, Vec<&Dependency>> {
    let mut grouped = BTreeMap::new();
    for dependency in dependencies {
        grouped
            .entry(dependency.name.clone())
            .or_insert_with(Vec::new)
            .push(dependency);
    }
    grouped
}

/// Select highest version that satisfies all constraints for one package.
fn select_best_matching_version(
    package_name: &str,
    constraints: &[&Dependency],
    available: &[PackageVersion],
) -> Result<Version, BuildError> {
    let mut candidates = Vec::new();
    for candidate in available {
        if candidate.name != package_name {
            continue;
        }

        let mut all_constraints_satisfied = true;
        for constraint in constraints {
            if !version_satisfies_constraint(&candidate.version, &constraint.version_constraint) {
                all_constraints_satisfied = false;
                break;
            }
        }

        if all_constraints_satisfied {
            candidates.push(candidate.version.clone());
        }
    }

    if candidates.is_empty() {
        if constraints.len() > 1 {
            return Err(BuildError::DependencyConflict(package_name.to_owned()));
        }
        return Err(BuildError::PackageNotFound(package_name.to_owned()));
    }

    candidates.sort();
    if let Some(best) = candidates.pop() {
        return Ok(best);
    }

    Err(BuildError::PackageNotFound(package_name.to_owned()))
}
