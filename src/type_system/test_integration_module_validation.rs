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
                    "Description: Generated module-validation test documentation".to_owned(),
                    span,
                ));
            }
        }
    }

    program
}

fn function_symbol(name: &str, parameters: Vec<CoreType>, return_type: CoreType) -> SymbolInfo {
    SymbolInfo {
        name: name.to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters,
            return_types: vec![return_type],
            error_types: Vec::new(),
        },
        visibility: Visibility::Public,
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
    fn test_importing_same_name_from_two_modules_reports_import_name_conflict() {
        const SOURCE: &str = "
import sqrt from math
import sqrt from ./local_math

entry main = f(): float64 =>
    return sqrt(9.0)
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let mut local_math = ModuleInterface::new(String::from("./local_math"));
        let register_result = local_math.register_symbol(function_symbol(
            "sqrt",
            vec![CoreType::Float64],
            CoreType::Float64,
        ));
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(local_math);

        let result = checker.type_check_program(&program);
        assert!(
            result.is_err(),
            "duplicate import names must fail type checking"
        );
        let errors = result.err().map_or_else(Vec::new, |errs| errs);
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::ImportNameConflict { .. })),
            "expected ImportNameConflict error, got: {errors:?}",
        );
    }

    #[test]
    fn test_private_symbol_access_from_other_module_reports_private_access_error() {
        const SOURCE: &str = "
import hidden_fn from ./local_lib

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();

        let mut local_lib = ModuleInterface::new(String::from("./local_lib"));
        let register_result = local_lib.register_symbol(SymbolInfo {
            name: String::from("hidden_fn"),
            symbol_type: SymbolType::Function,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            visibility: Visibility::Private,
            source_location: Span::single(Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(local_lib);

        let result = checker.type_check_program(&program);
        assert!(result.is_err(), "private import must fail type checking");
        let errors = result.err().map_or_else(Vec::new, |errs| errs);
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::PrivateSymbolAccess { .. })),
            "expected PrivateSymbolAccess error, got: {errors:?}",
        );
    }

    #[test]
    fn test_module_alias_import_resolves_member_call() {
        const SOURCE: &str = "
import math as m from math

entry main = f(): float64 =>
    return m.sqrt(4.0)
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "aliased module member call should resolve correctly: {result:?}",
        );
    }

    #[test]
    fn test_module_alias_import_reports_type_mismatch_for_bad_argument() {
        const SOURCE: &str = "
import math as m from math

entry main = f(): float64 =>
    return m.sqrt('bad')
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_err(),
            "calling imported function with wrong args must fail"
        );
        let errors = result.err().map_or_else(Vec::new, |errs| errs);
        assert!(
            errors
                .iter()
                .any(|error| matches!(*error, TypeError::TypeMismatch { .. })),
            "expected TypeMismatch error, got: {errors:?}",
        );
    }

    #[test]
    fn test_two_modules_with_same_public_function_can_be_disambiguated_by_aliases() {
        const SOURCE: &str = "
import sqrt as std_sqrt from math
import sqrt as local_sqrt from ./local_math

entry main = f(): float64 => {
    let a: float64 = std_sqrt(4.0)
    let b: float64 = local_sqrt(9.0)
    return a + b
}
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let mut local_math = ModuleInterface::new(String::from("./local_math"));
        let register_result = local_math.register_symbol(function_symbol(
            "sqrt",
            vec![CoreType::Float64],
            CoreType::Float64,
        ));
        assert!(
            register_result.is_ok(),
            "module symbol setup should succeed"
        );
        checker.register_module_interface(local_math);

        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "aliased imports should disambiguate same symbol names: {result:?}",
        );
    }

    #[test]
    fn test_module_interface_collects_public_symbols_only() {
        let mut checker = TypeChecker::new();
        checker.set_current_module_path(String::from("./sample"));
        let source = "
public let answer: int32 = 42
let hidden: int32 = 7

entry main = f(): void =>
    return void
";

        let program = parse_pipeline(source);
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "module interface generation fixture should type-check: {result:?}",
        );

        let interface_opt = checker.module_interface("./sample");
        assert!(
            interface_opt.is_some(),
            "module interface should be generated"
        );
        let interface = interface_opt.map_or_else(
            || ModuleInterface::new(String::from("__missing_interface__")),
            |present_interface| present_interface,
        );
        assert!(
            interface.exports.contains_key("answer"),
            "public symbol should be exported",
        );
        assert!(
            !interface.exports.contains_key("hidden"),
            "private symbol must not be exported",
        );
        assert!(
            interface.private_symbols.contains_key("hidden"),
            "private symbol should be tracked as private",
        );
    }

    #[test]
    fn test_process_module_imports_all_frozen_symbols() {
        const SOURCE: &str = "
import current_working_directory_sync from process
import current_executable_path_sync from process
import current_executable_directory_sync from process
import set_current_working_directory_sync from process
import get_environment_variable from process
import get_environment_variable_or from process
import environment_variable_exists from process
import exit_process from process
import CurrentWorkingDirectoryUnavailableError from process

entry main = f(): void errors CurrentWorkingDirectoryUnavailableError =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "all frozen process module symbols must be importable: {result:?}",
        );
    }

    #[test]
    fn test_all_19_new_standard_conversion_functions_are_importable() {
        const SOURCE: &str = "
import string_to_int8 from standard
import string_to_int16 from standard
import string_to_uint8 from standard
import string_to_uint16 from standard
import string_to_uint32 from standard
import string_to_uint64 from standard
import string_to_float32 from standard
import string_to_float64 from standard
import int8_to_string from standard
import int16_to_string from standard
import int32_to_string from standard
import int64_to_string from standard
import uint8_to_string from standard
import uint16_to_string from standard
import uint32_to_string from standard
import uint64_to_string from standard
import float32_to_string from standard
import float64_to_string from standard
import bool_to_string from standard

entry main = f(args: string[]): void =>
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "all 19 new standard conversion functions must be importable: {result:?}",
        );
    }

    #[test]
    fn test_expression_loop_with_break_value_type_checks_correctly() {
        const SOURCE: &str = "
import int64_to_string from standard
import println from standard

entry main = f(args: string[]): void =>
    let x = loop =>
        break x: 42
    println(int64_to_string(x))
    return void
";

        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "let x = loop => break x: 42 must type-check without errors: {result:?}",
        );
    }

    #[test]
    fn test_expression_loop_with_bare_break_returns_unit() {
        // A loop with a bare `break` (no value) should type-check as Unit
        const SOURCE: &str = "
entry main = f(args: string[]): void =>
    let x = loop =>
        break
    return void
";
        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "expression loop with bare break should type-check as Unit: {result:?}",
        );
    }

    #[test]
    fn test_expression_loop_with_mismatched_break_types_reports_error() {
        // A loop with breaks of different types should produce a type error
        const SOURCE: &str = "
import int64_to_string from standard
import bool_to_string from standard

entry main = f(args: string[]): void =>
    let x = loop =>
        if true:
            break x: 42
        break x: true
    return void
";
        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_err(),
            "expression loop with mismatched break types should produce a type error",
        );
    }

    #[test]
    fn test_nested_expression_loops_have_independent_break_types() {
        // Nested loops should each track their own break type independently
        const SOURCE: &str = "
import int64_to_string from standard
import println from standard

entry main = f(args: string[]): void =>
    let outer = loop =>
        let inner = loop =>
            break inner: 10
        break outer: inner
    println(int64_to_string(outer))
    return void
";
        let program = parse_pipeline(SOURCE);
        let mut checker = TypeChecker::new();
        let result = checker.type_check_program(&program);
        assert!(
            result.is_ok(),
            "nested expression loops should each track break types independently: {result:?}",
        );
    }
}
