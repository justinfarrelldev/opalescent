//! Native managed-callback typing and generated nominal World handle tests.
use super::TypeChecker;
use crate::ast::testing::{ManagedCallback, TestItem};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::types::CoreType;

/// Extract one ordinary callback through the real test-module frontend.
pub(super) fn parse_callback(body: &str) -> ManagedCallback {
    let source = format!("scenario typing:\n    it should type check: {body}");
    let (tokens, lex_errors) = Lexer::new_test_module(&source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, parse_errors) = Parser::new(tokens).parse_test_module();
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    let module = module.expect("module");
    let mut callbacks = module.items.into_iter().filter_map(|item| match item {
        TestItem::Scenario(scenario) => scenario.items.into_iter().find_map(|child| match child {
            TestItem::Test(test) => Some(test.callback),
            _ => None,
        }),
        _ => None,
    });
    callbacks.next().expect("test callback")
}

#[test]
fn generated_handles_are_nominal_and_expose_the_typed_state() {
    let mut checker = TypeChecker::new();
    let span = Span::single(Position::start());
    let first = checker
        .register_native_test_world("First", "tests/worlds.op::First", CoreType::Int64, span)
        .expect("first handle");
    let second = checker
        .register_native_test_world("Second", "tests/worlds.op::Second", CoreType::Int64, span)
        .expect("second handle");
    assert_ne!(first, second);
    assert_ne!(first, CoreType::Int64);
    let callback = parse_callback(
        "f(ref first: First): void =>\n        print(first.state)\n        return void\n",
    );
    let result = checker.type_check_native_callback(&callback, &["first".to_owned()]);
    assert!(result.is_ok(), "{result:?}");
    let unused_body = parse_callback("f(ref first: First): void =>\n        return void\n");
    assert!(
        checker
            .type_check_native_callback(&unused_body, &["first".to_owned()])
            .is_err(),
        "another callback's use must not satisfy this callback's contract"
    );
}

#[test]
fn body_world_parameters_must_match_nominal_identity_order_and_reference_modes() {
    for body in [
        "f(ref second: Second, ref first: First): void =>\n        print(first.state + second.state)\n        return void\n",
        "f(ref first: First): void =>\n        print(first.state)\n        return void\n",
        "f(ref first: First, ref second: Second, ref extra: First): void =>\n        print(first.state + second.state + extra.state)\n        return void\n",
        "f(first: First, ref second: Second): void =>\n        print(first.state + second.state)\n        return void\n",
        "f(ref first: int64, ref second: Second): void =>\n        print(first + second.state)\n        return void\n",
    ] {
        let mut checker = TypeChecker::new();
        let span = Span::single(Position::start());
        let first = checker
            .register_native_test_world("First", "tests/worlds.op::First", CoreType::Int64, span)
            .expect("first");
        let second = checker
            .register_native_test_world("Second", "tests/worlds.op::Second", CoreType::Int64, span)
            .expect("second");
        let callback = parse_callback(body);
        assert!(
            checker
                .type_check_native_case_callback(&callback, &[first, second], None)
                .is_err(),
            "accepted invalid World injection: {body}"
        );
    }
}

