//! File system trait abstraction for the Opalescent standard library.
//!
//! All file system operations are behind the [`FileSystem`] trait so that:
//! 1. The implementation is fully mockable in tests (no actual I/O ever occurs).
//! 2. The core language components remain `no_std` compatible.
//! 3. Host implementations can be provided per platform without changing generated code.
//!
//! The only concrete implementation in this module is [`MockFileSystem`], which stores
//! file contents in a `BTreeMap` entirely in memory. Real host implementations are
//! expected to live outside the core language library.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Errors that can occur during file system operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    /// The requested file or directory does not exist.
    NotFound {
        /// The path that was not found.
        path: String,
    },

    /// The operation was denied due to permissions or other policy.
    PermissionDenied {
        /// The path on which the operation was denied.
        path: String,
    },

    /// An unspecified I/O error occurred.
    IoError {
        /// Human-readable description of the error.
        message: String,
    },
}

impl core::fmt::Display for FsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::NotFound { ref path } => write!(f, "file not found: {path}"),
            Self::PermissionDenied { ref path } => {
                write!(f, "permission denied: {path}")
            }
            Self::IoError { ref message } => write!(f, "I/O error: {message}"),
        }
    }
}

/// Abstract file system API for Opalescent programs.
///
/// Implementations must not perform real I/O in the standard library — this
/// trait is the boundary between generated code and the host environment.
pub trait FileSystem {
    /// Read the entire contents of `path` as a UTF-8 string.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::NotFound`] when the file does not exist, or
    /// [`FsError::IoError`] for other read failures.
    fn read_file(&self, path: &str) -> Result<String, FsError>;

    /// Write `content` to `path`, creating or overwriting as needed.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::PermissionDenied`] or [`FsError::IoError`] on failure.
    fn write_file(&mut self, path: &str, content: &str) -> Result<(), FsError>;

    /// Return `true` when `path` refers to an existing file.
    fn file_exists(&self, path: &str) -> bool;

    /// List all file paths directly inside the directory `path`.
    ///
    /// Returns paths as strings, not guaranteed to be sorted.
    ///
    /// # Errors
    ///
    /// Returns [`FsError::NotFound`] when the directory does not exist.
    fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError>;
}

/// In-memory mock file system for use in tests.
///
/// All files are stored in a `BTreeMap<String, String>` keyed by path.
/// No actual file I/O is performed — all operations are purely in-memory.
/// This is the canonical test double for the [`FileSystem`] trait.
#[derive(Debug, Default, Clone)]
pub struct MockFileSystem {
    /// In-memory store mapping path to file content.
    files: BTreeMap<String, String>,
}

impl MockFileSystem {
    /// Create a new empty mock file system.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            files: BTreeMap::new(),
        }
    }
}

impl FileSystem for MockFileSystem {
    fn read_file(&self, path: &str) -> Result<String, FsError> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::NotFound {
                path: path.to_owned(),
            })
    }

    fn write_file(&mut self, path: &str, content: &str) -> Result<(), FsError> {
        self.files.insert(path.to_owned(), content.to_owned());
        Ok(())
    }

    fn file_exists(&self, path: &str) -> bool {
        self.files.contains_key(path)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, FsError> {
        let prefix = if path.ends_with('/') {
            path.to_owned()
        } else {
            let mut with_slash = path.to_owned();
            with_slash.push('/');
            with_slash
        };

        let entries: Vec<String> = self
            .files
            .keys()
            .filter(|file_path| file_path.starts_with(prefix.as_str()))
            .cloned()
            .collect();

        Ok(entries)
    }
}
