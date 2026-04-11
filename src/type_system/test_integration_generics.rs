extern crate alloc;

use crate::ast::Program;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_product_constructor_infers_single_type_arg() {
        const SOURCE: &str = "
type Node<T>:
    value: T

entry main = f(): Node<int64> =>
    return Node { value: 42 }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "generic Node constructor should infer T=int64: {result:?}",
        );
    }

    #[test]
    fn test_generic_product_constructor_infers_multiple_type_args() {
        const SOURCE: &str = "
type Pair<T, U>:
    first: T
    second: U

entry main = f(): Pair<string, boolean> =>
    return Pair { first: 'hello', second: true }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "generic Pair constructor should infer T=string,U=boolean: {result:?}",
        );
    }

    #[test]
    fn test_generic_function_call_site_inference_identity() {
        const SOURCE: &str = "
public identity = f<T>(x: T): T =>
    return x

entry main = f(): int64 =>
    return identity(42)
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "identity(42) should infer T=int64 and return int64: {result:?}",
        );
    }

    #[test]
    fn test_generic_adt_constraint_violation_reports_type_error() {
        const SOURCE: &str = "
type NumberBox<T: int64>:
    value: T

entry main = f(): NumberBox<string> =>
    return NumberBox { value: 'hello' }
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        let errors = result.expect_err("constraint-violating generic ADT instantiation must fail");
        assert!(
            errors.iter().any(|error| matches!(
                *error,
                TypeError::UnificationFailed { .. } | TypeError::ConstraintSolvingFailed { .. }
            )),
            "expected generic constraint violation diagnostic, got: {errors:?}",
        );
    }

    #[test]
    fn test_generic_instantiation_metadata_records_unique_call_and_constructor_instantiations() {
        const SOURCE: &str = "
type Pair<T, U>:
    first: T
    second: U

public identity = f<T>(x: T): T =>
    return x

entry main = f(): int64 => {
    let first: Pair<int64, boolean> = Pair { first: 42, second: true }
    let second: Pair<int64, boolean> = Pair { first: 7, second: false }
    let one: int64 = identity(42)
    let two: int64 = identity(7)
    return one
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "generic instantiation metadata test source must type check: {result:?}",
        );

        let recorded = checker.generic_instantiations();
        let pair_entries = recorded
            .get("Pair")
            .expect("Pair instantiations should be recorded");
        assert_eq!(
            pair_entries.len(),
            1,
            "Pair<int64, boolean> should be recorded uniquely",
        );
        assert_eq!(
            pair_entries[0],
            vec![CoreType::Int64, CoreType::Boolean],
            "Pair concrete type args should match inferred instantiation",
        );

        let identity_entries = recorded
            .get("identity")
            .expect("identity instantiations should be recorded");
        assert_eq!(
            identity_entries.len(),
            1,
            "identity<int64> should be recorded uniquely",
        );
        assert_eq!(
            identity_entries[0],
            vec![CoreType::Int64],
            "identity concrete type args should match inferred instantiation",
        );
    }
}
