//! Package registry — mockable interface for querying available packages.

extern crate alloc;

use crate::package_manager::manifest::ManifestDependency;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Error from a registry operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryError {
    /// Network or transport failure (simulated in tests).
    NetworkError(String),
    /// Package not found in registry.
    NotFound(String),
}

/// One package entry in the registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    /// Package name.
    pub name: String,
    /// Available version string.
    pub version: String,
    /// Download URL (may be empty in mock).
    pub url: String,
    /// SHA-256 checksum hex string (may be empty in mock).
    pub checksum: String,
}

/// Trait abstracting registry access so tests never touch a real network.
pub trait Registry {
    /// List all available versions for the named package.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the package is not found or a transport
    /// failure occurs.
    fn list_versions(&self, package_name: &str) -> Result<Vec<PackageEntry>, RegistryError>;

    /// Fetch metadata for a specific package version.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when the package/version pair is not found.
    fn fetch_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<PackageEntry, RegistryError>;

    /// List dependencies declared by a specific package version.
    ///
    /// Default implementation returns no dependencies, allowing simple registry
    /// implementations to omit transitive graph support.
    ///
    /// # Errors
    ///
    /// Returns [`RegistryError`] when dependency metadata cannot be read.
    fn list_dependencies(
        &self,
        _package_name: &str,
        _version: &str,
    ) -> Result<Vec<ManifestDependency>, RegistryError> {
        Ok(Vec::new())
    }
}

/// In-memory mock registry used in tests.
#[derive(Debug, Clone, Default)]
pub struct MockRegistry {
    /// Package entries keyed by `name::version`.
    entries: BTreeMap<String, PackageEntry>,
    /// Dependency lists keyed by `name::version`.
    dependencies: BTreeMap<String, Vec<ManifestDependency>>,
}

impl MockRegistry {
    /// Create a new, empty mock registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            dependencies: BTreeMap::new(),
        }
    }

    /// Register a package entry in the mock.
    pub fn register(&mut self, entry: PackageEntry) {
        self.register_with_dependencies(entry, Vec::new());
    }

    /// Register a package entry plus transitive dependency declarations.
    pub fn register_with_dependencies(
        &mut self,
        entry: PackageEntry,
        dependencies: Vec<ManifestDependency>,
    ) {
        let key = alloc::format!("{}::{}", entry.name, entry.version);
        self.entries.insert(key.clone(), entry);
        self.dependencies.insert(key, dependencies);
    }
}

impl Registry for MockRegistry {
    fn list_versions(&self, package_name: &str) -> Result<Vec<PackageEntry>, RegistryError> {
        let results: Vec<PackageEntry> = self
            .entries
            .values()
            .filter(|entry| entry.name == package_name)
            .cloned()
            .collect();
        if results.is_empty() {
            return Err(RegistryError::NotFound(package_name.to_owned()));
        }
        Ok(results)
    }

    fn fetch_metadata(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<PackageEntry, RegistryError> {
        let key = alloc::format!("{package_name}::{version}");
        self.entries
            .get(&key)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(alloc::format!("{package_name}@{version}")))
    }

    fn list_dependencies(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<Vec<ManifestDependency>, RegistryError> {
        let key = alloc::format!("{package_name}::{version}");
        self.dependencies
            .get(&key)
            .cloned()
            .ok_or_else(|| RegistryError::NotFound(alloc::format!("{package_name}@{version}")))
    }
}
