//! Affine monotonic timers built on generic wait readiness sources.
//!
//! Timers own one stable [`SystemReadinessSource`] identity for their entire
//! lifetime. Arm and disarm operations advance a never-reused generation before
//! publishing the timer's new readiness state, allowing callers to reject stale
//! ready wakes without exposing source IDs, OS handles, or scheduler state.

extern crate std;

#[path = "timer/clock.rs"]
mod clock;

use core::fmt;
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

#[cfg(test)]
pub(crate) use clock::FakeMonotonicClock;
pub(crate) use clock::MonotonicClockHandle;
pub use clock::monotonic_clock_now;

use crate::runtime::wait::{SourceAvailability, SourceWakePublication, SystemReadinessSource};

/// Monotonic deadline represented as duration since a process-local origin.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MonotonicDeadline {
    /// Duration from the clock origin to this deadline.
    since_origin: Duration,
}

impl MonotonicDeadline {
    /// Zero deadline used by deterministic tests.
    #[cfg(test)]
    pub(crate) const ZERO: Self = Self {
        since_origin: Duration::ZERO,
    };

    /// Largest representable deadline used by saturating test-clock math.
    #[cfg(test)]
    pub(crate) const MAX: Self = Self {
        since_origin: Duration::MAX,
    };

    /// Create a deadline from a duration since the clock origin.
    #[must_use]
    pub(crate) const fn from_duration_since_origin(since_origin: Duration) -> Self {
        Self { since_origin }
    }

    /// Return the duration from the clock origin to this deadline.
    #[cfg(test)]
    #[must_use]
    pub(crate) const fn duration_since_origin(self) -> Duration {
        self.since_origin
    }

    /// Return a deadline after `duration`, or `None` on overflow.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn checked_add_duration(self, duration: Duration) -> Option<Self> {
        self.since_origin
            .checked_add(duration)
            .map(Self::from_duration_since_origin)
    }

    /// Return a deadline after `duration`, saturating at the maximum deadline.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn saturating_add_duration(self, duration: Duration) -> Self {
        self.checked_add_duration(duration).unwrap_or(Self::MAX)
    }

    /// Return the duration from `earlier` to this deadline, or `None` when earlier is after it.
    #[must_use]
    pub(crate) const fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.since_origin.checked_sub(earlier.since_origin)
    }

    /// Return whether `now` has reached or passed this deadline.
    #[must_use]
    pub(crate) fn is_expired_by(self, now: Self) -> bool {
        now >= self
    }
}

impl fmt::Debug for MonotonicDeadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonotonicDeadline").finish_non_exhaustive()
    }
}

/// Errors produced by monotonic timer state transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicTimerError {
    /// The timer cannot issue another generation without reuse or wraparound.
    GenerationExhausted {
        /// The current valid generation that remains unchanged.
        last_issued_generation: u64,
    },
}

impl fmt::Display for MonotonicTimerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::GenerationExhausted { .. } => {
                f.write_str("MonotonicTimerError.GenerationExhausted")
            }
        }
    }
}

impl std::error::Error for MonotonicTimerError {}

/// Error returned when reading a disarmed timer deadline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotonicTimerNotArmedError {
    /// The timer currently has no armed deadline.
    NotArmed,
}

impl fmt::Display for MonotonicTimerNotArmedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::NotArmed => f.write_str("MonotonicTimerNotArmedError.NotArmed"),
        }
    }
}

impl std::error::Error for MonotonicTimerNotArmedError {}

/// Affine monotonic timer with one stable readiness source.
pub struct MonotonicTimer {
    /// Stable readiness source identity retained for the timer lifetime.
    source: SystemReadinessSource,
    /// Mutable generation and deadline state.
    state: Mutex<MonotonicTimerState>,
    /// Clock used to evaluate deadlines.
    clock: MonotonicClockHandle,
}

impl MonotonicTimer {
    /// Create a new disarmed monotonic timer backed by the process monotonic clock.
    #[must_use]
    pub fn new() -> Self {
        Self::with_clock(MonotonicClockHandle::system())
    }

    /// Return a clone of the timer's stable readiness source identity.
    #[must_use]
    pub fn readiness_source(&self) -> SystemReadinessSource {
        self.source.clone()
    }

