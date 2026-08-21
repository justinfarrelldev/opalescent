use super::*;
use crate::ast::{Expr, LiteralValue, Program};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::affine_aggregates::{
    AffineAggregateSlotSpec, terminal_aggregate_fixture_spec,
};

/// Return a deterministic test span.
fn test_span() -> Span {
    Span::single(Position::start())
}

/// Build a node identifier for hand-authored test AST.
fn node_id(value: usize) -> crate::ast::NodeId {
    crate::ast::NodeId(value)
}

/// Build an identifier expression.
fn ident(id: usize, name: &str) -> Expr {
    Expr::Identifier {
        name: name.to_owned(),
        span: test_span(),
        id: node_id(id),
    }
}

/// Build a mutable-ref borrow argument expression.
fn mutable_ref(id: usize, target: Expr) -> Expr {
    Expr::BorrowArgument {
        target: Box::new(target),
        borrow_kind: BorrowKind::MutableRef,
        span: test_span(),
        id: node_id(id),
    }
}

/// Build an immutable-ref borrow argument expression.
fn immutable_ref(id: usize, target: Expr) -> Expr {
    Expr::BorrowArgument {
        target: Box::new(target),
        borrow_kind: BorrowKind::Ref,
        span: test_span(),
        id: node_id(id),
    }
}

/// Parse a source snippet through the normal lexer/parser pipeline.
fn parse_source(source: &str) -> Program {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "aggregate source should lex without errors: {:?}",
        lex_errors.errors,
    );
    let parser = Parser::new(tokens);
    let (program, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "aggregate source should parse without errors: {:?}",
        parse_errors.errors,
    );
    program.expect("parser should return a program for valid aggregate source")
}

/// Type-check a source snippet with the synthetic aggregate fixture registered.
fn type_check_fixture_source(source: &str) -> Result<(), Vec<TypeError>> {
    let program = parse_source(source);
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    checker.type_check_program(&program)
}

/// Build a call expression.
fn call(id: usize, operation: &str, args: Vec<Expr>) -> Expr {
    Expr::Call {
        callee: Box::new(ident(id.saturating_add(1), operation)),
        generic_args: None,
        args,
        span: test_span(),
        id: node_id(id),
    }
}

/// Build a member chain expression.
fn member(object: Expr, id: usize, name: &str) -> Expr {
    Expr::Member {
        object: Box::new(object),
        member: name.to_owned(),
        span: test_span(),
        id: node_id(id),
    }
}

/// Build a call statement.
fn call_statement(id: usize, operation: &str, args: Vec<Expr>) -> Stmt {
    Stmt::Expression {
        expr: call(id, operation, args),
        span: test_span(),
        id: node_id(id.saturating_add(10)),
    }
}

/// Build a propagate statement wrapping a call.
fn propagate_statement(id: usize, operation: &str, args: Vec<Expr>) -> Stmt {
    Stmt::Expression {
        expr: Expr::Propagate {
            call: Box::new(call(id, operation, args)),
            cause: None,
            span: test_span(),
            id: node_id(id.saturating_add(20)),
        },
        span: test_span(),
        id: node_id(id.saturating_add(30)),
    }
}

/// Seed an aggregate root binding.
fn seed_runtime(checker: &mut TypeChecker) {
    checker.symbol_table.register(SymbolInfo {
        name: "runtime".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: nominal_type("TerminalAggregateFixture"),
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: true,
        is_mutable: true,
        read_count: 0,
        is_pure: false,
    });
}

#[test]
fn terminal_aggregate_duplicate_obligations_are_rejected() {
    let mut spec = terminal_aggregate_fixture_spec();
    spec.slots.push(AffineAggregateSlotSpec::owning(
        "first",
        "TerminalAggregateMember",
    ));
    let error = validate_affine_aggregate_spec(&spec)
        .expect_err("duplicate aggregate obligations must fail validation");
    assert!(
        error.contains("duplicate obligation"),
        "diagnostic should mention duplicate obligations: {error}"
    );
}

#[test]
fn terminal_aggregate_missing_cleanup_is_rejected() {
    let mut spec = terminal_aggregate_fixture_spec();
    spec.cleanup_order = vec!["first".to_owned()];
    let error = validate_affine_aggregate_spec(&spec)
        .expect_err("missing aggregate cleanup must fail validation");
    assert!(
        error.contains("missing cleanup"),
        "diagnostic should mention missing cleanup: {error}"
    );
}

#[test]
fn terminal_aggregate_constructor_accepts_fresh_provisional_members() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    let field_expr = Expr::Propagate {
        call: Box::new(call(1_000, "terminal_aggregate_member_new", vec![])),
        cause: None,
        span: test_span(),
        id: node_id(1_001),
    };
    let accepted = checker
        .allow_affine_aggregate_constructor_field(
            "TerminalAggregateFixture",
            "first",
            &field_expr,
            &nominal_type("TerminalAggregateMember"),
        )
        .expect("fresh aggregate member acquisition should be accepted");
    assert!(
        accepted,
        "aggregate field should be handled by transaction metadata"
    );
}

