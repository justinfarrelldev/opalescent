use super::{
    TerminalChord, TerminalChordKey, TerminalChordModifiers, TerminalChordProcessError,
    TerminalChordRouter, TerminalChordRouterPolicy, TerminalChordSequence, TerminalChordTextPolicy,
    TerminalChordTrigger, TerminalOrdinaryFeature, TerminalTestActivation,
    TerminalTestFactoryError, TerminalTestScenario,
};

#[test]
fn terminal_security_chord_wrong_stream_and_replay_are_rejected_before_mutation() {
    let scenario = TerminalTestScenario::new();
    let capabilities = scenario
        .capabilities(&[TerminalOrdinaryFeature::EnhancedKeyIdentity])
        .expect("capabilities should be constructible");
    let mut router = TerminalChordRouter::new(capabilities, TerminalChordRouterPolicy::default())
        .expect("router should construct");
    let binding = router
        .register(
            TerminalChordSequence::single(TerminalChord::new(
                TerminalChordKey::EnhancedText {
                    text: "q".to_owned(),
                },
                TerminalChordModifiers::default(),
                TerminalChordTrigger::Press,
            )),
            0,
            TerminalChordTextPolicy::SuppressLinkedText,
        )
        .expect("registration should succeed");

    let foreign_event = TerminalTestScenario::new()
        .key_press(1, "q")
        .expect("foreign event should construct");
    assert_eq!(
        router
            .process(foreign_event)
            .expect_err("foreign stream must be rejected"),
        TerminalChordProcessError::WrongInputStream
    );

    let event = scenario.key_press(1, "q").expect("event should construct");
    router.process(event.clone()).expect("event should process");
    assert_eq!(
        router
            .process(event)
            .expect_err("replayed delivery ordinal must be rejected"),
        TerminalChordProcessError::DeliveryAlreadyConsumed
    );

    let next = scenario
        .key_press(2, "q")
        .expect("next event should construct");
    let output = router
        .process(next)
        .expect("next event should still process");
    assert!(
        format!("{output:?}").contains(&binding.ordinal().to_string()),
        "prior wrong-stream/replay attempts must not mutate binding state"
    );
}

#[test]
fn terminal_security_fake_backend_activation_rejects_duplicate_authority() {
    let scenario = TerminalTestScenario::new();
    let activation =
        TerminalTestActivation::activate(&scenario).expect("first activation should succeed");
    assert_eq!(
        TerminalTestActivation::activate(&scenario)
            .expect_err("same scenario authority cannot be duplicated"),
        TerminalTestFactoryError::AlreadyActive
    );
    activation
        .deactivate()
        .expect("original activation should deactivate");
}
