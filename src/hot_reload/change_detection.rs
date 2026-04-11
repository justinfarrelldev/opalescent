//! Change detection abstractions for hot-reload planning.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Error variants produced by file-change detection implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeDetectionError {
    /// The watcher could not be started.
    StartFailed { reason: String },
}

/// File change payload emitted by [`FileWatcher`] implementations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChangeEvent {
    /// Path (or module-relative key) identifying the changed file.
    pub file_path: String,
}

impl FileChangeEvent {
    /// Creates a change event from a file path.
    #[must_use]
    pub fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_owned(),
        }
    }
}

/// Abstraction for file watching used by the hot-reload orchestrator.
///
/// Implementations are intentionally decoupled from real OS watchers so tests
/// can use fully in-memory mocks.
pub trait FileWatcher {
    /// Starts the watcher.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeDetectionError::StartFailed`] when startup fails.
    fn start(&mut self) -> Result<(), ChangeDetectionError>;

    /// Returns all accumulated file changes since the previous poll.
    fn poll_changes(&mut self) -> Vec<FileChangeEvent>;
}

/// In-memory watcher used by tests and deterministic simulations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockFileWatcher {
    /// Queue of pending synthetic file changes.
    queued_changes: Vec<FileChangeEvent>,
    /// Startup failure reason used to simulate watcher init errors.
    fail_reason: Option<String>,
}

impl MockFileWatcher {
    /// Creates a mock watcher with pre-seeded change events.
    #[must_use]
    pub const fn new(queued_changes: Vec<FileChangeEvent>) -> Self {
        Self {
            queued_changes,
            fail_reason: None,
        }
    }

    /// Creates a mock watcher that fails on startup.
    #[must_use]
    pub fn failing_start(reason: &str) -> Self {
        Self {
            queued_changes: Vec::new(),
            fail_reason: Some(reason.to_owned()),
        }
    }
}

impl FileWatcher for MockFileWatcher {
    fn start(&mut self) -> Result<(), ChangeDetectionError> {
        if let Some(reason) = self.fail_reason.clone() {
            return Err(ChangeDetectionError::StartFailed { reason });
        }
        Ok(())
    }

    fn poll_changes(&mut self) -> Vec<FileChangeEvent> {
        let mut drained = Vec::new();
        core::mem::swap(&mut drained, &mut self.queued_changes);
        drained
    }
}
