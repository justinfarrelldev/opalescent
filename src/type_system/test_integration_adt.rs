extern crate alloc;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

fn parse_pipeline(source: &str) -> Program {
    let lexer = Lexer::new(source);
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

    program_opt.map_or_else(
        || Program {
            declarations: Vec::new(),
            span: Span::single(Position::start()),
            id: crate::ast::NodeId(0),
        },
        |program| program,
    )
}

fn span_with_offset(start_offset: usize, len: usize) -> Span {
    let start_column = start_offset.saturating_add(1);
    let end_offset = start_offset.saturating_add(len);
    let end_column = end_offset.saturating_add(1);
    let start = Position::new(1, start_column, start_offset);
    let end = Position::new(1, end_column, end_offset);
    Span::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sum_type_variant_constructor_type_checks() {
        const SOURCE: &str = "
type Message:
    Text:
        sender: string
        body: string

entry main = f(): Message =>
    return Message.Text { sender: 'alice', body: 'hello' }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "sum variant constructor should type check: {result:?}",
        );
    }

    #[test]
    fn test_sum_type_unknown_variant_reports_error() {
        const SOURCE: &str = "
type Message:
    Text:
        sender: string
        body: string

entry main = f(): Message =>
    return Message.UnknownVariant { sender: 'alice', body: 'hello' }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("unknown sum variant must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::UnknownVariant { .. })),
            "expected UnknownVariant error, got: {errors:?}",
        );
    }

    #[test]
    fn test_product_type_field_access_type_checks() {
        const SOURCE: &str = "
type Person:
    name: string
    age: int32

entry main = f(): string => {
    let person: Person = Person { name: 'bob', age: 30 }
    return person.name
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "product field access should type check: {result:?}"
        );
    }

    #[test]
    fn test_product_type_field_type_mismatch_reports_error() {
        const SOURCE: &str = "
type Person:
    name: string
    age: int32

entry main = f(): Person =>
    return Person { name: 42, age: 30 }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("field type mismatch must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::FieldTypeMismatch { .. })),
            "expected FieldTypeMismatch error, got: {errors:?}",
        );
    }

    #[test]
    fn test_has_field_constraint_solving_works() {
        let mut checker = TypeChecker::new();
        checker.register_adt_fields(
            "Person".to_owned(),
            alloc::collections::BTreeMap::from([("name".to_owned(), CoreType::String)]),
        );

        checker.add_constraint(TypeConstraint::HasField {
            owner: CoreType::Generic {
                name: "Person".to_owned(),
                type_args: Vec::new(),
            },
            field_name: "name".to_owned(),
            field_type: CoreType::String,
            owner_span: Some(span_with_offset(10, 6)),
            field_span: Some(span_with_offset(17, 4)),
        });

        let result = checker.solve_constraints();
        assert!(
            result.is_ok(),
            "HasField constraint should solve: {result:?}"
        );
    }

    #[test]
    fn test_adt_missing_field_reports_error() {
        const SOURCE: &str = "
type Message:
    Text:
        sender: string
        body: string

entry main = f(): Message =>
    return Message.Text { sender: 'alice' }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("missing constructor field must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::MissingField { .. })),
            "expected MissingField error, got: {errors:?}",
        );
    }

    #[test]
    fn test_adt_duplicate_field_reports_error() {
        const SOURCE: &str = "
type Message:
    Text:
        sender: string
        body: string

entry main = f(): Message =>
    return Message.Text { sender: 'alice', sender: 'bob', body: 'hello' }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("duplicate constructor field must fail type checking");
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::DuplicateField { .. })),
            "expected DuplicateField error, got: {errors:?}",
        );
    }
}
