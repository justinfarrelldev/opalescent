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
};
use crate::runtime::terminal::formatting::collection_metadata_bytes_for_tests;
use crate::runtime::terminal::model::{TerminalCompositionEnd, next_hidden_stream_id};
use crate::runtime::terminal::{
    SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES, TerminalBackend, TerminalCapabilities,
    TerminalCapabilitySupportedEvidence, TerminalCapabilityUnsupportedEvidence,
    TerminalColorCapability, TerminalCoordinatorState, TerminalDiagnostic,
    TerminalDiagnosticCollection, TerminalDiagnosticCollectionLimits,
    TerminalDiagnosticRetryability, TerminalDiagnosticSessionState, TerminalDiagnosticStage,
    TerminalFeatureCapability, TerminalInputEvent, TerminalInputEventKind,
    TerminalInputResetReason, TerminalKeyOccurrence, TerminalLinkedTextPhase, TerminalLogicalKey,
    TerminalModifiers, TerminalMouseAction, TerminalMouseTracking, TerminalNamedKey,
    TerminalNativeEventKind, TerminalNativeMetadata, TerminalOperation, TerminalOrdinaryFeature,
    TerminalOsCode, TerminalPastePhase, TerminalScrollDirection, TerminalSessionFeaturePolicy,
    TerminalSessionOptions, TerminalSessionOptionsError, TerminalSessionResourceLimits,
    TerminalSize, TerminalTextInputOrigin, TerminalTrustedPasteCapability,
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
    terminal_diagnostics_was_truncated, terminal_session_options_default,
    terminal_session_options_validate, terminal_session_options_with_feature_policy,
    terminal_session_options_with_resource_limits, trusted_terminal_output_from_application_text,
};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "Task 24 requires explicit boundary and neighbor checks for every constrained type"
)]
fn terminal_numeric_constraints_accept_boundaries_and_reject_neighbors() {
    assert!(TerminalControlCode::new(31).is_ok());
    assert!(TerminalControlCode::new(127).is_ok());
    assert!(TerminalControlCode::new(32).is_err());
    assert!(TerminalFunctionKeyNumber::new(1).is_ok());
    assert!(TerminalFunctionKeyNumber::new(0x7FFF).is_ok());
    assert!(TerminalFunctionKeyNumber::new(0).is_err());
    assert!(TerminalColumnCount::new(1).is_ok());
    assert!(TerminalColumnCount::new(0).is_err());
    assert!(TerminalRowCount::new(1).is_ok());
    assert!(TerminalRowCount::new(0).is_err());
    assert!(TerminalColumnIndex::new(0).is_ok());
    assert!(TerminalColumnIndex::new(-1).is_err());
    assert!(TerminalRowIndex::new(0).is_ok());
    assert!(TerminalRowIndex::new(-1).is_err());
    assert!(TerminalKeyRepeatCount::new(1).is_ok());
    assert!(TerminalKeyRepeatCount::new(0).is_err());
    assert!(TerminalInputSequenceTimeoutMilliseconds::new(1).is_ok());
    assert!(TerminalInputSequenceTimeoutMilliseconds::new(60_000).is_ok());
    assert!(TerminalInputSequenceTimeoutMilliseconds::new(60_001).is_err());
    assert!(TerminalCommittedTextByteLimit::new(4).is_ok());
    assert!(TerminalCommittedTextByteLimit::new(3).is_err());
    assert!(TerminalCompositionPreeditByteLimit::new(4).is_ok());
    assert!(TerminalCompositionPreeditByteLimit::new(3).is_err());
    assert!(TerminalPasteChunkByteLimit::new(4).is_ok());
    assert!(TerminalPasteChunkByteLimit::new(3).is_err());
    assert!(TerminalUnknownByteChunkLimit::new(1).is_ok());
    assert!(TerminalUnknownByteChunkLimit::new(0).is_err());
    assert!(TerminalPendingSequenceByteLimit::new(4).is_ok());
    assert!(TerminalPendingSequenceByteLimit::new(3).is_err());
    assert!(TerminalRetainedEventLimit::new(8).is_ok());
    assert!(TerminalRetainedEventLimit::new(7).is_err());
    assert!(TerminalRetainedByteLimit::new(4_096).is_ok());
    assert!(TerminalRetainedByteLimit::new(4_095).is_err());
    assert!(TerminalCorrelatedEventLimit::new(2).is_ok());
    assert!(TerminalCorrelatedEventLimit::new(1).is_err());
    assert!(TerminalCorrelatedByteLimit::new(64).is_ok());
    assert!(TerminalCorrelatedByteLimit::new(63).is_err());
    assert!(TerminalDiagnosticCountLimit::new(1).is_ok());
    assert!(TerminalDiagnosticCountLimit::new(0).is_err());
    assert!(TerminalDiagnosticCollectionByteLimit::new(256).is_ok());
    assert!(TerminalDiagnosticCollectionByteLimit::new(255).is_err());
    assert!(TerminalColorCount::new(1).is_ok());
    assert!(TerminalColorCount::new(0).is_err());
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
    assert_eq!(defaults.resource_limits().input_sequence_timeout.get(), 25_i32);
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

    let swapped = terminal_session_options_with_feature_policy(
        &terminal_session_options_with_resource_limits(
            &defaults,
            TerminalSessionResourceLimits::default(),
        ),
        TerminalSessionFeaturePolicy::default(),
    );
    let reversed = terminal_session_options_with_resource_limits(
        &terminal_session_options_with_feature_policy(
            &defaults,
            TerminalSessionFeaturePolicy::default(),
        ),
        TerminalSessionResourceLimits::default(),
    );
    assert_eq!(swapped, reversed);
    assert!(terminal_session_options_validate(&defaults).is_ok());
}

#[test]
fn terminal_options_validation_reports_exact_invalid_metadata() {
    let invalid_limits = TerminalSessionResourceLimits {
        maximum_retained_bytes: TerminalRetainedByteLimit::new(4_096).unwrap(),
        maximum_correlated_bytes: TerminalCorrelatedByteLimit::new(8_192).unwrap(),
        ..TerminalSessionResourceLimits::default()
    };
    let invalid = terminal_session_options_with_resource_limits(
        &TerminalSessionOptions::default(),
        invalid_limits,
    );
    let error =
        terminal_session_options_validate(&invalid).expect_err("must reject bytes mismatch");
    let TerminalSessionOptionsError::InvalidOptions { invalid_options } = error;
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
