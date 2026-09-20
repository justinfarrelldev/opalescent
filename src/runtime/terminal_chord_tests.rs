use super::{
    TerminalChord, TerminalChordKey, TerminalChordModifiers, TerminalChordPrefixPolicy,
    TerminalChordProcessError, TerminalChordRouter, TerminalChordRouterOutput,
    TerminalChordRouterPolicy, TerminalChordSequence, TerminalChordTextPolicy,
    TerminalChordTrigger, TerminalChordValidationError, TerminalInputEventKind,
    TerminalOrdinaryFeature, TerminalTestScenario,
};

fn enhanced_text_chord(text: &str) -> TerminalChord {
    TerminalChord::new(
        TerminalChordKey::EnhancedText {
            text: text.to_owned(),
        },
        TerminalChordModifiers::default(),
        TerminalChordTrigger::Press,
    )
}

fn scenario_with_router() -> (TerminalTestScenario, TerminalChordRouter) {
    let scenario = TerminalTestScenario::new();
    let capabilities = scenario
        .capabilities(&[
            TerminalOrdinaryFeature::EnhancedKeyIdentity,
            TerminalOrdinaryFeature::KeyReleaseEvents,
        ])
        .expect("capabilities should be constructible");
    let router = TerminalChordRouter::new(capabilities, TerminalChordRouterPolicy::default())
        .expect("router should authenticate capabilities");
    (scenario, router)
}

#[test]
fn terminal_chord_router_activates_key_once_and_releases_by_policy() {
    let (scenario, mut router) = scenario_with_router();
    let sequence = TerminalChordSequence::single(enhanced_text_chord("q"));
    let binding_id = router
        .register(sequence, 10, TerminalChordTextPolicy::PreserveLinkedText)
        .expect("registration should succeed");

    let event = scenario.key_press(1, "q").expect("event should build");
    let output = router.process(event).expect("event should process");

    assert!(matches!(
        output,
        TerminalChordRouterOutput::Activated { .. }
    ));
    let TerminalChordRouterOutput::Activated {
        binding_id: actual,
        released_input,
        ..
    } = output
    else {
        unreachable!("activation shape checked above");
    };
    assert_eq!(actual.ordinal(), binding_id.ordinal());
    assert_eq!(released_input.len(), 1_i64);
}

#[test]
fn terminal_chord_router_validates_capabilities_and_prefixes() {
    let scenario = TerminalTestScenario::new();
    let capabilities = scenario
        .capabilities(&[])
        .expect("capabilities should be constructible");
    let mut router = TerminalChordRouter::new(capabilities, TerminalChordRouterPolicy::default())
        .expect("router construction should succeed");
    assert_eq!(
        router
            .register(
                TerminalChordSequence::single(enhanced_text_chord("x")),
                0,
                TerminalChordTextPolicy::SuppressLinkedText,
            )
            .expect_err("enhanced text should require capability"),
        TerminalChordValidationError::EnhancedKeyIdentityRequired
    );

    let (_enabled_scenario, mut enabled_router) = scenario_with_router();
    let q = TerminalChordSequence::single(enhanced_text_chord("q"));
    enabled_router
        .register(q.clone(), 0, TerminalChordTextPolicy::SuppressLinkedText)
        .expect("first registration should succeed");
    assert!(matches!(
        enabled_router.register(q, 0, TerminalChordTextPolicy::SuppressLinkedText),
        Err(TerminalChordValidationError::DuplicateBinding { .. })
    ));

    let mut prefixed = TerminalChordSequence::single(enhanced_text_chord("q"));
    prefixed = prefixed
        .append(enhanced_text_chord("x"))
        .expect("append should succeed");
    assert_eq!(
        enabled_router
            .register(prefixed, 1, TerminalChordTextPolicy::SuppressLinkedText)
            .expect_err("ambiguous prefix should fail"),
        TerminalChordValidationError::PrefixAmbiguity
    );
}

#[test]
fn terminal_chord_router_enforces_stream_order_and_replay_rejection() {
    let (scenario, mut router) = scenario_with_router();
    router
        .register(
            TerminalChordSequence::single(enhanced_text_chord("q")),
            0,
            TerminalChordTextPolicy::SuppressLinkedText,
        )
        .expect("registration should succeed");
    let first = scenario
        .key_press(1, "q")
        .expect("first event should build");
    let second = scenario
        .key_press(2, "q")
        .expect("second event should build");
    assert_eq!(
        router
            .process(second)
            .expect_err("future event should be out of order"),
        TerminalChordProcessError::DeliveryOutOfOrder
    );
    router.process(first.clone()).expect("first should process");
    assert_eq!(
        router
            .process(first)
            .expect_err("accepted event cannot replay"),
        TerminalChordProcessError::DeliveryAlreadyConsumed
    );

    let foreign = TerminalTestScenario::new()
        .key_press(1, "q")
        .expect("foreign event should build");
    assert_eq!(
        router
            .process(foreign)
            .expect_err("wrong stream should fail"),
        TerminalChordProcessError::WrongInputStream
    );
}

#[test]
fn terminal_chord_router_pending_capacity_and_reset_release_once() {
    let scenario = TerminalTestScenario::new();
    let capabilities = scenario
        .capabilities(&[TerminalOrdinaryFeature::EnhancedKeyIdentity])
        .expect("capabilities should be constructible");
    let policy = TerminalChordRouterPolicy {
        maximum_buffered_events: 64,
        prefix_policy: TerminalChordPrefixPolicy::LongestThenPriority,
        ..TerminalChordRouterPolicy::default()
    };
    let mut router = TerminalChordRouter::new(capabilities, policy)
        .expect("router should construct when buffer equals correlated limit");
    let mut sequence = TerminalChordSequence::single(enhanced_text_chord("q"));
    sequence = sequence
        .append(enhanced_text_chord("x"))
        .expect("append should succeed");
    router
        .register(sequence, 0, TerminalChordTextPolicy::PreserveLinkedText)
        .expect("prefix registration should succeed");

    let first = scenario
        .key_press(1, "q")
        .expect("first event should build");
    assert!(matches!(
        router.process(first).expect("prefix should pend"),
        TerminalChordRouterOutput::Pending { .. }
    ));
    let second = scenario
        .key_press(2, "z")
        .expect("second event should build");
    let err = router
        .process(second.clone())
        .expect_err("full reservation should flush buffered capacity");
    assert!(matches!(
        err,
        TerminalChordProcessError::BufferedCapacityExceeded { .. }
    ));
    let TerminalChordProcessError::BufferedCapacityExceeded { released_input, .. } = err else {
        unreachable!("capacity shape checked above");
    };
    assert_eq!(released_input.len(), 1_i64);

    let output = router.process(second).expect("retry should now fit");
    assert!(matches!(
        output,
        TerminalChordRouterOutput::ReleasedInput { .. }
    ));
    let TerminalChordRouterOutput::ReleasedInput { input } = output else {
        unreachable!("release shape checked above");
    };
    assert_eq!(input.len(), 1_i64);
    assert!(matches!(
        input.at(0).expect("released current event").kind(),
        &TerminalInputEventKind::Key { .. }
    ));

    let reset = router.reset(super::TerminalChordResetReason::ApplicationRequested);
    assert_eq!(reset.len(), 0_i64);
}
