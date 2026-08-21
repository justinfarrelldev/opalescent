extern crate alloc;

use crate::ast::{Decl, Documentation, Program, Visibility as AstVisibility};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::module_resolver::ModuleInterface;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;

/// Inject required doc comments for public/entry functions in inline test sources.
fn with_required_function_docs(source: &str) -> String {
    const DOC_COMMENT_BLOCK: &str =
        "##\n    Description: Test helper generated function documentation text\n##\n";

    let mut rewritten_source = String::new();
    let mut last_non_empty_line: Option<String> = None;

    for line in source.lines() {
        let trimmed_start = line.trim_start();
        let is_public_or_entry_function = (trimmed_start.starts_with("entry ")
            || trimmed_start.starts_with("public "))
            && (trimmed_start.contains("= f(") || trimmed_start.contains("= f<"));
        let has_doc_block_before = last_non_empty_line
            .as_deref()
            .is_some_and(|previous_line| previous_line.trim_start().starts_with("##"));

        if is_public_or_entry_function && !has_doc_block_before {
            rewritten_source.push_str(DOC_COMMENT_BLOCK);
        }

        rewritten_source.push_str(line);
        rewritten_source.push('\n');

        if !trimmed_start.is_empty() {
            last_non_empty_line = Some(trimmed_start.to_owned());
        }
    }

    rewritten_source
}

fn parse_pipeline(source: &str) -> Program {
    let source_with_docs = with_required_function_docs(source);
    let lexer = Lexer::new(&source_with_docs);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "integration source must lex without errors; lex errors: {:?}",
        lex_errors.errors,
    );

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "integration source must parse without errors; parse errors: {:?}",
        parse_errors.errors,
    );

    let mut program = program_opt.map_or_else(
        || Program {
            declarations: Vec::new(),
            span: Span::single(Position::start()),
            id: crate::ast::NodeId(0),
        },
        |program| program,
    );

    for declaration in &mut program.declarations {
        if let &mut Decl::Function {
            visibility: ref function_visibility,
            is_entry,
            doc_comment: ref mut function_doc_comment,
            span,
            ..
        } = declaration
        {
            let requires_doc = is_entry || matches!(function_visibility, &AstVisibility::Public);
            if requires_doc && function_doc_comment.is_none() {
                *function_doc_comment = Some(Documentation::from_raw(
                    "Description: Generated module integration test documentation".to_owned(),
                    span,
                ));
            }
        }
    }

    program
}

