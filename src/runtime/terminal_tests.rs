extern crate alloc;

use crate::runtime::terminal::constraints::{
    TerminalColorCount, TerminalColumnCount, TerminalColumnIndex, TerminalCommittedText,
    TerminalCommittedTextByteLimit, TerminalCompositionId, TerminalCompositionPreeditByteLimit,
    TerminalCompositionPreeditText, TerminalCompositionScalarIndex, TerminalControlCode,
    TerminalCorrelatedByteLimit, TerminalCorrelatedEventLimit,
    TerminalDiagnosticCollectionByteLimit, TerminalDiagnosticCountLimit, TerminalDiagnosticDetail,
    TerminalEventId, TerminalFunctionKeyNumber, TerminalInputSequenceTimeoutMilliseconds,
    TerminalKeyRepeatCount, TerminalNativeEventName, TerminalPasteChunkByteLimit,
    TerminalPasteText, TerminalPendingSequenceByteLimit, TerminalRetainedByteLimit,
    TerminalRetainedEventLimit, TerminalRowCount, TerminalRowIndex, TerminalUnknownByteChunkLimit,
    TerminalWaitMilliseconds,
};
use crate::runtime::terminal::formatting::collection_metadata_bytes_for_tests;
use crate::runtime::terminal::lifecycle::{
    clear_next_recovery_generation_for_tests, set_next_recovery_generation_for_tests,
};
use crate::runtime::terminal::model::{TerminalCompositionEnd, next_hidden_stream_id};
use crate::runtime::terminal::{
    SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES, TerminalBackend, TerminalCapabilities,
    TerminalCapabilitySupportedEvidence, TerminalCapabilityUnsupportedEvidence,
    TerminalCloseOutcome, TerminalColorCapability, TerminalCoordinatorState, TerminalDiagnostic,
    TerminalDiagnosticCollection, TerminalDiagnosticCollectionLimits,
    TerminalDiagnosticRetryability, TerminalDiagnosticSessionState, TerminalDiagnosticStage,
    TerminalFeatureCapability, TerminalInputEvent, TerminalInputEventKind,
    TerminalInputResetReason, TerminalKeyOccurrence, TerminalLinkedTextPhase, TerminalLogicalKey,
    TerminalModifiers, TerminalMouseAction, TerminalMouseTracking, TerminalNamedKey,
    TerminalNativeEventKind, TerminalNativeMetadata, TerminalOperation, TerminalOrdinaryFeature,
    TerminalOsCode, TerminalPastePhase, TerminalRecoveryLedgerKind, TerminalScrollDirection,
    TerminalSessionFeaturePolicy, TerminalSessionOpenError, TerminalSessionOptions,
    TerminalSessionOptionsError, TerminalSessionResourceLimits, TerminalSessionRestoreError,
    TerminalSessionState, TerminalSize, TerminalTextInputOrigin, TerminalTrustedPasteCapability,
    TerminalTrustedPasteEvidence, TerminalUnknownBytesReason, required_ordinary_features,
    safe_terminal_diagnostic_collection_format, safe_terminal_diagnostic_format,
};
use crate::stdlib::bytes::Bytes;
use crate::stdlib::terminal::{
    terminal_capabilities_color, terminal_capabilities_feature,
    terminal_capabilities_trusted_paste_framing, terminal_diagnostic_backend,
    terminal_diagnostic_coordinator_state, terminal_diagnostic_detail,
    terminal_diagnostic_operation, terminal_diagnostic_os_code, terminal_diagnostic_retryability,
    terminal_diagnostic_session_state, terminal_diagnostic_stage,
    terminal_diagnostic_was_truncated, terminal_diagnostics_at, terminal_diagnostics_length,
    terminal_diagnostics_omitted_bytes, terminal_diagnostics_omitted_count,
    terminal_diagnostics_retained_bytes, terminal_diagnostics_retained_count,
    terminal_diagnostics_was_truncated, terminal_recovery_token_generation,
    terminal_recovery_token_kind, terminal_session_open_sync, terminal_session_options_default,
    terminal_session_options_validate, terminal_session_options_with_feature_policy,
    terminal_session_options_with_resource_limits, terminal_session_recover_close_sync,
    terminal_session_recover_open_sync, terminal_session_state,
    trusted_terminal_output_from_application_text,
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "Task 24 requires explicit boundary and neighbor checks for every constrained type"
)]
fn terminal_numeric_constraints_accept_boundaries_and_reject_neighbors() {
    macro_rules! assert_i32_constraint {
        ($type_name:ident, $minimum:expr, $maximum:expr) => {{
            assert!($type_name::new($minimum).is_ok());
            assert!($type_name::new($maximum).is_ok());
            if $minimum > i32::MIN {
                assert!($type_name::new($minimum - 1_i32).is_err());
            }
            if $maximum < i32::MAX {
                assert!($type_name::new($maximum + 1_i32).is_err());
            }
        }};
    }

    assert!(TerminalControlCode::new(0).is_ok());
    assert!(TerminalControlCode::new(31).is_ok());
    assert!(TerminalControlCode::new(127).is_ok());
    assert!(TerminalControlCode::new(32).is_err());
    assert!(TerminalControlCode::new(126).is_err());

    assert_i32_constraint!(TerminalFunctionKeyNumber, 1_i32, 0x7FFF_i32);
    assert_i32_constraint!(TerminalColumnCount, 1_i32, i32::MAX);
    assert_i32_constraint!(TerminalRowCount, 1_i32, i32::MAX);
    assert_i32_constraint!(TerminalColumnIndex, 0_i32, i32::MAX);
    assert_i32_constraint!(TerminalRowIndex, 0_i32, i32::MAX);
    assert_i32_constraint!(TerminalKeyRepeatCount, 1_i32, i32::MAX);
    assert_i32_constraint!(TerminalWaitMilliseconds, 1_i32, i32::MAX);
    assert_i32_constraint!(TerminalInputSequenceTimeoutMilliseconds, 1_i32, 60_000_i32);
    assert_i32_constraint!(TerminalCommittedTextByteLimit, 4_i32, 0x0010_0000_i32);
    assert_i32_constraint!(TerminalCompositionPreeditByteLimit, 4_i32, 0x0010_0000_i32);
    assert_i32_constraint!(TerminalPasteChunkByteLimit, 4_i32, 0x0100_0000_i32);
    assert_i32_constraint!(TerminalUnknownByteChunkLimit, 1_i32, 0x0010_0000_i32);
    assert_i32_constraint!(TerminalPendingSequenceByteLimit, 4_i32, 0x0010_0000_i32);
    assert_i32_constraint!(TerminalRetainedEventLimit, 8_i32, 0x0010_0000_i32);
    assert_i32_constraint!(TerminalRetainedByteLimit, 4_096_i32, 0x4000_0000_i32);
    assert_i32_constraint!(TerminalCorrelatedEventLimit, 2_i32, 0x0001_0000_i32);
    assert_i32_constraint!(TerminalCorrelatedByteLimit, 64_i32, 0x0100_0000_i32);
    assert_i32_constraint!(TerminalDiagnosticCountLimit, 1_i32, 256_i32);
    assert_i32_constraint!(
        TerminalDiagnosticCollectionByteLimit,
        256_i32,
        0x0010_0000_i32
    );
    assert_i32_constraint!(TerminalColorCount, 1_i32, 0x0100_0000_i32);
}

