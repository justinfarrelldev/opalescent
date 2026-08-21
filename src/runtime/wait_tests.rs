//! Focused tests for generic wait-set and cancellation primitives.

use crate::runtime::wait::{
    CancellationSource, SystemOwnedWaitRegistration, SystemReadyWakeStatus, SystemWaitRegistration,
    SystemWaitSet, SystemWaitSetError, SystemWaitWake,
};
use crate::runtime::wait_test_support::FakeReadinessSource;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Extract a ready wake and assert it names `source`.
fn assert_ready_for(wake: &SystemWaitWake, source: &FakeReadinessSource) -> u64 {
    assert_eq!(
        wake.readiness_against(&source.source()),
        SystemReadyWakeStatus::Current
    );
    wake.ready_generation()
        .expect("ready wake should carry a generation")
}

#[test]
fn wait_set_register_remove_is_idempotent_and_releases_once() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();

    let registration = wait_set
        .register(&source.source())
        .expect("registration should succeed");
    assert_eq!(source.registration_retain_count(), 1);
    assert_eq!(source.registration_release_count(), 0);

    wait_set
        .remove(&registration)
        .expect("first removal should succeed");
    assert_eq!(source.registration_release_count(), 1);

    wait_set
        .remove(&registration)
        .expect("repeated removal should be idempotent");
    assert_eq!(source.registration_release_count(), 1);
}

#[test]
fn ordinary_registration_errors_distinguish_wrong_set_unauthenticated_and_lifetime() {
    let source = FakeReadinessSource::new();
    let mut owner = SystemWaitSet::new();
    let mut other = SystemWaitSet::new();
    let registration = owner
        .register(&source.source())
        .expect("registration should succeed");

    assert_eq!(
        other.remove(&registration),
        Err(SystemWaitSetError::WrongSet)
    );

    let unauthenticated = SystemWaitRegistration::unauthenticated_for_tests(&owner);
    assert_eq!(
        owner.remove(&unauthenticated),
        Err(SystemWaitSetError::UnauthenticatedRegistration)
    );

    let lifetime_registration = {
        let mut temporary = SystemWaitSet::new();
        temporary
            .register(&source.source())
            .expect("temporary registration should succeed")
    };
    assert_eq!(
        other.remove(&lifetime_registration),
        Err(SystemWaitSetError::RegistrationLifetimeInvalid)
    );
}

#[test]
fn owned_registration_retarget_remove_and_drop_release_exact_sources() {
    let first = FakeReadinessSource::new();
    let second = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    let mut owned = wait_set
        .register_owned(&first.source())
        .expect("owned registration should succeed");

    assert_eq!(first.registration_retain_count(), 1);
    owned
        .retarget(&first.source())
        .expect("retargeting to same source is idempotent");
    assert_eq!(first.registration_retain_count(), 1);
    assert_eq!(first.registration_release_count(), 0);

    owned
        .retarget(&second.source())
        .expect("retargeting to new source should succeed");
    assert_eq!(first.registration_release_count(), 1);
    assert_eq!(second.registration_retain_count(), 1);

    owned.remove().expect("owned removal should succeed");
    assert_eq!(second.registration_release_count(), 1);

    owned
        .remove()
        .expect("owned repeated removal should be idempotent");
    assert_eq!(second.registration_release_count(), 1);
    drop(owned);
    assert_eq!(second.registration_release_count(), 1);
}

#[test]
fn owned_registration_drop_removes_live_entry_once() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();

    {
        let _owned = wait_set
            .register_owned(&source.source())
            .expect("owned registration should succeed");
        assert_eq!(source.registration_retain_count(), 1);
    }

    assert_eq!(source.registration_release_count(), 1);
}

#[test]
fn owned_registration_errors_distinguish_unauthenticated_and_lifetime() {
    let source = FakeReadinessSource::new();
    let wait_set = SystemWaitSet::new();
    let mut unauthenticated = SystemOwnedWaitRegistration::unauthenticated_for_tests(&wait_set);

    assert_eq!(
        unauthenticated.remove(),
        Err(SystemWaitSetError::UnauthenticatedRegistration)
    );

    let mut owned = {
        let mut temporary = SystemWaitSet::new();
        temporary
            .register_owned(&source.source())
            .expect("owned registration should succeed")
    };
    assert_eq!(
        owned.remove(),
        Err(SystemWaitSetError::RegistrationLifetimeInvalid)
    );
    assert_eq!(
        owned.retarget(&source.source()),
        Err(SystemWaitSetError::RegistrationLifetimeInvalid)
    );
}

