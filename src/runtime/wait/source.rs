//! Readiness-source identities and publication state for generic wait sets.

extern crate alloc;
extern crate std;

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;
use core::fmt;
use std::sync::Mutex;
use std::time::Duration;

use crate::runtime::timer::{MonotonicClockHandle, MonotonicDeadline};

use super::{
    NEXT_SOURCE_ID, SystemReadyWakeStatus, WaitSetInner, lock_or_recover, next_event_sequence,
    next_identity,
};

/// Observable readiness after a source transition.
#[derive(Debug, Clone)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "later process/terminal sources publish generic readiness transitions"
    )
)]
pub enum SourceAvailability {
    /// The source's level condition is ready.
    Ready,
    /// The source transitioned but is not level-ready.
    Idle,
    /// The source becomes level-ready when the clock reaches `deadline`.
    TimerDeadline {
        /// Exact deadline that makes the source ready.
        deadline: MonotonicDeadline,
        /// Clock used to evaluate the deadline.
        clock: MonotonicClockHandle,
    },
}

impl SourceAvailability {
    /// Return whether this availability is level-ready right now.
    #[must_use]
    pub(crate) fn is_ready(&self) -> bool {
        match *self {
            Self::Ready => true,
            Self::Idle => false,
            Self::TimerDeadline {
                deadline,
                ref clock,
            } => deadline.is_expired_by(clock.now()),
        }
    }

    /// Return the real wait duration until this source can become ready.
    #[must_use]
    fn wait_duration_until_ready(&self) -> Option<Duration> {
        match *self {
            Self::Ready | Self::Idle => None,
            Self::TimerDeadline {
                deadline,
                ref clock,
            } => clock.wait_duration_until(deadline),
        }
    }
}

/// How a source state transition should notify subscribed wait sets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWakePublication {
    /// Queue one ready hint for each matching registration.
    QueueReady,
    /// Only notify waiters so they recompute generic readiness predicates.
    NotifyOnly,
}

impl SourceWakePublication {
    /// Return whether this publication queues ready hints.
    const fn queues_ready(self) -> bool {
        matches!(self, Self::QueueReady)
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

    /// Return the source identity for wait-set owned authority bookkeeping.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Arc deref in this identity accessor is not accepted as const on the current toolchain"
    )]
    pub(super) fn identity_for_wait_set(&self) -> u64 {
        self.inner.id
    }

    /// Validate `generation` against the source's current observable state.
    #[must_use]
    pub fn readiness_status_for_generation(&self, generation: u64) -> SystemReadyWakeStatus {
        let state = lock_or_recover(&self.inner.state);
        if state.generation != generation {
            return SystemReadyWakeStatus::StaleGeneration;
        }
        if state.availability.is_ready() {
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
            reason = "later process/terminal sources publish readiness through this hook"
        )
    )]
    pub(crate) fn publish_transition(&self, availability: SourceAvailability) -> u64 {
        self.publish_state_change(None, availability, SourceWakePublication::QueueReady)
    }

    /// Publish an exact generation transition for owner-managed generation sources.
    pub(crate) fn publish_generation_transition(
        &self,
        generation: u64,
        availability: SourceAvailability,
        publication: SourceWakePublication,
    ) {
        self.publish_state_change(Some(generation), availability, publication);
    }

    /// Publish a source state change and notify subscribed wait sets.
    fn publish_state_change(
        &self,
        exact_generation: Option<u64>,
        availability: SourceAvailability,
        publication: SourceWakePublication,
    ) -> u64 {
        let (generation, sequence, wait_sets) = {
            let mut state = lock_or_recover(&self.inner.state);
            let generation = exact_generation.unwrap_or_else(|| state.generation.saturating_add(1));
            state.generation = generation;
            state.availability = availability;
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
            if publication.queues_ready() {
                wait_set.enqueue_transition(self, generation, sequence);
            } else {
                wait_set.notify_source_state_change();
            }
        }
        generation
    }

    /// Return the generation when the source is currently level-ready.
    pub(super) fn current_ready_generation(&self) -> Option<u64> {
        let state = lock_or_recover(&self.inner.state);
        state.availability.is_ready().then_some(state.generation)
    }

    /// Return the duration before this source can become level-ready.
    pub(super) fn wait_duration_until_ready(&self) -> Option<Duration> {
        lock_or_recover(&self.inner.state)
            .availability
            .wait_duration_until_ready()
    }

    /// Record one live wait-set registration retaining this source.
    pub(super) fn retain_registration(&self, wait_set: &Arc<WaitSetInner>) {
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
    pub(super) fn release_registration(&self, wait_set: &Arc<WaitSetInner>) {
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
struct ReadinessSourceState {
    /// Current generation, incremented on every published transition.
    generation: u64,
    /// Current level-readiness predicate.
    availability: SourceAvailability,
    /// Wait sets that have at least one live registration for this source.
    registered_wait_sets: Vec<RegisteredWaitSet>,
    /// Number of registration retains, used by deterministic tests.
    registration_retain_count: u64,
    /// Number of registration releases, used by deterministic tests.
    registration_release_count: u64,
}

impl Default for ReadinessSourceState {
    fn default() -> Self {
        Self {
            generation: 0,
            availability: SourceAvailability::Idle,
            registered_wait_sets: Vec::new(),
            registration_retain_count: 0,
            registration_release_count: 0,
        }
    }
}
