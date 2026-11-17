//! Native managed-callback typing and generated nominal World handle tests.
use super::TypeChecker;
use crate::ast::testing::{ManagedCallback, TestItem};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::types::CoreType;

/// Extract one ordinary callback through the real test-module frontend.
fn parse_callback(body: &str) -> ManagedCallback {
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
