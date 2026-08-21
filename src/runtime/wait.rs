//! Generic wait-set, readiness-source, and cancellation runtime primitives.
//!
//! The types in this module are host-opaque: callers can clone and compare
//! readiness sources, but they never receive file descriptors, handles, raw
//! registration keys, or mutable host state.

extern crate alloc;
extern crate std;

use alloc::collections::{BTreeSet, VecDeque};
use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt;
use core::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Condvar, Mutex, MutexGuard};
use std::time::Duration;

#[path = "wait/cancellation.rs"]
mod cancellation;
#[path = "wait/fairness.rs"]
mod fairness;
#[path = "wait/source.rs"]
mod source;

pub use cancellation::{CancellationSource, CancellationToken};
use fairness::{
    adjust_cursor_after_remove, next_timer_wait_duration, select_level_ready_wake,
    select_pending_wake,
};
pub use source::SystemReadinessSource;
pub(crate) use source::{SourceAvailability, SourceWakePublication};

/// Monotonic identity source for readiness sources.
static NEXT_SOURCE_ID: AtomicU64 = AtomicU64::new(1);
/// Monotonic identity source for wait sets.
static NEXT_WAIT_SET_ID: AtomicU64 = AtomicU64::new(1);
/// Monotonic identity source for registrations.
static NEXT_REGISTRATION_ID: AtomicU64 = AtomicU64::new(1);
/// Monotonic identity source for authentication secrets.
static NEXT_AUTH_SECRET: AtomicU64 = AtomicU64::new(1);
/// Monotonic ordering for source and cancellation transitions.
static NEXT_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Return the next value from an atomic identity counter.
fn next_identity(counter: &AtomicU64) -> u64 {
    counter.fetch_add(1, Ordering::Relaxed)
}

/// Return the next global transition sequence.
fn next_event_sequence() -> u64 {
    NEXT_EVENT_SEQUENCE.fetch_add(1, Ordering::SeqCst)
}

/// Acquire a mutex and recover the inner value if a previous test panicked.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Wait on a condition variable and recover from poisoning.
fn wait_or_recover<'state, T>(
    condvar: &Condvar,
    guard: MutexGuard<'state, T>,
) -> MutexGuard<'state, T> {
    match condvar.wait(guard) {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Wait on a condition variable with a timeout and recover from poisoning.
fn wait_timeout_or_recover<'state, T>(
    condvar: &Condvar,
    guard: MutexGuard<'state, T>,
    duration: Duration,
) -> MutexGuard<'state, T> {
    match condvar.wait_timeout(guard, duration) {
        Ok((guard, _timeout_result)) => guard,
        Err(poisoned) => poisoned.into_inner().0,
    }
}

/// Errors produced by wait-set registration and lifetime checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemWaitSetError {
    /// A registration belongs to a different live wait set.
    WrongSet,
    /// A registration or owned authority is not authentic for its set.
    UnauthenticatedRegistration,
    /// The owning wait set, authority, or required lifetime is no longer live.
    RegistrationLifetimeInvalid,
}

impl fmt::Display for SystemWaitSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::WrongSet => f.write_str("SystemWaitSetError.WrongSet"),
            Self::UnauthenticatedRegistration => {
                f.write_str("SystemWaitSetError.UnauthenticatedRegistration")
            }
            Self::RegistrationLifetimeInvalid => {
                f.write_str("SystemWaitSetError.RegistrationLifetimeInvalid")
            }
        }
    }
}

impl std::error::Error for SystemWaitSetError {}

/// Status returned when a caller validates a ready wake against a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemReadyWakeStatus {
    /// The wake names the expected source and its currently observable generation.
    Current,
    /// The wake was a cancellation wake, not a ready wake.
    Cancelled,
    /// The wake names a different source identity than the caller expected.
    StaleSource,
    /// The wake's generation no longer matches the source generation.
    StaleGeneration,
    /// The source generation still matches, but the level condition is no longer ready.
    StaleReadiness,
}

/// A sealed ordinary registration authority for one wait-set entry.
#[derive(Clone)]
pub struct SystemWaitRegistration {
    /// Registration identity.
    registration_id: u64,
    /// Owning wait-set identity.
    wait_set_id: u64,
    /// Sealed authenticity token copied from the owning wait set.
    auth_secret: u64,
    /// Weak reference to the owning wait set lifetime.
    owner: Weak<WaitSetInner>,
}

