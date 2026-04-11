//! Host state preservation primitives for hot-reload restart paths.

extern crate alloc;

use alloc::vec::Vec;

/// State serialization and restoration failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateError {
    /// Input byte payload is not valid for the expected state encoding.
    InvalidEncoding,
    /// Input payload structure is malformed for the state format.
    InvalidFormat,
}

/// Serializable host-process state contract used across reload boundaries.
pub trait HostState: Sized {
    /// Serializes state into bytes for temporary preservation.
    fn serialize(&self) -> Vec<u8>;

    /// Restores state from bytes.
    ///
    /// # Errors
    ///
    /// Returns [`StateError`] when the payload cannot be decoded safely.
    fn deserialize(data: &[u8]) -> Result<Self, StateError>;
}

/// Stateless state-preservation helper for save/restore flows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StatePreserver;

impl StatePreserver {
    /// Saves the host state into a byte buffer.
    #[must_use]
    pub fn save_state<TState: HostState>(state: &TState) -> Vec<u8> {
        state.serialize()
    }

    /// Restores host state from a serialized byte payload.
    ///
    /// # Errors
    ///
    /// Returns an error when payload decoding fails.
    pub fn restore_state<TState: HostState>(data: &[u8]) -> Result<TState, StateError> {
        TState::deserialize(data)
    }
}
