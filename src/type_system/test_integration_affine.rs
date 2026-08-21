//! Integration tests for affine resources and second-class borrow checking.

extern crate alloc;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use alloc::{format, string::String};

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

/// Parse a source snippet through the normal lexer/parser pipeline.
fn parse_pipeline(source: &str) -> Program {
    let source_with_docs = with_required_function_docs(source);
    let lexer = Lexer::new(&source_with_docs);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "affine source should lex without errors: {:?}",
        lex_errors.errors,
    );

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "affine source should parse without errors: {:?}",
        parse_errors.errors,
    );

    program_opt.map_or_else(
        || Program {
            declarations: Vec::new(),
            span: Span::single(Position::start()),
            id: crate::ast::NodeId(0),
        },
        |program| program,
    )
}

/// Type-check source with terminal proposal imports enabled for focused tests.
fn type_check_terminal_source(source: &str) -> Result<(), Vec<TypeError>> {
    let program = parse_pipeline(source);
    let mut checker = TypeChecker::new();
    checker.enable_terminal_proposal_imports_for_tests();
    checker.type_check_program(&program)
}

/// Type-check source without terminal proposal imports.
fn type_check_plain_source(source: &str) -> Result<(), Vec<TypeError>> {
    let program = parse_pipeline(source);
    let mut checker = TypeChecker::new();
    checker.type_check_program(&program)
}

/// Assert that at least one type error reason contains every requested snippet.
fn assert_error_reasons_contain(errors: &[TypeError], expected_snippets: &[&str]) {
    for expected in expected_snippets {
        assert!(
            errors.iter().any(|error| {
                matches!(
                    *error,
                    TypeError::ConstraintSolvingFailed { ref reason, .. } if reason.contains(expected)
                )
            }),
            "expected ownership diagnostic containing '{expected}', got: {errors:?}",
        );
    }
}

#[test]
fn affine_resource_declaration_makes_local_type_noncopyable() {
    const SOURCE: &str = "
public compiler_registered affine resource type LocalResource

let consume = f(resource: LocalResource): void =>
    return void

let double_move = f(resource: LocalResource): void =>
    consume(resource)
    consume(resource)
    return void

entry main = f(): void =>
    return void
";

    let errors = type_check_plain_source(SOURCE)
        .expect_err("local compiler_registered affine resource should reject double move");
    assert_error_reasons_contain(&errors, &["affine resource 'resource'", "already moved"]);
}

#[test]
fn terminal_session_ref_and_mutable_ref_calls_type_check() {
    const SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let inspect = f(ref session: TerminalSession): void =>
    return void

let mutate = f(mutable ref session: TerminalSession): void =>
    return void

let exercise = f(session: TerminalSession): void =>
    inspect(ref session)
    mutate(mutable ref session)
    return void

entry main = f(): void =>
    return void
";

    let result = type_check_terminal_source(SOURCE);
    assert!(
        result.is_ok(),
        "valid ref/mutable ref TerminalSession calls should type-check: {result:?}",
    );
}

#[test]
fn affine_owner_escape_cases_are_rejected() {
    const SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

type SessionBox:
    value: TerminalSession

let consume = f(session: TerminalSession): void =>
    return void

let inspect = f(ref session: TerminalSession): void =>
    return void

let copy_owner = f(session: TerminalSession): void =>
    let copied: TerminalSession = session
    return void

let use_after_move = f(session: TerminalSession): void =>
    consume(session)
    inspect(ref session)
    return void

let store_array = f(session: TerminalSession): void =>
    let stored: TerminalSession[] = [session]
    return void

let store_field = f(session: TerminalSession): void =>
    let boxed: SessionBox = new SessionBox:
        value: session
    return void

let return_owner = f(session: TerminalSession): TerminalSession =>
    return session

let capture_owner = f(session: TerminalSession): void =>
    let closure = f(): void => inspect(ref session)
    return void

let assign_owner = f(session: TerminalSession): void =>
    let mutable other: int64 = 0
    other = session
    return void

entry main = f(): void =>
    return void
";

    let errors = type_check_terminal_source(SOURCE)
        .expect_err("affine owner escape cases should be rejected");
    assert_error_reasons_contain(
        &errors,
        &[
            "escape through a let binding",
            "already moved",
            "escape through an array literal",
            "escape through a constructor field",
            "escape through return",
            "captured by a lambda",
            "escape through assignment",
        ],
    );
}