#[test]
fn final_context_follows_injected_worlds_without_becoming_a_world_requirement() {
    let mut checker = TypeChecker::new();
    let first = checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let callback = parse_callback(
        "f(ref first: First, mutable ref test: TestContext): void =>\n        print(first.state)\n        print(test.failed)\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[first], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn world_state_mutability_follows_the_handle_reference_mode() {
    for (mode, valid) in [("ref", false), ("mutable ref", true)] {
        let mut checker = TypeChecker::new();
        let world = checker
            .register_native_test_world(
                "First",
                "tests/worlds.op::First",
                CoreType::Int64,
                Span::single(Position::start()),
            )
            .expect("handle");
        let callback = parse_callback(&format!(
            "f({mode} first: First): void =>\n        first.state = 2\n        return void\n"
        ));
        let result = checker.type_check_native_case_callback(&callback, &[world], None);
        assert_eq!(result.is_ok(), valid, "{mode}: {result:?}");
    }
}

#[test]
fn world_state_can_be_borrowed_by_helpers_without_erasing_reference_modes() {
    for (mode, operation, valid) in [
        ("ref", "observe(ref first.state)", true),
        ("mutable ref", "change(mutable ref first.state)", true),
        ("ref", "change(mutable ref first.state)", false),
    ] {
        let mut checker = TypeChecker::new();
        let world = checker
            .register_native_test_world(
                "First",
                "tests/worlds.op::First",
                CoreType::Int64,
                Span::single(Position::start()),
            )
            .expect("handle");
        let helper_source = "let observe = f(ref state: int64): void =>\n    print(state)\n    return void\nlet change = f(mutable ref state: int64): void =>\n    print(state)\n    return void\n";
        let (tokens, _) = Lexer::new(helper_source).tokenize();
        let (program, parse_errors) = Parser::new(tokens).parse();
        assert!(parse_errors.is_empty(), "{parse_errors:?}");
        let errors = checker
            .type_check_program(&program.expect("helpers"))
            .expect_err("no program entry");
        assert!(
            errors.iter().all(|error| matches!(
                *error,
                crate::type_system::errors::TypeError::MissingEntryPoint { .. }
            )),
            "{errors:?}"
        );
        let callback = parse_callback(&format!(
            "f({mode} first: First): void =>\n        {operation}\n        return void\n"
        ));
        let result = checker.type_check_native_case_callback(&callback, &[world], None);
        assert_eq!(result.is_ok(), valid, "{operation}: {result:?}");
    }
}

#[test]
fn mutable_and_immutable_borrows_of_the_same_world_state_cannot_overlap() {
    let mut checker = TypeChecker::new();
    let world = checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let (tokens, _) = Lexer::new("let two = f(mutable ref left: int64, ref right: int64): void =>\n    print(left + right)\n    return void\n").tokenize();
    let (program, parse_errors) = Parser::new(tokens).parse();
    assert!(parse_errors.is_empty());
    let errors = checker
        .type_check_program(&program.expect("helper"))
        .expect_err("no entry");
    assert!(errors.iter().all(|error| matches!(
        *error,
        crate::type_system::errors::TypeError::MissingEntryPoint { .. }
    )));
    let callback = parse_callback(
        "f(mutable ref first: First): void =>\n        two(mutable ref first.state, ref first.state)\n        return void\n",
    );
    assert!(
        checker
            .type_check_native_case_callback(&callback, &[world], None)
            .is_err(),
        "overlapping mutable World-state borrows must be rejected"
    );
}

#[test]
fn nested_helper_world_parameters_shadow_outer_names_without_capturing_them() {
    let mut checker = TypeChecker::new();
    let world = checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let callback = parse_callback(
        "f(ref first: First): void =>\n        let helper = f(ref first: First): void =>\n            if true:\n                expect first.state to be 1\n            return void\n        helper(ref first)\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[world], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn ordinary_helpers_cannot_take_generated_world_handles_by_value() {
    let mut checker = TypeChecker::new();
    checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let helper = parse_callback(
        "f(first: First): void =>\n        print(first.state)\n        return void\n",
    );
    assert!(
        checker.type_check_native_callback(&helper, &[]).is_err(),
        "generated World parameters must remain second-class references even in ordinary helpers"
    );
}

#[test]
fn context_parameter_names_do_not_shadow_the_generated_context_type() {
    let mut checker = TypeChecker::new();
    let callback = parse_callback(
        "f(mutable ref TestContext: TestContext): void =>\n        let inspect = f(mutable ref metadata: TestContext): void =>\n            print(metadata.failed)\n            return void\n        inspect(mutable ref TestContext)\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn generic_owned_parameters_cannot_be_used_to_copy_injected_world_handles() {
    let mut checker = TypeChecker::new();
    let world = checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let (tokens, _) =
        Lexer::new("let identity = f<T>(value: T): T =>\n    return value\n").tokenize();
    let (program, parse_errors) = Parser::new(tokens).parse();
    assert!(parse_errors.is_empty());
    let errors = checker
        .type_check_program(&program.expect("generic helper"))
        .expect_err("no entry");
    assert!(errors.iter().all(|error| matches!(
        *error,
        crate::type_system::errors::TypeError::MissingEntryPoint { .. }
    )));
    let callback = parse_callback(
        "f(ref first: First): void =>\n        let copied = identity(first)\n        print(copied.state)\n        return void\n",
    );
    assert!(
        checker
            .type_check_native_case_callback(&callback, &[world], None)
            .is_err(),
        "generic owned arguments cannot bypass nominal World reference rules"
    );
}

#[test]
fn record_updates_cannot_forge_or_substitute_a_world_handle() {
    let mut checker = TypeChecker::new();
    let world = checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let callback = parse_callback(
        "f(ref first: First): void =>\n        let fake = first with:\n            state: 2\n        print(fake.state)\n        return void\n",
    );
    assert!(
        checker
            .type_check_native_case_callback(&callback, &[world], None)
            .is_err(),
        "World identity cannot be forged through a record update"
    );
}

#[test]
fn user_code_cannot_construct_a_runner_owned_world_handle() {
    let mut checker = TypeChecker::new();
    checker
        .register_native_test_world(
            "First",
            "tests/worlds.op::First",
            CoreType::Int64,
            Span::single(Position::start()),
        )
        .expect("handle");
    let callback = parse_callback(
        "f(): void =>\n        let fake = new First:\n            state: 2\n        print(fake.state)\n        return void\n",
    );
    let error = checker
        .type_check_native_callback(&callback, &[])
        .expect_err("World construction is runner-only");
    assert!(
        error
            .to_string()
            .contains("runner constructs World handles"),
        "{error:?}"
    );
}

#[test]
fn strict_usage_is_callback_local_and_cannot_be_satisfied_by_shadowed_names() {
    for body in [
        "f(ref first: First): void =>\n        return void\n",
        "f(ref first: First): void =>\n        let first = 1\n        print(first)\n        return void\n",
    ] {
        let mut checker = TypeChecker::new();
        checker
            .register_native_test_world(
                "First",
                "tests/worlds.op::First",
                CoreType::Int64,
                Span::single(Position::start()),
            )
            .expect("handle");
        let callback = parse_callback(body);
        assert!(
            checker
                .type_check_native_callback(&callback, &["first".to_owned()])
                .is_err(),
            "accepted unused injected World: {body}"
        );
    }
}