#[test]
fn wait_set_drop_releases_live_entries_and_invalidates_authorities() {
    let first = FakeReadinessSource::new();
    let second = FakeReadinessSource::new();
    let mut owned = {
        let mut wait_set = SystemWaitSet::new();
        let _ordinary = wait_set
            .register(&first.source())
            .expect("ordinary registration should succeed");
        wait_set
            .register_owned(&second.source())
            .expect("owned registration should succeed")
    };

    assert_eq!(first.registration_release_count(), 1);
    assert_eq!(second.registration_release_count(), 1);
    assert_eq!(
        owned.remove(),
        Err(SystemWaitSetError::RegistrationLifetimeInvalid)
    );
}

#[test]
fn duplicate_source_registration_stays_subscribed_after_one_remove() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    let first_registration = wait_set
        .register(&source.source())
        .expect("first registration should succeed");
    let _second_registration = wait_set
        .register(&source.source())
        .expect("second registration should succeed");
    let mut cancellation = CancellationSource::new();
    let token = cancellation.token();

    wait_set
        .remove(&first_registration)
        .expect("removing one duplicate registration should succeed");
    let transition_generation = source.publish_idle_transition();
    cancellation.request();

    let wake = wait_set
        .wait_sync(&token)
        .expect("remaining duplicate registration should receive transition");
    assert_eq!(wake.ready_generation(), Some(transition_generation));
    assert_eq!(
        wake.readiness_against(&source.source()),
        SystemReadyWakeStatus::StaleReadiness
    );
}

#[test]
fn source_clone_outlives_registration_and_preserves_identity() {
    let fake = FakeReadinessSource::new();
    let source = fake.source();
    let source_clone = source.clone();
    drop(source);
    let mut wait_set = SystemWaitSet::new();
    let registration = wait_set
        .register(&source_clone)
        .expect("registration should succeed");

    wait_set
        .remove(&registration)
        .expect("removal should succeed");
    assert!(fake.source().is_same_identity(&source_clone));
    let generation = fake.publish_ready();
    assert_eq!(source_clone.generation(), generation);
    assert_eq!(fake.registration_retain_count(), 1);
    assert_eq!(fake.registration_release_count(), 1);
}

#[test]
fn public_debug_output_redacts_internal_wait_and_registration_keys() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    let registration = wait_set
        .register(&source.source())
        .expect("ordinary registration should succeed");
    let owned = wait_set
        .register_owned(&source.source())
        .expect("owned registration should succeed");
    let mut cancellation = CancellationSource::new();
    let token = cancellation.token();
    cancellation.request();

    let source_handle = source.source();
    let debug_outputs = [
        format!("{source_handle:?}"),
        format!("{wait_set:?}"),
        format!("{registration:?}"),
        format!("{owned:?}"),
        format!("{cancellation:?}"),
        format!("{token:?}"),
    ];

    for rendered in debug_outputs {
        for forbidden in [
            "identity",
            "registration:",
            "wait_set",
            "auth",
            "secret",
            "generation:",
        ] {
            assert!(
                !rendered.contains(forbidden),
                "public Debug output must redact {forbidden}: {rendered}"
            );
        }
    }
}

