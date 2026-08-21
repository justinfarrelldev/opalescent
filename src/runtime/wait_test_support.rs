//! Test-only deterministic readiness sources for generic wait-set tests.
//!
//! This module is compiled only for crate tests and is intentionally not
//! re-exported from the production runtime surface.

use crate::runtime::wait::{SourceAvailability, SystemReadinessSource};

/// In-memory readiness source whose transitions are controlled by tests.
#[derive(Debug, Clone)]
pub struct FakeReadinessSource {
    /// Opaque runtime source identity used by wait sets.
    source: SystemReadinessSource,
}

impl FakeReadinessSource {
    /// Create a fake source with a stable host-opaque identity.
    pub fn new() -> Self {
        Self {
            source: SystemReadinessSource::new(),
        }
    }

    /// Return a clone of the opaque source handle.
    pub fn source(&self) -> SystemReadinessSource {
        self.source.clone()
    }

    /// Publish a ready transition and return the new generation.
    pub fn publish_ready(&self) -> u64 {
        self.source.publish_transition(SourceAvailability::Ready)
    }

    /// Publish an idle transition and return the new generation.
    pub fn publish_idle_transition(&self) -> u64 {
        self.source.publish_transition(SourceAvailability::Idle)
    }

    /// Return registration retain count for lifetime assertions.
    pub fn registration_retain_count(&self) -> u64 {
        self.source.registration_retain_count()
    }

    /// Return registration release count for lifetime assertions.
    pub fn registration_release_count(&self) -> u64 {
        self.source.registration_release_count()
    }
}

impl Default for FakeReadinessSource {
    fn default() -> Self {
        Self::new()
    }
}