#[test]
fn terminal_string_constraints_enforce_nul_and_length_rules() {
    assert!(TerminalDiagnosticDetail::new_runtime("plain detail").is_ok());
    assert!(TerminalDiagnosticDetail::new_runtime("bad\0detail").is_err());
    assert!(TerminalCommittedText::new_runtime("x").is_ok());
    assert!(TerminalCommittedText::new_runtime("").is_err());
    assert!(TerminalPasteText::new_runtime("paste").is_ok());
    assert!(TerminalPasteText::new_runtime("").is_err());
    assert!(TerminalCompositionPreeditText::new_runtime("").is_ok());
    assert!(TerminalCompositionPreeditText::new_runtime("bad\0preedit").is_err());
    let too_long_name = "a".repeat(257);
    assert!(TerminalNativeEventName::new_runtime(too_long_name).is_err());
    let too_long_detail = "a".repeat(4_097);
    assert!(TerminalDiagnosticDetail::new_runtime(too_long_detail).is_err());
}

#[test]
fn terminal_runtime_only_constrained_values_enforce_nonzero_and_scalar_bounds() {
    assert!(TerminalEventId::new_runtime(1).is_ok());
    assert!(TerminalEventId::new_runtime(0).is_err());
    assert!(TerminalCompositionId::new_runtime(1).is_ok());
    assert!(TerminalCompositionId::new_runtime(0).is_err());
    assert!(TerminalCompositionScalarIndex::new_runtime(0, 0).is_ok());
    assert!(TerminalCompositionScalarIndex::new_runtime(2, 1).is_err());
    assert!(TerminalCompositionScalarIndex::new_runtime(-1, 1).is_err());
}

