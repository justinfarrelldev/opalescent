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

#[path = "wait/cancellation.rs"]
mod cancellation;
#[path = "wait/fairness.rs"]
mod fairness;

pub use cancellation::{CancellationSource, CancellationToken};
use fairness::{
    adjust_cursor_after_remove, recorded_cancellation_sequence, select_level_ready_wake,
    select_pending_wake,
};

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

/// Observable readiness after a source transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "later timer/process/terminal sources publish generic readiness transitions"
    )
)]
pub(crate) enum SourceAvailability {
    /// The source's level condition is ready.
    Ready,
    /// The source transitioned but is not level-ready.
    Idle,
}

impl SourceAvailability {
    /// Return whether this availability is level-ready.
    const fn is_ready(self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// Cloneable, host-stable readiness source identity.
#[derive(Clone)]
pub struct SystemReadinessSource {
    /// Shared source state retained by clones and live registrations.
    inner: Arc<ReadinessSourceInner>,
}

impl SystemReadinessSource {
    /// Create a new host-opaque readiness source for runtime subsystems.
    #[must_use]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "later runtime source implementations allocate identities through this crate-internal hook"
        )
    )]
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new(ReadinessSourceInner {
                id: next_identity(&NEXT_SOURCE_ID),
                state: Mutex::new(ReadinessSourceState::default()),
            }),
        }
    }

    /// Return whether two handles name the same stable source identity.
    #[must_use]
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc deref in this identity comparison is not accepted as const on the current toolchain"
    )]
    pub fn is_same_identity(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }

    /// Return the source's current generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        lock_or_recover(&self.inner.state).generation
    }

    /// Validate `generation` against the source's current observable state.
    #[must_use]
    pub fn readiness_status_for_generation(&self, generation: u64) -> SystemReadyWakeStatus {
        let state = lock_or_recover(&self.inner.state);
        if state.generation != generation {
            return SystemReadyWakeStatus::StaleGeneration;
        }
        if state.level_ready {
            SystemReadyWakeStatus::Current
        } else {
            SystemReadyWakeStatus::StaleReadiness
        }
    }

    /// Publish a transition and wake every currently registered wait set.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "later runtime source implementations publish readiness through this hook"
        )
    )]
    pub(crate) fn publish_transition(&self, availability: SourceAvailability) -> u64 {
        let (generation, sequence, wait_sets) = {
            let mut state = lock_or_recover(&self.inner.state);
            state.generation = state.generation.saturating_add(1);
            state.level_ready = availability.is_ready();
            let sequence = next_event_sequence();
            let mut wait_sets = Vec::new();
            state.registered_wait_sets.retain(|candidate| {
                candidate.wait_set.upgrade().is_some_and(|wait_set| {
                    wait_sets.push(wait_set);
                    true
                })
            });
            (state.generation, sequence, wait_sets)
        };

        for wait_set in wait_sets {
            wait_set.enqueue_transition(self, generation, sequence);
        }
        generation
    }

    /// Return the generation when the source is currently level-ready.
    fn current_ready_generation(&self) -> Option<u64> {
        let state = lock_or_recover(&self.inner.state);
        state.level_ready.then_some(state.generation)
    }

    /// Record one live wait-set registration retaining this source.
    fn retain_registration(&self, wait_set: &Arc<WaitSetInner>) {
        let mut state = lock_or_recover(&self.inner.state);
        state.registration_retain_count = state.registration_retain_count.saturating_add(1);
        let wait_set_id = wait_set.id;
        let mut retained_existing = false;
        for registered in &mut state.registered_wait_sets {
            let Some(registered_wait_set) = registered.wait_set.upgrade() else {
                continue;
            };
            if registered_wait_set.id == wait_set_id {
                registered.registration_count = registered.registration_count.saturating_add(1);
                retained_existing = true;
            }
        }
        state.registered_wait_sets.retain(|candidate| {
            candidate.wait_set.upgrade().is_some_and(|registered| {
                registered.id == wait_set_id || candidate.registration_count > 0
            })
        });
        if !retained_existing {
            state.registered_wait_sets.push(RegisteredWaitSet {
                wait_set: Arc::downgrade(wait_set),
                registration_count: 1,
            });
        }
    }

    /// Release one live wait-set registration retaining this source.
    fn release_registration(&self, wait_set: &Arc<WaitSetInner>) {
        let mut state = lock_or_recover(&self.inner.state);
        state.registration_release_count = state.registration_release_count.saturating_add(1);
        let wait_set_id = wait_set.id;
        for registered in &mut state.registered_wait_sets {
            let Some(registered_wait_set) = registered.wait_set.upgrade() else {
                registered.registration_count = 0;
                continue;
            };
            if registered_wait_set.id == wait_set_id {
                registered.registration_count = registered.registration_count.saturating_sub(1);
            }
        }
        state.registered_wait_sets.retain(|candidate| {
            candidate.registration_count > 0 && candidate.wait_set.upgrade().is_some()
        });
    }

    /// Return the number of registration retains observed by tests.
    #[cfg(test)]
    pub(crate) fn registration_retain_count(&self) -> u64 {
        lock_or_recover(&self.inner.state).registration_retain_count
    }

    /// Return the number of registration releases observed by tests.
    #[cfg(test)]
    pub(crate) fn registration_release_count(&self) -> u64 {
        lock_or_recover(&self.inner.state).registration_release_count
    }
}