impl SystemWaitRegistration {
    /// Create an unauthenticated registration for negative tests.
    #[cfg(test)]
    pub(crate) fn unauthenticated_for_tests(wait_set: &SystemWaitSet) -> Self {
        Self {
            registration_id: next_identity(&NEXT_REGISTRATION_ID),
            wait_set_id: wait_set.inner.id,
            auth_secret: wait_set.inner.auth_secret.saturating_add(1),
            owner: Arc::downgrade(&wait_set.inner),
        }
    }
}

impl fmt::Debug for SystemWaitRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemWaitRegistration")
            .finish_non_exhaustive()
    }
}

/// A sealed affine owned registration authority.
pub struct SystemOwnedWaitRegistration {
    /// Shared authority cell retained by the language owner and wait-set entry.
    authority: Arc<Mutex<OwnedAuthorityState>>,
}

impl SystemOwnedWaitRegistration {
    /// Remove this owned registration explicitly.
    ///
    /// Repeating removal while the owning set is live is an idempotent success.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError`] for unauthenticated or invalid lifetimes.
    pub fn remove(&mut self) -> Result<(), SystemWaitSetError> {
        let snapshot = self.authority_snapshot();
        let owner = authenticate_owned_snapshot(&snapshot)?;
        if snapshot.removed {
            return Ok(());
        }
        let removed_source = owner.remove_entry(snapshot.registration_id)?;
        if let Some(source) = removed_source {
            source.release_registration(&owner);
        }
        lock_or_recover(&self.authority).removed = true;
        Ok(())
    }

    /// Retarget this owned registration to `source` while preserving fairness order.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError`] for unauthenticated, removed, or invalid
    /// authorities.
    pub fn retarget(&mut self, source: &SystemReadinessSource) -> Result<(), SystemWaitSetError> {
        let snapshot = self.authority_snapshot();
        let owner = authenticate_owned_snapshot(&snapshot)?;
        if snapshot.removed {
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        }
        if snapshot.current_source_id == source.identity_for_wait_set() {
            return Ok(());
        }

        let retargeted = owner.retarget_entry(snapshot.registration_id, source.clone())?;
        let Some(old_source) = retargeted else {
            return Ok(());
        };
        source.retain_registration(&owner);
        old_source.release_registration(&owner);
        lock_or_recover(&self.authority).current_source_id = source.identity_for_wait_set();
        Ok(())
    }

    /// Create an unauthenticated owned authority for negative tests.
    #[cfg(test)]
    pub(crate) fn unauthenticated_for_tests(wait_set: &SystemWaitSet) -> Self {
        Self {
            authority: Arc::new(Mutex::new(OwnedAuthorityState {
                registration_id: next_identity(&NEXT_REGISTRATION_ID),
                wait_set_id: wait_set.inner.id,
                auth_secret: wait_set.inner.auth_secret.saturating_add(1),
                owner: Arc::downgrade(&wait_set.inner),
                removed: false,
                current_source_id: 0,
            })),
        }
    }

    /// Snapshot the authority state without retaining the lock across set edits.
    fn authority_snapshot(&self) -> OwnedAuthoritySnapshot {
        let state = lock_or_recover(&self.authority);
        OwnedAuthoritySnapshot {
            registration_id: state.registration_id,
            wait_set_id: state.wait_set_id,
            auth_secret: state.auth_secret,
            owner: Weak::<WaitSetInner>::clone(&state.owner),
            removed: state.removed,
            current_source_id: state.current_source_id,
        }
    }
}

impl Drop for SystemOwnedWaitRegistration {
    fn drop(&mut self) {
        let snapshot = self.authority_snapshot();
        if snapshot.removed {
            return;
        }
        let Ok(owner) = authenticate_owned_snapshot(&snapshot) else {
            return;
        };
        let Ok(removed_source) = owner.remove_entry(snapshot.registration_id) else {
            return;
        };
        if let Some(source) = removed_source {
            source.release_registration(&owner);
        }
        lock_or_recover(&self.authority).removed = true;
    }
}

impl fmt::Debug for SystemOwnedWaitRegistration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = lock_or_recover(&self.authority);
        f.debug_struct("SystemOwnedWaitRegistration")
            .field("removed", &state.removed)
            .finish_non_exhaustive()
    }
}

/// Snapshot of an owned authority used outside the authority-cell lock.
struct OwnedAuthoritySnapshot {
    /// Registration identity.
    registration_id: u64,
    /// Owning wait-set identity.
    wait_set_id: u64,
    /// Expected wait-set authentication secret.
    auth_secret: u64,
    /// Owning wait-set lifetime.
    owner: Weak<WaitSetInner>,
    /// Whether explicit removal already happened.
    removed: bool,
    /// Currently targeted source identity.
    current_source_id: u64,
}

