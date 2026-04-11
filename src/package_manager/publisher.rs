//! Package publisher — creates distributable bundles and simulates publishing.

extern crate alloc;

use crate::package_manager::manifest::Manifest;
use alloc::string::String;
use alloc::vec::Vec;

/// Error during publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishError {
    /// Manifest validation failed before publishing.
    InvalidManifest(String),
    /// Remote registry rejected the upload.
    UploadFailed(String),
}

/// Description of a distributable package bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishPlan {
    /// Package name from manifest.
    pub name: String,
    /// Package version from manifest.
    pub version: String,
    /// Simulated archive byte length.
    pub archive_size_bytes: usize,
}

/// Trait abstracting the actual registry upload (mockable for tests).
pub trait Uploader {
    /// Upload the bundle bytes to the registry.
    ///
    /// # Errors
    ///
    /// Returns [`PublishError`] when the upload fails.
    fn upload(&self, name: &str, version: &str, bytes: &[u8]) -> Result<(), PublishError>;
}

/// Mock uploader that always succeeds.
#[derive(Debug, Clone, Default)]
pub struct MockUploader;

impl MockUploader {
    /// Create a new mock uploader.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Uploader for MockUploader {
    fn upload(&self, _name: &str, _version: &str, _bytes: &[u8]) -> Result<(), PublishError> {
        Ok(())
    }
}

/// Mock uploader that always fails.
#[derive(Debug, Clone, Default)]
pub struct FailingUploader;

impl FailingUploader {
    /// Create a new failing mock uploader.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Uploader for FailingUploader {
    fn upload(&self, name: &str, _version: &str, _bytes: &[u8]) -> Result<(), PublishError> {
        Err(PublishError::UploadFailed(name.to_owned()))
    }
}

/// Creates a distributable bundle and drives the upload step.
pub struct Publisher<'uploader> {
    /// Injected uploader.
    uploader: &'uploader dyn Uploader,
}

impl core::fmt::Debug for Publisher<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Publisher")
            .field("uploader", &"<dyn Uploader>")
            .finish()
    }
}

impl<'uploader> Publisher<'uploader> {
    /// Create a new publisher using the given [`Uploader`].
    #[must_use]
    pub const fn new(uploader: &'uploader dyn Uploader) -> Self {
        Self { uploader }
    }

    /// Build a [`PublishPlan`] from a manifest (no I/O).
    ///
    /// # Errors
    ///
    /// Returns [`PublishError`] when the manifest is invalid (empty name or
    /// version).
    pub fn plan(&self, manifest: &Manifest) -> Result<PublishPlan, PublishError> {
        if manifest.name.is_empty() {
            return Err(PublishError::InvalidManifest(String::from(
                "package name must not be empty",
            )));
        }
        if manifest.version.is_empty() {
            return Err(PublishError::InvalidManifest(String::from(
                "package version must not be empty",
            )));
        }

        let serialized = crate::package_manager::manifest::serialize_manifest(manifest);
        let archive_size_bytes = serialized.len();

        Ok(PublishPlan {
            name: manifest.name.clone(),
            version: manifest.version.clone(),
            archive_size_bytes,
        })
    }

    /// Execute the publish plan by uploading the bundle.
    ///
    /// # Errors
    ///
    /// Returns [`PublishError`] when the upload fails.
    pub fn publish(&self, plan: &PublishPlan, bytes: &[u8]) -> Result<(), PublishError> {
        self.uploader.upload(&plan.name, &plan.version, bytes)
    }

    /// Build and immediately publish a manifest.
    ///
    /// Returns the archive size if successful.
    ///
    /// # Errors
    ///
    /// Returns [`PublishError`] when planning or uploading fails.
    pub fn publish_manifest(&self, manifest: &Manifest) -> Result<usize, PublishError> {
        let plan = self.plan(manifest)?;
        let dummy_bytes: Vec<u8> = alloc::vec![0_u8; plan.archive_size_bytes];
        self.publish(&plan, &dummy_bytes)?;
        Ok(plan.archive_size_bytes)
    }
}