#[test]
fn second_class_borrow_escape_cases_are_rejected() {
    const SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let inspect = f(ref session: TerminalSession): void =>
    return void

let store_borrow = f(ref session: TerminalSession): void =>
    let stored = session
    return void

let return_borrow = f(ref session: TerminalSession): TerminalSession =>
    return session

let capture_borrow = f(ref session: TerminalSession): void =>
    let closure = f(): void => inspect(ref session)
    return void

let break_borrow = f(ref session: TerminalSession): void =>
    loop => { break escaped: session }
    return void

let continue_borrow = f(ref session: TerminalSession): void =>
    loop => { continue escaped: session }
    return void

let borrow_value_stored = f(session: TerminalSession): void =>
    let stored = ref session
    return void

entry main = f(): void =>
    return void
";

    let errors = type_check_terminal_source(SOURCE)
        .expect_err("second-class borrow escape cases should be rejected");
    assert_error_reasons_contain(
        &errors,
        &[
            "second-class borrow 'session' cannot escape through a let binding",
            "second-class borrow 'session' cannot escape through return",
            "captured by a lambda",
            "second-class borrow 'session' cannot escape through break",
            "second-class borrow 'session' cannot escape through continue",
            "valid only as direct call arguments",
        ],
    );
}

#[test]
fn borrow_argument_mode_mismatches_are_rejected() {
    const SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let inspect = f(ref session: TerminalSession): void =>
    return void

let mutate = f(mutable ref session: TerminalSession): void =>
    return void

let missing_ref = f(session: TerminalSession): void =>
    inspect(session)
    return void

let wrong_ref = f(session: TerminalSession): void =>
    mutate(ref session)
    return void

entry main = f(): void =>
    return void
";

    let errors = type_check_terminal_source(SOURCE)
        .expect_err("borrowed parameters should require matching call-site borrow modes");
    assert_error_reasons_contain(
        &errors,
        &[
            "parameter requires a 'ref' call-site borrow argument",
            "borrow argument mismatch: expected 'mutable ref', found 'ref'",
        ],
    );
}

#[test]
fn inferred_terminal_proposal_owners_are_affine_without_type_imports() {
    const SELECTED_SOURCE: &str = "
import terminal_session_open_sync from 'standard.terminal'
import type TerminalSessionOpenError, TerminalSessionOptions from 'standard.terminal'

let copy_opened = f(options: TerminalSessionOptions): void errors TerminalSessionOpenError =>
    let session = propagate terminal_session_open_sync(options)
    let copied = session
    return void

entry main = f(): void =>
    return void
";
    const CORE_SOURCE: &str = "
import cancellation_source_new from 'standard.system'

let copy_source = f(): void errors AllocationFailureError =>
    let source = propagate cancellation_source_new()
    let copied = source
    return void

entry main = f(): void =>
    return void
";
    const CHORD_SOURCE: &str = "
import terminal_chord_router_new from 'standard.terminal.chords'
import type TerminalCapabilities from 'standard.terminal'
import type TerminalChordRouterPolicy, TerminalChordValidationError from 'standard.terminal.chords'

let copy_router = f(capabilities: TerminalCapabilities, policy: TerminalChordRouterPolicy): void errors AllocationFailureError, TerminalChordValidationError =>
    let router = propagate terminal_chord_router_new(capabilities, policy)
    let copied = router
    return void

entry main = f(): void =>
    return void
";
    const TESTING_SOURCE: &str = "
import terminal_test_activate_backend from 'standard.testing.terminal'
import type TerminalTestFactoryError, TerminalTestScenario from 'standard.testing.terminal'

let copy_activation = f(scenario: TerminalTestScenario): void errors TerminalTestFactoryError =>
    let activation = propagate terminal_test_activate_backend(mutable ref scenario)
    let copied = activation
    return void

entry main = f(): void =>
    return void
";

    for source in [SELECTED_SOURCE, CORE_SOURCE, CHORD_SOURCE] {
        let errors = type_check_terminal_source(source)
            .expect_err("inferred proposal owner values should remain affine");
        assert_error_reasons_contain(&errors, &["escape through a let binding"]);
    }

    let program = parse_pipeline(TESTING_SOURCE);
    let mut checker = TypeChecker::new();
    checker.enable_terminal_proposal_imports_for_tests();
    checker.enable_test_only_imports();
    let errors = checker
        .type_check_program(&program)
        .expect_err("inferred test-only activation should remain affine");
    assert_error_reasons_contain(&errors, &["escape through a let binding"]);
}