#[test]
fn terminal_aggregate_exported_member_owner_is_rejected() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    let member_expr = member(ident(2_000, "runtime"), 2_001, "first");
    let error = checker
        .check_value_escape(
            &member_expr,
            &nominal_type("TerminalAggregateMember"),
            "escape through return",
            false,
        )
        .expect_err("exporting a member owner must fail");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected aggregate member owner escape diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("escape through return"),
        "diagnostic should mention exported member owner context: {reason}"
    );
}

#[test]
fn terminal_aggregate_retarget_requires_immediate_matching_commit() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    let target = member(ident(3_000, "runtime"), 3_001, "first");
    let retarget = propagate_statement(
        3_010,
        "terminal_aggregate_retarget_member",
        vec![mutable_ref(3_009, ident(3_008, "runtime")), target],
    );
    checker
        .update_affine_aggregate_statement_after(&retarget)
        .expect("retarget should create pending commit");
    let interposed = call_statement(
        3_020,
        "terminal_aggregate_observable_probe",
        vec![Expr::Literal {
            value: LiteralValue::Integer(1),
            span: test_span(),
            id: node_id(3_021),
        }],
    );
    let error = checker
        .check_affine_aggregate_statement_before(&interposed)
        .expect_err("observable interposition after retarget must fail");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected interposition diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("cannot interpose") && reason.contains("matching"),
        "diagnostic should require matching commit: {reason}"
    );
}

#[test]
fn terminal_aggregate_declared_commit_clears_retarget_barrier() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    let target = member(ident(4_000, "runtime"), 4_001, "first");
    let retarget = propagate_statement(
        4_010,
        "terminal_aggregate_retarget_member",
        vec![mutable_ref(4_009, ident(4_008, "runtime")), target],
    );
    checker
        .update_affine_aggregate_statement_after(&retarget)
        .expect("retarget should create pending commit");
    let commit = call_statement(
        4_020,
        "terminal_aggregate_commit_candidate",
        vec![mutable_ref(4_021, ident(4_022, "runtime"))],
    );
    checker
        .check_affine_aggregate_statement_before(&commit)
        .expect("matching commit may immediately follow retarget");
    checker
        .update_affine_aggregate_statement_after(&commit)
        .expect("matching commit should clear pending transition");
    checker
        .finish_affine_aggregate_statement_sequence()
        .expect("no pending aggregate commit should remain");
}

#[test]
fn terminal_aggregate_wrong_root_commit_does_not_clear_retarget_barrier() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    checker.symbol_table.register(SymbolInfo {
        name: "other_runtime".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: nominal_type("TerminalAggregateFixture"),
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: true,
        is_mutable: true,
        read_count: 0,
        is_pure: false,
    });
    let target = member(ident(4_100, "runtime"), 4_101, "first");
    let retarget = propagate_statement(
        4_110,
        "terminal_aggregate_retarget_member",
        vec![mutable_ref(4_109, ident(4_108, "runtime")), target],
    );
    checker
        .update_affine_aggregate_statement_after(&retarget)
        .expect("retarget should create pending commit");
    let wrong_root_commit = call_statement(
        4_120,
        "terminal_aggregate_commit_candidate",
        vec![mutable_ref(4_121, ident(4_122, "other_runtime"))],
    );
    let error = checker
        .check_affine_aggregate_statement_before(&wrong_root_commit)
        .expect_err("wrong-root commit must not clear the retarget barrier");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected wrong-root interposition diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("matching") && reason.contains("runtime"),
        "diagnostic should require matching root commit: {reason}"
    );
}

#[test]
fn terminal_aggregate_commit_requires_mutable_aggregate_argument() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    let target = member(ident(4_200, "runtime"), 4_201, "first");
    let retarget = propagate_statement(
        4_210,
        "terminal_aggregate_retarget_member",
        vec![mutable_ref(4_209, ident(4_208, "runtime")), target],
    );
    checker
        .update_affine_aggregate_statement_after(&retarget)
        .expect("retarget should create pending commit");
    let non_mutable_commit = call_statement(
        4_220,
        "terminal_aggregate_commit_candidate",
        vec![immutable_ref(4_221, ident(4_222, "runtime"))],
    );
    let error = checker
        .check_affine_aggregate_statement_before(&non_mutable_commit)
        .expect_err("commit without mutable aggregate argument must not clear the barrier");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected non-mutable commit diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("matching") && reason.contains("runtime"),
        "diagnostic should require mutable matching root commit: {reason}"
    );
}

