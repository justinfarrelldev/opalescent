//! Typed expectation operands use the ordinary expression/type checker.
use super::TypeChecker;
use super::native_testing_tests::parse_callback;

#[test]
fn accepts_typed_scalar_string_and_ordered_collection_matchers() {
    let mut checker = TypeChecker::new();
    let callback = parse_callback(
        "f(): void =>\n        expect 4 to be 4\n        expect true not to be false\n        expect 3 to be greater than 2\n        expect 'café' to contain 'é'\n        expect 'draft' not to end with '\\n'\n        expect '' to be empty\n        expect ['a', 'b'] to contain exactly ['a', 'b'] in order\n        return void\n",
    );
    let result = checker.type_check_native_callback(&callback, &[]);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn variant_matching_resolves_a_genuine_variant_and_nominal_type() {
    let (tokens, lex_errors) = crate::lexer::Lexer::new(
        "type Status:\n    Written\n    Pending\ntype OtherStatus:\n    Written\n",
    )
    .tokenize();
    assert!(lex_errors.is_empty());
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse();
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    let program = program.expect("type declarations");
    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("types module has no entry point");
    assert!(
        errors.iter().all(|error| matches!(
            *error,
            crate::type_system::errors::TypeError::MissingEntryPoint { .. }
        )),
        "{errors:?}"
    );
    let valid = parse_callback(
        "f(): void =>\n        let status = new Status.Written\n        expect status to be the Written variant of Status\n        return void\n",
    );
    let result = checker.type_check_native_callback(&valid, &[]);
    assert!(result.is_ok(), "{result:?}");
    let invalid = parse_callback(
        "f(): void =>\n        let status = new Status.Written\n        expect status to be the Written variant of OtherStatus\n        return void\n",
    );
    assert!(checker.type_check_native_callback(&invalid, &[]).is_err());
}

#[test]
fn unsupported_matcher_operand_types_are_compiler_errors() {
    for expectation in [
        "expect 4 to be '4'",
        "expect true to be greater than false",
        "expect 3 to contain 2",
        "expect 1 to be empty",
        "expect ['a'] to contain exactly [1] in order",
        "expect 1 to be the Missing variant of UnknownStatus",
    ] {
        let mut checker = TypeChecker::new();
        let callback = parse_callback(&format!(
            "f(): void =>\n        {expectation}\n        return void\n"
        ));
        assert!(
            checker.type_check_native_callback(&callback, &[]).is_err(),
            "accepted {expectation}"
        );
    }
}
