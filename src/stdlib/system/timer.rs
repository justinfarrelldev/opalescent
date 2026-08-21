//! Generic monotonic timer facade for the future `standard.system` surface.
//!
//! These functions preserve the proposal names while delegating to the runtime
//! timer object. They expose only opaque readiness sources and monotonic
//! deadlines, never OS timer handles or wait-set internals.

use crate::runtime::timer::{
    MonotonicDeadline, MonotonicTimer, MonotonicTimerError, MonotonicTimerNotArmedError,
    monotonic_clock_now as runtime_monotonic_clock_now,
};
use crate::runtime::wait::SystemReadinessSource;

/// Create a new disarmed monotonic timer.
#[must_use]
pub fn monotonic_timer_new() -> MonotonicTimer {
    MonotonicTimer::new()
}

/// Return the timer's stable readiness source identity.
#[must_use]
pub fn monotonic_timer_readiness_source(timer: &MonotonicTimer) -> SystemReadinessSource {
    timer.readiness_source()
}

/// Arm `timer` to `deadline` and return the new generation.
///
/// # Errors
///
/// Returns [`MonotonicTimerError::GenerationExhausted`] when the timer cannot
/// issue another never-reused generation.
pub fn monotonic_timer_arm(
    timer: &mut MonotonicTimer,
    deadline: MonotonicDeadline,
) -> Result<u64, MonotonicTimerError> {
    timer.arm(deadline)
}

/// Disarm `timer` and return the new generation.
///
/// # Errors
///
/// Returns [`MonotonicTimerError::GenerationExhausted`] when the timer cannot
/// issue another never-reused generation.
pub fn monotonic_timer_disarm(timer: &mut MonotonicTimer) -> Result<u64, MonotonicTimerError> {
    timer.disarm()
}

/// Return the timer's current generation.
#[must_use]
pub fn monotonic_timer_generation(timer: &MonotonicTimer) -> u64 {
    timer.generation()
}

/// Return the timer's currently armed deadline.
///
/// # Errors
///
/// Returns [`MonotonicTimerNotArmedError::NotArmed`] while `timer` is disarmed.
pub fn monotonic_timer_deadline(
    timer: &MonotonicTimer,
) -> Result<MonotonicDeadline, MonotonicTimerNotArmedError> {
    timer.deadline()
}

/// Return the process monotonic clock's current deadline value.
#[must_use]
pub fn monotonic_clock_now() -> MonotonicDeadline {
    runtime_monotonic_clock_now()
}
