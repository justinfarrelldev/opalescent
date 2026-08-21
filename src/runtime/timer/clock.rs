//! Monotonic clock handles shared by affine timers and generic wait sources.
//!
//! Real clocks derive deadlines from [`std::time::Instant`]. Test clocks are
//! available only to crate tests, so deterministic chord scenarios can advance
//! time without exporting fake-clock constructors from the production runtime.

extern crate alloc;
extern crate std;

use alloc::sync::Arc;
use core::fmt;
use std::sync::OnceLock;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use super::MonotonicDeadline;

/// Process-local origin used to express monotonic deadlines as durations.
static PROCESS_ORIGIN: OnceLock<Instant> = OnceLock::new();

/// Return the process-local monotonic origin.
fn process_origin() -> Instant {
    *PROCESS_ORIGIN.get_or_init(Instant::now)
}

/// Recover a fake-clock mutex after a test panic.
#[cfg(test)]
fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Monotonic clock behavior needed by timer readiness sources.
pub trait MonotonicClock: Send + Sync {
    /// Return this clock's current monotonic deadline value.
    fn now(&self) -> MonotonicDeadline;

    /// Return a real condvar timeout for `deadline`, if this clock advances by wall waiting.
    fn wait_duration_until(&self, deadline: MonotonicDeadline) -> Option<Duration>;
}

/// Cloneable clock handle stored by timer readiness metadata.
#[derive(Clone)]
pub struct MonotonicClockHandle {
    /// Shared clock implementation.
    inner: Arc<dyn MonotonicClock>,
}

impl MonotonicClockHandle {
    /// Create a handle backed by the process monotonic clock.
    #[must_use]
    pub(crate) fn system() -> Self {
        Self {
            inner: Arc::new(SystemMonotonicClock),
        }
    }

    /// Return the current monotonic deadline for this clock.
    #[must_use]
    pub(crate) fn now(&self) -> MonotonicDeadline {
        self.inner.now()
    }

    /// Return how long a real wait may sleep before `deadline` can be ready.
    #[must_use]
    pub(crate) fn wait_duration_until(&self, deadline: MonotonicDeadline) -> Option<Duration> {
        self.inner.wait_duration_until(deadline)
    }
}

impl fmt::Debug for MonotonicClockHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MonotonicClockHandle")
            .finish_non_exhaustive()
    }
}

/// Process monotonic clock implementation.
struct SystemMonotonicClock;

impl MonotonicClock for SystemMonotonicClock {
    fn now(&self) -> MonotonicDeadline {
        MonotonicDeadline::from_duration_since_origin(process_origin().elapsed())
    }

    fn wait_duration_until(&self, deadline: MonotonicDeadline) -> Option<Duration> {
        Some(
            deadline
                .checked_duration_since(self.now())
                .unwrap_or(Duration::ZERO),
        )
    }
}

/// Return the process monotonic clock's current deadline value.
#[must_use]
pub fn monotonic_clock_now() -> MonotonicDeadline {
    SystemMonotonicClock.now()
}

/// Deterministic monotonic clock available only inside crate tests.
#[cfg(test)]
#[derive(Clone)]
pub struct FakeMonotonicClock {
    /// Shared fake clock state.
    inner: Arc<FakeMonotonicClockInner>,
}

#[cfg(test)]
impl FakeMonotonicClock {
    /// Create a fake clock starting at zero.
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::new_at(MonotonicDeadline::ZERO)
    }

    /// Create a fake clock starting at `deadline`.
    #[must_use]
    pub(crate) fn new_at(deadline: MonotonicDeadline) -> Self {
        Self {
            inner: Arc::new(FakeMonotonicClockInner {
                current: Mutex::new(deadline),
            }),
        }
    }

    /// Return a timer-compatible handle for this fake clock.
    #[must_use]
    pub(crate) fn handle(&self) -> MonotonicClockHandle {
        let concrete_clock = Arc::clone(&self.inner);
        let inner: Arc<dyn MonotonicClock> = concrete_clock;
        MonotonicClockHandle { inner }
    }

    /// Return the current fake deadline.
    #[must_use]
    pub(crate) fn now(&self) -> MonotonicDeadline {
        self.inner.now()
    }

    /// Return a deadline after `duration` on this fake clock.
    #[must_use]
    pub(crate) fn deadline_after(&self, duration: Duration) -> MonotonicDeadline {
        self.now().saturating_add_duration(duration)
    }

    /// Set the fake clock to an exact deadline.
    pub(crate) fn set(&self, deadline: MonotonicDeadline) {
        *lock_or_recover(&self.inner.current) = deadline;
    }

    /// Advance the fake clock by `duration`, saturating at the largest duration.
    pub(crate) fn advance_by(&self, duration: Duration) {
        let mut current = lock_or_recover(&self.inner.current);
        *current = current.saturating_add_duration(duration);
    }
}

#[cfg(test)]
impl Default for FakeMonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared fake monotonic clock state.
#[cfg(test)]
struct FakeMonotonicClockInner {
    /// Current fake monotonic deadline.
    current: Mutex<MonotonicDeadline>,
}

#[cfg(test)]
impl MonotonicClock for FakeMonotonicClockInner {
    fn now(&self) -> MonotonicDeadline {
        *lock_or_recover(&self.current)
    }

    fn wait_duration_until(&self, deadline: MonotonicDeadline) -> Option<Duration> {
        deadline.is_expired_by(self.now()).then_some(Duration::ZERO)
    }
}