#[test]
fn terminal_options_defaults_and_validation_match_proposal() {
    let defaults = terminal_session_options_default();
    assert_eq!(
        defaults.resource_limits().input_sequence_timeout.get(),
        25_i32
    );
    assert_eq!(
        defaults.resource_limits().maximum_retained_events.get(),
        1_024_i32
    );
    assert_eq!(
        defaults.resource_limits().maximum_correlated_events.get(),
        64_i32
    );
    assert_eq!(defaults.resource_limits().maximum_diagnostics.get(), 16_i32);
    assert_eq!(
        defaults.feature_policy().mouse_tracking,
        TerminalMouseTracking::Disabled
    );
    assert!(!defaults.feature_policy().require_trusted_paste_framing);

    let custom_policy = TerminalSessionFeaturePolicy {
        use_alternate_screen: true,
        hide_cursor: true,
        enable_bracketed_paste: true,
        require_trusted_paste_framing: false,
        enable_enhanced_key_identity: true,
        enable_focus_events: true,
        mouse_tracking: TerminalMouseTracking::AllMotion,
        capture_control_keys: true,
        require_requested_features: true,
    };
    let custom_limits = TerminalSessionResourceLimits {
        maximum_retained_events: TerminalRetainedEventLimit::new(128).unwrap(),
        maximum_retained_bytes: TerminalRetainedByteLimit::new(8_192).unwrap(),
        maximum_correlated_events: TerminalCorrelatedEventLimit::new(64).unwrap(),
        maximum_correlated_bytes: TerminalCorrelatedByteLimit::new(4_096).unwrap(),
        ..TerminalSessionResourceLimits::default()
    };

    let with_policy = terminal_session_options_with_feature_policy(&defaults, custom_policy);
    assert_ne!(with_policy.feature_policy(), defaults.feature_policy());
    assert_eq!(
        with_policy.resource_limits(),
        defaults.resource_limits(),
        "feature setter must not mutate resource limits"
    );
    assert_eq!(
        defaults.feature_policy().mouse_tracking,
        TerminalMouseTracking::Disabled,
        "feature setter must leave the original options snapshot unchanged"
    );

    let with_limits = terminal_session_options_with_resource_limits(&defaults, custom_limits);
    assert_eq!(
        with_limits.feature_policy(),
        defaults.feature_policy(),
        "resource-limit setter must not mutate feature policy"
    );
    assert_ne!(with_limits.resource_limits(), defaults.resource_limits());
    assert_eq!(
        defaults.resource_limits().maximum_retained_bytes.get(),
        0x0010_0000_i32,
        "resource-limit setter must leave the original options snapshot unchanged"
    );

    let swapped = terminal_session_options_with_feature_policy(
        &terminal_session_options_with_resource_limits(&defaults, custom_limits),
        custom_policy,
    );
    let reversed = terminal_session_options_with_resource_limits(
        &terminal_session_options_with_feature_policy(&defaults, custom_policy),
        custom_limits,
    );
    assert_eq!(swapped, reversed);
    assert!(terminal_session_options_validate(&defaults).is_ok());
    assert!(terminal_session_options_validate(&swapped).is_ok());
}

