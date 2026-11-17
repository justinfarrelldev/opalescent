//! Native double configuration uses real callable signatures and nominal errors.
use super::TypeChecker;
use super::native_testing_tests::parse_callback;
use crate::ast::BorrowKind;
use crate::token::{Position, Span};
use crate::type_system::NativeDoubleSignature;
use crate::type_system::types::CoreType;

/// A concrete direct boundary suitable for typed configuration tests.
fn signature() -> NativeDoubleSignature {
    NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Owned],
        return_types: vec![CoreType::Int64],
        error_types: vec![CoreType::Generic {
            name: "ParseError".into(),
            type_args: Vec::new(),
        }],
    }
}

#[test]
fn accepts_typed_returns_errors_handlers_spies_history_and_explicit_restore() {
    let mut checker = TypeChecker::new();
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature(),
            Span::single(Position::start()),
        )
        .expect("typed boundary");
    let callback = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        stub writer:\n            otherwise fail the test\n        stub writer(1) returns 2\n        stub writer(3) fails with ParseError\n        let replacement = f(value: int64): int64 =>\n            return value + 1\n        stub writer using replacement\n        spy writer using original\n        expect writer to have been called once\n        expect writer to have been called exactly 2 times\n        expect writer to have been called with (1)\n        verify writer\n        restore writer\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn filesystem_paths_have_explicit_value_equality_and_owned_snapshot_support() {
    let mut checker = TypeChecker::new();
    let (tokens, _) = crate::lexer::Lexer::new("import path_from from standard\n").tokenize();
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse();
    assert!(parse_errors.is_empty());
    let errors = checker
        .type_check_program(&program.expect("filesystem import"))
        .expect_err("no entry");
    assert!(errors.iter().all(|error| matches!(
        *error,
        crate::type_system::errors::TypeError::MissingEntryPoint { .. }
    )));
    let signature = NativeDoubleSignature {
        parameters: vec![
            CoreType::Generic {
                name: "FilesystemPath".to_owned(),
                type_args: Vec::new(),
            },
            CoreType::String,
        ],
        parameter_modes: vec![BorrowKind::Owned, BorrowKind::Owned],
        return_types: vec![CoreType::Unit],
        error_types: vec![CoreType::Generic {
            name: "FilesystemFullError".to_owned(),
            type_args: Vec::new(),
        }],
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("filesystem boundary");
    let callback = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        stub writer(path_from('note.txt'), 'Draft') fails with FilesystemFullError\n        expect writer to have been called with (path_from('note.txt'), 'Draft')\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn exact_rules_check_reference_modes_but_history_compares_owned_snapshots() {
    let mut checker = TypeChecker::new();
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Ref],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("borrowed boundary");
    let valid = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        let value = 1\n        stub writer(ref value) returns void\n        expect writer to have been called with (1)\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&valid, &[], None);
    assert!(result.is_ok(), "{result:?}");
    let invalid = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        stub writer(1) returns void\n        return void\n",
    );
    assert!(
        checker
            .type_check_native_case_callback(&invalid, &[], None)
            .is_err()
    );
}

#[test]
fn conditional_configuration_borrows_values_without_capturing_borrow_addresses() {
    let mut checker = TypeChecker::new();
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Ref],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("borrowed boundary");
    let callback = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        let value = 1\n        if true:\n            stub writer(ref value) returns void\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn unsupported_argument_snapshots_keep_count_only_spying_available() {
    let mut checker = TypeChecker::new();
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Unit],
            error_types: Vec::new(),
        }],
        parameter_modes: vec![BorrowKind::Owned],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("callback boundary");
    let count_only = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        spy writer using original\n        expect writer to have never been called\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&count_only, &[], None);
    assert!(result.is_ok(), "{result:?}");
    let unsupported_history = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        let handler = f(): void =>\n            return void\n        expect writer to have been called with (handler)\n        return void\n",
    );
    let error = checker
        .type_check_native_case_callback(&unsupported_history, &[], None)
        .expect_err("function values have no argument snapshot equality");
    assert!(error.to_string().contains("owned snapshots"), "{error:?}");
}

#[test]
fn multi_result_stubs_preserve_the_existing_ordered_application_return_shape() {
    let mut checker = TypeChecker::new();
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Owned],
        return_types: vec![CoreType::Int64, CoreType::String],
        error_types: Vec::new(),
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("multi-result boundary");
    let callback = parse_callback(
        "f(): void =>\n        let writer = new Writer\n        let pair = f(): first: int64, second: string =>\n            return first: 1, second: 'one'\n        stub writer(1) returns pair()\n        return void\n",
    );
    let result = checker.type_check_native_case_callback(&callback, &[], None);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn stored_stub_results_cannot_duplicate_existing_affine_owners() {
    let mut checker = TypeChecker::new();
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Owned],
        return_types: vec![CoreType::Generic {
            name: "TerminalSession".to_owned(),
            type_args: Vec::new(),
        }],
        error_types: Vec::new(),
    };
    checker
        .register_native_test_double(
            "Writer",
            "tests::Writer",
            signature,
            Span::single(Position::start()),
        )
        .expect("resource boundary");
    let callback = parse_callback(
        "f(ref session: TerminalSession): void =>\n        let writer = new Writer\n        stub writer(1) returns session\n        return void\n",
    );
    assert!(
        checker.type_check_native_callback(&callback, &[]).is_err(),
        "repeated exact matches cannot duplicate an affine owner"
    );
}

#[test]
fn invalid_arguments_returns_nominal_errors_and_handlers_fail_compilation() {
    for rule in [
        "stub writer('bad') returns 2",
        "stub writer(1) returns 'bad'",
        "stub writer(1) fails with FilesystemFullError",
        "stub writer using f(value: string): string => value",
        "expect writer to have been called with ('bad')",
        "expect writer to have been called exactly 'two' times",
    ] {
        let mut checker = TypeChecker::new();
        checker
            .register_native_test_double(
                "Writer",
                "tests::Writer",
                signature(),
                Span::single(Position::start()),
            )
            .expect("typed boundary");
        let callback = parse_callback(&format!(
            "f(): void =>\n        let writer = new Writer\n        {rule}\n        return void\n"
        ));
        assert!(
            checker
                .type_check_native_case_callback(&callback, &[], None)
                .is_err(),
            "accepted invalid mock signature: {rule}"
        );
    }
}
