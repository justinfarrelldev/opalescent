use super::{
    TerminalFeatureCapability, TerminalInputEventKind, TerminalOperation, TerminalOrdinaryFeature,
    TerminalTestActivation, TerminalTestFactoryError, TerminalTestFault, TerminalTestScenario,
};

#[test]
fn terminal_test_factories_preserve_hidden_stream_and_delivery_order() {
    let scenario = TerminalTestScenario::new();
    let key = scenario
        .key_press(7, "q")
        .expect("key event should be constructible");
    let text = scenario
        .linked_text(&key, "q")
        .expect("linked text should accept same-stream key");
    let composition = scenario
        .composition_started(3)
        .expect("composition start should be constructible");

    assert_eq!(key.hidden_stream_id(), scenario.hidden_stream_id());
    assert_eq!(text.hidden_stream_id(), scenario.hidden_stream_id());
    assert_eq!(composition.hidden_stream_id(), scenario.hidden_stream_id());
    assert_eq!(key.hidden_delivery_ordinal(), 1);
    assert_eq!(text.hidden_delivery_ordinal(), 2);
    assert_eq!(composition.hidden_delivery_ordinal(), 3);

    assert!(matches!(key.kind(), &TerminalInputEventKind::Key { .. }));
    assert!(matches!(
        text.kind(),
        &TerminalInputEventKind::TextInput { .. }
    ));
    assert!(matches!(
        composition.kind(),
        &TerminalInputEventKind::CompositionStarted { .. }
    ));
}

#[test]
fn terminal_test_factories_reject_cross_scenario_and_exhausted_ordinals() {
    let left = TerminalTestScenario::new();
    let right = TerminalTestScenario::new();
    let foreign_key = right
        .key_press(1, "x")
        .expect("foreign key should be constructible");
    assert_eq!(
        left.linked_text(&foreign_key, "x")
            .expect_err("cross-scenario linked text must fail"),
        TerminalTestFactoryError::WrongScenario
    );

    let exhausted = TerminalTestScenario::with_next_delivery_ordinal_for_tests(u64::MAX);
    let first = exhausted
        .key_press(1, "x")
        .expect_err("publishing max ordinal would prevent advancing without reuse");
    assert_eq!(first, TerminalTestFactoryError::DeliveryOrdinalExhausted);
}

#[test]
fn terminal_test_capability_and_diagnostic_factories_use_production_shapes() {
    let scenario = TerminalTestScenario::new();
    let capabilities = scenario
        .capabilities(&[
            TerminalOrdinaryFeature::AlternateScreen,
            TerminalOrdinaryFeature::EnhancedKeyIdentity,
        ])
        .expect("capabilities should be constructible");

    assert_eq!(capabilities.hidden_stream_id(), scenario.hidden_stream_id());
    assert!(matches!(
        capabilities.feature(TerminalOrdinaryFeature::AlternateScreen),
        TerminalFeatureCapability::Enabled { .. }
    ));
    assert!(matches!(
        capabilities.feature(TerminalOrdinaryFeature::FocusEvents),
        TerminalFeatureCapability::Unsupported { .. }
    ));
    assert_eq!(
        scenario
            .capabilities(&[
                TerminalOrdinaryFeature::FocusEvents,
                TerminalOrdinaryFeature::FocusEvents,
            ])
            .expect_err("duplicate features must be rejected"),
        TerminalTestFactoryError::InvalidValue {
            type_name: "TerminalCapabilities",
            reason: "duplicate ordinary feature".to_owned(),
        }
    );

    let diagnostic = scenario
        .diagnostic("decode detail")
        .expect("diagnostic should be constructible");
    assert_eq!(diagnostic.operation(), TerminalOperation::Read);
    assert_eq!(diagnostic.detail().as_str(), "decode detail");

    let limits = TerminalTestScenario::diagnostic_collection_limits(2, 4096)
        .expect("collection limits should use constrained constructors");
    assert_eq!(limits.maximum_diagnostics.get(), 2_i32);
}

#[test]
fn terminal_fake_backend_activation_is_task_local_lifo() {
    let outer = TerminalTestScenario::new();
    let inner = TerminalTestScenario::new();
    let outer_guard =
        TerminalTestActivation::activate(&outer).expect("outer activation should succeed");
    assert!(TerminalTestActivation::is_current(&outer));
    assert_eq!(
        TerminalTestActivation::activate(&outer).expect_err("same scenario cannot be active twice"),
        TerminalTestFactoryError::AlreadyActive
    );

    let inner_guard =
        TerminalTestActivation::activate(&inner).expect("nested distinct scenario should activate");
    assert!(TerminalTestActivation::is_current(&inner));
    inner_guard
        .deactivate()
        .expect("inner deactivation should be LIFO");
    assert!(TerminalTestActivation::is_current(&outer));
    outer_guard
        .deactivate()
        .expect("outer deactivation should complete stack");
}

#[test]
fn terminal_fake_backend_fault_plan_is_deterministic() {
    let scenario = TerminalTestScenario::new();
    assert_eq!(
        scenario
            .consume_fault(TerminalTestFault::OpenAfterMutation)
            .expect_err("unplanned fault should fail"),
        TerminalTestFactoryError::FaultNotPlanned
    );

    scenario.plan_fault(TerminalTestFault::OpenAfterMutation);
    scenario.plan_fault(TerminalTestFault::CloseInverseRestore);
    scenario
        .consume_fault(TerminalTestFault::CloseInverseRestore)
        .expect("planned close fault should be consumable out of chronological host order");
    scenario
        .consume_fault(TerminalTestFault::OpenAfterMutation)
        .expect("planned open fault should be consumed exactly once");
    assert_eq!(
        scenario
            .consume_fault(TerminalTestFault::OpenAfterMutation)
            .expect_err("consumed fault cannot replay"),
        TerminalTestFactoryError::FaultNotPlanned
    );
}
