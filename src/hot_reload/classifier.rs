//! ABI change classification for hot-reload orchestration.

extern crate alloc;

use crate::hot_reload::abi::AbiSignature;
use alloc::string::String;
use alloc::vec::Vec;

/// Reload category selected from ABI-level change analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotReloadCategory {
    /// Function-body-only change; ABI surface is stable.
    HotSwappable,
    /// Function signature change; requires process restart.
    RequiresRestart,
    /// Type-layout change; requires full restart and full state rebuild.
    FullRestart,
}

/// Decision payload emitted by change classification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadDecision {
    /// Classification category for this ABI transition.
    pub category: HotReloadCategory,
    /// Modules that should be invalidated/rebuilt by the caller.
    pub invalidated_modules: Vec<String>,
}

impl ReloadDecision {
    /// Creates a decision with no invalidated modules.
    #[must_use]
    pub const fn from_category(category: HotReloadCategory) -> Self {
        Self {
            category,
            invalidated_modules: Vec::new(),
        }
    }
}

/// Compares old/new ABI signatures and chooses reload strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ChangeClassifier;

impl ChangeClassifier {
    /// Classifies the ABI transition from `old` to `new`.
    #[must_use]
    pub fn classify(old: &AbiSignature, new: &AbiSignature) -> HotReloadCategory {
        if old.exported_pod_types != new.exported_pod_types {
            return HotReloadCategory::FullRestart;
        }

        if old.exported_functions != new.exported_functions {
            return HotReloadCategory::RequiresRestart;
        }

        HotReloadCategory::HotSwappable
    }

    /// Produces a full reload decision with optional module invalidation payload.
    #[must_use]
    pub fn classify_with_invalidation(
        old: &AbiSignature,
        new: &AbiSignature,
        invalidated_modules: Vec<String>,
    ) -> ReloadDecision {
        ReloadDecision {
            category: Self::classify(old, new),
            invalidated_modules,
        }
    }
}
