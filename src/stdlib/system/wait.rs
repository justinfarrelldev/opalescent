//! Generic system wait-set and cancellation facade.
//!
//! The functions here mirror the future `standard.system` core prerequisite
//! names while delegating to the host-opaque runtime primitives. They expose no
//! OS descriptors, handles, registration keys, or terminal-specific identities.

use crate::runtime::wait::{
    CancellationSource, CancellationToken, SystemOwnedWaitRegistration, SystemReadinessSource,
    SystemWaitRegistration, SystemWaitSet, SystemWaitSetError, SystemWaitWake,
};

/// Create a new generic wait set.
#[must_use]
pub fn system_wait_set_new() -> SystemWaitSet {
    SystemWaitSet::new()
}

/// Register `source` in `wait_set` and return an ordinary registration.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] when the wait-set lifetime is invalid.
pub fn system_wait_set_register(
    wait_set: &mut SystemWaitSet,
    source: &SystemReadinessSource,
) -> Result<SystemWaitRegistration, SystemWaitSetError> {
    wait_set.register(source)
}

/// Remove `registration` from `wait_set`.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] for wrong-set, unauthenticated, or invalid
/// registration lifetimes.
pub fn system_wait_set_remove(
    wait_set: &mut SystemWaitSet,
    registration: &SystemWaitRegistration,
) -> Result<(), SystemWaitSetError> {
    wait_set.remove(registration)
}

/// Register `source` and return the sole owned registration authority.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] when the wait-set lifetime is invalid.
pub fn system_wait_set_register_owned(
    wait_set: &mut SystemWaitSet,
    source: &SystemReadinessSource,
) -> Result<SystemOwnedWaitRegistration, SystemWaitSetError> {
    wait_set.register_owned(source)
}

/// Retarget an owned registration to `source` without changing fairness order.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] for unauthenticated or invalid authorities.
pub fn system_owned_wait_registration_retarget(
    registration: &mut SystemOwnedWaitRegistration,
    source: &SystemReadinessSource,
) -> Result<(), SystemWaitSetError> {
    registration.retarget(source)
}

/// Remove an owned registration authority.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] for unauthenticated or invalid authorities.
pub fn system_owned_wait_registration_remove(
    registration: &mut SystemOwnedWaitRegistration,
) -> Result<(), SystemWaitSetError> {
    registration.remove()
}

/// Wait for a ready source or cancellation.
///
/// # Errors
///
/// Returns [`SystemWaitSetError`] if the wait set lifetime becomes invalid.
pub fn system_wait_set_wait_sync(
    wait_set: &mut SystemWaitSet,
    cancellation: &CancellationToken,
) -> Result<SystemWaitWake, SystemWaitSetError> {
    wait_set.wait_sync(cancellation)
}

/// Create a new sticky cancellation source generation.
#[must_use]
pub fn cancellation_source_new() -> CancellationSource {
    CancellationSource::new()
}

/// Return a token for this exact cancellation source generation.
#[must_use]
pub fn cancellation_token(source: &CancellationSource) -> CancellationToken {
    source.token()
}

/// Request cancellation for this source generation.
pub fn cancellation_request(source: &mut CancellationSource) {
    source.request();
}