#[test]
fn terminal_options_validation_reports_exact_invalid_metadata() {
    let invalid_bytes = terminal_session_options_with_resource_limits(
        &TerminalSessionOptions::default(),
        TerminalSessionResourceLimits {
            maximum_retained_bytes: TerminalRetainedByteLimit::new(4_096).unwrap(),
            maximum_correlated_bytes: TerminalCorrelatedByteLimit::new(8_192).unwrap(),
            ..TerminalSessionResourceLimits::default()
        },
    );
    let bytes_error =
        terminal_session_options_validate(&invalid_bytes).expect_err("must reject bytes mismatch");
    let TerminalSessionOptionsError::InvalidOptions { invalid_options } = bytes_error;
    assert!(matches!(
        invalid_options,
        crate::runtime::terminal::TerminalInvalidOptions::RetainedCapacityTooSmall { .. }
    ));
    let crate::runtime::terminal::TerminalInvalidOptions::RetainedCapacityTooSmall {
        required_bytes,
        configured_bytes,
    } = invalid_options
    else {
        return;
    };
    assert_eq!(required_bytes, 8_192);
    assert_eq!(configured_bytes, 4_096);

    let invalid_events = terminal_session_options_with_resource_limits(
        &TerminalSessionOptions::default(),
        TerminalSessionResourceLimits {
            maximum_retained_events: TerminalRetainedEventLimit::new(8).unwrap(),
            maximum_correlated_events: TerminalCorrelatedEventLimit::new(64).unwrap(),
            ..TerminalSessionResourceLimits::default()
        },
    );
    let events_error = terminal_session_options_validate(&invalid_events)
        .expect_err("must reject correlated-event mismatch");
    let TerminalSessionOptionsError::InvalidOptions {
        invalid_options: event_invalid_options,
    } = events_error;
    assert!(matches!(
        event_invalid_options,
        crate::runtime::terminal::TerminalInvalidOptions::CorrelatedGroupTooLarge { .. }
    ));
    let crate::runtime::terminal::TerminalInvalidOptions::CorrelatedGroupTooLarge {
        required_events,
        configured_events,
    } = event_invalid_options
    else {
        return;
    };
    assert_eq!(required_events, 64);
    assert_eq!(configured_events, 8);
}

fn sample_diagnostic(detail: &str) -> TerminalDiagnostic {
    TerminalDiagnostic::new_runtime(
        TerminalBackend::LinuxVt,
        TerminalOperation::ValidateOptions,
        TerminalDiagnosticStage::ValidateOptions,
        TerminalCoordinatorState::Free,
        TerminalDiagnosticSessionState::Unavailable,
        TerminalOsCode::PosixErrno { value: 22 },
        TerminalDiagnosticDetail::new_runtime(detail).unwrap(),
        TerminalDiagnosticRetryability::NonRetryable,
        false,
    )
}

#[test]
fn terminal_diagnostic_inspectors_and_safe_formatting_are_stable() {
    let diagnostic = sample_diagnostic("bad\u{1B}[31m\u{202E}detail");
    assert_eq!(
        terminal_diagnostic_backend(&diagnostic),
        TerminalBackend::LinuxVt
    );
    assert_eq!(
        terminal_diagnostic_operation(&diagnostic),
        TerminalOperation::ValidateOptions
    );
    assert_eq!(
        terminal_diagnostic_stage(&diagnostic),
        TerminalDiagnosticStage::ValidateOptions
    );
    assert_eq!(
        terminal_diagnostic_coordinator_state(&diagnostic),
        TerminalCoordinatorState::Free
    );
    assert_eq!(
        terminal_diagnostic_session_state(&diagnostic),
        TerminalDiagnosticSessionState::Unavailable
    );
    assert_eq!(
        terminal_diagnostic_os_code(&diagnostic),
        TerminalOsCode::PosixErrno { value: 22_i32 }
    );
    assert_eq!(
        terminal_diagnostic_detail(&diagnostic).as_str(),
        "bad\u{1B}[31m\u{202E}detail"
    );
    assert_eq!(
        terminal_diagnostic_retryability(&diagnostic),
        TerminalDiagnosticRetryability::NonRetryable
    );
    assert!(!terminal_diagnostic_was_truncated(&diagnostic));
    let safe = safe_terminal_diagnostic_format(&diagnostic);
    assert!(!safe.as_str().contains('\u{1B}'));
    assert!(safe.as_str().contains("\\u{1B}"));
    assert!(safe.as_str().contains("\\u{202E}"));
}

#[test]
fn terminal_safe_output_is_bounded_after_escaping() {
    let detail = "\u{1B}".repeat(4_096);
    let diagnostics = (0_i32..20_i32)
        .map(|_| sample_diagnostic(detail.as_str()))
        .collect::<Vec<_>>();
    let collection = TerminalDiagnosticCollection::new_runtime(
        diagnostics,
        TerminalDiagnosticCollectionLimits {
            maximum_diagnostics: TerminalDiagnosticCountLimit::new(64).unwrap(),
            maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit::new(0x0010_0000)
                .unwrap(),
        },
    );
    let safe = safe_terminal_diagnostic_collection_format(&collection);
    assert!(safe.as_str().len() <= SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES);
    assert!(safe.as_str().contains("...[truncated]"));
    assert!(!safe.as_str().contains('\u{1B}'));
}

