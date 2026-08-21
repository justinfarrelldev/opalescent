//! Tests for affine monotonic timer runtime behavior.

extern crate std;

use std::time::Duration;

use super::timer::{
    FakeMonotonicClock, MonotonicDeadline, MonotonicTimer, MonotonicTimerError,
    MonotonicTimerNotArmedError, monotonic_clock_now,
};
use super::wait::{CancellationSource, SystemReadyWakeStatus, SystemWaitSet};

/// Return a fake-clock-backed timer and its controlling clock.
fn fake_timer() -> (FakeMonotonicClock, MonotonicTimer) {
    let clock = FakeMonotonicClock::new();
    let timer = MonotonicTimer::new_with_clock_for_tests(clock.handle());
    (clock, timer)
}

#[test]
fn monotonic_timer_new_is_disarmed_with_stable_readiness_source() {
    let (clock, mut timer) = fake_timer();
    let first_source = timer.readiness_source();
    let second_source = timer.readiness_source();

    assert!(
        first_source.is_same_identity(&second_source),
        "readiness source clones should preserve stable identity"
    );
    assert_eq!(timer.generation(), 0);
    assert_eq!(timer.deadline(), Err(MonotonicTimerNotArmedError::NotArmed));

    let deadline = clock.deadline_after(Duration::from_millis(5));
    let arm_generation = timer.arm(deadline).expect("arm should issue generation");
    assert_eq!(arm_generation, 1);
    assert!(
        first_source.is_same_identity(&timer.readiness_source()),
        "arm should not replace the readiness source identity"
    );

    let disarm_generation = timer.disarm().expect("disarm should issue generation");
    assert_eq!(disarm_generation, 2);
    assert!(
        first_source.is_same_identity(&timer.readiness_source()),
        "disarm should not replace the readiness source identity"
    );
}

#[test]
fn monotonic_timer_arm_sets_deadline_and_future_readiness() {
    let (clock, mut timer) = fake_timer();
    let source = timer.readiness_source();
    let deadline = clock.deadline_after(Duration::from_nanos(10));

    let generation = timer.arm(deadline).expect("arm should succeed");

    assert_eq!(generation, 1);
    assert_eq!(timer.generation(), generation);
    assert_eq!(timer.deadline(), Ok(deadline));
    assert_eq!(source.generation(), generation);
    assert_eq!(
        source.readiness_status_for_generation(generation),
        SystemReadyWakeStatus::StaleReadiness
    );
}

#[test]
fn monotonic_timer_repeated_disarm_advances_generation() {
    let (_clock, mut timer) = fake_timer();
    let source = timer.readiness_source();

    let first_generation = timer.disarm().expect("first disarm should succeed");
    let second_generation = timer.disarm().expect("second disarm should succeed");

    assert_eq!(first_generation, 1);
    assert_eq!(second_generation, 2);
    assert_eq!(timer.generation(), second_generation);
    assert_eq!(source.generation(), second_generation);
    assert_eq!(timer.deadline(), Err(MonotonicTimerNotArmedError::NotArmed));
    assert_eq!(
        source.readiness_status_for_generation(second_generation),
        SystemReadyWakeStatus::StaleReadiness
    );
}