/// Mutable state for one owned registration authority.
struct OwnedAuthorityState {
    /// Registration identity.
    registration_id: u64,
    /// Owning wait-set identity.
    wait_set_id: u64,
    /// Expected wait-set authentication secret.
    auth_secret: u64,
    /// Owning wait-set lifetime.
    owner: Weak<WaitSetInner>,
    /// Whether explicit removal already happened.
    removed: bool,
    /// Currently targeted source identity.
    current_source_id: u64,
}

/// Authenticate an owned-authority snapshot and return its owning set.
fn authenticate_owned_snapshot(
    snapshot: &OwnedAuthoritySnapshot,
) -> Result<Arc<WaitSetInner>, SystemWaitSetError> {
    let owner = snapshot
        .owner
        .upgrade()
        .ok_or(SystemWaitSetError::RegistrationLifetimeInvalid)?;
    if owner.id != snapshot.wait_set_id {
        return Err(SystemWaitSetError::UnauthenticatedRegistration);
    }
    if owner.auth_secret != snapshot.auth_secret {
        return Err(SystemWaitSetError::UnauthenticatedRegistration);
    }
    if owner.is_destroyed() {
        return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
    }
    Ok(owner)
}

/// A single affine wait set with bounded-fair registration order.
pub struct SystemWaitSet {
    /// Shared wait-set state referenced by registrations.
    inner: Arc<WaitSetInner>,
}

impl SystemWaitSet {
    /// Create a new empty wait set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(WaitSetInner {
                id: next_identity(&NEXT_WAIT_SET_ID),
                auth_secret: next_identity(&NEXT_AUTH_SECRET),
                state: Mutex::new(WaitSetState::default()),
                ready_changed: Condvar::new(),
            }),
        }
    }

    /// Register `source` and return an ordinary registration authority.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError::RegistrationLifetimeInvalid`] if the set
    /// has already been destroyed.
    pub fn register(
        &mut self,
        source: &SystemReadinessSource,
    ) -> Result<SystemWaitRegistration, SystemWaitSetError> {
        let registration_id = next_identity(&NEXT_REGISTRATION_ID);
        let registration = SystemWaitRegistration {
            registration_id,
            wait_set_id: self.inner.id,
            auth_secret: self.inner.auth_secret,
            owner: Arc::downgrade(&self.inner),
        };
        self.inner.push_entry(WaitSetEntry {
            registration_id,
            source: source.clone(),
            owned_authority: None,
        })?;
        source.retain_registration(&self.inner);
        Ok(registration)
    }

    /// Remove an ordinary registration from this wait set.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError`] when the registration is from another set,
    /// unauthenticated, or its owning lifetime is invalid.
    pub fn remove(
        &mut self,
        registration: &SystemWaitRegistration,
    ) -> Result<(), SystemWaitSetError> {
        let owner = registration
            .owner
            .upgrade()
            .ok_or(SystemWaitSetError::RegistrationLifetimeInvalid)?;
        if owner.auth_secret != registration.auth_secret {
            return Err(SystemWaitSetError::UnauthenticatedRegistration);
        }
        if owner.id != registration.wait_set_id {
            return Err(SystemWaitSetError::UnauthenticatedRegistration);
        }
        if owner.is_destroyed() {
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        }
        if owner.id != self.inner.id {
            return Err(SystemWaitSetError::WrongSet);
        }

        let removed_source = self.inner.remove_entry(registration.registration_id)?;
        if let Some(source) = removed_source {
            source.release_registration(&self.inner);
        }
        Ok(())
    }

    /// Register `source` and return the sole owned registration authority.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError::RegistrationLifetimeInvalid`] if the set
    /// has already been destroyed.
    pub fn register_owned(
        &mut self,
        source: &SystemReadinessSource,
    ) -> Result<SystemOwnedWaitRegistration, SystemWaitSetError> {
        let registration_id = next_identity(&NEXT_REGISTRATION_ID);
        let authority = Arc::new(Mutex::new(OwnedAuthorityState {
            registration_id,
            wait_set_id: self.inner.id,
            auth_secret: self.inner.auth_secret,
            owner: Arc::downgrade(&self.inner),
            removed: false,
            current_source_id: source.identity_for_wait_set(),
        }));
        self.inner.push_entry(WaitSetEntry {
            registration_id,
            source: source.clone(),
            owned_authority: Some(Arc::downgrade(&authority)),
        })?;
        source.retain_registration(&self.inner);
        Ok(SystemOwnedWaitRegistration { authority })
    }

    /// Wait until a queued ready transition, cancellation, or level-ready source exists.
    ///
    /// Already-published ready work is returned before a cancellation request.
    /// Once that older queued work drains, cancellation wins over newly ready
    /// source work and consumes no source readiness.
    ///
    /// # Errors
    ///
    /// Returns [`SystemWaitSetError::RegistrationLifetimeInvalid`] if the set is
    /// destroyed while waiting.
    pub fn wait_sync(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<SystemWaitWake, SystemWaitSetError> {
        let _watch = cancellation.watch_wait_set(&self.inner);
        let mut state = lock_or_recover(&self.inner.state);
        loop {
            if state.destroyed {
                return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
            }

            let cancel_sequence = cancellation.request_sequence();
            if let Some(wake) = select_pending_wake(&mut state, cancel_sequence) {
                return Ok(wake);
            }
            if cancel_sequence.is_some() {
                return Ok(SystemWaitWake::Cancelled);
            }
            if let Some(wake) = select_level_ready_wake(&mut state) {
                return Ok(wake);
            }

            if let Some(duration) = next_timer_wait_duration(&state) {
                if duration.is_zero() {
                    continue;
                }
                state = wait_timeout_or_recover(&self.inner.ready_changed, state, duration);
            } else {
                state = wait_or_recover(&self.inner.ready_changed, state);
            }
        }
    }
}