#[test]
fn local_let_bound_lambdas_require_matching_borrow_syntax() {
    const VALID_SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let exercise = f(session: TerminalSession): void =>
    let inspect = f(ref borrowed: TerminalSession): void => { return void }
    inspect(ref session)
    return void

entry main = f(): void =>
    return void
";
    const MISSING_REF_SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let exercise = f(session: TerminalSession): void =>
    let inspect = f(ref borrowed: TerminalSession): void => { return void }
    inspect(session)
    return void

entry main = f(): void =>
    return void
";
    const WRONG_MUTABLE_MODE_SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let exercise = f(session: TerminalSession): void =>
    let mutate = f(mutable ref borrowed: TerminalSession): void => { return void }
    mutate(ref session)
    return void

entry main = f(): void =>
    return void
";

    let valid_result = type_check_terminal_source(VALID_SOURCE);
    assert!(
        valid_result.is_ok(),
        "local let-bound lambda should accept explicit matching borrow syntax: {valid_result:?}",
    );

    let missing_errors = type_check_terminal_source(MISSING_REF_SOURCE)
        .expect_err("local let-bound lambda should require explicit ref syntax");
    assert_error_reasons_contain(
        &missing_errors,
        &["parameter requires a 'ref' call-site borrow argument"],
    );

    let wrong_mode_errors = type_check_terminal_source(WRONG_MUTABLE_MODE_SOURCE)
        .expect_err("local let-bound lambda should reject wrong borrow mode");
    assert_error_reasons_contain(
        &wrong_mode_errors,
        &["borrow argument mismatch: expected 'mutable ref', found 'ref'"],
    );
}

#[test]
fn function_borrow_metadata_shadows_and_restores_with_let_bindings() {
    const SOURCE: &str = "
import type TerminalSession from 'standard.terminal'

let exercise = f(session: TerminalSession): void =>
    let inspect = f(ref borrowed: TerminalSession): void => { return void }
    inspect(ref session)
    {
        let inspect = f(value: int64): void => { return void }
        inspect(1)
    }
    inspect(ref session)
    return void

entry main = f(): void =>
    return void
";

    let result = type_check_terminal_source(SOURCE);
    assert!(
        result.is_ok(),
        "inner non-borrow lambda should clear stale metadata and scope exit should restore outer metadata: {result:?}",
    );
}

