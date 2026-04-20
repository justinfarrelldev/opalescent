extern crate alloc;

use crate::ast::{Decl, Documentation, Program, Visibility};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

/// Inject required doc comments for public/entry functions in inline test sources.
fn with_required_function_docs(source: &str) -> String {
    const DOC_COMMENT_BLOCK: &str =
        "##\n    Description: Test helper generated function documentation text\n##\n";

    let mut rewritten_source = String::new();
    let mut last_non_empty_line: Option<String> = None;

    for line in source.lines() {
        let trimmed_start = line.trim_start();
        let is_public_or_entry_function =
            (trimmed_start.starts_with("entry ") || trimmed_start.starts_with("public "))
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
            let requires_doc = is_entry || matches!(function_visibility, &Visibility::Public);
            if requires_doc && function_doc_comment.is_none() {
                *function_doc_comment = Some(Documentation::from_raw(
                    "Description: Generated generic integration documentation".to_owned(),
                    span,
                ));
            }
        }
    }

    program
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_product_constructor_infers_single_type_arg() {
        const SOURCE: &str = "
type Node<T>:
    value: T

## Description: Entry validates generic node constructor inference ##
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

## Description: Entry validates multi-parameter generic constructor inference ##
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

## Description: Entry validates generic constraint violation diagnostics ##
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

## Description: Entry validates generic instantiation metadata recording ##
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