#[test]
fn wait_set_selects_ready_sources_in_bounded_fair_registration_order() {
    let first = FakeReadinessSource::new();
    let second = FakeReadinessSource::new();
    let third = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    wait_set
        .register(&first.source())
        .expect("first registration should succeed");
    wait_set
        .register(&second.source())
        .expect("second registration should succeed");
    wait_set
        .register(&third.source())
        .expect("third registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    third.publish_ready();
    second.publish_ready();
    first.publish_ready();

    let first_wake = wait_set
        .wait_sync(&token)
        .expect("wait should return first");
    assert_ready_for(&first_wake, &first);
    let second_wake = wait_set
        .wait_sync(&token)
        .expect("wait should return second");
    assert_ready_for(&second_wake, &second);
    let third_wake = wait_set
        .wait_sync(&token)
        .expect("wait should return third");
    assert_ready_for(&third_wake, &third);
    let wrapped_wake = wait_set
        .wait_sync(&token)
        .expect("level readiness should wrap");
    assert_ready_for(&wrapped_wake, &first);
}

#[test]
fn stale_ready_wake_reports_stale_generation() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    wait_set
        .register(&source.source())
        .expect("registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    let stale_generation = source.publish_ready();
    source.publish_idle_transition();

    let wake = wait_set
        .wait_sync(&token)
        .expect("old transition should return");
    assert_eq!(wake.ready_generation(), Some(stale_generation));
    assert_eq!(
        wake.readiness_against(&source.source()),
        SystemReadyWakeStatus::StaleGeneration
    );
}

#[test]
fn retarget_preserves_old_published_ready_as_stale_source_hint() {
    let old_source = FakeReadinessSource::new();
    let new_source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    let mut owned = wait_set
        .register_owned(&old_source.source())
        .expect("owned registration should succeed");
    let cancellation = CancellationSource::new();
    let token = cancellation.token();

    old_source.publish_ready();
    owned
        .retarget(&new_source.source())
        .expect("retarget should preserve fairness position");

    let wake = wait_set
        .wait_sync(&token)
        .expect("old hint should remain queued");
    assert_eq!(
        wake.readiness_against(&new_source.source()),
        SystemReadyWakeStatus::StaleSource
    );
    assert!(
        wake.ready_source()
            .expect("ready wake should carry a source")
            .is_same_identity(&old_source.source()),
        "ready wake should still name the old source identity"
    );
}

#[test]
fn cancellation_request_wakes_concurrently_blocked_wait() {
    let mut wait_set = SystemWaitSet::new();
    let mut cancellation = CancellationSource::new();
    let token = cancellation.token();
    let (started_sender, started_receiver) = mpsc::channel();
    let (done_sender, done_receiver) = mpsc::channel();

    let waiter = thread::spawn(move || {
        started_sender
            .send(())
            .expect("waiter should report startup before blocking");
        let wake = wait_set
            .wait_sync(&token)
            .expect("cancelled token should wake blocked wait");
        done_sender
            .send(wake)
            .expect("waiter should report the cancellation wake");
    });

    started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("waiter should reach wait_sync");
    thread::sleep(Duration::from_millis(25));
    cancellation.request();

    let wake = done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("cancellation should wake the blocked wait without a lost notify");
    waiter.join().expect("waiter thread should not panic");
    assert_eq!(wake, SystemWaitWake::Cancelled);
    assert_eq!(wake.ready_source(), None);
    assert_eq!(wake.ready_generation(), None);
}

#[test]
fn cancellation_drains_old_published_work_then_wins_over_new_ready() {
    let source = FakeReadinessSource::new();
    let mut wait_set = SystemWaitSet::new();
    wait_set
        .register(&source.source())
        .expect("registration should succeed");
    let mut cancellation = CancellationSource::new();
    let token = cancellation.token();

    let first_generation = source.publish_ready();
    cancellation.request();
    source.publish_ready();

    let ready = wait_set
        .wait_sync(&token)
        .expect("old ready should drain first");
    assert_eq!(ready.ready_generation(), Some(first_generation));
    assert_eq!(
        ready.readiness_against(&source.source()),
        SystemReadyWakeStatus::StaleGeneration
    );

    let cancelled = wait_set
        .wait_sync(&token)
        .expect("cancellation should win after old work drains");
    assert_eq!(cancelled, SystemWaitWake::Cancelled);
    assert_eq!(cancelled.ready_source(), None);
    assert_eq!(cancelled.ready_generation(), None);
    assert_eq!(
        cancelled.readiness_against(&source.source()),
        SystemReadyWakeStatus::Cancelled
    );
}

#[test]
fn cancellation_is_sticky_per_generation_and_tokens_survive_source_drop() {
    let token = {
        let mut source = CancellationSource::new();
        let token = source.token();
        source.request();
        token
    };
    assert!(token.is_cancelled());

    let fresh_source = CancellationSource::new();
    let fresh_token = fresh_source.token();
    assert_ne!(token.generation(), fresh_token.generation());
    assert!(!fresh_token.is_cancelled());
}
