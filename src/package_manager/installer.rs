//! Package installer — resolves and "installs" packages from a registry.
//!
//! All actual download/extraction is behind the [`Downloader`] trait so tests
//! never perform real I/O.

extern crate alloc;

use crate::package_manager::resolver::{DependencyNode, ResolvedGraph};
use alloc::string::String;
use alloc::vec::Vec;

/// Error during installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallerError {
    /// Download step failed (simulated in tests).
    DownloadFailed(String),
    /// Checksum verification failed.
    ChecksumMismatch(String),
    /// Extraction step failed.
    ExtractionFailed(String),
}

/// A download-and-extract operation for a single package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallPlan {
    /// Package name.
    pub name: String,
    /// Concrete version to install.
    pub version: String,
    /// Source URL.
    pub url: String,
}

/// Trait abstracting the actual download/extract step (mockable for tests).
pub trait Downloader {
    /// Simulate downloading a package from `url`.
    ///
    /// # Errors
    ///
    /// Returns [`InstallerError`] when the (mock) download fails.
    fn download(&self, name: &str, url: &str) -> Result<Vec<u8>, InstallerError>;
}

/// Mock downloader that always succeeds with dummy bytes.
#[derive(Debug, Clone, Default)]
pub struct MockDownloader;

impl MockDownloader {
    /// Create a new mock downloader.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Downloader for MockDownloader {
    fn download(&self, _name: &str, _url: &str) -> Result<Vec<u8>, InstallerError> {
        // Return minimal dummy archive bytes.
        Ok(alloc::vec![0_u8, 1_u8, 2_u8, 3_u8])
    }
}

/// Mock downloader that always fails.
#[derive(Debug, Clone, Default)]
pub struct FailingDownloader;

impl FailingDownloader {
    /// Create a new failing mock downloader.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Downloader for FailingDownloader {
    fn download(&self, name: &str, _url: &str) -> Result<Vec<u8>, InstallerError> {
        Err(InstallerError::DownloadFailed(name.to_owned()))
    }
}

/// Package installer that drives the resolved graph through download/extract.
pub struct Installer<'downloader> {
    /// Injected downloader implementation.
    downloader: &'downloader dyn Downloader,
}

impl core::fmt::Debug for Installer<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Installer")
            .field("downloader", &"<dyn Downloader>")
            .finish()
    }
}

impl<'downloader> Installer<'downloader> {
    /// Create a new installer using the given [`Downloader`].
    #[must_use]
    pub const fn new(downloader: &'downloader dyn Downloader) -> Self {
        Self { downloader }
    }

    /// Build an [`InstallPlan`] list from a resolved dependency graph.
    #[must_use]
    pub fn plan_from_graph(&self, graph: &ResolvedGraph) -> Vec<InstallPlan> {
        graph
            .nodes
            .values()
            .map(|node| InstallPlan {
                name: node.name.clone(),
                version: node.version.clone(),
                url: node.url.clone(),
            })
            .collect()
    }

    /// Execute an install plan using the injected downloader.
    ///
    /// Returns the list of installed package names.
    ///
    /// # Errors
    ///
    /// Returns [`InstallerError`] when any package download fails.
    pub fn execute(&self, plan: &[InstallPlan]) -> Result<Vec<String>, InstallerError> {
        let mut installed = Vec::new();
        for step in plan {
            self.downloader.download(&step.name, &step.url)?;
            installed.push(step.name.clone());
        }
        Ok(installed)
    }

    /// Install a single [`DependencyNode`] immediately.
    ///
    /// # Errors
    ///
    /// Returns [`InstallerError`] when the download fails.
    pub fn install_node(&self, node: &DependencyNode) -> Result<(), InstallerError> {
        self.downloader.download(&node.name, &node.url)?;
        Ok(())
    }
}