    /// Arm the timer to `deadline` and return the newly issued generation.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimerError::GenerationExhausted`] without changing the
    /// timer state when no fresh generation can be issued.
    pub fn arm(&mut self, deadline: MonotonicDeadline) -> Result<u64, MonotonicTimerError> {
        let next_generation = {
            let mut state = lock_or_recover(&self.state);
            let candidate_generation = Self::next_generation(&state)?;
            state.generation = candidate_generation;
            state.deadline = Some(deadline);
            candidate_generation
        };
        self.publish_source_state(next_generation, Some(deadline));
        Ok(next_generation)
    }

    /// Disarm the timer and return the newly issued generation.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimerError::GenerationExhausted`] without changing the
    /// timer state when no fresh generation can be issued. Repeated disarm while
    /// already disarmed is otherwise successful and still advances generation.
    pub fn disarm(&mut self) -> Result<u64, MonotonicTimerError> {
        let next_generation = {
            let mut state = lock_or_recover(&self.state);
            let candidate_generation = Self::next_generation(&state)?;
            state.generation = candidate_generation;
            state.deadline = None;
            candidate_generation
        };
        self.publish_source_state(next_generation, None);
        Ok(next_generation)
    }

    /// Return the timer's current generation.
    #[must_use]
    pub fn generation(&self) -> u64 {
        lock_or_recover(&self.state).generation
    }

    /// Return the currently armed deadline.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimerNotArmedError::NotArmed`] while the timer is disarmed.
    pub fn deadline(&self) -> Result<MonotonicDeadline, MonotonicTimerNotArmedError> {
        lock_or_recover(&self.state)
            .deadline
            .ok_or(MonotonicTimerNotArmedError::NotArmed)
    }

    /// Create a disarmed timer with a custom clock for deterministic tests.
    #[cfg(test)]
    pub(crate) fn new_with_clock_for_tests(clock: MonotonicClockHandle) -> Self {
        Self::with_clock(clock)
    }

    /// Create a timer with exact state for generation-exhaustion tests.
    #[cfg(test)]
    pub(crate) fn new_with_state_for_tests(
        clock: MonotonicClockHandle,
        generation: u64,
        deadline: Option<MonotonicDeadline>,
    ) -> Self {
        Self::with_initial_state(clock, generation, deadline)
    }

    /// Create a disarmed timer from `clock`.
    fn with_clock(clock: MonotonicClockHandle) -> Self {
        Self::with_initial_state(clock, 0, None)
    }

    /// Create a timer with exact generation and deadline state.
    fn with_initial_state(
        clock: MonotonicClockHandle,
        generation: u64,
        deadline: Option<MonotonicDeadline>,
    ) -> Self {
        let timer = Self {
            source: SystemReadinessSource::new(),
            state: Mutex::new(MonotonicTimerState {
                generation,
                deadline,
            }),
            clock,
        };
        timer.publish_source_state(generation, deadline);
        timer
    }

    /// Return the next generation or report deterministic exhaustion.
    fn next_generation(state: &MonotonicTimerState) -> Result<u64, MonotonicTimerError> {
        state
            .generation
            .checked_add(1)
            .ok_or(MonotonicTimerError::GenerationExhausted {
                last_issued_generation: state.generation,
            })
    }

    /// Publish current timer state to the stable readiness source.
    fn publish_source_state(&self, generation: u64, deadline: Option<MonotonicDeadline>) {
        let availability = deadline.map_or_else(
            || SourceAvailability::Idle,
            |armed_deadline| SourceAvailability::TimerDeadline {
                deadline: armed_deadline,
                clock: self.clock.clone(),
            },
        );
        let publication = if availability.is_ready() {
            SourceWakePublication::QueueReady
        } else {
            SourceWakePublication::NotifyOnly
        };
        self.source
            .publish_generation_transition(generation, availability, publication);
    }
}

impl Default for MonotonicTimer {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for MonotonicTimer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonotonicTimer").finish_non_exhaustive()
    }
}

/// Mutable monotonic timer state.
struct MonotonicTimerState {
    /// Last successfully issued generation.
    generation: u64,
    /// Armed deadline, absent while disarmed.
    deadline: Option<MonotonicDeadline>,
}

/// Acquire timer state and recover after a test panic.
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}