#[test]
fn terminal_diagnostic_collection_retains_longest_prefix_and_exact_accounting() {
    let first = sample_diagnostic("short");
    let second = sample_diagnostic("this one should be omitted because the byte limit is low");
    let limits = TerminalDiagnosticCollectionLimits {
        maximum_diagnostics: TerminalDiagnosticCountLimit::new(4).unwrap(),
        maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit::new(256).unwrap(),
    };
    let collection =
        TerminalDiagnosticCollection::new_runtime(vec![first.clone(), second.clone()], limits);
    assert_eq!(terminal_diagnostics_length(&collection), 1);
    assert_eq!(terminal_diagnostics_retained_count(&collection), 1);
    assert_eq!(terminal_diagnostics_omitted_count(&collection), 1);
    assert!(terminal_diagnostics_was_truncated(&collection));
    assert_eq!(
        terminal_diagnostics_retained_bytes(&collection),
        collection_metadata_bytes_for_tests() + first.accounted_bytes()
    );
    assert_eq!(
        terminal_diagnostics_omitted_bytes(&collection),
        second.accounted_bytes()
    );
    assert_eq!(terminal_diagnostics_at(&collection, 0).unwrap(), first);
    assert!(terminal_diagnostics_at(&collection, 1).is_none());
}

#[test]
fn terminal_diagnostic_collection_omitted_bytes_saturate_without_wrap() {
    let first = sample_diagnostic("a").with_accounted_bytes_for_tests(u64::MAX - 4);
    let second = sample_diagnostic("b").with_accounted_bytes_for_tests(10);
    let collection = TerminalDiagnosticCollection::new_runtime(
        vec![first, second],
        TerminalDiagnosticCollectionLimits {
            maximum_diagnostics: TerminalDiagnosticCountLimit::new(1).unwrap(),
            maximum_diagnostic_bytes: TerminalDiagnosticCollectionByteLimit::new(256).unwrap(),
        },
    );
    assert_eq!(collection.omitted_bytes(), u64::MAX);
    assert_eq!(collection.omitted_count(), 2);
    assert!(collection.was_truncated());
}

#[test]
fn terminal_trust_conversion_is_explicit_and_redacted() {
    let trusted = trusted_terminal_output_from_application_text("render \u{1B}[2J");
    let debug = format!("{trusted:?}");
    assert!(!debug.contains("render"));
    assert!(debug.contains("utf8_bytes"));
}