#[test]
fn terminal_aggregate_late_matching_argument_does_not_clear_retarget_barrier() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    checker.symbol_table.register(SymbolInfo {
        name: "other_runtime".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: nominal_type("TerminalAggregateFixture"),
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: true,
        is_mutable: true,
        read_count: 0,
        is_pure: false,
    });
    let target = member(ident(4_300, "runtime"), 4_301, "first");
    let retarget = propagate_statement(
        4_310,
        "terminal_aggregate_retarget_member",
        vec![mutable_ref(4_309, ident(4_308, "runtime")), target],
    );
    checker
        .update_affine_aggregate_statement_after(&retarget)
        .expect("retarget should create pending commit");
    let wrong_first_arg_commit = call_statement(
        4_320,
        "terminal_aggregate_commit_candidate",
        vec![
            mutable_ref(4_321, ident(4_322, "other_runtime")),
            mutable_ref(4_323, ident(4_324, "runtime")),
        ],
    );
    let error = checker
        .check_affine_aggregate_statement_before(&wrong_first_arg_commit)
        .expect_err("late matching argument must not satisfy aggregate commit");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected late-argument interposition diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("matching") && reason.contains("runtime"),
        "diagnostic should require the first mutable aggregate argument to match: {reason}"
    );
}

#[test]
fn terminal_aggregate_source_constructor_and_commit_type_check() {
    const SOURCE: &str = "
## Description: Aggregate fixture constructor and same-root commit succeed ##
entry main = f(): void =>
    let mutable runtime: TerminalAggregateFixture = new TerminalAggregateFixture:
        first: terminal_aggregate_member_new()
        second: terminal_aggregate_member_new()
    let candidate = terminal_aggregate_member_new()
    terminal_aggregate_retarget_member(mutable ref runtime, candidate)
    terminal_aggregate_commit_candidate(mutable ref runtime)
    return void
";
    let result = type_check_fixture_source(SOURCE);
    assert!(
        result.is_ok(),
        "source aggregate constructor plus matching same-root commit should type-check: {result:?}"
    );
}

#[test]
fn terminal_aggregate_source_wrong_root_commit_is_rejected() {
    const SOURCE: &str = "
## Description: Aggregate fixture rejects wrong-root commit after retarget ##
entry main = f(): void =>
    let mutable runtime: TerminalAggregateFixture = new TerminalAggregateFixture:
        first: terminal_aggregate_member_new()
        second: terminal_aggregate_member_new()
    let mutable other_runtime: TerminalAggregateFixture = new TerminalAggregateFixture:
        first: terminal_aggregate_member_new()
        second: terminal_aggregate_member_new()
    let candidate = terminal_aggregate_member_new()
    terminal_aggregate_retarget_member(mutable ref runtime, candidate)
    terminal_aggregate_commit_candidate(mutable ref other_runtime)
    return void
";
    let errors = type_check_fixture_source(SOURCE)
        .expect_err("wrong-root source commit should fail aggregate barrier");
    let rendered = format!("{errors:?}");
    assert!(
        rendered.contains("cannot interpose") && rendered.contains("runtime"),
        "wrong-root source commit should require matching aggregate root: {errors:?}"
    );
}

#[test]
fn terminal_aggregate_source_member_owner_escape_is_rejected() {
    const SOURCE: &str = "
## Description: Aggregate fixture rejects public member owner extraction ##
entry main = f(): TerminalAggregateMember =>
    let mutable runtime: TerminalAggregateFixture = new TerminalAggregateFixture:
        first: terminal_aggregate_member_new()
        second: terminal_aggregate_member_new()
    return runtime.first
";
    let errors = type_check_fixture_source(SOURCE)
        .expect_err("source member owner escape should be rejected");
    let rendered = format!("{errors:?}");
    assert!(
        rendered.contains("escape through return"),
        "source member owner escape should use normal ownership diagnostics: {errors:?}"
    );
}

#[test]
fn terminal_aggregate_undeclared_layout_transition_is_rejected() {
    let mut checker = TypeChecker::new();
    checker
        .register_terminal_aggregate_fixture_for_tests()
        .expect("fixture aggregate spec should register");
    seed_runtime(&mut checker);
    let statement = call_statement(
        5_000,
        "terminal_aggregate_commit_undeclared",
        vec![mutable_ref(5_001, ident(5_002, "runtime"))],
    );
    let error = checker
        .update_affine_aggregate_statement_after(&statement)
        .expect_err("undeclared aggregate transition must fail");
    let reason = match error {
        TypeError::ConstraintSolvingFailed { reason, .. } => reason,
        unexpected => {
            assert!(
                matches!(unexpected, TypeError::ConstraintSolvingFailed { .. }),
                "expected undeclared transition diagnostic, got: {unexpected:?}"
            );
            String::new()
        }
    };
    assert!(
        reason.contains("undeclared layout transition"),
        "diagnostic should mention undeclared transition: {reason}"
    );
}