impl Default for SystemWaitSet {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SystemWaitSet {
    fn drop(&mut self) {
        let released_sources = self.inner.destroy();
        for source in released_sources {
            source.release_registration(&self.inner);
        }
        self.inner.ready_changed.notify_all();
    }
}

impl fmt::Debug for SystemWaitSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemWaitSet").finish_non_exhaustive()
    }
}

/// Shared wait-set state.
struct WaitSetInner {
    /// Stable wait-set identity.
    id: u64,
    /// Sealed authentication token for registrations from this set.
    auth_secret: u64,
    /// Mutable wait-set state.
    state: Mutex<WaitSetState>,
    /// Notification for source and cancellation transitions.
    ready_changed: Condvar,
}

impl WaitSetInner {
    /// Synchronize a cancellation request with the wait-set predicate mutex.
    fn synchronize_cancellation_request(&self) -> bool {
        let state = lock_or_recover(&self.state);
        let should_notify = !state.destroyed;
        drop(state);
        should_notify
    }

    /// Return whether this wait set has been destroyed.
    fn is_destroyed(&self) -> bool {
        lock_or_recover(&self.state).destroyed
    }

    /// Push a new live wait-set entry.
    fn push_entry(&self, entry: WaitSetEntry) -> Result<(), SystemWaitSetError> {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            drop(state);
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        }
        state.entries.push(entry);
        drop(state);
        Ok(())
    }

    /// Queue one transition wake for every matching live entry.
    fn enqueue_transition(&self, source: &SystemReadinessSource, generation: u64, sequence: u64) {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            return;
        }
        let mut queued_any = false;
        let matching_registration_ids: Vec<u64> = state
            .entries
            .iter()
            .filter(|entry| entry.source.is_same_identity(source))
            .map(|entry| entry.registration_id)
            .collect();
        for registration_id in matching_registration_ids {
            state.pending.push_back(PendingWake {
                registration_id,
                source: source.clone(),
                generation,
                sequence,
            });
            queued_any = true;
        }
        drop(state);
        if queued_any {
            self.ready_changed.notify_all();
        }
    }

    /// Notify waiters after a source predicate changes without queueing a wake.
    fn notify_source_state_change(&self) {
        let state = lock_or_recover(&self.state);
        let should_notify = !state.destroyed;
        drop(state);
        if should_notify {
            self.ready_changed.notify_all();
        }
    }

    /// Remove one live entry or recognize an idempotently removed registration.
    fn remove_entry(
        &self,
        registration_id: u64,
    ) -> Result<Option<SystemReadinessSource>, SystemWaitSetError> {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            drop(state);
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        }
        if let Some(index) = state
            .entries
            .iter()
            .position(|entry| entry.registration_id == registration_id)
        {
            let entry = state.entries.remove(index);
            state.removed_registrations.insert(registration_id);
            state
                .pending
                .retain(|wake| wake.registration_id != registration_id);
            adjust_cursor_after_remove(&mut state, index);
            let removed_source = entry.source.clone();
            drop(state);
            return Ok(Some(removed_source));
        }
        if state.removed_registrations.contains(&registration_id) {
            drop(state);
            return Ok(None);
        }
        drop(state);
        Err(SystemWaitSetError::UnauthenticatedRegistration)
    }

    /// Retarget one live entry and return the previous source on change.
    fn retarget_entry(
        &self,
        registration_id: u64,
        new_source: SystemReadinessSource,
    ) -> Result<Option<SystemReadinessSource>, SystemWaitSetError> {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            drop(state);
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        }
        let Some(entry) = state
            .entries
            .iter_mut()
            .find(|entry| entry.registration_id == registration_id)
        else {
            drop(state);
            return Err(SystemWaitSetError::RegistrationLifetimeInvalid);
        };
        if entry.source.is_same_identity(&new_source) {
            drop(state);
            return Ok(None);
        }
        let old_source = core::mem::replace(&mut entry.source, new_source);
        drop(state);
        Ok(Some(old_source))
    }

    /// Destroy the wait set and return sources released in reverse registration order.
    fn destroy(&self) -> Vec<SystemReadinessSource> {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            drop(state);
            return Vec::new();
        }
        state.destroyed = true;
        state.pending.clear();
        state.cursor = 0;
        let entries = core::mem::take(&mut state.entries);
        drop(state);
        entries
            .into_iter()
            .rev()
            .map(|entry| entry.source.clone())
            .collect()
    }
}

