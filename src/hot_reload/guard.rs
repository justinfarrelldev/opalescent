//! ABI guard decisions for safe hot-reload transitions.

use crate::hot_reload::abi::{signatures_compatible, AbiSignature};
use crate::hot_reload::loader::HotReloadError;

/// ABI guard decision for an incoming module candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbiGuardResult {
    /// Incoming ABI is compatible and may be loaded safely.
    Accept,
    /// Incoming ABI is incompatible and must be rejected.
    Reject,
}

/// Machine-checkable ABI guard that compares current and incoming signatures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AbiGuard;

impl AbiGuard {
    /// Compares host-active and incoming ABI signatures.
    #[must_use]
    pub fn check(current: &AbiSignature, incoming: &AbiSignature) -> AbiGuardResult {
        if signatures_compatible(current, incoming) {
            AbiGuardResult::Accept
        } else {
            AbiGuardResult::Reject
        }
    }
}

/// Fallback restart trigger used when ABI compatibility checks fail.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FallbackRestartTrigger;

impl FallbackRestartTrigger {
    /// Signals that the host must perform a full orchestrated restart.
    #[must_use]
    pub const fn trigger() -> HotReloadError {
        HotReloadError::RequiresFullRestart
    }
}