fn symbol(name: &str, core_type: CoreType, visibility: Visibility) -> SymbolInfo {
    SymbolInfo {
        name: name.to_owned(),
        symbol_type: SymbolType::Function,
        core_type,
        visibility,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_standard_import_resolves_symbols_in_scope() {
        const SOURCE: &str = "
import print, take_input from standard

entry main = f(): void => {
    let s: string = take_input()
    print(s)
    return void
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "imported standard symbols should resolve: {result:?}",
        );
    }

    #[test]
    fn test_import_unknown_symbol_reports_symbol_not_found() {
        const SOURCE: &str = "
import missing_fn from standard

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("unknown import must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::SymbolNotFound { .. })),
            "expected SymbolNotFound error, got: {errors:?}",
        );
    }

    #[test]
    fn test_circular_dependency_reports_error() {
        const SOURCE: &str = "
import b_fn from ./module_b

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.set_current_module_path(String::from("./module_a"));

        let mut module_b = ModuleInterface::new(String::from("./module_b"));
        let register_result = module_b.register_symbol(symbol(
            "b_fn",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            Visibility::Public,
        ));
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(module_b);
        checker.register_module_dependency("./module_b", "./module_a");

        let result = checker.type_check_program(&program);
        let errors = result.expect_err("circular dependency must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::CircularDependency { .. })),
            "expected CircularDependency error, got: {errors:?}",
        );
    }

    #[test]
    fn test_private_symbol_import_reports_private_access_error() {
        const SOURCE: &str = "
import hidden_fn from ./local_lib

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();

        let mut local_lib = ModuleInterface::new(String::from("./local_lib"));
        let register_result = local_lib.register_symbol(symbol(
            "hidden_fn",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            Visibility::Private,
        ));
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(local_lib);

        let result = checker.type_check_program(&program);
        let errors = result.expect_err("private import must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::PrivateSymbolAccess { .. })),
            "expected PrivateSymbolAccess error, got: {errors:?}",
        );
    }

    #[test]
    fn test_cross_module_multi_return_labels_survive_export_import() {
        const PRODUCER_SOURCE: &str = "
##
    Description: Produces a labeled pair for module tests.
##
public pair = f(): x: int64, y: int64 =>
    return x: 1, y: 2
entry main = f(): void =>
    return void
";
        const CONSUMER_SOURCE: &str = "
import pair from ./producer

entry main = f(): void =>
    return void
";

        let producer_program = parse_pipeline(PRODUCER_SOURCE);
        let mut producer_checker = TypeChecker::new();
        producer_checker.set_current_module_path(String::from("./producer"));
        let producer_result = producer_checker.type_check_program(&producer_program);
        assert!(
            producer_result.is_ok(),
            "producer module should type-check: {producer_result:?}"
        );

        let producer_interface = producer_checker
            .module_interface("./producer")
            .expect("producer module interface should be registered");
        assert_eq!(
            producer_interface
                .function_return_labels("pair")
                .map(|labels| labels.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["x", "y"]),
            "exported module interface should preserve ordered return labels"
        );

        let consumer_program = parse_pipeline(CONSUMER_SOURCE);
        let mut consumer_checker = TypeChecker::new();
        consumer_checker.register_module_interface(producer_interface);
        consumer_checker.set_current_module_path(String::from("./consumer"));
        let consumer_result = consumer_checker.type_check_program(&consumer_program);
        assert!(
            consumer_result.is_ok(),
            "consumer module should type-check with imported label metadata: {consumer_result:?}"
        );
        assert_eq!(
            consumer_checker
                .function_return_labels("pair")
                .map(|labels| labels.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["x", "y"]),
            "imported caller should see the callee's ordered return labels"
        );
    }

    #[test]
    fn test_multi_return_labels_do_not_affect_underlying_function_type_identity() {
        const FIRST_SOURCE: &str = "
##
    Description: First labeled pair shape.
##
public first_pair = f(): x: int64, y: int64 =>
    return x: 1, y: 2
entry main = f(): void =>
    return void
";
        const SECOND_SOURCE: &str = "
##
    Description: Second labeled pair shape.
##
public second_pair = f(): left: int64, right: int64 =>
    return left: 1, right: 2
entry main = f(): void =>
    return void
";

        let first_program = parse_pipeline(FIRST_SOURCE);
        let second_program = parse_pipeline(SECOND_SOURCE);
        let mut first_checker = TypeChecker::new();
        let mut second_checker = TypeChecker::new();
        first_checker.set_current_module_path(String::from("./first"));
        second_checker.set_current_module_path(String::from("./second"));
        assert!(first_checker.type_check_program(&first_program).is_ok());
        assert!(second_checker.type_check_program(&second_program).is_ok());

        let first_interface = first_checker
            .module_interface("./first")
            .expect("first module interface should exist");
        let second_interface = second_checker
            .module_interface("./second")
            .expect("second module interface should exist");
        let first_symbol = first_interface
            .exports
            .get("first_pair")
            .expect("first export should exist");
        let second_symbol = second_interface
            .exports
            .get("second_pair")
            .expect("second export should exist");

        assert_eq!(
            first_symbol.core_type, second_symbol.core_type,
            "different return labels should preserve the same underlying ordered function type"
        );
        assert_eq!(
            first_interface
                .function_return_labels("first_pair")
                .map(|labels| labels.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["x", "y"])
        );
        assert_eq!(
            second_interface
                .function_return_labels("second_pair")
                .map(|labels| labels.iter().map(String::as_str).collect::<Vec<_>>()),
            Some(vec!["left", "right"])
        );
    }

    #[test]
    fn test_cross_module_function_call_type_mismatch_is_reported() {
        const SOURCE: &str = "
import to_int from ./conversions

entry main = f(): int32 => {
    return to_int('hello')
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();

        let mut conversions = ModuleInterface::new(String::from("./conversions"));
        let register_result = conversions.register_symbol(symbol(
            "to_int",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Int32],
                return_types: vec![CoreType::Int32],
                error_types: Vec::new(),
            },
            Visibility::Public,
        ));
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(conversions);

        let result = checker.type_check_program(&program);
        let errors = result.expect_err("wrong imported argument type must fail");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::TypeMismatch { .. })),
            "expected TypeMismatch error, got: {errors:?}",
        );
    }

    #[test]
    fn test_standard_testing_terminal_import_is_rejected_in_production_mode() {
        const SOURCE: &str = "
import type TerminalTestAuthority from 'standard.testing.terminal'

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("test-only terminal imports must fail in production mode");
        assert!(
            errors.iter().any(|error| matches!(
                *error,
                TypeError::ModuleUnavailable { ref module, ref reason, .. }
                    if module == "standard.testing.terminal" && reason.contains("test-only")
            )),
            "expected test-only ModuleUnavailable diagnostic, got: {errors:?}",
        );
    }

    #[test]
    fn test_standard_testing_terminal_import_resolves_in_test_mode() {
        const SOURCE: &str = "
import type TerminalTestAuthority from 'standard.testing.terminal'

entry main = f(): TerminalTestAuthority =>
    return authority
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_test_only_imports();
        let result = checker.type_check_program(&program);

        assert!(
            result.is_err(),
            "test-mode import should pass availability and continue to body checking",
        );
        let errors = result.expect_err("undefined value should remain the only failure");
        assert!(
            errors
                .iter()
                .all(|error| !matches!(*error, TypeError::ModuleUnavailable { .. })),
            "test-mode import should not report availability errors: {errors:?}",
        );
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::SymbolNotFound { ref name, .. } if name == "authority")),
            "body should reach ordinary symbol checking after import succeeds: {errors:?}",
        );
    }

    #[test]
    fn test_public_terminal_and_chord_modules_remain_future_gated() {
        for (module_path, type_name) in [
            ("standard.terminal", "TerminalSession"),
            ("standard.terminal.chords", "TerminalChordRouter"),
        ] {
            let source = format!(
                "\nimport type {type_name} from '{module_path}'\n\nentry main = f(): void =>\n    return void\n"
            );
            let program = parse_pipeline(&source);
            let mut checker = TypeChecker::new();
            checker.enable_test_only_imports();
            let result = checker.type_check_program(&program);
            let errors = result.expect_err("future public terminal modules must remain gated");
            assert!(
                errors.iter().any(|error| matches!(
                    *error,
                    TypeError::ModuleUnavailable { ref module, ref reason, .. }
                        if module == module_path && reason.contains("public terminal API gate")
                )),
                "expected future-gate ModuleUnavailable diagnostic for {module_path}, got: {errors:?}",
            );
        }
    }

    #[test]
    fn test_selected_terminal_signatures_type_check_under_internal_gate() {
        const SOURCE: &str = "
import terminal_session_options_default, terminal_session_options_with_feature_policy, terminal_session_options_with_resource_limits, terminal_session_options_validate, terminal_session_open_sync, terminal_session_recover_open_sync, terminal_session_recover_close_sync, terminal_recovery_token_kind, terminal_recovery_token_generation, terminal_session_state, terminal_session_capabilities, terminal_capabilities_feature, terminal_capabilities_trusted_paste_framing, terminal_capabilities_color, terminal_session_readiness_source, terminal_session_size_sync, terminal_session_read_event_sync, terminal_session_write_sync, terminal_session_write_diagnostic_sync, terminal_session_flush_sync, terminal_session_set_cursor_visible_sync, terminal_session_set_cursor_shape_sync, terminal_session_pause_sync, terminal_session_resume_sync, terminal_session_close_sync, trusted_terminal_output_from_application_text, safe_terminal_diagnostic_format, safe_terminal_diagnostic_collection_format, terminal_pause_events_length, terminal_pause_events_at, terminal_diagnostic_backend, terminal_diagnostic_operation, terminal_diagnostic_stage, terminal_diagnostic_coordinator_state, terminal_diagnostic_session_state, terminal_diagnostic_os_code, terminal_diagnostic_detail, terminal_diagnostic_retryability, terminal_diagnostic_was_truncated, terminal_diagnostics_length, terminal_diagnostics_at, terminal_diagnostics_retained_count, terminal_diagnostics_omitted_count, terminal_diagnostics_retained_bytes, terminal_diagnostics_omitted_bytes, terminal_diagnostics_was_truncated from 'standard.terminal'
import type TerminalBackend, TerminalCapabilities, TerminalCloseOutcome, TerminalColorCapability, TerminalCoordinatorState, TerminalCursorShape, TerminalDiagnostic, TerminalDiagnosticCollection, TerminalDiagnosticDetail, TerminalDiagnosticRetryability, TerminalDiagnosticSessionState, TerminalDiagnosticStage, TerminalFeatureCapability, TerminalInputEvent, TerminalOperation, TerminalOrdinaryFeature, TerminalOsCode, TerminalPauseEvents, TerminalPauseResult, TerminalRecoveryLedgerKind, TerminalRecoveryToken, TerminalSession, TerminalSessionFeaturePolicy, TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionOptionsError, TerminalSessionReadError, TerminalSessionResourceLimits, TerminalSessionRestoreError, TerminalSessionState, TerminalSessionStateError, TerminalSessionWriteError, TerminalSize, TerminalTrustedPasteCapability, TerminalWait, SafeTerminalDiagnosticOutput, TrustedTerminalOutput from 'standard.terminal'

let exercise_selected = f(options: TerminalSessionOptions, policy: TerminalSessionFeaturePolicy, limits: TerminalSessionResourceLimits, recovery_token: TerminalRecoveryToken, session: TerminalSession, capabilities: TerminalCapabilities, feature: TerminalOrdinaryFeature, wait: TerminalWait, cancellation: CancellationToken, output: TrustedTerminalOutput, diagnostic_output: SafeTerminalDiagnosticOutput, shape: TerminalCursorShape, diagnostic: TerminalDiagnostic, diagnostics: TerminalDiagnosticCollection, events: TerminalPauseEvents): void errors AllocationFailureError, IndexOutOfBoundsError, TerminalSessionOpenError, TerminalSessionOptionsError, TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError, TerminalSessionWriteError =>
    let default_options: TerminalSessionOptions = terminal_session_options_default()
    let policy_options: TerminalSessionOptions = propagate terminal_session_options_with_feature_policy(options, policy)
    let limited_options: TerminalSessionOptions = propagate terminal_session_options_with_resource_limits(options, limits)
    let valid_options: TerminalSessionOptions = propagate terminal_session_options_validate(options)
    let opened: TerminalSession = propagate terminal_session_open_sync(options)
    propagate terminal_session_recover_open_sync(recovery_token)
    propagate terminal_session_recover_close_sync(recovery_token)
    let recovery_kind: TerminalRecoveryLedgerKind = terminal_recovery_token_kind(recovery_token)
    let recovery_generation: uint64 = terminal_recovery_token_generation(recovery_token)
    let state: TerminalSessionState = terminal_session_state(session)
    let session_capabilities: TerminalCapabilities = terminal_session_capabilities(session)
    let feature_capability: TerminalFeatureCapability = terminal_capabilities_feature(capabilities, feature)
    let paste_capability: TerminalTrustedPasteCapability = terminal_capabilities_trusted_paste_framing(capabilities)
    let color_capability: TerminalColorCapability = terminal_capabilities_color(capabilities)
    let readiness: SystemReadinessSource = terminal_session_readiness_source(session)
    let size: TerminalSize = propagate terminal_session_size_sync(session)
    let event: TerminalInputEvent = propagate terminal_session_read_event_sync(session, wait, cancellation)
    propagate terminal_session_write_sync(session, output)
    propagate terminal_session_write_diagnostic_sync(session, diagnostic_output)
    propagate terminal_session_flush_sync(session)
    propagate terminal_session_set_cursor_visible_sync(session, true)
    propagate terminal_session_set_cursor_shape_sync(session, shape)
    let pause_result: TerminalPauseResult = propagate terminal_session_pause_sync(session)
    propagate terminal_session_resume_sync(session)
    let close_outcome: TerminalCloseOutcome = propagate terminal_session_close_sync(session)
    let trusted_output: TrustedTerminalOutput = propagate trusted_terminal_output_from_application_text('safe text')
    let formatted_diagnostic: SafeTerminalDiagnosticOutput = propagate safe_terminal_diagnostic_format(diagnostic)
    let formatted_collection: SafeTerminalDiagnosticOutput = propagate safe_terminal_diagnostic_collection_format(diagnostics)
    let pause_length: int64 = terminal_pause_events_length(events)
    let pause_event: TerminalInputEvent = propagate terminal_pause_events_at(events, 0)
    let backend: TerminalBackend = terminal_diagnostic_backend(diagnostic)
    let operation: TerminalOperation = terminal_diagnostic_operation(diagnostic)
    let stage: TerminalDiagnosticStage = terminal_diagnostic_stage(diagnostic)
    let coordinator: TerminalCoordinatorState = terminal_diagnostic_coordinator_state(diagnostic)
    let diagnostic_state: TerminalDiagnosticSessionState = terminal_diagnostic_session_state(diagnostic)
    let os_code: TerminalOsCode = terminal_diagnostic_os_code(diagnostic)
    let detail: TerminalDiagnosticDetail = terminal_diagnostic_detail(diagnostic)
    let retryability: TerminalDiagnosticRetryability = terminal_diagnostic_retryability(diagnostic)
    let diagnostic_truncated: boolean = terminal_diagnostic_was_truncated(diagnostic)
    let diagnostics_length: int64 = terminal_diagnostics_length(diagnostics)
    let diagnostic_at: TerminalDiagnostic = propagate terminal_diagnostics_at(diagnostics, 0)
    let retained_count: uint64 = terminal_diagnostics_retained_count(diagnostics)
    let omitted_count: uint64 = terminal_diagnostics_omitted_count(diagnostics)
    let retained_bytes: uint64 = terminal_diagnostics_retained_bytes(diagnostics)
    let omitted_bytes: uint64 = terminal_diagnostics_omitted_bytes(diagnostics)
    let collection_truncated: boolean = terminal_diagnostics_was_truncated(diagnostics)
    return void

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_terminal_proposal_imports_for_tests();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "selected terminal signatures should type-check under the internal gate: {result:?}",
        );
    }

    #[test]
    fn test_terminal_chord_signatures_type_check_under_internal_gate() {
        const SOURCE: &str = "
import type TerminalCapabilities, TerminalInputEvent, TerminalKeyOccurrence from 'standard.terminal'
import terminal_chord_modifiers, terminal_chord_new, terminal_chord_with_lock_modifier_mask, terminal_chord_sequence_single, terminal_chord_sequence_append, terminal_chord_router_new, terminal_chord_router_register, terminal_chord_router_unregister, terminal_chord_router_replace, terminal_chord_binding_id_ordinal, terminal_chord_router_process, terminal_chord_router_expire_sync, terminal_chord_router_reset, terminal_chord_released_input_length, terminal_chord_released_input_at from 'standard.terminal.chords'
import type TerminalChord, TerminalChordBindingId, TerminalChordKey, TerminalChordModifiers, TerminalChordMutationError, TerminalChordMutationResult, TerminalChordPriority, TerminalChordProcessError, TerminalChordReleasedInput, TerminalChordResetReason, TerminalChordRouter, TerminalChordRouterOutput, TerminalChordRouterPolicy, TerminalChordSequence, TerminalChordTextPolicy, TerminalChordTrigger, TerminalChordValidationError, TerminalLockModifierMask from 'standard.terminal.chords'

let exercise_chords = f(key: TerminalChordKey, trigger: TerminalChordTrigger, chord: TerminalChord, mask: TerminalLockModifierMask, sequence: TerminalChordSequence, capabilities: TerminalCapabilities, policy: TerminalChordRouterPolicy, router: TerminalChordRouter, priority: TerminalChordPriority, text_policy: TerminalChordTextPolicy, binding_id: TerminalChordBindingId, event: TerminalInputEvent, reset_reason: TerminalChordResetReason, released_input: TerminalChordReleasedInput): void errors AllocationFailureError, IndexOutOfBoundsError, TerminalChordMutationError, TerminalChordProcessError, TerminalChordValidationError =>
    let modifiers: TerminalChordModifiers = terminal_chord_modifiers(true, false, true, false)
    let built_chord: TerminalChord = terminal_chord_new(key, modifiers, trigger)
    let masked_chord: TerminalChord = terminal_chord_with_lock_modifier_mask(chord, mask)
    let single_sequence: TerminalChordSequence = propagate terminal_chord_sequence_single(chord)
    let appended_sequence: TerminalChordSequence = propagate terminal_chord_sequence_append(sequence, chord)
    let new_router: TerminalChordRouter = propagate terminal_chord_router_new(capabilities, policy)
    let registered_id: TerminalChordBindingId = propagate terminal_chord_router_register(router, sequence, priority, text_policy)
    let unregistered: TerminalChordMutationResult = propagate terminal_chord_router_unregister(router, binding_id)
    let replaced: TerminalChordMutationResult = propagate terminal_chord_router_replace(router, binding_id, sequence, priority, text_policy)
    let ordinal: uint64 = terminal_chord_binding_id_ordinal(binding_id)
    let processed: TerminalChordRouterOutput = propagate terminal_chord_router_process(router, event)
    let expired: TerminalChordRouterOutput = propagate terminal_chord_router_expire_sync(router)
    let reset: TerminalChordReleasedInput = propagate terminal_chord_router_reset(router, reset_reason)
    let released_length: int64 = terminal_chord_released_input_length(released_input)
    let released_event: TerminalInputEvent = propagate terminal_chord_released_input_at(released_input, 0)
    return void

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_terminal_proposal_imports_for_tests();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "terminal chord signatures should type-check under the internal gate: {result:?}",
        );
    }

    #[test]
    fn test_standard_testing_terminal_signatures_type_check_only_in_test_mode() {
        const SOURCE: &str = "
import type TerminalCapabilities, TerminalCompositionId, TerminalDiagnostic, TerminalDiagnosticCollection, TerminalEventId, TerminalInputEvent, TerminalSessionOptionsError, TerminalTrustedPasteEvidence from 'standard.terminal'
import test_runner_terminal_authority, terminal_test_scenario_new, terminal_test_event_id_new, terminal_test_composition_id_new, terminal_test_trusted_paste_evidence, terminal_test_key_event, terminal_test_text_input_event, terminal_test_composition_started_event, terminal_test_composition_updated_event, terminal_test_composition_ended_event, terminal_test_paste_event, terminal_test_mouse_event, terminal_test_resize_event, terminal_test_focus_gained_event, terminal_test_focus_lost_event, terminal_test_unknown_bytes_event, terminal_test_unknown_native_event, terminal_test_input_reset_event, terminal_test_capabilities, terminal_test_diagnostic, terminal_test_diagnostic_collection, terminal_test_fake_backend, terminal_test_fake_backend_with_fault, terminal_test_bind_fake_backend, terminal_test_activate_backend from 'standard.testing.terminal'
import type TerminalTestAuthority, TerminalTestBackendActivation, TerminalTestColorCapabilitySpec, TerminalTestCompositionEnd, TerminalTestDiagnosticCollectionLimits, TerminalTestDiagnosticSpec, TerminalTestFactoryError, TerminalTestFakeBackend, TerminalTestFakeBackendFault, TerminalTestInputResetReason, TerminalTestKeyOccurrence, TerminalTestLinkedTextPhase, TerminalTestLogicalKey, TerminalTestModifiers, TerminalTestMouseAction, TerminalTestNativeMetadata, TerminalTestOrdinaryCapabilityEntry, TerminalTestPastePhase, TerminalTestScenario, TerminalTestScenarioLimits, TerminalTestTextInputOrigin, TerminalTestTrustedPasteBoundary, TerminalTestTrustedPasteCapabilitySpec, TerminalTestUnknownBytesReason from 'standard.testing.terminal'

let exercise_testing = f(authority: TerminalTestAuthority, limits: TerminalTestScenarioLimits, scenario: TerminalTestScenario, event_id: TerminalEventId, composition_id: TerminalCompositionId, boundary: TerminalTestTrustedPasteBoundary, key: TerminalTestLogicalKey, occurrence: TerminalTestKeyOccurrence, modifiers: TerminalTestModifiers, text_origin: TerminalTestTextInputOrigin, linked_phase: TerminalTestLinkedTextPhase, composition_end: TerminalTestCompositionEnd, paste_phase: TerminalTestPastePhase, evidence: TerminalTrustedPasteEvidence, mouse_action: TerminalTestMouseAction, raw_bytes: Bytes, unknown_reason: TerminalTestUnknownBytesReason, native_metadata: TerminalTestNativeMetadata, reset_reason: TerminalTestInputResetReason, ordinary: TerminalTestOrdinaryCapabilityEntry[], trusted_paste: TerminalTestTrustedPasteCapabilitySpec, color: TerminalTestColorCapabilitySpec, diagnostic_spec: TerminalTestDiagnosticSpec, diagnostics: TerminalDiagnostic[], diagnostic_limits: TerminalTestDiagnosticCollectionLimits, backend: TerminalTestFakeBackend, fault: TerminalTestFakeBackendFault): void errors AllocationFailureError, ConstraintViolationError, TerminalSessionOptionsError, TerminalTestFactoryError =>
    let issued_authority: TerminalTestAuthority = test_runner_terminal_authority()
    let new_scenario: TerminalTestScenario = propagate terminal_test_scenario_new(authority, limits)
    let issued_event_id: TerminalEventId = propagate terminal_test_event_id_new(scenario)
    let issued_composition_id: TerminalCompositionId = propagate terminal_test_composition_id_new(scenario)
    let issued_evidence: TerminalTrustedPasteEvidence = propagate terminal_test_trusted_paste_evidence(scenario, boundary)
    let key_event: TerminalInputEvent = propagate terminal_test_key_event(scenario, event_id, key, occurrence, modifiers)
    let text_event: TerminalInputEvent = propagate terminal_test_text_input_event(scenario, 'text', text_origin, linked_phase)
    let composition_started: TerminalInputEvent = propagate terminal_test_composition_started_event(scenario, composition_id)
    let composition_updated: TerminalInputEvent = propagate terminal_test_composition_updated_event(scenario, composition_id, 'preedit', 0)
    let composition_ended: TerminalInputEvent = propagate terminal_test_composition_ended_event(scenario, composition_id, composition_end)
    let paste_event: TerminalInputEvent = propagate terminal_test_paste_event(scenario, 'paste', paste_phase, evidence)
    let mouse_event: TerminalInputEvent = propagate terminal_test_mouse_event(scenario, mouse_action, modifiers, 1, 1)
    let resize_event: TerminalInputEvent = propagate terminal_test_resize_event(scenario, 80, 24)
    let focus_gained: TerminalInputEvent = propagate terminal_test_focus_gained_event(scenario)
    let focus_lost: TerminalInputEvent = propagate terminal_test_focus_lost_event(scenario)
    let unknown_bytes: TerminalInputEvent = propagate terminal_test_unknown_bytes_event(scenario, raw_bytes, unknown_reason)
    let unknown_native: TerminalInputEvent = propagate terminal_test_unknown_native_event(scenario, native_metadata)
    let reset_event: TerminalInputEvent = propagate terminal_test_input_reset_event(scenario, reset_reason)
    let capabilities: TerminalCapabilities = propagate terminal_test_capabilities(scenario, ordinary, trusted_paste, color)
    let diagnostic: TerminalDiagnostic = propagate terminal_test_diagnostic(scenario, diagnostic_spec)
    let diagnostic_collection: TerminalDiagnosticCollection = propagate terminal_test_diagnostic_collection(authority, diagnostics, diagnostic_limits)
    let fake_backend: TerminalTestFakeBackend = propagate terminal_test_fake_backend(authority)
    let faulted_backend: TerminalTestFakeBackend = propagate terminal_test_fake_backend_with_fault(backend, fault)
    propagate terminal_test_bind_fake_backend(scenario, backend)
    let activation: TerminalTestBackendActivation = propagate terminal_test_activate_backend(scenario)
    return void

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_terminal_proposal_imports_for_tests();
        checker.enable_test_only_imports();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "test-only terminal signatures should type-check only when both gates are enabled: {result:?}",
        );
    }

    #[test]
    fn test_core_prerequisite_signatures_type_check_under_internal_gate() {
        const SOURCE: &str = "
import system_wait_set_new, system_wait_set_register, system_wait_set_remove, system_wait_set_register_owned, system_owned_wait_registration_retarget, system_owned_wait_registration_remove, system_wait_set_wait_sync, cancellation_source_new, cancellation_token, cancellation_request, monotonic_timer_new, monotonic_timer_readiness_source, monotonic_timer_arm, monotonic_timer_disarm, monotonic_timer_generation, monotonic_timer_deadline, monotonic_clock_now, process_control_source_new, process_control_readiness_source, process_control_poll, process_control_acknowledge_suspend, process_control_resume_application, error_cause, error_suppressed_length, error_suppressed_at, error_attachment_truncation, error_attachment_truncation_cause_depth, error_attachment_truncation_suppressed_count, error_attachment_truncation_bytes from 'standard.system'

let exercise_core_prerequisites = f(wait_set: SystemWaitSet, source: SystemReadinessSource, registration: SystemWaitRegistration, owned_registration: SystemOwnedWaitRegistration, cancellation_source: CancellationSource, token: CancellationToken, timer: MonotonicTimer, deadline: MonotonicDeadline, process_source: ProcessControlSource, error_value: Error, truncation: ErrorAttachmentTruncation): void errors AllocationFailureError, ErrorAttachmentAbsentError, IndexOutOfBoundsError, MonotonicTimerError, MonotonicTimerNotArmedError, ProcessControlAcknowledgementError, ProcessControlError, ProcessControlResumeError, ProcessControlUnavailableError, SystemWaitSetError =>
    let new_wait_set: SystemWaitSet = propagate system_wait_set_new()
    let ordinary_registration: SystemWaitRegistration = propagate system_wait_set_register(wait_set, source)
    propagate system_wait_set_remove(wait_set, registration)
    let new_owned_registration: SystemOwnedWaitRegistration = propagate system_wait_set_register_owned(wait_set, source)
    propagate system_owned_wait_registration_retarget(owned_registration, source)
    propagate system_owned_wait_registration_remove(owned_registration)
    let wake: SystemWaitWake = propagate system_wait_set_wait_sync(wait_set, token)
    let new_cancellation_source: CancellationSource = propagate cancellation_source_new()
    let issued_token: CancellationToken = cancellation_token(cancellation_source)
    cancellation_request(cancellation_source)
    let new_timer: MonotonicTimer = propagate monotonic_timer_new()
    let timer_source: SystemReadinessSource = monotonic_timer_readiness_source(timer)
    let arm_generation: uint64 = propagate monotonic_timer_arm(timer, deadline)
    let disarm_generation: uint64 = propagate monotonic_timer_disarm(timer)
    let current_generation: uint64 = monotonic_timer_generation(timer)
    let current_deadline: MonotonicDeadline = propagate monotonic_timer_deadline(timer)
    let now: MonotonicDeadline = monotonic_clock_now()
    let new_process_source: ProcessControlSource = propagate process_control_source_new()
    let process_readiness: SystemReadinessSource = process_control_readiness_source(process_source)
    let poll_result: ProcessControlPollResult = propagate process_control_poll(process_source)
    propagate process_control_acknowledge_suspend(process_source, 1)
    propagate process_control_resume_application(process_source, 1)
    let cause: Error = propagate error_cause(error_value)
    let suppressed_length: int64 = error_suppressed_length(error_value)
    let suppressed: Error = propagate error_suppressed_at(error_value, 0)
    let observed_truncation: ErrorAttachmentTruncation = error_attachment_truncation(error_value)
    let cause_depth_truncated: boolean = error_attachment_truncation_cause_depth(truncation)
    let suppressed_count_truncated: boolean = error_attachment_truncation_suppressed_count(truncation)
    let bytes_truncated: boolean = error_attachment_truncation_bytes(truncation)
    return void

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_terminal_proposal_imports_for_tests();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "core prerequisite signatures should type-check under the internal gate: {result:?}",
        );
    }

    #[test]
    fn test_core_prerequisite_surface_is_future_gated_in_production_mode() {
        const SOURCE: &str = "
import system_wait_set_new from 'standard.system'

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let errors = checker
            .type_check_program(&program)
            .expect_err("core prerequisite imports must remain future-gated");
        assert!(
            errors.iter().any(|error| matches!(
                *error,
                TypeError::ModuleUnavailable { ref module, ref reason, .. }
                    if module == "standard.system" && reason.contains("gated")
            )),
            "expected future-gated standard.system rejection, got: {errors:?}",
        );
    }

    #[test]
    fn test_terminal_session_write_rejects_raw_string_output() {
        const SOURCE: &str = "
import terminal_session_write_sync from 'standard.terminal'
import type TerminalSession, TerminalSessionStateError, TerminalSessionWriteError from 'standard.terminal'

let bad_write = f(session: TerminalSession): void errors TerminalSessionStateError, TerminalSessionWriteError =>
    propagate terminal_session_write_sync(session, 'raw string')

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_terminal_proposal_imports_for_tests();
        let errors = checker
            .type_check_program(&program)
            .expect_err("raw string terminal writes must be rejected");
        assert!(
            errors.iter().any(|error| matches!(
                *error,
                TypeError::TypeMismatch { ref expected, ref found, .. }
                    if expected == "TrustedTerminalOutput" && found == "string"
            )),
            "expected TrustedTerminalOutput vs string mismatch, got: {errors:?}",
        );
    }

    #[test]
    fn test_terminal_constructor_visibility_rejects_sealed_runtime_values() {
        for (type_name, constructor_expr) in [
            ("TerminalSession", "new TerminalSession"),
            ("TerminalRecoveryToken", "new TerminalRecoveryToken"),
            ("TerminalInputEvent", "new TerminalInputEvent.TimedOut"),
            ("TerminalDiagnostic", "new TerminalDiagnostic"),
            ("TerminalSessionState", "new TerminalSessionState.Active"),
        ] {
            let source = format!(
                "\nimport type {type_name} from 'standard.terminal'\n\nlet sealed = f(): {type_name} =>\n    return {constructor_expr}\n\nentry main = f(): void =>\n    return void\n"
            );
            let program = parse_pipeline(&source);
            let mut checker = TypeChecker::new();
            checker.enable_terminal_proposal_imports_for_tests();
            let errors = checker
                .type_check_program(&program)
                .expect_err("sealed terminal constructors must be rejected");
            assert!(
                errors.iter().any(|error| matches!(
                    *error,
                    TypeError::ConstructorUnavailable { type_name: ref rejected, ref visibility, .. }
                        if rejected == type_name && visibility == "runtime"
                )),
                "expected runtime constructor rejection for {type_name}, got: {errors:?}",
            );
        }
    }

    #[test]
    fn test_terminal_constructor_visibility_rejects_test_runner_authority() {
        const SOURCE: &str = "
import type TerminalTestAuthority from 'standard.testing.terminal'

let sealed = f(): TerminalTestAuthority =>
    return new TerminalTestAuthority

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        checker.enable_test_only_imports();
        let errors = checker
            .type_check_program(&program)
            .expect_err("test-runner-only authority construction must be rejected");
        assert!(
            errors.iter().any(|error| matches!(
                *error,
                TypeError::ConstructorUnavailable { ref type_name, ref visibility, .. }
                    if type_name == "TerminalTestAuthority" && visibility == "test_runner"
            )),
            "expected test-runner constructor rejection, got: {errors:?}",
        );
    }
}