impl fmt::Debug for SystemReadinessSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SystemReadinessSource")
            .finish_non_exhaustive()
    }
}

impl PartialEq for SystemReadinessSource {
    fn eq(&self, other: &Self) -> bool {
        self.is_same_identity(other)
    }
}

impl Eq for SystemReadinessSource {}

/// Shared readiness-source state.
struct ReadinessSourceInner {
    /// Stable source identity.
    id: u64,
    /// Mutable source readiness state.
    state: Mutex<ReadinessSourceState>,
}

/// A wait set currently subscribed to source transitions.
struct RegisteredWaitSet {
    /// Weak wait-set identity.
    wait_set: Weak<WaitSetInner>,
    /// Number of live registrations in that wait set targeting this source.
    registration_count: u64,
}

/// Mutable readiness-source state guarded by [`ReadinessSourceInner::state`].
#[derive(Default)]
struct ReadinessSourceState {
    /// Current generation, incremented on every published transition.
    generation: u64,
    /// Whether the source is currently level-ready.
    level_ready: bool,
    /// Wait sets that have at least one live registration for this source.
    registered_wait_sets: Vec<RegisteredWaitSet>,
    /// Number of registration retains, used by deterministic tests.
    registration_retain_count: u64,
    /// Number of registration releases, used by deterministic tests.
    registration_release_count: u64,
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
        if snapshot.current_source_id == source.inner.id {
            return Ok(());
        }

        let retargeted = owner.retarget_entry(snapshot.registration_id, source.clone())?;
        let Some(old_source) = retargeted else {
            return Ok(());
        };
        source.retain_registration(&owner);
        old_source.release_registration(&owner);
        lock_or_recover(&self.authority).current_source_id = source.inner.id;
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
            current_source_id: source.inner.id,
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

            let cancel_sequence = cancellation
                .request_sequence()
                .or_else(|| recorded_cancellation_sequence(&state, cancellation.generation()));
            if let Some(wake) = select_pending_wake(&mut state, cancel_sequence) {
                return Ok(wake);
            }
            if cancel_sequence.is_some() {
                return Ok(SystemWaitWake::Cancelled);
            }
            if let Some(wake) = select_level_ready_wake(&mut state) {
                return Ok(wake);
            }

            state = wait_or_recover(&self.inner.ready_changed, state);
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
    /// Record a cancellation request under the wait-set predicate mutex.
    fn record_cancellation_request(&self, generation: u64, sequence: u64) -> bool {
        let mut state = lock_or_recover(&self.state);
        if state.destroyed {
            drop(state);
            return false;
        }
        if state
            .cancellation_requests
            .iter()
            .any(|request| request.generation == generation)
        {
            drop(state);
            return true;
        }
        state.cancellation_requests.push(CancellationWake {
            generation,
            sequence,
        });
        drop(state);
        true
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
    /// Cancellation requests observed under this wait-set predicate mutex.
    cancellation_requests: Vec<CancellationWake>,
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

/// Cancellation request observed by this wait set.
struct CancellationWake {
    /// Cancellation generation identity.
    generation: u64,
    /// Global request sequence.
    sequence: u64,
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