#[test]
fn terminal_capability_inspectors_cover_every_feature_and_hide_metadata() {
    let stream_id = next_hidden_stream_id();
    let ordinary = BTreeMap::from([
        (
            TerminalOrdinaryFeature::AlternateScreen,
            TerminalFeatureCapability::Available {
                evidence: TerminalCapabilitySupportedEvidence::NativeConfirmed,
            },
        ),
        (
            TerminalOrdinaryFeature::CursorShape,
            TerminalFeatureCapability::Enabled {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolQueried,
            },
        ),
        (
            TerminalOrdinaryFeature::BracketedPaste,
            TerminalFeatureCapability::Available {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolQueried,
            },
        ),
        (
            TerminalOrdinaryFeature::FocusEvents,
            TerminalFeatureCapability::Available {
                evidence: TerminalCapabilitySupportedEvidence::EnvironmentInferred,
            },
        ),
        (
            TerminalOrdinaryFeature::MouseButtons,
            TerminalFeatureCapability::Enabled {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolQueried,
            },
        ),
        (
            TerminalOrdinaryFeature::MouseMotion,
            TerminalFeatureCapability::Unsupported {
                evidence: TerminalCapabilityUnsupportedEvidence::EnvironmentMissing,
            },
        ),
        (
            TerminalOrdinaryFeature::KeyReleaseEvents,
            TerminalFeatureCapability::Available {
                evidence: TerminalCapabilitySupportedEvidence::NativeConfirmed,
            },
        ),
        (
            TerminalOrdinaryFeature::CompositionEvents,
            TerminalFeatureCapability::Available {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolQueried,
            },
        ),
        (
            TerminalOrdinaryFeature::EnhancedKeyIdentity,
            TerminalFeatureCapability::Enabled {
                evidence: TerminalCapabilitySupportedEvidence::ProtocolQueried,
            },
        ),
    ]);
    assert_eq!(ordinary.len(), required_ordinary_features().len());
    let capabilities = TerminalCapabilities::new_runtime(
        ordinary,
        TerminalTrustedPasteCapability::Enabled {
            evidence: TerminalTrustedPasteEvidence::NativeRecordBoundary,
        },
        TerminalColorCapability::Indexed {
            count: TerminalColorCount::new(256).unwrap(),
            evidence: TerminalCapabilitySupportedEvidence::EnvironmentInferred,
        },
        stream_id,
        TerminalCorrelatedEventLimit::new(64).unwrap(),
    );
    assert_eq!(
        terminal_capabilities_feature(&capabilities, TerminalOrdinaryFeature::AlternateScreen),
        TerminalFeatureCapability::Available {
            evidence: TerminalCapabilitySupportedEvidence::NativeConfirmed,
        }
    );
    assert_eq!(
        terminal_capabilities_trusted_paste_framing(&capabilities),
        TerminalTrustedPasteCapability::Enabled {
            evidence: TerminalTrustedPasteEvidence::NativeRecordBoundary,
        }
    );
    assert_eq!(
        terminal_capabilities_color(&capabilities),
        TerminalColorCapability::Indexed {
            count: TerminalColorCount::new(256).unwrap(),
            evidence: TerminalCapabilitySupportedEvidence::EnvironmentInferred,
        }
    );
    let debug = format!("{capabilities:?}");
    assert!(!debug.contains("hidden_stream_id"));
    assert!(!debug.contains("hidden_correlated_event_limit"));
    assert_eq!(capabilities.hidden_stream_id(), stream_id);
    assert_eq!(capabilities.hidden_correlated_event_limit().get(), 64_i32);
}