#[test]
fn using_terminal_session_requires_acquisition_body_and_cleanup_errors() {
    const VALID_SOURCE: &str = "
import terminal_session_open_sync, terminal_session_size_sync from 'standard.terminal'
import type TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError from 'standard.terminal'

let run = f(options: TerminalSessionOptions): void errors TerminalSessionOpenError, TerminalSessionReadError, TerminalSessionRestoreError, TerminalSessionStateError =>
    using session = propagate terminal_session_open_sync(options):
        let size = propagate terminal_session_size_sync(ref session)
        return void

entry main = f(): void =>
    return void
";
    const MISSING_ACQUISITION_ERROR: &str = "
import terminal_session_open_sync from 'standard.terminal'
import type TerminalSessionOptions, TerminalSessionRestoreError from 'standard.terminal'

let run = f(options: TerminalSessionOptions): void errors TerminalSessionRestoreError =>
    using session = propagate terminal_session_open_sync(options):
        return void

entry main = f(): void =>
    return void
";
    const MISSING_BODY_ERROR: &str = "
import terminal_session_open_sync, terminal_session_size_sync from 'standard.terminal'
import type TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionRestoreError from 'standard.terminal'

let run = f(options: TerminalSessionOptions): void errors TerminalSessionOpenError, TerminalSessionRestoreError =>
    using session = propagate terminal_session_open_sync(options):
        let size = propagate terminal_session_size_sync(ref session)
        return void

entry main = f(): void =>
    return void
";
    const MISSING_CLEANUP_ERROR: &str = "
import terminal_session_open_sync from 'standard.terminal'
import type TerminalSessionOpenError, TerminalSessionOptions from 'standard.terminal'

let run = f(options: TerminalSessionOptions): void errors TerminalSessionOpenError =>
    using session = propagate terminal_session_open_sync(options):
        return void

entry main = f(): void =>
    return void
";

    assert!(
        type_check_terminal_source(VALID_SOURCE).is_ok(),
        "using must accept acquisition, body, and cleanup errors when all are declared",
    );
    for (source, expected_error) in [
        (MISSING_ACQUISITION_ERROR, "TerminalSessionOpenError"),
        (MISSING_BODY_ERROR, "TerminalSessionReadError"),
        (MISSING_CLEANUP_ERROR, "TerminalSessionRestoreError"),
    ] {
        let errors = type_check_terminal_source(source)
            .expect_err("omitting any using error family should be rejected");
        let rendered_errors = format!("{errors:?}");
        assert!(
            rendered_errors.contains(expected_error),
            "expected using diagnostic to mention {expected_error}, got {errors:?}",
        );
    }
}

#[test]
fn using_infallible_cleanup_resources_need_no_extra_errors() {
    const SOURCE: &str = "
import system_wait_set_new from 'standard.system'

let run = f(): void errors AllocationFailureError =>
    using wait_set = propagate system_wait_set_new():
        return void

entry main = f(): void =>
    return void
";

    let result = type_check_terminal_source(SOURCE);
    assert!(
        result.is_ok(),
        "infallible compiler-only cleanup must not require an extra errors clause: {result:?}",
    );
}

#[test]
fn using_rejects_affine_resource_without_cleanup_registration() {
    const SOURCE: &str = "
public compiler_registered affine resource type LocalResource

let run = f(resource: LocalResource): void =>
    using local = resource:
        return void

entry main = f(): void =>
    return void
";

    let errors = type_check_plain_source(SOURCE)
        .expect_err("using a local affine resource without cleanup metadata should be rejected");
    assert_error_reasons_contain(&errors, &["no declared using cleanup registration"]);
}

#[test]
fn using_rejects_owner_escape_through_scope_exit_values() {
    const SOURCE: &str = "
import terminal_session_open_sync from 'standard.terminal'
import type TerminalSession, TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionRestoreError from 'standard.terminal'

let return_session = f(options: TerminalSessionOptions): TerminalSession errors TerminalSessionOpenError, TerminalSessionRestoreError =>
    using session = propagate terminal_session_open_sync(options):
        return session

entry main = f(): void =>
    return void
";

    let errors = type_check_terminal_source(SOURCE)
        .expect_err("using owner must not escape before cleanup or registered transfer");
    assert_error_reasons_contain(&errors, &["escape through return"]);
}

#[test]
fn using_explicit_close_keeps_binding_inspectable() {
    const SOURCE: &str = "
import terminal_session_close_sync, terminal_session_open_sync, terminal_session_state from 'standard.terminal'
import type TerminalSessionOpenError, TerminalSessionOptions, TerminalSessionRestoreError from 'standard.terminal'

let close_then_inspect = f(options: TerminalSessionOptions): void errors TerminalSessionOpenError, TerminalSessionRestoreError =>
    using session = propagate terminal_session_open_sync(options):
        let outcome = propagate terminal_session_close_sync(mutable ref session)
        let state = terminal_session_state(ref session)
        return void

entry main = f(): void =>
    return void
";

    let result = type_check_terminal_source(SOURCE);
    assert!(
        result.is_ok(),
        "successful explicit close should consume cleanup obligation without moving the inspectable binding: {result:?}",
    );
}
