//! Package registry — mockable interface for querying available packages.

extern crate alloc;

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
}

/// In-memory mock registry used in tests.
#[derive(Debug, Clone, Default)]
pub struct MockRegistry {
    /// Package entries keyed by `name::version`.
    entries: BTreeMap<String, PackageEntry>,
}

impl MockRegistry {
    /// Create a new, empty mock registry.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    /// Register a package entry in the mock.
    pub fn register(&mut self, entry: PackageEntry) {
        let key = alloc::format!("{}::{}", entry.name, entry.version);
        self.entries.insert(key, entry);
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
}