#[test]
#[expect(
    clippy::too_many_lines,
    reason = "Task 24 requires a direct per-variant proof over every selected input event shape"
)]
fn terminal_input_events_cover_every_variant_and_hide_hidden_metadata() {
    let stream_id = next_hidden_stream_id();
    let event_id = TerminalEventId::new_runtime(1).unwrap();
    let composition_id = TerminalCompositionId::new_runtime(1).unwrap();
    let modifiers = TerminalModifiers {
        shift: false,
        control: true,
        alt: false,
        super_key: false,
        caps_lock: false,
        num_lock: false,
    };
    let events = [
        TerminalInputEvent::new_runtime(
            stream_id,
            1,
            TerminalInputEventKind::Key {
                event_id,
                key: TerminalLogicalKey::Named {
                    key: TerminalNamedKey::Enter,
                },
                occurrence: TerminalKeyOccurrence::Press {
                    count: TerminalKeyRepeatCount::new(1).unwrap(),
                },
                modifiers,
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            2,
            TerminalInputEventKind::TextInput {
                text: TerminalCommittedText::new_runtime("text").unwrap(),
                origin: TerminalTextInputOrigin::Key { event_id },
                linked_phase: TerminalLinkedTextPhase::Complete,
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            3,
            TerminalInputEventKind::CompositionStarted { composition_id },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            4,
            TerminalInputEventKind::CompositionUpdated {
                composition_id,
                preedit_text: TerminalCompositionPreeditText::new_runtime("pré").unwrap(),
                cursor: TerminalCompositionScalarIndex::new_runtime(2, 3).unwrap(),
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            5,
            TerminalInputEventKind::CompositionEnded {
                composition_id,
                outcome: TerminalCompositionEnd::Committed,
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            6,
            TerminalInputEventKind::Paste {
                text: TerminalPasteText::new_runtime("paste").unwrap(),
                phase: TerminalPastePhase::Complete,
                evidence: TerminalTrustedPasteEvidence::NativeRecordBoundary,
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            7,
            TerminalInputEventKind::Mouse {
                action: TerminalMouseAction::Scroll {
                    direction: TerminalScrollDirection::Down,
                },
                modifiers,
                row: TerminalRowIndex::new(2).unwrap(),
                column: TerminalColumnIndex::new(3).unwrap(),
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            8,
            TerminalInputEventKind::Resize {
                size: TerminalSize {
                    columns: TerminalColumnCount::new(80).unwrap(),
                    rows: TerminalRowCount::new(24).unwrap(),
                },
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(stream_id, 9, TerminalInputEventKind::FocusGained).unwrap(),
        TerminalInputEvent::new_runtime(stream_id, 10, TerminalInputEventKind::FocusLost).unwrap(),
        TerminalInputEvent::new_runtime(stream_id, 11, TerminalInputEventKind::TimedOut).unwrap(),
        TerminalInputEvent::new_runtime(stream_id, 12, TerminalInputEventKind::Cancelled).unwrap(),
        TerminalInputEvent::new_runtime(stream_id, 13, TerminalInputEventKind::EndOfInput).unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            14,
            TerminalInputEventKind::UnknownBytes {
                raw_bytes: Bytes::from_slice(&[0x1B, 0x00]),
                reason: TerminalUnknownBytesReason::PasteContainsNul,
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            15,
            TerminalInputEventKind::UnknownNative {
                metadata: TerminalNativeMetadata {
                    kind: TerminalNativeEventKind::Other {
                        name: TerminalNativeEventName::new_runtime("custom").unwrap(),
                    },
                    code: TerminalOsCode::Unavailable,
                },
            },
        )
        .unwrap(),
        TerminalInputEvent::new_runtime(
            stream_id,
            16,
            TerminalInputEventKind::InputReset {
                reason: TerminalInputResetReason::PauseBoundary,
            },
        )
        .unwrap(),
    ];

    for (ordinal, event) in events.iter().enumerate() {
        assert_eq!(event.hidden_stream_id(), stream_id);
        assert_eq!(
            event.hidden_delivery_ordinal(),
            u64::try_from(ordinal + 1).unwrap()
        );
        let debug = format!("{event:?}");
        assert!(!debug.contains("stream_id"));
        assert!(!debug.contains("delivery_ordinal"));
    }
}

#[test]
fn terminal_session_lifecycle_state_matrix_and_close_obligation_are_exact() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Active
    );
    assert!(session.cleanup_obligation());
    assert_ne!(session.session_id_for_tests(), 0);
    assert_ne!(session.reserved_recovery_generation_for_tests(), 0);
    assert!(session.size_sync().is_ok());

    let resume_error = session
        .resume_sync()
        .expect_err("active resume must reject");
    assert_eq!(resume_error.state(), TerminalSessionState::Active);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Active
    );

    session
        .pause_sync()
        .expect("active pause should transition");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Paused
    );
    assert!(session.size_sync().is_ok());
    session
        .pause_sync()
        .expect("paused pause is a non-transition");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Paused
    );
    session
        .resume_sync()
        .expect("paused resume should transition");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Active
    );

    let close_outcome = session.close_sync().expect("active close should succeed");
    assert_eq!(close_outcome, TerminalCloseOutcome::Clean);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Closed
    );
    assert!(!session.cleanup_obligation());
    assert_eq!(session.reserved_recovery_generation_for_tests(), 0);
    assert_eq!(session.close_sync(), Ok(TerminalCloseOutcome::Clean));

    let size_error = session.size_sync().expect_err("closed size must reject");
    assert_eq!(size_error.state(), TerminalSessionState::Closed);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Closed
    );
}

#[test]
fn terminal_session_restore_pending_rejections_are_non_mutating() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    session.force_restore_pending_for_tests();
    let before_generation = session.reserved_recovery_generation_for_tests();

    let size_error = session
        .size_sync()
        .expect_err("restore-pending size must reject");
    assert_eq!(size_error.state(), TerminalSessionState::RestorePending);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::RestorePending
    );
    assert_eq!(
        session.reserved_recovery_generation_for_tests(),
        before_generation
    );

    let pause_error = session
        .pause_sync()
        .expect_err("restore-pending pause must reject");
    assert_eq!(pause_error.state(), TerminalSessionState::RestorePending);
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::RestorePending
    );
}