#[test]
fn monotonic_timer_equality_is_expiration() {
    let (clock, mut timer) = fake_timer();
    let source = timer.readiness_source();
    let mut wait_set = SystemWaitSet::new();
    let _registration = wait_set
        .register(&source)
        .expect("timer source registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    let generation = timer.arm(clock.now()).expect("arm at now should succeed");
    let wake = wait_set
        .wait_sync(&token)
        .expect("equality deadline should wake the wait set");

    assert_eq!(wake.ready_generation(), Some(generation));
    assert_eq!(
        wake.readiness_against(&source),
        SystemReadyWakeStatus::Current
    );
}

#[test]
fn monotonic_timer_stale_ready_wake_does_not_expire_newer_deadline() {
    let (clock, mut timer) = fake_timer();
    let source = timer.readiness_source();
    let mut wait_set = SystemWaitSet::new();
    let _registration = wait_set
        .register(&source)
        .expect("timer source registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    let stale_generation = timer.arm(clock.now()).expect("ready arm should succeed");
    let newer_deadline = clock.deadline_after(Duration::from_millis(5));
    let current_generation = timer
        .arm(newer_deadline)
        .expect("newer future arm should succeed");

    let wake = wait_set
        .wait_sync(&token)
        .expect("old ready hint should still be observable");

    assert_eq!(wake.ready_generation(), Some(stale_generation));
    assert_eq!(timer.generation(), current_generation);
    assert_eq!(timer.deadline(), Ok(newer_deadline));
    assert_eq!(
        wake.readiness_against(&source),
        SystemReadyWakeStatus::StaleGeneration
    );
}

#[test]
fn monotonic_timer_wait_set_observes_fake_clock_expiration() {
    let (clock, mut timer) = fake_timer();
    let source = timer.readiness_source();
    let deadline = clock.deadline_after(Duration::from_millis(10));
    let generation = timer.arm(deadline).expect("future arm should succeed");
    let mut wait_set = SystemWaitSet::new();
    let _registration = wait_set
        .register(&source)
        .expect("timer source registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    clock.set(deadline);
    let wake = wait_set
        .wait_sync(&token)
        .expect("advanced fake clock should make timer ready");

    assert_eq!(wake.ready_generation(), Some(generation));
    assert_eq!(
        wake.readiness_against(&source),
        SystemReadyWakeStatus::Current
    );
}

#[test]
fn monotonic_timer_fake_clock_is_deterministic_without_generation_change() {
    let (clock, mut timer) = fake_timer();
    let source = timer.readiness_source();
    let deadline = clock.deadline_after(Duration::from_millis(2));
    let generation = timer.arm(deadline).expect("future arm should succeed");

    assert_eq!(clock.now().duration_since_origin(), Duration::ZERO);
    assert_eq!(
        source.readiness_status_for_generation(generation),
        SystemReadyWakeStatus::StaleReadiness
    );

    clock.advance_by(Duration::from_millis(1));
    assert_eq!(
        source.readiness_status_for_generation(generation),
        SystemReadyWakeStatus::StaleReadiness
    );

    clock.advance_by(Duration::from_millis(1));
    assert_eq!(timer.generation(), generation);
    assert_eq!(
        source.readiness_status_for_generation(generation),
        SystemReadyWakeStatus::Current
    );
}

#[test]
fn monotonic_timer_generation_exhaustion_preserves_armed_state() {
    let clock = FakeMonotonicClock::new();
    let prior_deadline = clock.deadline_after(Duration::from_millis(10));
    let replacement_deadline = clock.deadline_after(Duration::from_millis(20));
    let mut timer =
        MonotonicTimer::new_with_state_for_tests(clock.handle(), u64::MAX, Some(prior_deadline));
    let source = timer.readiness_source();

    let arm_error = timer
        .arm(replacement_deadline)
        .expect_err("arm at exhausted generation should fail");
    let disarm_error = timer
        .disarm()
        .expect_err("disarm at exhausted generation should fail");

    assert_eq!(
        arm_error,
        MonotonicTimerError::GenerationExhausted {
            last_issued_generation: u64::MAX,
        }
    );
    assert_eq!(arm_error, disarm_error);
    assert_eq!(timer.generation(), u64::MAX);
    assert_eq!(source.generation(), u64::MAX);
    assert_eq!(timer.deadline(), Ok(prior_deadline));
    assert_eq!(
        source.readiness_status_for_generation(u64::MAX),
        SystemReadyWakeStatus::StaleReadiness
    );
}

#[test]
fn monotonic_timer_generation_exhaustion_preserves_disarmed_state() {
    let clock = FakeMonotonicClock::new();
    let replacement_deadline = clock.deadline_after(Duration::from_millis(20));
    let mut timer = MonotonicTimer::new_with_state_for_tests(clock.handle(), u64::MAX, None);
    let source = timer.readiness_source();

    let disarm_error = timer
        .disarm()
        .expect_err("disarm at exhausted generation should fail");
    let arm_error = timer
        .arm(replacement_deadline)
        .expect_err("arm at exhausted generation should fail");

    assert_eq!(
        disarm_error,
        MonotonicTimerError::GenerationExhausted {
            last_issued_generation: u64::MAX,
        }
    );
    assert_eq!(disarm_error, arm_error);
    assert_eq!(timer.generation(), u64::MAX);
    assert_eq!(source.generation(), u64::MAX);
    assert_eq!(timer.deadline(), Err(MonotonicTimerNotArmedError::NotArmed));
    assert_eq!(
        source.readiness_status_for_generation(u64::MAX),
        SystemReadyWakeStatus::StaleReadiness
    );
}

#[test]
fn monotonic_clock_now_is_monotonic() {
    let first = monotonic_clock_now();
    let second = monotonic_clock_now();

    assert!(
        second >= first,
        "monotonic clock readings should never move backward"
    );
}

#[test]
fn monotonic_deadline_saturating_test_math_caps_at_maximum() {
    let clock = FakeMonotonicClock::new_at(MonotonicDeadline::MAX);

    clock.advance_by(Duration::from_nanos(1));

    assert_eq!(clock.now(), MonotonicDeadline::MAX);
}