/// Mutable wait-set state guarded by [`WaitSetInner::state`].
#[derive(Default)]
struct WaitSetState {
    /// Whether the wait set has been destroyed.
    destroyed: bool,
    /// Live entries in registration/fairness order.
    entries: Vec<WaitSetEntry>,
    /// Registrations removed once and eligible for idempotent removal.
    removed_registrations: BTreeSet<u64>,
    /// Next fairness cursor index.
    cursor: usize,
    /// Transition wakes already published to this wait set.
    pending: VecDeque<PendingWake>,
}

/// One live wait-set entry.
struct WaitSetEntry {
    /// Registration identity.
    registration_id: u64,
    /// Currently targeted source.
    source: SystemReadinessSource,
    /// Owned authority cell when this entry is owned.
    owned_authority: Option<Weak<Mutex<OwnedAuthorityState>>>,
}

impl Drop for WaitSetEntry {
    fn drop(&mut self) {
        if let Some(authority) = self.owned_authority.as_ref().and_then(Weak::upgrade) {
            lock_or_recover(&authority).removed = true;
        }
    }
}

/// A ready transition queued for one registration.
struct PendingWake {
    /// Registration identity at publication time.
    registration_id: u64,
    /// Source identity observed at publication time.
    source: SystemReadinessSource,
    /// Source generation observed at publication time.
    generation: u64,
    /// Global publication sequence.
    sequence: u64,
}

/// Result of waiting on a [`SystemWaitSet`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SystemWaitWake {
    /// A readiness source produced a ready hint.
    Ready {
        /// Source identity observed by the wait set.
        source: SystemReadinessSource,
        /// Source generation observed by the wait set.
        generation: u64,
    },
    /// The supplied cancellation token was requested.
    Cancelled,
}

impl SystemWaitWake {
    /// Return the ready source if this wake is [`SystemWaitWake::Ready`].
    #[must_use]
    pub const fn ready_source(&self) -> Option<&SystemReadinessSource> {
        match *self {
            Self::Ready { ref source, .. } => Some(source),
            Self::Cancelled => None,
        }
    }

    /// Return the ready generation if this wake is [`SystemWaitWake::Ready`].
    #[must_use]
    pub const fn ready_generation(&self) -> Option<u64> {
        match *self {
            Self::Ready { generation, .. } => Some(generation),
            Self::Cancelled => None,
        }
    }

    /// Validate this wake against `expected_source`.
    #[must_use]
    pub fn readiness_against(
        &self,
        expected_source: &SystemReadinessSource,
    ) -> SystemReadyWakeStatus {
        match *self {
            Self::Cancelled => SystemReadyWakeStatus::Cancelled,
            Self::Ready {
                ref source,
                generation,
            } => {
                if !source.is_same_identity(expected_source) {
                    return SystemReadyWakeStatus::StaleSource;
                }
                expected_source.readiness_status_for_generation(generation)
            }
        }
    }
}