#[test]
fn terminal_session_open_generation_exhaustion_is_preflight_and_not_restore_family() {
    set_next_recovery_generation_for_tests(u64::MAX);
    let options = terminal_session_options_default();
    let error = terminal_session_open_sync(&options).expect_err("generation exhaustion fails open");
    assert!(matches!(
        error,
        TerminalSessionOpenError::GenerationExhausted {
            last_issued_generation: u64::MAX,
            ..
        }
    ));

    set_next_recovery_generation_for_tests(1);
    let session = terminal_session_open_sync(&options).expect("reset generation should open");
    assert_eq!(
        terminal_session_state(&session),
        TerminalSessionState::Active
    );
    clear_next_recovery_generation_for_tests();
}

#[test]
fn terminal_session_open_validation_surfaces_structured_invalid_options() {
    let invalid_options = terminal_session_options_with_resource_limits(
        &terminal_session_options_default(),
        TerminalSessionResourceLimits {
            maximum_retained_events: TerminalRetainedEventLimit::new(8).unwrap(),
            maximum_correlated_events: TerminalCorrelatedEventLimit::new(64).unwrap(),
            ..TerminalSessionResourceLimits::default()
        },
    );
    let error = terminal_session_open_sync(&invalid_options).expect_err("invalid options fail");
    assert!(matches!(
        error,
        TerminalSessionOpenError::InvalidOptions { .. }
    ));
    let TerminalSessionOpenError::InvalidOptions {
        invalid_options: reported_invalid_options,
        ..
    } = error
    else {
        return;
    };
    assert!(matches!(
        reported_invalid_options,
        crate::runtime::terminal::TerminalInvalidOptions::CorrelatedGroupTooLarge { .. }
    ));
}

#[test]
fn terminal_recovery_token_validation_order_and_one_shot_aliases_are_exact() {
    let options = terminal_session_options_default();
    let mut session = terminal_session_open_sync(&options).expect("open should succeed");
    session.force_restore_pending_for_tests();
    let close_token = session.cleanup_transfer_token_for_tests();
    let close_alias = Clone::clone(&close_token);

    assert_eq!(
        terminal_recovery_token_kind(&close_token),
        TerminalRecoveryLedgerKind::CloseRestore
    );
    assert_eq!(
        terminal_recovery_token_generation(&close_token),
        session.reserved_recovery_generation_for_tests()
    );
    let wrong_kind = terminal_session_recover_open_sync(&close_token)
        .expect_err("close token cannot recover open");
    assert!(matches!(
        wrong_kind,
        TerminalSessionRestoreError::WrongKind { .. }
    ));
    assert!(!close_token.is_consumed());

    terminal_session_recover_close_sync(&close_token).expect("first close recovery succeeds");
    assert!(close_token.is_consumed());
    let consumed = terminal_session_recover_close_sync(&close_alias)
        .expect_err("alias must observe consumed authority");
    assert!(matches!(
        consumed,
        TerminalSessionRestoreError::Consumed { .. }
    ));
}

#[test]
fn terminal_recovery_token_wrong_session_stale_and_in_progress_are_non_mutating() {
    let wrong_session = crate::runtime::terminal::TerminalRecoveryToken::new_runtime(
        999,
        1,
        1,
        1,
        TerminalRecoveryLedgerKind::CloseRestore,
    );
    let wrong_session_error = terminal_session_recover_close_sync(&wrong_session)
        .expect_err("wrong host must reject first");
    assert!(matches!(
        wrong_session_error,
        TerminalSessionRestoreError::WrongSession { .. }
    ));
    assert!(!wrong_session.is_consumed());

    let stale = crate::runtime::terminal::TerminalRecoveryToken::new_runtime(
        1,
        1,
        1,
        0,
        TerminalRecoveryLedgerKind::CloseRestore,
    );
    let stale_error =
        terminal_session_recover_close_sync(&stale).expect_err("zero generation is stale");
    assert!(matches!(
        stale_error,
        TerminalSessionRestoreError::Stale { .. }
    ));
    assert!(!stale.is_consumed());

    let in_progress = crate::runtime::terminal::TerminalRecoveryToken::new_runtime(
        1,
        1,
        1,
        1,
        TerminalRecoveryLedgerKind::CloseRestore,
    );
    in_progress
        .claimed()
        .store(true, core::sync::atomic::Ordering::Release);
    let busy = terminal_session_recover_close_sync(&in_progress)
        .expect_err("claimed token rejects as in progress");
    assert!(matches!(
        busy,
        TerminalSessionRestoreError::RecoveryInProgress { .. }
    ));
    assert!(!in_progress.is_consumed());
}
