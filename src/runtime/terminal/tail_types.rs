//! Small terminal tail enums kept separate to satisfy line-count limits.

extern crate alloc;

use super::constraints::TerminalWaitMilliseconds;
use super::model::TerminalOrdinaryFeature;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalSessionOptionsError {
    InvalidOptions {
        invalid_options: TerminalInvalidOptions,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalInvalidOptions {
    RetainedCapacityTooSmall {
        required_bytes: u64,
        configured_bytes: u64,
    },
    CorrelatedGroupTooLarge {
        required_events: u64,
        configured_events: u64,
    },
}
use alloc::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalRecoveryLedgerKind {
    OpenRollback,
    CloseRestore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalSessionState {
    Active,
    Paused,
    RestorePending,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalWait {
    Poll,
    Forever,
    For {
        milliseconds: TerminalWaitMilliseconds,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalFeature {
    AlternateScreen,
    CursorShape,
    BracketedPaste,
    FocusEvents,
    MouseButtons,
    MouseMotion,
    KeyReleaseEvents,
    CompositionEvents,
    EnhancedKeyIdentity,
    TrustedPasteFraming,
}

#[must_use]
pub fn required_ordinary_features() -> BTreeSet<TerminalOrdinaryFeature> {
    BTreeSet::from([
        TerminalOrdinaryFeature::AlternateScreen,
        TerminalOrdinaryFeature::CursorShape,
        TerminalOrdinaryFeature::BracketedPaste,
        TerminalOrdinaryFeature::FocusEvents,
        TerminalOrdinaryFeature::MouseButtons,
        TerminalOrdinaryFeature::MouseMotion,
        TerminalOrdinaryFeature::KeyReleaseEvents,
        TerminalOrdinaryFeature::CompositionEvents,
        TerminalOrdinaryFeature::EnhancedKeyIdentity,
    ])
}
