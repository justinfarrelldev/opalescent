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
}
