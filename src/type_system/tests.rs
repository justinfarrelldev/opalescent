#![allow(
    clippy::pattern_type_mismatch,
    clippy::too_many_lines,
    reason = "large regression tests and borrowed-pattern checks are intentional here"
)]
//! Tests for the type system

extern crate alloc;
use alloc::collections::BTreeMap;

use super::checker::TypeChecker;
use super::constraints::TypeConstraint;
use super::environment::TypeEnvironment;
use super::errors::{TypeError, Warning};
use super::substitution::Substitution;
use super::symbol_table::{ScopeId, SymbolInfo, SymbolTable, SymbolType, Visibility};
use super::type_mapping::ast_type_to_core_type;
use super::types::{CoreType, TypeVar};
use crate::ast::{
    Decl, Documentation, Expr, Field, FunctionModifier, HotReloadMetadata, LabeledValue,
    LambdaBody, LetBinding, LiteralValue, NodeId, Parameter, Program, Stmt, StringPart, Type,
    TypeDef, TypeParameter, Variant, Visibility as AstVisibility,
};
use crate::errors::renderer::render_diagnostic;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::token::{Position, Span};
use miette::Diagnostic;

// Test constants for semantic meaning instead of magic numbers
const TEST_VAR_ID: usize = 0;
const ANOTHER_TEST_VAR_ID: usize = 1;
const THIRD_TEST_VAR_ID: usize = 42;

// Helper function to create test spans
fn test_span() -> Span {
    Span::single(Position::start())
}

/// Create a `Span` that begins at the provided byte offset and spans `len` bytes.
fn span_with_offset(offset: usize, len: usize) -> Span {
    let start = Position::new(1, offset.saturating_add(1), offset);
    let end_offset = offset.saturating_add(len);
    let end = Position::new(1, end_offset.saturating_add(1), end_offset);
    Span::new(start, end)
}

fn node_id(id: usize) -> NodeId {
    NodeId(id)
}

/// Source text for the guard/propagate integration sample program.
const ERROR_HANDLING_SAMPLE_SOURCE: &str =
    include_str!("../../language-spec/error_handling_samples.op");

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

/// Parse the guard/propagate sample program into an AST for integration testing.
#[expect(
    clippy::panic,
    reason = "Test helper uses panic for unrecoverable errors"
)]
fn parse_error_handling_sample_program() -> Program {
    let source_with_docs = with_required_function_docs(ERROR_HANDLING_SAMPLE_SOURCE);
    let lexer = Lexer::new(&source_with_docs);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "sample program should tokenize without lex errors: {:?}",
        lex_errors.errors
    );

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "sample program should parse without errors: {:?}",
        parse_errors.errors
    );

    program_opt.unwrap_or_else(|| panic!("parser returned no program for valid sample"))
}

#[expect(
    clippy::panic,
    reason = "Test helper uses panic for unrecoverable parse/lex failures"
)]
fn parse_program_from_source(source: &str) -> Program {
    let source_with_docs = with_required_function_docs(source);
    let lexer = Lexer::new(&source_with_docs);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "source should tokenize without lex errors: {:?}",
        lex_errors.errors
    );

    let parser = Parser::new(tokens);
    let (program_opt, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "source should parse without errors: {:?}",
        parse_errors.errors
    );

    program_opt.unwrap_or_else(|| panic!("parser returned no program for valid source"))
}

fn parse_program_from_source_with_spaces(source: &str) -> Program {
    let normalized = source.replace('\t', "    ");
    parse_program_from_source(&normalized)
}

fn literal_expr(value: LiteralValue, id: usize) -> Expr {
    Expr::Literal {
        value,
        span: test_span(),
        id: node_id(id),
    }
}

fn identifier_expr(name: &str, id: usize) -> Expr {
    Expr::Identifier {
        name: name.to_owned(),
        span: test_span(),
        id: node_id(id),
    }
}

fn int_type(name: &str) -> Type {
    Type::Basic {
        name: name.to_owned(),
        span: test_span(),
    }
}

fn create_program(declarations: Vec<Decl>) -> Program {
    Program {
        declarations,
        span: test_span(),
        id: node_id(900_000),
    }
}

fn create_entry_program(declarations: Vec<Decl>) -> Program {
    let mut all_declarations = declarations;
    all_declarations.push(make_function_decl(
        "main",
        Vec::new(),
        Some(int_type("int32")),
        return_stmt(literal_expr(LiteralValue::Integer(0), 9_100_000), 9_100_001),
        9_100_002,
    ));

    if let Some(&mut Decl::Function {
        ref mut is_entry,
        ref mut doc_comment,
        ..
    }) = all_declarations.last_mut()
    {
        *is_entry = true;
        *doc_comment = Some(Documentation::from_raw(
            "Description: Entry function generated by test helper for validation".to_owned(),
            test_span(),
        ));
    }

    create_program(all_declarations)
}

fn make_parameter(name: &str, ty: Type) -> Parameter {
    Parameter {
        name: name.to_owned(),
        param_type: ty,
        span: test_span(),
    }
}

fn make_function_decl(
    name: &str,
    params: Vec<Parameter>,
    return_type: Option<Type>,
    body: Stmt,
    id: usize,
) -> Decl {
    Decl::Function {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: params,
        return_types: return_type.map(|single_return_type| vec![single_return_type]),
        error_types: Vec::new(),
        body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(id),
        metadata: HotReloadMetadata::for_function(),
    }
}

fn make_let_decl(name: &str, annotation: Option<Type>, initializer: Expr, id: usize) -> Decl {
    let next_id = id.checked_add(1).unwrap_or(id);
    Decl::Let {
        binding: LetBinding {
            name: name.to_owned(),
            type_annotation: annotation,
            is_mutable: false,
            span: test_span(),
            id: node_id(id),
        },
        initializer,
        visibility: AstVisibility::Private,
        doc_comment: None,
        span: test_span(),
        id: node_id(next_id),
        metadata: HotReloadMetadata::for_let_declaration(),
    }
}

fn return_stmt(value: Expr, id: usize) -> Stmt {
    return_stmt_values(
        vec![LabeledValue {
            label: String::new(),
            span: test_span(),
            value,
            id: node_id(id.checked_add(10).unwrap_or(id)),
        }],
        id,
    )
}

fn return_stmt_values(values: Vec<LabeledValue>, id: usize) -> Stmt {
    Stmt::Return {
        values,
        span: test_span(),
        id: node_id(id),
    }
}

fn labeled_value(label: &str, value: Expr, id: usize) -> LabeledValue {
    LabeledValue {
        label: label.to_owned(),
        span: test_span(),
        value,
        id: node_id(id),
    }
}

fn constructor_expr(callee: Expr, fields: Vec<(&str, Expr)>, id: usize) -> Expr {
    Expr::Constructor {
        callee: Box::new(callee),
        fields: fields
            .into_iter()
            .map(|(name, value)| crate::ast::ConstructorField {
                name: name.to_owned(),
                value,
                span: test_span(),
            })
            .collect(),
        span: test_span(),
        id: node_id(id),
    }
}

// ============================================================================
// Error Handling: `propagate` Expression Tests
// ============================================================================

/// Build a simple function declaration with an explicit errors clause.
/// This helper keeps construction localized to tests so production code
/// remains minimal and focused. The function body is provided by caller.
fn make_function_decl_with_errors(
    name: &str,
    params: Vec<Parameter>,
    return_type: Option<Type>,
    error_types: Vec<&str>,
    body: Stmt,
    id: usize,
) -> Decl {
    Decl::Function {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: params,
        return_types: return_type.map(|single_return_type| vec![single_return_type]),
        error_types: error_types.into_iter().map(str::to_owned).collect(),
        body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(id),
        metadata: HotReloadMetadata::for_function(),
    }
}

/// Create a simple unit type declaration for use as an error type placeholder.
fn make_unit_type_decl(name: &str, id: usize) -> Decl {
    Decl::Type {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        type_def: TypeDef::Alias {
            target_type: Type::Basic {
                name: "unit".to_owned(),
                span: test_span(),
            },
            span: test_span(),
        },
        visibility: AstVisibility::Private,
        doc_comment: None,
        span: test_span(),
        id: node_id(id),
        metadata: HotReloadMetadata::for_type_declaration(),
    }
}

fn make_product_type_decl(name: &str, fields: Vec<(&str, Type)>, id: usize) -> Decl {
    Decl::Type {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        type_def: TypeDef::Product {
            fields: fields
                .into_iter()
                .map(|(field_name, type_annotation)| Field {
                    name: field_name.to_owned(),
                    type_annotation,
                    span: test_span(),
                })
                .collect(),
            span: test_span(),
        },
        visibility: AstVisibility::Private,
        doc_comment: None,
        span: test_span(),
        id: node_id(id),
        metadata: HotReloadMetadata::for_type_declaration(),
    }
}

/// Create a call expression `callee(arg_names...)`.
fn call_expr(callee_name: &str, arg_names: &[&str], id: usize) -> Expr {
    Expr::Call {
        callee: Box::new(identifier_expr(callee_name, id)),
        generic_args: None,
        args: arg_names.iter().map(|n| identifier_expr(n, id)).collect(),
        span: test_span(),
        id: node_id(id.checked_add(10).unwrap_or(id)),
    }
}

/// Wrap a call expression with `propagate`.
fn propagate_call(call: Expr, id: usize) -> Expr {
    Expr::Propagate {
        call: Box::new(call),
        span: test_span(),
        id: node_id(id),
    }
}

/// Build a guard expression around a call expression.
fn guard_call_expr(
    call: Expr,
    binding_name: &str,
    binding_type: Option<Type>,
    is_mutable: bool,
    else_branch: Stmt,
    id: usize,
) -> Expr {
    Expr::Guard {
        expr: Box::new(call),
        binding_name: binding_name.to_owned(),
        binding_type,
        is_mutable,
        else_branch: Box::new(else_branch),
        span: test_span(),
        id: node_id(id),
    }
}

#[test]
fn test_propagate_succeeds_with_subset_errors() {
    // Callee: string_to_int32(s: string): int32 errors ParseError => return 0
    let inner_fn = make_function_decl_with_errors(
        "string_to_int32",
        vec![make_parameter("s", int_type("string"))],
        Some(int_type("int32")),
        vec!["ParseError"],
        return_stmt(
            literal_expr(LiteralValue::Integer(0), TEST_VAR_ID),
            ANOTHER_TEST_VAR_ID,
        ),
        100,
    );

    // Caller: parse_and_return(s: string): int32 errors ParseError =>
    //            let n: int32 = propagate string_to_int32(s); return n
    let let_stmt = Stmt::Let {
        binding: LetBinding {
            name: "n".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(2000),
        },
        initializer: Some(propagate_call(
            call_expr("string_to_int32", &["s"], 2001),
            2002,
        )),
        span: test_span(),
        id: node_id(2003),
    };
    let caller_body = Stmt::Block {
        statements: vec![let_stmt, return_stmt(identifier_expr("n", 2004), 2005)],
        span: test_span(),
        id: node_id(2006),
    };
    let outer_fn = make_function_decl_with_errors(
        "parse_and_return",
        vec![make_parameter("s", int_type("string"))],
        Some(int_type("int32")),
        vec!["ParseError"],
        caller_body,
        101,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 1500),
        inner_fn,
        outer_fn,
    ]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "propagate should succeed when callee errors are subset of caller"
    );
}

#[test]
fn test_propagate_fails_outside_error_function() {
    // Callee that can error
    let inner_fn = make_function_decl_with_errors(
        "string_to_int32",
        vec![make_parameter("s", int_type("string"))],
        Some(int_type("int32")),
        vec!["ParseError"],
        return_stmt(literal_expr(LiteralValue::Integer(0), 3000), 3001),
        102,
    );

    // Caller without errors clause uses propagate => should error PropagateOutsideErrorFunction
    let let_stmt = Stmt::Let {
        binding: LetBinding {
            name: "n".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(3100),
        },
        initializer: Some(propagate_call(
            call_expr("string_to_int32", &["s"], 3101),
            3102,
        )),
        span: test_span(),
        id: node_id(3103),
    };
    let caller_body = Stmt::Block {
        statements: vec![let_stmt, return_stmt(identifier_expr("n", 3104), 3105)],
        span: test_span(),
        id: node_id(3106),
    };
    let outer_fn = make_function_decl(
        "parse_and_return",
        vec![make_parameter("s", int_type("string"))],
        Some(int_type("int32")),
        caller_body,
        103,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 3200),
        inner_fn,
        outer_fn,
    ]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_err(),
        "expected error when using propagate outside error-declaring fn"
    );
    let errors = result.unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, &TypeError::PropagateOutsideErrorFunction { .. })),
        "expected PropagateOutsideErrorFunction error, got: {errors:?}"
    );
}

#[test]
fn test_propagate_fails_when_error_types_mismatch() {
    // Callee errors IoError
    let inner_fn = make_function_decl_with_errors(
        "read_file",
        vec![make_parameter("path", int_type("string"))],
        Some(int_type("string")),
        vec!["IoError"],
        return_stmt(
            literal_expr(LiteralValue::String(String::new()), 4000),
            4001,
        ),
        104,
    );

    // Caller declares ParseError only and uses propagate read_file => mismatch
    let let_stmt = Stmt::Let {
        binding: LetBinding {
            name: "data".to_owned(),
            type_annotation: Some(int_type("string")),
            is_mutable: false,
            span: test_span(),
            id: node_id(4100),
        },
        initializer: Some(propagate_call(
            call_expr("read_file", &["path"], 4101),
            4102,
        )),
        span: test_span(),
        id: node_id(4103),
    };
    let caller_body = Stmt::Block {
        statements: vec![let_stmt, return_stmt(identifier_expr("data", 4104), 4105)],
        span: test_span(),
        id: node_id(4106),
    };
    let outer_fn = make_function_decl_with_errors(
        "load_data",
        vec![make_parameter("path", int_type("string"))],
        Some(int_type("string")),
        vec!["ParseError"],
        caller_body,
        105,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("IoError", 4200),
        make_unit_type_decl("ParseError", 4201),
        inner_fn,
        outer_fn,
    ]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_err(),
        "expected error when propagated errors not subset of caller errors"
    );
    let errors = result.unwrap_err();
    assert!(
        errors
            .iter()
            .any(|e| matches!(e, &TypeError::PropagateErrorMismatch { .. })),
        "expected PropagateErrorMismatch error, got: {errors:?}"
    );
}

#[test]
fn test_type_check_pure_function_rejects_print_call() {
    let pure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print", 7_100_000)),
                generic_args: None,
                args: vec![literal_expr(
                    LiteralValue::String(String::from("hello")),
                    7_100_001,
                )],
                span: test_span(),
                id: node_id(7_100_002),
            },
            span: test_span(),
            id: node_id(7_100_003),
        }],
        span: test_span(),
        id: node_id(7_100_004),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_100_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(result.is_err(), "pure function calling print should fail");
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "print"
            )
        }),
        "expected PurityViolation about impure print call, got: {errors:?}"
    );
}

#[test]
fn test_lambda_inside_pure_function_inherits_purity() {
    let lambda_call_print = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: Vec::new(),
        return_types: vec![int_type("void")],
        error_types: Vec::new(),
        body: LambdaBody::Block(vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print", 7_130_000)),
                generic_args: None,
                args: vec![literal_expr(
                    LiteralValue::String(String::from("hello")),
                    7_130_001,
                )],
                span: test_span(),
                id: node_id(7_130_002),
            },
            span: test_span(),
            id: node_id(7_130_003),
        }]),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(7_130_004),
    };

    let pure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: lambda_call_print,
            span: test_span(),
            id: node_id(7_130_005),
        }],
        span: test_span(),
        id: node_id(7_130_006),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker_lambda"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_130_007),
        metadata: HotReloadMetadata::for_function(),
    };

    let mut main_fn = make_function_decl(
        "main",
        Vec::new(),
        Some(int_type("int32")),
        return_stmt(literal_expr(LiteralValue::Integer(0), 7_130_008), 7_130_009),
        7_130_010,
    );
    if let Decl::Function {
        ref mut is_entry,
        ref mut doc_comment,
        ..
    } = main_fn
    {
        *is_entry = true;
        *doc_comment = Some(Documentation::from_raw(
            "Description: Entry function generated for lambda purity integration".to_owned(),
            test_span(),
        ));
    }

    let program = create_program(vec![pure_fn, main_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_err(),
        "lambda inside pure function calling print should fail"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "print"
            )
        }),
        "expected PurityViolation about impure print call in lambda, got: {errors:?}"
    );
}

#[test]
fn test_lambda_inside_non_pure_function_allows_print() {
    let lambda_call_print = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: Vec::new(),
        return_types: vec![int_type("void")],
        error_types: Vec::new(),
        body: LambdaBody::Block(vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print", 7_140_000)),
                generic_args: None,
                args: vec![literal_expr(
                    LiteralValue::String(String::from("hello")),
                    7_140_001,
                )],
                span: test_span(),
                id: node_id(7_140_002),
            },
            span: test_span(),
            id: node_id(7_140_003),
        }]),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(7_140_004),
    };

    let impure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: lambda_call_print,
            span: test_span(),
            id: node_id(7_140_005),
        }],
        span: test_span(),
        id: node_id(7_140_006),
    };

    let impure_fn = Decl::Function {
        name: String::from("impure_worker_lambda"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: impure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_140_007),
        metadata: HotReloadMetadata::for_function(),
    };

    let mut main_fn = make_function_decl(
        "main",
        Vec::new(),
        Some(int_type("int32")),
        return_stmt(literal_expr(LiteralValue::Integer(0), 7_140_008), 7_140_009),
        7_140_010,
    );
    if let Decl::Function {
        ref mut is_entry,
        ref mut doc_comment,
        ..
    } = main_fn
    {
        *is_entry = true;
        *doc_comment = Some(Documentation::from_raw(
            "Description: Entry function generated for lambda impurity integration".to_owned(),
            test_span(),
        ));
    }

    let program = create_program(vec![impure_fn, main_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "lambda inside non-pure function should allow print"
    );
}

#[test]
fn test_type_check_pure_function_rejects_print_int32() {
    let pure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print_int32", 7_110_000)),
                generic_args: None,
                args: vec![literal_expr(LiteralValue::Integer(123), 7_110_001)],
                span: test_span(),
                id: node_id(7_110_002),
            },
            span: test_span(),
            id: node_id(7_110_003),
        }],
        span: test_span(),
        id: node_id(7_110_004),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker_print_int32"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_110_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_err(),
        "pure function calling print_int32 should fail"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "print_int32"
            )
        }),
        "expected PurityViolation about impure print_int32 call, got: {errors:?}"
    );
}

#[test]
fn test_type_check_pure_function_rejects_random_uint64() {
    let pure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("random_uint64", 7_120_000)),
                generic_args: None,
                args: vec![
                    literal_expr(LiteralValue::Integer(1), 7_120_001),
                    literal_expr(LiteralValue::Integer(10), 7_120_002),
                ],
                span: test_span(),
                id: node_id(7_120_003),
            },
            span: test_span(),
            id: node_id(7_120_004),
        }],
        span: test_span(),
        id: node_id(7_120_005),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker_random_uint64"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_120_006),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_err(),
        "pure function calling random_uint64 should fail"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "random_uint64"
            )
        }),
        "expected PurityViolation about impure random_uint64 call, got: {errors:?}"
    );
}

#[test]
fn test_type_check_pure_function_rejects_print_string() {
    let pure_body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print_string", 7_130_000)),
                generic_args: None,
                args: vec![literal_expr(
                    LiteralValue::String(String::from("hello")),
                    7_130_001,
                )],
                span: test_span(),
                id: node_id(7_130_002),
            },
            span: test_span(),
            id: node_id(7_130_003),
        }],
        span: test_span(),
        id: node_id(7_130_004),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker_print_string"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_130_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_err(),
        "pure function calling print_string should fail"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "print_string"
            )
        }),
        "expected PurityViolation about impure print_string call, got: {errors:?}"
    );
}

#[test]
fn test_pure_function_cannot_call_non_pure_user_function() {
    let impure_helper = Decl::Function {
        name: String::from("impure_helper"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int64")]),
        error_types: Vec::new(),
        body: return_stmt(literal_expr(LiteralValue::Integer(1), 7_135_000), 7_135_001),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_135_002),
        metadata: HotReloadMetadata::for_function(),
    };

    let pure_caller = Decl::Function {
        name: String::from("pure_caller"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int32")]),
        error_types: Vec::new(),
        body: return_stmt(call_expr("impure_helper", &[], 7_135_003), 7_135_004),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_135_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![impure_helper, pure_caller]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_err(),
        "pure function calling non-pure user function should fail"
    );
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "impure_helper"
            )
        }),
        "expected PurityViolation about non-pure impure_helper call, got: {errors:?}"
    );
}

#[test]
fn test_pure_function_can_call_pure_user_function() {
    let pure_helper = Decl::Function {
        name: String::from("pure_helper"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int32")]),
        error_types: Vec::new(),
        body: return_stmt(
            literal_expr(LiteralValue::Integer(42), 7_136_000),
            7_136_001,
        ),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_136_002),
        metadata: HotReloadMetadata::for_function(),
    };

    let pure_caller = Decl::Function {
        name: String::from("pure_caller"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int32")]),
        error_types: Vec::new(),
        body: return_stmt(call_expr("pure_helper", &[], 7_136_003), 7_136_004),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_136_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_helper, pure_caller]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "pure function calling pure user function should pass: {result:?}"
    );
}

#[test]
fn test_pure_function_allows_local_mutation() {
    let pure_body = Stmt::Block {
        statements: vec![
            Stmt::Let {
                binding: LetBinding {
                    name: "x".to_owned(),
                    type_annotation: Some(int_type("int32")),
                    is_mutable: true,
                    span: test_span(),
                    id: node_id(7_137_000),
                },
                initializer: Some(literal_expr(LiteralValue::Integer(0), 7_137_001)),
                span: test_span(),
                id: node_id(7_137_002),
            },
            Stmt::Assignment {
                target: identifier_expr("x", 7_137_003),
                value: literal_expr(LiteralValue::Integer(1), 7_137_004),
                span: test_span(),
                id: node_id(7_137_005),
            },
            return_stmt(identifier_expr("x", 7_137_006), 7_137_007),
        ],
        span: test_span(),
        id: node_id(7_137_008),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_local_mutation"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int32")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_137_009),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "pure function local mutation should be allowed: {result:?}"
    );
}

#[test]
fn test_pure_function_allows_collection_member_calls() {
    let pure_body = Stmt::Block {
        statements: vec![
            Stmt::Let {
                binding: LetBinding {
                    name: "arr".to_owned(),
                    type_annotation: Some(Type::Array {
                        element_type: Box::new(int_type("int32")),
                        span: test_span(),
                    }),
                    is_mutable: true,
                    span: test_span(),
                    id: node_id(7_138_000),
                },
                initializer: Some(Expr::Array {
                    elements: vec![Expr::Cast {
                        expr: Box::new(literal_expr(LiteralValue::Integer(1), 7_138_001)),
                        target_type: int_type("int32"),
                        span: test_span(),
                        id: node_id(7_138_015),
                    }],
                    span: test_span(),
                    id: node_id(7_138_002),
                }),
                span: test_span(),
                id: node_id(7_138_003),
            },
            Stmt::Expression {
                expr: Expr::Call {
                    callee: Box::new(Expr::Member {
                        object: Box::new(identifier_expr("arr", 7_138_004)),
                        member: "push".to_owned(),
                        span: test_span(),
                        id: node_id(7_138_005),
                    }),
                    generic_args: None,
                    args: vec![Expr::Cast {
                        expr: Box::new(literal_expr(LiteralValue::Integer(1), 7_138_006)),
                        target_type: int_type("int32"),
                        span: test_span(),
                        id: node_id(7_138_016),
                    }],
                    span: test_span(),
                    id: node_id(7_138_007),
                },
                span: test_span(),
                id: node_id(7_138_008),
            },
            return_stmt(
                Expr::Member {
                    object: Box::new(identifier_expr("arr", 7_138_009)),
                    member: "length".to_owned(),
                    span: test_span(),
                    id: node_id(7_138_010),
                },
                7_138_011,
            ),
        ],
        span: test_span(),
        id: node_id(7_138_013),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_collection_member_calls"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int64")]),
        error_types: Vec::new(),
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_138_014),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "pure function collection member calls should be allowed: {result:?}"
    );
}

#[test]
fn test_type_check_pure_function_allows_string_to_int32() {
    let pure_body = Stmt::Block {
        statements: vec![
            Stmt::Let {
                binding: LetBinding {
                    name: "n".to_owned(),
                    type_annotation: Some(int_type("int32")),
                    is_mutable: false,
                    span: test_span(),
                    id: node_id(7_140_000),
                },
                initializer: Some(propagate_call(
                    Expr::Call {
                        callee: Box::new(identifier_expr("string_to_int32", 7_140_001)),
                        generic_args: None,
                        args: vec![literal_expr(
                            LiteralValue::String(String::from("12")),
                            7_140_002,
                        )],
                        span: test_span(),
                        id: node_id(7_140_003),
                    },
                    7_140_006,
                )),
                span: test_span(),
                id: node_id(7_140_007),
            },
            return_stmt(identifier_expr("n", 7_140_010), 7_140_011),
        ],
        span: test_span(),
        id: node_id(7_140_008),
    };

    let pure_fn = Decl::Function {
        name: String::from("pure_worker_string_to_int32"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("int32")]),
        error_types: vec![String::from("ParseError")],
        body: pure_body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_140_009),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "pure function calling string_to_int32 should be allowed: {result:?}"
    );
}

#[test]
fn test_type_check_non_pure_function_allows_print_call() {
    let body = Stmt::Block {
        statements: vec![Stmt::Expression {
            expr: Expr::Call {
                callee: Box::new(identifier_expr("print", 7_200_000)),
                generic_args: None,
                args: vec![literal_expr(
                    LiteralValue::String(String::from("ok")),
                    7_200_001,
                )],
                span: test_span(),
                id: node_id(7_200_002),
            },
            span: test_span(),
            id: node_id(7_200_003),
        }],
        span: test_span(),
        id: node_id(7_200_004),
    };

    let normal_fn = Decl::Function {
        name: String::from("worker"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body,
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_200_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![normal_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(result.is_ok(), "non-pure function should allow print call");
}

#[test]
fn test_pure_entry_combination_rejected() {
    let pure_entry_fn = Decl::Function {
        name: String::from("main"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: return_stmt(literal_expr(LiteralValue::Void, 7_205_000), 7_205_001),
        visibility: AstVisibility::Private,
        is_entry: true,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: Some(Documentation::from_raw(
            "Description: Pure entry declaration used to assert purity diagnostics".to_owned(),
            test_span(),
        )),
        span: test_span(),
        id: node_id(7_205_002),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = Program {
        declarations: vec![pure_entry_fn],
        span: test_span(),
        id: node_id(7_205_003),
    };

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(result.is_err(), "pure entry function should be rejected");
    let errors = result.unwrap_err();
    assert!(
        errors.iter().any(|error| {
            matches!(*error,
                TypeError::PurityViolation { ref callee_name, .. }
                if callee_name == "entry"
            )
        }),
        "expected PurityViolation about pure entry declaration, got: {errors:?}"
    );
}

#[test]
fn test_symbol_info_tracks_purity() {
    let pure_fn = Decl::Function {
        name: String::from("pure_symbol_worker"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: return_stmt(literal_expr(LiteralValue::Void, 7_210_000), 7_210_001),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![FunctionModifier::Pure],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_210_002),
        metadata: HotReloadMetadata::for_function(),
    };

    let non_pure_fn = Decl::Function {
        name: String::from("non_pure_symbol_worker"),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![int_type("void")]),
        error_types: Vec::new(),
        body: return_stmt(literal_expr(LiteralValue::Void, 7_210_003), 7_210_004),
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(7_210_005),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![pure_fn, non_pure_fn]);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "program with pure and non-pure functions should type check: {result:?}"
    );

    let pure_symbol = checker
        .symbol_table()
        .lookup("pure_symbol_worker")
        .expect("pure function should be registered");
    assert!(
        pure_symbol.is_pure,
        "pure function symbol should be marked as pure"
    );

    let non_pure_symbol = checker
        .symbol_table()
        .lookup("non_pure_symbol_worker")
        .expect("non-pure function should be registered");
    assert!(
        !non_pure_symbol.is_pure,
        "non-pure function symbol should not be marked as pure"
    );
}

/// Ensure propagate succeeds when the callee exposes a subset of the caller's error list
/// and fails when the callee introduces a new error variant.
#[test]
fn test_propagate_multiple_error_types_subset_and_superset() {
    let subset_program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6100),
        make_unit_type_decl("IoError", 6101),
        make_unit_type_decl("NetworkError", 6102),
        make_function_decl_with_errors(
            "decode_record",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError"],
            return_stmt(literal_expr(LiteralValue::Integer(1), 6103), 6104),
            6105,
        ),
        make_function_decl_with_errors(
            "subset_handler",
            vec![make_parameter("payload", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError", "NetworkError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "value".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6106),
                        },
                        initializer: Some(propagate_call(
                            call_expr("decode_record", &["payload"], 6107),
                            6108,
                        )),
                        span: test_span(),
                        id: node_id(6109),
                    },
                    return_stmt(identifier_expr("value", 6110), 6111),
                ],
                span: test_span(),
                id: node_id(6112),
            },
            6113,
        ),
    ]);

    let mut subset_checker = TypeChecker::new();
    let subset_result = subset_checker.type_check_program(&subset_program);
    assert!(
        subset_result.is_ok(),
        "subset case should succeed: {subset_result:?}"
    );

    let superset_program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6200),
        make_unit_type_decl("IoError", 6201),
        make_function_decl_with_errors(
            "decode_record",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError"],
            return_stmt(literal_expr(LiteralValue::Integer(1), 6202), 6203),
            6204,
        ),
        make_function_decl_with_errors(
            "superset_handler",
            vec![make_parameter("payload", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "value".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6205),
                        },
                        initializer: Some(propagate_call(
                            call_expr("decode_record", &["payload"], 6206),
                            6207,
                        )),
                        span: test_span(),
                        id: node_id(6208),
                    },
                    return_stmt(identifier_expr("value", 6209), 6210),
                ],
                span: test_span(),
                id: node_id(6211),
            },
            6212,
        ),
    ]);

    let mut superset_checker = TypeChecker::new();
    let superset_result = superset_checker
        .type_check_program(&superset_program)
        .expect_err("superset should fail due to missing IoError");
    assert!(
        superset_result
            .into_iter()
            .any(|error| matches!(error, TypeError::PropagateErrorMismatch { .. })),
        "expected PropagateErrorMismatch when propagated errors exceed caller declaration"
    );
}

/// Propagate must reject calls to functions that do not declare error types.
#[test]
fn test_propagate_rejects_empty_error_list() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6300),
        make_function_decl_with_errors(
            "always_ok",
            vec![make_parameter("value", int_type("int32"))],
            Some(int_type("int32")),
            Vec::new(),
            return_stmt(identifier_expr("value", 6301), 6302),
            6303,
        ),
        make_function_decl_with_errors(
            "caller",
            vec![make_parameter("value", int_type("int32"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "result".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6304),
                        },
                        initializer: Some(propagate_call(
                            call_expr("always_ok", &["value"], 6305),
                            6306,
                        )),
                        span: test_span(),
                        id: node_id(6307),
                    },
                    return_stmt(identifier_expr("result", 6308), 6309),
                ],
                span: test_span(),
                id: node_id(6310),
            },
            6311,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate should reject empty error lists");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::PropagateOnNonErrorExpression { .. })),
        "expected PropagateOnNonErrorExpression error when propagating from zero-error callee"
    );
}

/// Nested propagate expressions should type-check when each layer respects error subsets.
#[test]
fn test_propagate_nested_expressions() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6400),
        make_function_decl_with_errors(
            "parse_int",
            vec![make_parameter("value", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(7), 6401), 6402),
            6403,
        ),
        make_function_decl_with_errors(
            "double_checked",
            vec![make_parameter("value", int_type("int32"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(identifier_expr("value", 6404), 6405),
            6406,
        ),
        make_function_decl_with_errors(
            "pipeline",
            vec![make_parameter("raw", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "parsed".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6407),
                        },
                        initializer: Some(propagate_call(
                            call_expr("parse_int", &["raw"], 6408),
                            6409,
                        )),
                        span: test_span(),
                        id: node_id(6410),
                    },
                    Stmt::Let {
                        binding: LetBinding {
                            name: "doubled".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6411),
                        },
                        initializer: Some(propagate_call(
                            Expr::Call {
                                callee: Box::new(identifier_expr("double_checked", 6412)),
                                generic_args: None,
                                args: vec![propagate_call(
                                    call_expr("parse_int", &["raw"], 6413),
                                    6414,
                                )],
                                span: test_span(),
                                id: node_id(6415),
                            },
                            6416,
                        )),
                        span: test_span(),
                        id: node_id(6417),
                    },
                    return_stmt(identifier_expr("doubled", 6418), 6419),
                ],
                span: test_span(),
                id: node_id(6420),
            },
            6421,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "nested propagate expressions should succeed: {result:?}"
    );
}

/// Ensure error type name mismatches are treated as incompatible even if their structure matches.
#[test]
fn test_propagate_rejects_structurally_identical_error_names() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6500),
        make_unit_type_decl("ParseProblem", 6501),
        make_function_decl_with_errors(
            "parse_problematic",
            vec![make_parameter("value", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseProblem"],
            return_stmt(literal_expr(LiteralValue::Integer(42), 6502), 6503),
            6504,
        ),
        make_function_decl_with_errors(
            "parse_wrapper",
            vec![make_parameter("value", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "parsed".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(6505),
                        },
                        initializer: Some(propagate_call(
                            call_expr("parse_problematic", &["value"], 6506),
                            6507,
                        )),
                        span: test_span(),
                        id: node_id(6508),
                    },
                    return_stmt(identifier_expr("parsed", 6509), 6510),
                ],
                span: test_span(),
                id: node_id(6511),
            },
            6512,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate should fail when error names differ");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::PropagateErrorMismatch { .. })),
        "expected PropagateErrorMismatch when callee error name differs"
    );
}

/// Propagate should be permitted inside lambdas that declare compatible error sets.
#[test]
fn test_propagate_inside_lambda_with_errors() {
    let lambda = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: vec![make_parameter("input", int_type("string"))],
        return_types: vec![int_type("int32")],
        error_types: vec!["ParseError".to_owned()],
        body: LambdaBody::Block(vec![Stmt::Return {
            values: vec![LabeledValue {
                label: String::new(),
                value: propagate_call(call_expr("parse_value", &["input"], 6601), 6602),
                span: test_span(),
                id: node_id(6603),
            }],
            span: test_span(),
            id: node_id(6603),
        }]),
        captured_variables: Vec::new(),
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(6604),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6600),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 6605), 6606),
            6607,
        ),
        Decl::Let {
            binding: LetBinding {
                name: "handler".to_owned(),
                type_annotation: Some(Type::Function {
                    parameters: vec![int_type("string")],
                    return_types: vec![int_type("int32")],
                    errors: Some(vec![Type::Basic {
                        name: "ParseError".to_owned(),
                        span: test_span(),
                    }]),
                    span: test_span(),
                }),
                is_mutable: false,
                span: test_span(),
                id: node_id(6608),
            },
            initializer: lambda,
            visibility: AstVisibility::Private,
            doc_comment: None,
            span: test_span(),
            id: node_id(6609),
            metadata: HotReloadMetadata::for_let_declaration(),
        },
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "lambda propagate should succeed: {result:?}"
    );
}

/// Propagate diagnostics should point to both the caller signature and the failing callee call site.
#[test]
fn test_propagate_error_span_accuracy() {
    let propagate_span = span_with_offset(100, 9);
    let call_span = span_with_offset(120, 8);

    let failing_call = Expr::Call {
        callee: Box::new(identifier_expr("read_file", 6701)),
        generic_args: None,
        args: vec![identifier_expr("path", 6702)],
        span: call_span,
        id: node_id(6703),
    };
    let failing_propagate = Expr::Propagate {
        call: Box::new(failing_call),
        span: propagate_span,
        id: node_id(6704),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6700),
        make_unit_type_decl("IoError", 6705),
        make_function_decl_with_errors(
            "read_file",
            vec![make_parameter("path", int_type("string"))],
            Some(int_type("string")),
            vec!["IoError"],
            return_stmt(
                literal_expr(LiteralValue::String(String::new()), 6706),
                6707,
            ),
            6708,
        ),
        make_function_decl_with_errors(
            "load_config",
            vec![make_parameter("path", int_type("string"))],
            Some(int_type("string")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![Stmt::Return {
                    values: vec![LabeledValue {
                        label: String::new(),
                        value: failing_propagate,
                        span: test_span(),
                        id: node_id(6709),
                    }],
                    span: test_span(),
                    id: node_id(6709),
                }],
                span: span_with_offset(10, 5),
                id: node_id(6710),
            },
            6711,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate mismatch should emit diagnostic");
    let mut found = false;
    for error in errors {
        if let TypeError::PropagateErrorMismatch {
            span, callee_span, ..
        } = error
        {
            assert_eq!(
                span,
                TypeError::span_from_span(program.declarations[3].span_const()),
                "function span should map to TypeError span"
            );
            assert_eq!(
                callee_span,
                TypeError::span_from_span(call_span),
                "callee span should reference call site"
            );
            found = true;
        }
    }
    assert!(
        found,
        "expected PropagateErrorMismatch with span information"
    );
}

/// Propagate error mismatch diagnostics should render human-readable error names.
#[test]
fn test_propagate_error_mismatch_reports_readable_types() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6750),
        make_unit_type_decl("IoError", 6751),
        make_function_decl_with_errors(
            "read_file",
            vec![make_parameter("path", int_type("string"))],
            Some(int_type("string")),
            vec!["IoError"],
            return_stmt(
                literal_expr(LiteralValue::String(String::new()), 6752),
                6753,
            ),
            6754,
        ),
        make_function_decl_with_errors(
            "load_config",
            vec![make_parameter("path", int_type("string"))],
            Some(int_type("string")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![Stmt::Return {
                    values: vec![LabeledValue {
                        label: String::new(),
                        value: propagate_call(call_expr("read_file", &["path"], 6755), 6756),
                        span: test_span(),
                        id: node_id(6757),
                    }],
                    span: test_span(),
                    id: node_id(6757),
                }],
                span: test_span(),
                id: node_id(6758),
            },
            6759,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate mismatch should emit readable diagnostic");

    let mismatch = errors
        .into_iter()
        .find_map(|error| match error {
            TypeError::PropagateErrorMismatch { .. } => Some(error),
            _ => None,
        })
        .expect("expected PropagateErrorMismatch diagnostic");

    if let TypeError::PropagateErrorMismatch {
        expected, found, ..
    } = mismatch
    {
        assert_eq!(
            expected.as_str(),
            "ParseError",
            "caller error list should be user-facing"
        );
        assert_eq!(
            found.as_str(),
            "IoError",
            "callee error list should be user-facing"
        );
    }
}

/// Guard must operate on fallible call expressions; guarding identifiers should be rejected.
#[test]
fn test_guard_requires_call_expression() {
    let guard_expr = Expr::Guard {
        expr: Box::new(identifier_expr("parse_value", 6800)),
        binding_name: "value".to_owned(),
        binding_type: Some(int_type("int32")),
        is_mutable: false,
        else_branch: Box::new(Stmt::Expression {
            expr: literal_expr(LiteralValue::Integer(0), 6801),
            span: test_span(),
            id: node_id(6802),
        }),
        span: test_span(),
        id: node_id(6803),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6804),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(1), 6805), 6806),
            6807,
        ),
        make_function_decl_with_errors(
            "wrapper",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(6808),
                    },
                    return_stmt(literal_expr(LiteralValue::Integer(2), 6809), 6810),
                ],
                span: test_span(),
                id: node_id(6811),
            },
            6812,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guarding identifiers should fail");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardOnNonErrorExpression { .. })),
        "expected GuardOnNonErrorExpression when guarding a non-call"
    );
}

/// Guard should reject callees that cannot error.
#[test]
fn test_guard_rejects_empty_error_list() {
    let guard_expr = guard_call_expr(
        call_expr("always_ok", &["input"], 6900),
        "value",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: literal_expr(LiteralValue::Integer(0), 6901),
            span: test_span(),
            id: node_id(6902),
        },
        6903,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 6904),
        make_function_decl_with_errors(
            "always_ok",
            vec![make_parameter("input", int_type("int32"))],
            Some(int_type("int32")),
            Vec::new(),
            return_stmt(identifier_expr("input", 6905), 6906),
            6907,
        ),
        make_function_decl_with_errors(
            "caller",
            vec![make_parameter("input", int_type("int32"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(6908),
                    },
                    return_stmt(identifier_expr("input", 6909), 6910),
                ],
                span: test_span(),
                id: node_id(6911),
            },
            6912,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guard should reject callees without errors");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardOnNonErrorExpression { .. })),
        "expected GuardOnNonErrorExpression when guarding zero-error callee"
    );
}

/// Guard success bindings should be available after the guard statement in the surrounding scope.
#[test]
fn test_guard_binding_available_after_guard() {
    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7000),
        "value",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: literal_expr(LiteralValue::Void, 7001),
            span: test_span(),
            id: node_id(7002),
        },
        7003,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7004),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(8), 7005), 7006),
            7007,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(7008),
                    },
                    return_stmt(identifier_expr("value", 7009), 7010),
                ],
                span: test_span(),
                id: node_id(7011),
            },
            7012,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard binding should be usable after guard: {result:?}"
    );
}

#[test]
fn test_guard_statement_success_binding_is_hidden_inside_else_clause() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): void =>
            guard string_to_int32('5') into value else err =>
                let leaked: int32 = value
                return void
            return void
        ",
    );

    let mut checker = TypeChecker::new();
    let errors = checker.type_check_program(&program).expect_err(
        "statement guard success binding should not be visible inside the error clause",
    );
    let error_text = format!("{errors:?}");
    assert!(
        error_text.contains("success binding is not available inside guard error clause"),
        "expected guard success-binding scope diagnostic, got: {error_text}"
    );
}

#[test]
fn test_guard_statement_binds_success_and_error_types() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): void errors ParseError =>
            guard string_to_int32('5') into n else e =>
                let err_value: ParseError = e
                propagate e
            let parsed: int32 = n
            return void
        ",
    );

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard statement should bind success value and real error type in expected scopes: {result:?}"
    );
}

#[test]
fn type_check_guard_shorthand_discards_success_binding() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): void errors ParseError =>
            guard string_to_int32('5') else err =>
                let err_value: ParseError = err
                propagate err
            return void
        ",
    );

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard shorthand without success binding should type-check while preserving else error binding: {result:?}"
    );
}

#[test]
fn type_check_guard_shorthand_success_binding_not_in_scope() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): int32 errors ParseError =>
            guard string_to_int32('5') else err =>
                let err_value: ParseError = err
                propagate err
            return n
        ",
    );

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("omitted guard success binding should not introduce a success symbol");
    assert!(
        errors.into_iter().any(
            |error| matches!(error, TypeError::SymbolNotFound { ref name, .. } if name == "n")
        ),
        "expected SymbolNotFound for omitted success binding"
    );
}

#[test]
fn type_check_named_guard_binding_still_available_after_guard() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): int32 errors ParseError =>
            guard string_to_int32('5') into n else err =>
                let err_value: ParseError = err
                propagate err
            return n
        ",
    );

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "named guard success binding should remain available after guard statement: {result:?}"
    );
}

#[test]
fn type_check_guard_into_underscore_still_valid() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): void errors ParseError =>
            guard string_to_int32('5') into _ else err =>
                let err_value: ParseError = err
                propagate err
            return void
        ",
    );

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "explicit guard discard binding into _ should remain valid: {result:?}"
    );
}

#[test]
fn type_check_guard_into_underscore_does_not_introduce_binding_after_guard() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): int32 errors ParseError =>
            guard string_to_int32('5') into _ else err =>
                let err_value: ParseError = err
                propagate err
            return _
        ",
    );

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("discard guard binding into _ should not introduce a usable success symbol");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::SymbolNotFound { name, .. } if name == "_")),
        "expected SymbolNotFound for discarded success binding, got: {errors:?}"
    );
}

#[test]
fn test_guard_statement_multi_error_binding_stays_contextual() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7_519_900),
        make_unit_type_decl("IoError", 7_519_901),
        make_function_decl_with_errors(
            "parse_or_load",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError"],
            return_stmt(literal_expr(LiteralValue::Integer(1), 7_519_902), 7_519_903),
            7_519_904,
        ),
        make_function_decl_with_errors(
            "worker",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError", "IoError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Guard {
                        expression: Box::new(call_expr("parse_or_load", &["input"], 7_519_905)),
                        success_binding: Some("value".to_owned()),
                        success_binding_type: Some(int_type("int32")),
                        success_binding_is_mutable: false,
                        error_binding: "err".to_owned(),
                        else_body: Box::new(Stmt::Block {
                            statements: vec![Stmt::Let {
                                binding: LetBinding {
                                    name: "copy".to_owned(),
                                    type_annotation: Some(int_type("ParseError")),
                                    is_mutable: false,
                                    span: test_span(),
                                    id: node_id(7_519_906),
                                },
                                initializer: Some(identifier_expr("err", 7_519_907)),
                                span: test_span(),
                                id: node_id(7_519_908),
                            }],
                            span: test_span(),
                            id: node_id(7_519_909),
                        }),
                        span: test_span(),
                        id: node_id(7_519_910),
                    },
                    return_stmt(literal_expr(LiteralValue::Void, 7_519_911), 7_519_912),
                ],
                span: test_span(),
                id: node_id(7_519_913),
            },
            7_519_914,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker.type_check_program(&program).expect_err(
        "multi-error guard binding should not masquerade as a concrete single error type",
    );
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::TypeMismatch { .. })),
        "expected multi-error guard binding to reject narrowing into a concrete single error type, got: {errors:?}"
    );
}

#[test]
fn test_guard_statement_return_err_uses_dedicated_guard_diagnostic() {
    let error = TypeError::GuardReturnErrInvalid {
        return_span: TypeError::span_from_span(test_span()),
    };

    assert_eq!(
        error
            .code()
            .map(|diagnostic_code| diagnostic_code.to_string())
            .as_deref(),
        Some("opalescent::guard::return_err_invalid"),
        "strict guard return-err diagnostic should expose the exact diagnostic code"
    );
    assert!(
        error.help().is_some(),
        "strict guard return-err diagnostic should expose help text"
    );
    assert!(
        error
            .labels()
            .expect("guard return-err diagnostic should have a labeled span")
            .next()
            .is_some(),
        "strict guard return-err diagnostic should include at least one labeled span"
    );
    let rendered = render_diagnostic("test.op", "return err", &error);
    assert!(
        rendered.contains("opalescent::guard::return_err_invalid"),
        "rendered strict guard diagnostic should include the diagnostic code"
    );
    assert!(
        rendered.contains("returning `err` directly loses the required guard propagation shape"),
        "rendered strict guard diagnostic should include label text"
    );
}

#[test]
fn test_strict_guard_diagnostic_codes_and_labels() {
    let diagnostics: Vec<(TypeError, &str, &str, &str, &[&str])> = vec![
        (
            TypeError::GuardErrorClauseMissingTerminal {
                clause_span: TypeError::span_from_span(test_span()),
            },
            "opalescent::guard::missing_terminal",
            "A named `else err =>` clause must end by forwarding `err` with `propagate err`, or by returning an error wrapper whose `source` field is exactly `err`. Logging, cleanup, fallback values, and `return void` do not count as handling the error.",
            "this clause exits without propagating or wrapping the bound error",
            &["propagate err", "source", "return void"],
        ),
        (
            TypeError::GuardPropagateErrNotFinal {
                propagate_span: TypeError::span_from_span(test_span()),
            },
            "opalescent::guard::propagate_not_final",
            "Move `propagate err` to the end of the `else err =>` body, or use `let value = propagate fallible_call()` when you only want to forward a fallible call outside a guard handler.",
            "this propagation is not the terminal action for the bound guard error",
            &["propagate err", "fallible_call", "guard handler"],
        ),
        (
            TypeError::GuardReturnErrInvalid {
                return_span: TypeError::span_from_span(test_span()),
            },
            "opalescent::guard::return_err_invalid",
            "Use `propagate err` to forward the exact bound error, or wrap it explicitly with `return new YourError.Variant: source: err` if the caller expects a higher-level error type.",
            "returning `err` directly loses the required guard propagation shape",
            &["propagate err", "YourError.Variant", "source"],
        ),
        (
            TypeError::GuardWrapperSourceInvalid {
                source_span: TypeError::span_from_span(test_span()),
            },
            "opalescent::guard::wrapper_source_invalid",
            "When wrapping a guard error, write the source field as exactly `source: err`. Aliases, shadowed bindings, and unrelated expressions are rejected so the original error flow stays explicit.",
            "expected this wrapper source to be the active guard error binding",
            &["source", "Aliases", "original error flow"],
        ),
        (
            TypeError::GuardShorthandRequired {
                span: TypeError::span_from_span(test_span()),
            },
            "opalescent::guard::shorthand_required",
            "If the handler only forwards the fallible result, prefer `let value = propagate fallible_call()` over `guard ... else err => propagate err`. Use a named guard only when you add context before the terminal propagation or wrapper return.",
            "this guard handler only rethrows; shorthand propagation is clearer",
            &[
                "propagate fallible_call",
                "guard ... else err",
                "named guard",
            ],
        ),
    ];

    for (error, expected_code, expected_help, expected_label, rendered_help_snippets) in diagnostics
    {
        assert_eq!(
            error
                .code()
                .map(|diagnostic_code| diagnostic_code.to_string())
                .as_deref(),
            Some(expected_code),
            "strict guard diagnostics should expose exact diagnostic codes"
        );
        let help = error.help().map(|help| help.to_string());
        assert_eq!(
            help.as_deref(),
            Some(expected_help),
            "strict guard diagnostics should expose the expected help text"
        );
        assert!(
            error
                .labels()
                .expect("strict guard diagnostics should have a labeled span")
                .next()
                .is_some(),
            "strict guard diagnostics should include at least one labeled span"
        );
        let rendered = render_diagnostic("test.op", "guard call() else err => return err", &error);
        assert!(
            rendered.contains(expected_code),
            "rendered strict guard diagnostic should include its diagnostic code"
        );
        for snippet in rendered_help_snippets {
            assert!(
                rendered.contains(snippet),
                "rendered strict guard diagnostic should include help snippet `{snippet}`"
            );
        }
        assert!(
            rendered.contains(expected_label),
            "rendered strict guard diagnostic should include its label text"
        );
    }
}

/// Guard used as an expression should reject else branches whose fallback type
/// does not match the guarded call's success type.
#[test]
fn test_guard_else_expression_requires_matching_success_type() {
    let mismatched_else = Stmt::Expression {
        expr: literal_expr(LiteralValue::Boolean(true), 7201),
        span: test_span(),
        id: node_id(7202),
    };

    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7200),
        "value",
        Some(int_type("int32")),
        false,
        mismatched_else,
        7203,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "result".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7204),
        },
        initializer: Some(guard_expr),
        span: test_span(),
        id: node_id(7205),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7206),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(9), 7207), 7208),
            7209,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("result", 7210), 7211),
                ],
                span: test_span(),
                id: node_id(7212),
            },
            7213,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guard expression fallback with mismatched type should error");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardElseIncompatibleError { .. })),
        "expected GuardElseIncompatibleError when fallback type mismatches"
    );
}

/// Guard used as an expression should allow else branches that produce a fallback
/// value matching the success type of the guarded call.
#[test]
fn test_guard_else_expression_allows_matching_success_type() {
    let matching_else = Stmt::Expression {
        expr: literal_expr(LiteralValue::Integer(42), 7220),
        span: test_span(),
        id: node_id(7221),
    };

    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7219),
        "value",
        Some(int_type("int32")),
        false,
        matching_else,
        7222,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "result".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7223),
        },
        initializer: Some(guard_expr),
        span: test_span(),
        id: node_id(7224),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7225),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(7), 7226), 7227),
            7228,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("result", 7229), 7230),
                ],
                span: test_span(),
                id: node_id(7231),
            },
            7232,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard fallback matching success type should pass: {result:?}"
    );
}

/// Guard fallback expressions cannot currently support heterogeneous error sets;
/// ensure a helpful diagnostic is emitted so future union support can build on it.
#[test]
fn test_guard_else_expression_rejects_heterogeneous_error_sets() {
    let fallback_else = Stmt::Expression {
        expr: literal_expr(LiteralValue::Integer(11), 7240),
        span: test_span(),
        id: node_id(7241),
    };

    let guard_expr = guard_call_expr(
        call_expr("parse_pair", &["input"], 7239),
        "value",
        Some(int_type("int32")),
        false,
        fallback_else,
        7242,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "result".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7243),
        },
        initializer: Some(guard_expr),
        span: test_span(),
        id: node_id(7244),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7245),
        make_unit_type_decl("IoError", 7246),
        make_function_decl_with_errors(
            "parse_pair",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7247), 7248),
            7249,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError", "IoError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("result", 7250), 7251),
                ],
                span: test_span(),
                id: node_id(7252),
            },
            7253,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("heterogeneous error sets should reject guard fallback expressions");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardElseIncompatibleError { .. })),
        "expected GuardElseIncompatibleError when guard handles heterogeneous errors"
    );
}

/// Guard used as a statement should require the else branch to match the declared
/// error types (unit aliases in this phase), rejecting mismatched handler types.
#[test]
fn test_guard_statement_else_expression_requires_unit() {
    let mismatched_else = Stmt::Expression {
        expr: literal_expr(LiteralValue::Integer(13), 7260),
        span: test_span(),
        id: node_id(7261),
    };

    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7259),
        "value",
        Some(int_type("int32")),
        false,
        mismatched_else,
        7262,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7263),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(12), 7264), 7265),
            7266,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(7267),
                    },
                    return_stmt(identifier_expr("value", 7268), 7269),
                ],
                span: test_span(),
                id: node_id(7270),
            },
            7271,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guard statement handler should reject non-unit else expression");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardElseIncompatibleError { .. })),
        "expected GuardElseIncompatibleError for guard statement mismatched handler"
    );
}

/// Guard used as a statement should accept else branches that resolve to `unit`,
/// such as calling a logging helper.
#[test]
fn test_guard_statement_else_expression_accepts_unit_handler() {
    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7279),
        "value",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: call_expr("log_error", &[], 7280),
            span: test_span(),
            id: node_id(7281),
        },
        7282,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7283),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(21), 7284), 7285),
            7286,
        ),
        make_function_decl(
            "log_error",
            Vec::new(),
            Some(int_type("unit")),
            return_stmt(literal_expr(LiteralValue::Void, 7287), 7288),
            7289,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(7290),
                    },
                    return_stmt(identifier_expr("value", 7291), 7292),
                ],
                span: test_span(),
                id: node_id(7293),
            },
            7294,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "unit-valued else handler should be accepted: {result:?}"
    );
}

/// Guard used as a statement should allow block handlers that perform control-flow
/// operations such as logging before returning.
#[test]
fn test_guard_statement_allows_block_handler() {
    let else_block = Stmt::Block {
        statements: vec![
            Stmt::Expression {
                expr: call_expr("log_error", &[], 7301),
                span: test_span(),
                id: node_id(7302),
            },
            Stmt::Return {
                values: vec![LabeledValue {
                    label: String::new(),
                    value: literal_expr(LiteralValue::Void, 7303),
                    span: test_span(),
                    id: node_id(7304),
                }],
                span: test_span(),
                id: node_id(7304),
            },
        ],
        span: test_span(),
        id: node_id(7300),
    };

    let guard_expr = guard_call_expr(
        call_expr("parse_value", &["input"], 7305),
        "value",
        Some(int_type("int32")),
        false,
        else_block,
        7306,
    );

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7307),
        make_function_decl_with_errors(
            "parse_value",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(33), 7308), 7309),
            7310,
        ),
        make_function_decl(
            "log_error",
            Vec::new(),
            Some(int_type("unit")),
            return_stmt(literal_expr(LiteralValue::Void, 7311), 7312),
            7313,
        ),
        make_function_decl_with_errors(
            "wrap_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Expression {
                        expr: guard_expr,
                        span: test_span(),
                        id: node_id(7314),
                    },
                    return_stmt(identifier_expr("value", 7315), 7316),
                ],
                span: test_span(),
                id: node_id(7317),
            },
            7318,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "block else handler should be permitted: {result:?}"
    );
}

/// Guard else branches must not introduce new error types via nested guard expressions.
#[test]
fn test_guard_else_rejects_chained_guard_with_mismatched_errors() {
    let nested_guard = guard_call_expr(
        call_expr("load_defaults", &[], 7401),
        "fallback",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: literal_expr(LiteralValue::Integer(0), 7402),
            span: test_span(),
            id: node_id(7403),
        },
        7404,
    );

    let outer_guard = guard_call_expr(
        call_expr("load_config", &[], 7400),
        "config",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: nested_guard,
            span: test_span(),
            id: node_id(7405),
        },
        7406,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "config".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7407),
        },
        initializer: Some(outer_guard),
        span: test_span(),
        id: node_id(7408),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ConfigError", 7409),
        make_unit_type_decl("DefaultError", 7410),
        make_function_decl_with_errors(
            "load_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(7), 7411), 7412),
            7413,
        ),
        make_function_decl_with_errors(
            "load_defaults",
            Vec::new(),
            Some(int_type("int32")),
            vec!["DefaultError"],
            return_stmt(literal_expr(LiteralValue::Integer(3), 7414), 7415),
            7416,
        ),
        make_function_decl_with_errors(
            "initialize_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("config", 7417), 7418),
                ],
                span: test_span(),
                id: node_id(7419),
            },
            7420,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("nested guard with mismatched error types should error");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardChainedErrorMismatch { .. })),
        "expected GuardChainedErrorMismatch diagnostic"
    );
}

/// Guard else branch may include a nested guard when both guards manage identical error sets.
#[test]
fn test_guard_else_allows_chained_guard_with_identical_errors() {
    let nested_guard = guard_call_expr(
        call_expr("load_defaults", &[], 7431),
        "fallback",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: literal_expr(LiteralValue::Integer(0), 7432),
            span: test_span(),
            id: node_id(7433),
        },
        7434,
    );

    let outer_guard = guard_call_expr(
        call_expr("load_config", &[], 7430),
        "config",
        Some(int_type("int32")),
        false,
        Stmt::Expression {
            expr: nested_guard,
            span: test_span(),
            id: node_id(7435),
        },
        7436,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "config".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7437),
        },
        initializer: Some(outer_guard),
        span: test_span(),
        id: node_id(7438),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ConfigError", 7439),
        make_function_decl_with_errors(
            "load_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(9), 7440), 7441),
            7442,
        ),
        make_function_decl_with_errors(
            "load_defaults",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7443), 7444),
            7445,
        ),
        make_function_decl_with_errors(
            "initialize_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("config", 7446), 7447),
                ],
                span: test_span(),
                id: node_id(7448),
            },
            7449,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "nested guard with matching error types should succeed: {result:?}"
    );
}

/// Guard else branch must not use `propagate` to surface mismatched error types.
#[test]
fn test_guard_else_rejects_propagate_with_mismatched_errors() {
    let propagate_else = Stmt::Expression {
        expr: propagate_call(call_expr("load_defaults", &[], 7461), 7462),
        span: test_span(),
        id: node_id(7463),
    };

    let guard_expr = guard_call_expr(
        call_expr("load_config", &[], 7460),
        "config",
        Some(int_type("int32")),
        false,
        propagate_else,
        7464,
    );

    let let_guard = Stmt::Let {
        binding: LetBinding {
            name: "config".to_owned(),
            type_annotation: Some(int_type("int32")),
            is_mutable: false,
            span: test_span(),
            id: node_id(7465),
        },
        initializer: Some(guard_expr),
        span: test_span(),
        id: node_id(7466),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ConfigError", 7467),
        make_unit_type_decl("DefaultError", 7468),
        make_function_decl_with_errors(
            "load_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(13), 7469), 7470),
            7471,
        ),
        make_function_decl_with_errors(
            "load_defaults",
            Vec::new(),
            Some(int_type("int32")),
            vec!["DefaultError"],
            return_stmt(literal_expr(LiteralValue::Integer(21), 7472), 7473),
            7474,
        ),
        make_function_decl_with_errors(
            "initialize_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            Stmt::Block {
                statements: vec![
                    let_guard,
                    return_stmt(identifier_expr("config", 7475), 7476),
                ],
                span: test_span(),
                id: node_id(7477),
            },
            7478,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate with mismatched errors inside guard else should error");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardChainedErrorMismatch { .. })),
        "expected GuardChainedErrorMismatch diagnostic"
    );
}

/// Guard used as a statement should permit propagate handlers when error sets match.
#[test]
fn test_guard_statement_else_allows_matching_propagate() {
    let guard_stmt = Stmt::Expression {
        expr: guard_call_expr(
            call_expr("load_config", &[], 7480),
            "config",
            Some(int_type("int32")),
            false,
            Stmt::Expression {
                expr: propagate_call(call_expr("load_defaults", &[], 7481), 7482),
                span: test_span(),
                id: node_id(7483),
            },
            7484,
        ),
        span: test_span(),
        id: node_id(7485),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ConfigError", 7486),
        make_function_decl_with_errors(
            "load_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(17), 7487), 7488),
            7489,
        ),
        make_function_decl_with_errors(
            "load_defaults",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(23), 7490), 7491),
            7492,
        ),
        make_function_decl_with_errors(
            "initialize",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Integer(5), 7493), 7494),
                ],
                span: test_span(),
                id: node_id(7495),
            },
            7496,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard statement else should accept propagate with matching errors: {result:?}"
    );
}

/// Guard statement `else` should still reject propagate expressions when error sets differ.
#[test]
fn test_guard_statement_else_rejects_mismatched_propagate_errors() {
    let guard_stmt = Stmt::Expression {
        expr: guard_call_expr(
            call_expr("load_config", &[], 7500),
            "config",
            Some(int_type("int32")),
            false,
            Stmt::Expression {
                expr: propagate_call(call_expr("load_defaults", &[], 7501), 7502),
                span: test_span(),
                id: node_id(7503),
            },
            7504,
        ),
        span: test_span(),
        id: node_id(7505),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ConfigError", 7506),
        make_unit_type_decl("DefaultError", 7507),
        make_function_decl_with_errors(
            "load_config",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            return_stmt(literal_expr(LiteralValue::Integer(31), 7508), 7509),
            7510,
        ),
        make_function_decl_with_errors(
            "load_defaults",
            Vec::new(),
            Some(int_type("int32")),
            vec!["DefaultError"],
            return_stmt(literal_expr(LiteralValue::Integer(42), 7511), 7512),
            7513,
        ),
        make_function_decl_with_errors(
            "initialize",
            Vec::new(),
            Some(int_type("int32")),
            vec!["ConfigError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Integer(0), 7514), 7515),
                ],
                span: test_span(),
                id: node_id(7516),
            },
            7517,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("expected guard-propagate mismatch to error");
    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::GuardChainedErrorMismatch { .. })),
        "expected GuardChainedErrorMismatch when guard statement else propagates mismatched errors"
    );
}

#[test]
fn test_guard_error_clause_success_binding_does_not_leak_over_outer_shadowing() {
    let leaking_guard_stmt = Stmt::Expression {
        expr: guard_call_expr(
            call_expr("string_to_int32", &["input"], 7600),
            "value",
            Some(int_type("int32")),
            false,
            Stmt::Block {
                statements: vec![Stmt::Let {
                    binding: LetBinding {
                        name: "seen".to_owned(),
                        type_annotation: Some(int_type("string")),
                        is_mutable: false,
                        span: test_span(),
                        id: node_id(7601),
                    },
                    initializer: Some(identifier_expr("value", 7602)),
                    span: test_span(),
                    id: node_id(7603),
                }],
                span: test_span(),
                id: node_id(7604),
            },
            7605,
        ),
        span: test_span(),
        id: node_id(7606),
    };

    let shadowing_guard_stmt = Stmt::Expression {
        expr: guard_call_expr(
            call_expr("string_to_int32", &["input"], 7618),
            "value",
            Some(int_type("int32")),
            false,
            Stmt::Block {
                statements: vec![Stmt::Let {
                    binding: LetBinding {
                        name: "shadow_copy".to_owned(),
                        type_annotation: Some(int_type("string")),
                        is_mutable: false,
                        span: test_span(),
                        id: node_id(7619),
                    },
                    initializer: Some(identifier_expr("value", 7620)),
                    span: test_span(),
                    id: node_id(7621),
                }],
                span: test_span(),
                id: node_id(7622),
            },
            7623,
        ),
        span: test_span(),
        id: node_id(7624),
    };

    let leaking_program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7625),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(1), 7626), 7627),
            7628,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    leaking_guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7629), 7630),
                ],
                span: test_span(),
                id: node_id(7631),
            },
            7632,
        ),
    ]);

    let mut leak_checker = TypeChecker::new();
    let errors = leak_checker
        .type_check_program(&leaking_program)
        .expect_err("guard success binding should not be available inside the error clause");
    let error_text = format!("{errors:?}");
    assert!(
        error_text.contains("success binding is not available inside guard error clause"),
        "expected scope diagnostic for guard success binding leak, got: {error_text}"
    );

    let shadowing_program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7633),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(2), 7634), 7635),
            7636,
        ),
        make_function_decl_with_errors(
            "use_shadowed_outer",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "value".to_owned(),
                            type_annotation: Some(int_type("string")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(7637),
                        },
                        initializer: Some(identifier_expr("input", 7638)),
                        span: test_span(),
                        id: node_id(7639),
                    },
                    shadowing_guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7640), 7641),
                ],
                span: test_span(),
                id: node_id(7642),
            },
            7643,
        ),
    ]);

    let mut shadow_checker = TypeChecker::new();
    let shadow_result = shadow_checker.type_check_program(&shadowing_program);
    assert!(
        shadow_result.is_ok(),
        "outer lexical variable with same name should remain visible in guard error clause: {shadow_result:?}"
    );
}

#[test]
fn test_guard_error_binding_is_not_available_after_guard() {
    let program = parse_program_from_source_with_spaces(
        "
        entry main = f(): ParseError errors ParseError =>
            guard string_to_int32('5') into value else err =>
                let current_error: ParseError = err
                propagate err
            return err
        ",
    );

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guard error binding should not be available after the guard clause");
    assert!(
        errors.into_iter().any(
            |error| matches!(error, TypeError::SymbolNotFound { ref name, .. } if name == "err")
        ),
        "expected SymbolNotFound when using guard error binding after guard clause"
    );
}

#[test]
fn test_guard_error_clause_return_err_is_rejected() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7620)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![return_stmt(identifier_expr("err", 7621), 7622)],
            span: test_span(),
            id: node_id(7623),
        }),
        span: test_span(),
        id: node_id(7625),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7626),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(2), 7627), 7628),
            7629,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7630), 7631),
                ],
                span: test_span(),
                id: node_id(7632),
            },
            7633,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("return err should be rejected in a guard error clause");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardReturnErrInvalid { .. })),
        "expected GuardReturnErrInvalid diagnostic, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_propagate_err_must_be_terminal() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7640)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![
                Stmt::PropagateGuardError {
                    error_binding: "err".to_owned(),
                    span: test_span(),
                    id: node_id(7643),
                },
                Stmt::Expression {
                    expr: literal_expr(LiteralValue::Void, 7644),
                    span: test_span(),
                    id: node_id(7645),
                },
            ],
            span: test_span(),
            id: node_id(7646),
        }),
        span: test_span(),
        id: node_id(7648),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7649),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(3), 7650), 7651),
            7652,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7653), 7654),
                ],
                span: test_span(),
                id: node_id(7655),
            },
            7656,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("propagate err should be terminal inside guard error clauses");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardPropagateErrNotFinal { .. })),
        "expected GuardPropagateErrNotFinal diagnostic, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_only_propagate_is_rejected() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7660)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::PropagateGuardError {
            error_binding: "err".to_owned(),
            span: test_span(),
            id: node_id(7663),
        }),
        span: test_span(),
        id: node_id(7665),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7666),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(4), 7667), 7668),
            7669,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7670), 7671),
                ],
                span: test_span(),
                id: node_id(7672),
            },
            7673,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("only-propagate guard error clauses should be rejected");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardShorthandRequired { .. })),
        "expected GuardShorthandRequired diagnostic, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_side_effect_then_propagate_err_is_allowed() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7674)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![
                Stmt::Let {
                    binding: LetBinding {
                        name: "seen_error".to_owned(),
                        type_annotation: Some(Type::Basic {
                            name: "ParseError".to_owned(),
                            span: test_span(),
                        }),
                        is_mutable: false,
                        span: test_span(),
                        id: node_id(7675),
                    },
                    initializer: Some(identifier_expr("err", 7676)),
                    span: test_span(),
                    id: node_id(7677),
                },
                Stmt::PropagateGuardError {
                    error_binding: "err".to_owned(),
                    span: test_span(),
                    id: node_id(7678),
                },
            ],
            span: test_span(),
            id: node_id(7679),
        }),
        span: test_span(),
        id: node_id(7681),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7682),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7683), 7684),
            7685,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7686), 7687),
                ],
                span: test_span(),
                id: node_id(7688),
            },
            7689,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard error clause should allow handling before final propagate err: {result:?}"
    );
}

#[test]
fn test_guard_error_clause_must_handle_or_propagate_bound_error() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7680)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![Stmt::Expression {
                expr: literal_expr(LiteralValue::Void, 7681),
                span: test_span(),
                id: node_id(7682),
            }],
            span: test_span(),
            id: node_id(7683),
        }),
        span: test_span(),
        id: node_id(7685),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7686),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7687), 7688),
            7689,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7690), 7691),
                ],
                span: test_span(),
                id: node_id(7692),
            },
            7693,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("guard error clauses must handle or propagate the bound error");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardErrorClauseMissingTerminal { .. })),
        "expected GuardErrorClauseMissingTerminal diagnostic, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_rejects_void_return_after_side_effect() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7694)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![
                Stmt::Expression {
                    expr: call_expr("record_error", &["err"], 7695),
                    span: test_span(),
                    id: node_id(7696),
                },
                return_stmt(literal_expr(LiteralValue::Void, 7697), 7698),
            ],
            span: test_span(),
            id: node_id(7699),
        }),
        span: test_span(),
        id: node_id(7700),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7701),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7702), 7703),
            7704,
        ),
        make_function_decl(
            "record_error",
            vec![make_parameter("error", int_type("ParseError"))],
            Some(int_type("void")),
            return_stmt(literal_expr(LiteralValue::Void, 7705), 7706),
            7707,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("void")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(literal_expr(LiteralValue::Void, 7708), 7709),
                ],
                span: test_span(),
                id: node_id(7710),
            },
            7711,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("return void should not satisfy strict named guard error handling");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardErrorClauseMissingTerminal { .. })),
        "expected GuardErrorClauseMissingTerminal diagnostic for terminal return void, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_rejects_success_fallback_return() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7694)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(return_stmt(
            literal_expr(LiteralValue::Integer(0), 7695),
            7696,
        )),
        span: test_span(),
        id: node_id(7697),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7698),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7699), 7700),
            7701,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(identifier_expr("value", 7702), 7703),
                ],
                span: test_span(),
                id: node_id(7704),
            },
            7705,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("fallback success return should not satisfy strict guard handling");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardErrorClauseMissingTerminal { .. })),
        "expected GuardErrorClauseMissingTerminal diagnostic for fallback return, got: {errors:?}"
    );
}

#[test]
fn test_guard_error_clause_rejects_wrapper_return_with_aliased_source() {
    let guard_stmt = Stmt::Guard {
        expression: Box::new(call_expr("string_to_int32", &["input"], 7722)),
        success_binding: Some("value".to_owned()),
        success_binding_type: Some(int_type("int32")),
        success_binding_is_mutable: false,
        error_binding: "err".to_owned(),
        else_body: Box::new(Stmt::Block {
            statements: vec![
                Stmt::Let {
                    binding: LetBinding {
                        name: "alias_err".to_owned(),
                        type_annotation: Some(int_type("ParseError")),
                        is_mutable: false,
                        span: test_span(),
                        id: node_id(7723),
                    },
                    initializer: Some(identifier_expr("err", 7724)),
                    span: test_span(),
                    id: node_id(7725),
                },
                return_stmt_values(
                    vec![labeled_value(
                        "wrapped",
                        constructor_expr(
                            identifier_expr("WrappedParseError", 7726),
                            vec![("source", identifier_expr("alias_err", 7727))],
                            7728,
                        ),
                        7729,
                    )],
                    7730,
                ),
            ],
            span: test_span(),
            id: node_id(7731),
        }),
        span: test_span(),
        id: node_id(7732),
    };

    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7733),
        make_product_type_decl(
            "WrappedParseError",
            vec![("source", int_type("ParseError"))],
            7734,
        ),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(5), 7735), 7736),
            7737,
        ),
        make_function_decl_with_errors(
            "use_guard",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["WrappedParseError"],
            Stmt::Block {
                statements: vec![
                    guard_stmt,
                    return_stmt(identifier_expr("value", 7738), 7739),
                ],
                span: test_span(),
                id: node_id(7740),
            },
            7741,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("aliased wrapper source should be rejected by strict guard validation");
    assert!(
        errors
            .iter()
            .any(|error| matches!(error, TypeError::GuardWrapperSourceInvalid { .. })),
        "expected GuardWrapperSourceInvalid diagnostic for aliased source, got: {errors:?}"
    );
}

#[test]
fn test_propagate_call_remains_valid_unmodified() {
    let program = create_entry_program(vec![
        make_unit_type_decl("ParseError", 7700),
        make_function_decl_with_errors(
            "string_to_int32",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            return_stmt(literal_expr(LiteralValue::Integer(6), 7701), 7702),
            7703,
        ),
        make_function_decl_with_errors(
            "use_propagate",
            vec![make_parameter("input", int_type("string"))],
            Some(int_type("int32")),
            vec!["ParseError"],
            Stmt::Block {
                statements: vec![
                    Stmt::Let {
                        binding: LetBinding {
                            name: "value".to_owned(),
                            type_annotation: Some(int_type("int32")),
                            is_mutable: false,
                            span: test_span(),
                            id: node_id(7704),
                        },
                        initializer: Some(propagate_call(
                            call_expr("string_to_int32", &["input"], 7705),
                            7706,
                        )),
                        span: test_span(),
                        id: node_id(7707),
                    },
                    return_stmt(identifier_expr("value", 7708), 7709),
                ],
                span: test_span(),
                id: node_id(7710),
            },
            7711,
        ),
    ]);

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "ordinary propagate <call> behavior should remain valid: {result:?}"
    );
}

#[test]
fn test_generic_type_instantiation() {
    let span = test_span();
    let ast_type = Type::Generic {
        name: "Result".to_owned(),
        type_args: vec![
            Type::Basic {
                name: "int32".to_owned(),
                span,
            },
            Type::Basic {
                name: "string".to_owned(),
                span,
            },
        ],
        span,
    };
    let core_type = ast_type_to_core_type(&ast_type).unwrap();
    if let CoreType::Generic { name, type_args } = core_type {
        assert_eq!(name, "Result");
        assert_eq!(type_args.len(), 2);
        assert_eq!(type_args[0], CoreType::Int32);
        assert_eq!(type_args[1], CoreType::String);
    } else {
        unreachable!("Expected CoreType::Generic");
    }
}

#[test]
fn test_adt_type_validation_sum() {
    let span = Span::single(Position::start());
    let variant = Variant {
        name: "Some".to_owned(),
        fields: vec![Field {
            name: "value".to_owned(),
            type_annotation: Type::Basic {
                name: "int32".to_owned(),
                span,
            },
            span,
        }],
        span,
    };
    let type_def = TypeDef::Sum {
        variants: vec![variant],
        span,
    };
    let _checker = TypeChecker::new();
    assert!(TypeChecker::validate_adt_type(&type_def).is_ok());
}

#[test]
fn test_adt_type_validation_product() {
    let span = Span::single(Position::start());
    let field = Field {
        name: "count".to_owned(),
        type_annotation: Type::Basic {
            name: "int32".to_owned(),
            span,
        },
        span,
    };
    let type_def = TypeDef::Product {
        fields: vec![field],
        span,
    };
    let _checker = TypeChecker::new();
    assert!(TypeChecker::validate_adt_type(&type_def).is_ok());
}

#[test]
fn test_pattern_match_type_check() {
    let checker = TypeChecker::new();
    let matched_type = CoreType::Int32;
    let matched_span = span_with_offset(1, 1);
    let patterns = vec![
        (CoreType::Int32, span_with_offset(2, 1)),
        (CoreType::Int32, span_with_offset(3, 1)),
    ];
    let arm_types = vec![
        (CoreType::String, span_with_offset(4, 1)),
        (CoreType::String, span_with_offset(5, 1)),
    ];
    assert!(
        checker
            .type_check_pattern_match(&matched_type, matched_span, &patterns, &arm_types)
            .is_ok()
    );

    // Incompatible pattern
    let bad_patterns = vec![(CoreType::String, span_with_offset(6, 1))];
    assert!(
        checker
            .type_check_pattern_match(&matched_type, matched_span, &bad_patterns, &arm_types)
            .is_err()
    );

    // Incompatible arm types
    let bad_arms = vec![
        (CoreType::String, span_with_offset(7, 1)),
        (CoreType::Int32, span_with_offset(8, 1)),
    ];
    assert!(
        checker
            .type_check_pattern_match(&matched_type, matched_span, &patterns, &bad_arms)
            .is_err()
    );
}

#[test]
fn test_pattern_match_incompatible_pattern_reports_span() {
    let checker = TypeChecker::new();
    let matched_type = CoreType::Int32;
    let matched_span = span_with_offset(100, 2);
    let pattern_span = span_with_offset(200, 3);
    let patterns = vec![(CoreType::String, pattern_span)];
    let arm_span = span_with_offset(300, 3);
    let arm_types = vec![(CoreType::String, arm_span)];

    let result =
        checker.type_check_pattern_match(&matched_type, matched_span, &patterns, &arm_types);

    match result {
        Err(TypeError::TypeMismatch {
            expected_span,
            found_span,
            ..
        }) => {
            assert_eq!(
                expected_span,
                Some(TypeError::span_from_span(matched_span)),
                "expected span should reflect matched expression"
            );
            assert_eq!(
                found_span,
                TypeError::span_from_span(pattern_span),
                "found span should highlight pattern location"
            );
        }
        other => {
            assert!(
                matches!(other, Err(TypeError::TypeMismatch { .. })),
                "expected TypeMismatch with spans, got {other:?}"
            );
        }
    }
}

#[test]
fn test_pattern_match_incompatible_arm_reports_span() {
    let checker = TypeChecker::new();
    let matched_type = CoreType::Int32;
    let matched_span = span_with_offset(400, 2);
    let pattern_span = span_with_offset(500, 2);
    let patterns = vec![(CoreType::Int32, pattern_span)];
    let first_arm_span = span_with_offset(600, 2);
    let second_arm_span = span_with_offset(700, 2);
    let arm_types = vec![
        (CoreType::String, first_arm_span),
        (CoreType::Int32, second_arm_span),
    ];

    let result =
        checker.type_check_pattern_match(&matched_type, matched_span, &patterns, &arm_types);

    match result {
        Err(TypeError::TypeMismatch {
            expected_span,
            found_span,
            ..
        }) => {
            assert_eq!(
                expected_span,
                Some(TypeError::span_from_span(first_arm_span)),
                "expected span should reference first arm"
            );
            assert_eq!(
                found_span,
                TypeError::span_from_span(second_arm_span),
                "found span should reference mismatched arm"
            );
        }
        other => {
            assert!(
                matches!(other, Err(TypeError::TypeMismatch { .. })),
                "expected TypeMismatch with spans, got {other:?}"
            );
        }
    }
}

#[test]
fn test_type_environment_creation() {
    let env = TypeEnvironment::new();
    // Test basic types
    assert!(env.has_type("int32"));
    assert!(env.has_type("string"));
    assert!(env.has_type("boolean"));
    assert!(env.has_type("Pair"));

    // Test extended integer types
    assert!(env.has_type("int8"));
    assert!(env.has_type("int16"));
    assert!(env.has_type("int64"));
    assert!(env.has_type("uint8"));
    assert!(env.has_type("uint16"));
    assert!(env.has_type("uint32"));
    assert!(env.has_type("uint64"));

    // Test floating point types
    assert!(env.has_type("float32"));
    assert!(env.has_type("float64"));

    // Test that non-existent types are not found
    assert!(!env.has_type("nonexistent"));
    assert!(!env.has_type("char"));
    assert!(!env.has_type("i32"));
}

#[test]
fn test_type_environment_lookup() {
    let env = TypeEnvironment::new();
    let span = test_span();

    // Test basic types
    assert_eq!(env.lookup_type("int32", span).unwrap(), &CoreType::Int32);
    assert_eq!(env.lookup_type("string", span).unwrap(), &CoreType::String);
    assert_eq!(
        env.lookup_type("boolean", span).unwrap(),
        &CoreType::Boolean
    );

    // Test extended integer types
    assert_eq!(env.lookup_type("int8", span).unwrap(), &CoreType::Int8);
    assert_eq!(env.lookup_type("int16", span).unwrap(), &CoreType::Int16);
    assert_eq!(env.lookup_type("int64", span).unwrap(), &CoreType::Int64);
    assert_eq!(env.lookup_type("uint8", span).unwrap(), &CoreType::UInt8);
    assert_eq!(env.lookup_type("uint16", span).unwrap(), &CoreType::UInt16);
    assert_eq!(env.lookup_type("uint32", span).unwrap(), &CoreType::UInt32);
    assert_eq!(env.lookup_type("uint64", span).unwrap(), &CoreType::UInt64);

    // Test floating point types
    assert_eq!(
        env.lookup_type("float32", span).unwrap(),
        &CoreType::Float32
    );
    assert_eq!(
        env.lookup_type("float64", span).unwrap(),
        &CoreType::Float64
    );

    // Test unit type
    assert_eq!(env.lookup_type("unit", span).unwrap(), &CoreType::Unit);

    assert_eq!(
        env.lookup_type("Pair", span).unwrap(),
        &CoreType::Generic {
            name: "Pair".to_owned(),
            type_args: Vec::new(),
        }
    );

    // Test non-existent type
    assert!(env.lookup_type("nonexistent", span).is_err());
}

#[test]
fn test_type_environment_register() {
    let mut env = TypeEnvironment::new();
    let span = test_span();

    assert!(!env.has_type("custom"));
    env.register_type("custom".to_owned(), CoreType::Int32);
    assert!(env.has_type("custom"));
    assert_eq!(env.lookup_type("custom", span).unwrap(), &CoreType::Int32);
}

#[test]
fn test_type_checker_creation() {
    let checker = TypeChecker::new();
    assert!(checker.environment().has_type("int32"));
    assert!(checker.environment().has_type("string"));
    assert!(checker.environment().has_type("Pair"));
}

#[test]
fn test_centralized_ast_type_to_core_type_import() {
    use crate::token::{Position, Span};

    let start_pos = Position::new(1, 1, 0);
    let end_pos = Position::new(1, 5, 4);
    let span = Span::new(start_pos, end_pos);
    let int8_type = Type::Basic {
        name: "int8".to_owned(),
        span,
    };

    assert_eq!(ast_type_to_core_type(&int8_type).unwrap(), CoreType::Int8);
}

#[test]
fn test_ast_type_to_core_type() {
    use crate::token::{Position, Span};

    let start_pos = Position::new(1, 1, 0);
    let end_pos = Position::new(1, 6, 5);
    let span = Span::new(start_pos, end_pos);

    let int32_type = Type::Basic {
        name: "int32".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&int32_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Int32
    );

    let string_type = Type::Basic {
        name: "string".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&string_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::String
    );

    let invalid_type = Type::Basic {
        name: "nonexistent".to_owned(),
        span,
    };
    assert!(
        ast_type_to_core_type(&invalid_type)
            .map_err(TypeError::from)
            .is_err()
    );
}

#[test]
fn test_ast_type_to_core_type_extended_integers() {
    use crate::token::{Position, Span};

    let start_pos = Position::new(1, 1, 0);
    let end_pos = Position::new(1, 6, 5);
    let span = Span::new(start_pos, end_pos);

    // Test all integer types
    let int8_type = Type::Basic {
        name: "int8".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&int8_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Int8
    );

    let int16_type = Type::Basic {
        name: "int16".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&int16_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Int16
    );

    let uint8_type = Type::Basic {
        name: "uint8".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&uint8_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::UInt8
    );

    let uint16_type = Type::Basic {
        name: "uint16".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&uint16_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::UInt16
    );

    let uint32_type = Type::Basic {
        name: "uint32".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&uint32_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::UInt32
    );

    let uint64_type = Type::Basic {
        name: "uint64".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&uint64_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::UInt64
    );

    let int64_type = Type::Basic {
        name: "int64".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&int64_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Int64
    );
}

#[test]
fn test_ast_type_to_core_type_float_types() {
    use crate::token::{Position, Span};

    let start_pos = Position::new(1, 1, 0);
    let end_pos = Position::new(1, 6, 5);
    let span = Span::new(start_pos, end_pos);

    let float32_type = Type::Basic {
        name: "float32".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&float32_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Float32
    );

    let float64_type = Type::Basic {
        name: "float64".to_owned(),
        span,
    };
    assert_eq!(
        ast_type_to_core_type(&float64_type)
            .map_err(TypeError::from)
            .unwrap(),
        CoreType::Float64
    );
}

#[test]
fn test_ast_type_to_core_type_complex_types() {
    use crate::token::{Position, Span};

    let start_pos = Position::new(1, 1, 0);
    let end_pos = Position::new(1, 6, 5);
    let span = Span::new(start_pos, end_pos);

    // Test that complex types now succeed
    let array_type = Type::Array {
        element_type: Box::new(Type::Basic {
            name: "int32".to_owned(),
            span,
        }),
        span,
    };
    let array_result = ast_type_to_core_type(&array_type);
    assert!(array_result.is_ok());
    assert_eq!(
        array_result.unwrap(),
        CoreType::Array(Box::new(CoreType::Int32))
    );

    let function_type = Type::Function {
        parameters: vec![],
        return_types: vec![Type::Basic {
            name: "unit".to_owned(),
            span,
        }],
        errors: None,
        span,
    };
    let function_result = ast_type_to_core_type(&function_type);
    assert!(function_result.is_ok());
    assert_eq!(
        function_result.unwrap(),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![],
            return_types: vec![CoreType::Unit],
            error_types: vec![],
        }
    );

    let generic_type = Type::Generic {
        name: "Array".to_owned(),
        type_args: vec![Type::Basic {
            name: "int32".to_owned(),
            span,
        }],
        span,
    };
    let generic_result = ast_type_to_core_type(&generic_type);
    assert!(generic_result.is_ok());
    assert_eq!(
        generic_result.unwrap(),
        CoreType::Generic {
            name: "Array".to_owned(),
            type_args: vec![CoreType::Int32],
        }
    );
}

#[test]
fn test_types_compatible() {
    let checker = TypeChecker::new();
    assert!(checker.types_compatible(&CoreType::Int32, &CoreType::Int32));
    assert!(checker.types_compatible(&CoreType::String, &CoreType::String));
    assert!(!checker.types_compatible(&CoreType::Int32, &CoreType::String));
    assert!(!checker.types_compatible(&CoreType::Boolean, &CoreType::Float32));
}

#[test]
fn test_validate_type_name() {
    let checker = TypeChecker::new();
    let span = test_span();

    // Valid type name for existing type
    assert!(
        checker
            .validate_type_name("int32", &CoreType::Int32, span)
            .is_ok()
    );

    // Invalid type name for different type
    assert!(
        checker
            .validate_type_name("int32", &CoreType::String, span)
            .is_err()
    );

    // New type name should be valid
    assert!(
        checker
            .validate_type_name("custom", &CoreType::Int32, span)
            .is_ok()
    );
}

#[test]
fn test_core_type_equality() {
    assert_eq!(CoreType::Int32, CoreType::Int32);
    assert_ne!(CoreType::Int32, CoreType::Int64);
    assert_ne!(CoreType::String, CoreType::Boolean);
}

#[test]
fn test_type_error_messages() {
    let not_found = TypeError::TypeNotFound {
        type_name: "test".to_owned(),
        span: TypeError::unknown_span(),
    };
    assert!(not_found.to_string().contains("Type 'test' not found"));

    let mismatch = TypeError::TypeMismatch {
        expected: "int32".to_owned(),
        found: "string".to_owned(),
        found_span: TypeError::unknown_span(),
        expected_span: None,
    };
    assert!(mismatch.to_string().contains("Type mismatch"));
    assert!(mismatch.to_string().contains("expected 'int32'"));
    assert!(mismatch.to_string().contains("found 'string'"));
}

#[test]
fn test_environment_get_type_names() {
    let env = TypeEnvironment::new();
    let type_names = env.get_type_names();

    assert!(type_names.iter().any(|name| name == "int8"));
    assert!(type_names.iter().any(|name| name == "int16"));
    assert!(type_names.iter().any(|name| name == "int32"));
    assert!(type_names.iter().any(|name| name == "int64"));
    assert!(type_names.iter().any(|name| name == "uint8"));
    assert!(type_names.iter().any(|name| name == "uint16"));
    assert!(type_names.iter().any(|name| name == "uint32"));
    assert!(type_names.iter().any(|name| name == "uint64"));
    assert!(type_names.iter().any(|name| name == "float32"));
    assert!(type_names.iter().any(|name| name == "float64"));
    assert!(type_names.iter().any(|name| name == "string"));
    assert!(type_names.iter().any(|name| name == "boolean"));
    assert!(type_names.iter().any(|name| name == "unit"));

    // Ensure we have the minimum expected built-in types
    assert!(
        type_names.len() >= 13,
        "Expected at least 13 built-in types, found {}",
        type_names.len()
    );

    // Ensure names are sorted
    let mut sorted_names = type_names.clone();
    sorted_names.sort();
    assert_eq!(
        type_names, sorted_names,
        "Type names should be returned in sorted order"
    );
}

#[test]
fn test_type_var_creation() {
    let var = TypeVar::new(THIRD_TEST_VAR_ID, "test_var".to_owned());
    assert_eq!(var.id, THIRD_TEST_VAR_ID);
    assert_eq!(var.name, "test_var");
}

#[test]
fn test_substitution_empty() {
    let subst = Substitution::empty();
    assert!(subst.is_empty());
    assert_eq!(subst.mappings().len(), 0);
}

#[test]
fn test_substitution_single() {
    let var_id = 0;
    let core_type = CoreType::Int32;
    let subst = Substitution::single(var_id, core_type.clone());

    assert!(!subst.is_empty());
    assert_eq!(subst.mappings().len(), 1);
    assert_eq!(subst.mappings().get(&var_id), Some(&core_type));
}

#[test]
fn test_substitution_apply_primitive() {
    let subst = Substitution::empty();
    let int_type = CoreType::Int32;

    // Applying substitution to primitive type should return the same type
    assert_eq!(subst.apply(&int_type), int_type);
}

#[test]
fn test_substitution_apply_variable() {
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());
    let int_type = CoreType::Int32;

    // Apply substitution that maps the variable to int32
    let subst = Substitution::single(var.id, int_type.clone());
    assert_eq!(subst.apply(&var_type), int_type);

    // Apply empty substitution should return the variable unchanged
    let empty_subst = Substitution::empty();
    assert_eq!(empty_subst.apply(&var_type), var_type);
}

#[test]
fn test_substitution_apply_array() {
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());
    let array_var_type = CoreType::Array(Box::new(var_type));

    let subst = Substitution::single(var.id, CoreType::Int32);
    let expected = CoreType::Array(Box::new(CoreType::Int32));

    assert_eq!(subst.apply(&array_var_type), expected);
}

#[test]
fn test_substitution_apply_function() {
    let var1 = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var2 = TypeVar::new(ANOTHER_TEST_VAR_ID, "y".to_owned());
    let var1_type = CoreType::Variable(var1.clone());
    let var2_type = CoreType::Variable(var2.clone());

    let function_type = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![var1_type],
        return_types: vec![var2_type],
        error_types: vec![],
    };

    let mut mappings = BTreeMap::new();
    mappings.insert(var1.id, CoreType::Int32);
    mappings.insert(var2.id, CoreType::String);
    let subst = Substitution { mappings };

    let expected = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32],
        return_types: vec![CoreType::String],
        error_types: vec![],
    };

    assert_eq!(subst.apply(&function_type), expected);
}

#[test]
fn test_substitution_compose() {
    // s1 maps x -> int32
    let s1 = Substitution::single(TEST_VAR_ID, CoreType::Int32);

    // s2 maps y -> x (which should become int32 after composition)
    let var_x = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let s2 = Substitution::single(ANOTHER_TEST_VAR_ID, CoreType::Variable(var_x));

    // Compose s1 after s2: s1(s2(...))
    let composed = s1.compose(&s2);

    // Should have mapping for y -> int32 and x -> int32
    assert_eq!(composed.mappings().len(), 2);
    assert_eq!(
        composed.mappings().get(&TEST_VAR_ID),
        Some(&CoreType::Int32)
    );
    assert_eq!(
        composed.mappings().get(&ANOTHER_TEST_VAR_ID),
        Some(&CoreType::Int32)
    );
}

#[test]
fn test_fresh_type_var_generation() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    let var1 = checker
        .fresh_type_var("test".to_owned(), span)
        .expect("Should generate fresh type var");
    let var2 = checker
        .fresh_type_var_auto(span)
        .expect("Should generate fresh type var");

    // Should generate different variables
    assert_ne!(var1, var2);

    // Check they are variables
    assert!(matches!(var1, CoreType::Variable(_)));
    assert!(matches!(var2, CoreType::Variable(_)));
}

#[test]
fn test_unify_identical_primitives() {
    let checker = TypeChecker::new();

    let int_result = checker.unify(&CoreType::Int32, &CoreType::Int32, None, None);
    assert!(int_result.is_ok());
    assert!(int_result.unwrap().is_empty());

    let string_result = checker.unify(&CoreType::String, &CoreType::String, None, None);
    assert!(string_result.is_ok());
    assert!(string_result.unwrap().is_empty());
}

#[test]
fn test_unify_different_primitives() {
    let checker = TypeChecker::new();

    let mismatch_result = checker.unify(&CoreType::Int32, &CoreType::String, None, None);
    assert!(mismatch_result.is_err());

    if let Err(TypeError::UnificationFailed { left, right, .. }) = mismatch_result {
        assert!(left.contains("int32"));
        assert!(right.contains("string"));
    } else {
        unreachable!("Expected UnificationFailed error");
    }
}

#[test]
fn test_unify_variable_with_type() {
    let checker = TypeChecker::new();
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());
    let int_type = CoreType::Int32;

    let result = checker.unify(&var_type, &int_type, None, None);
    assert!(result.is_ok());

    let subst = result.unwrap();
    assert!(!subst.is_empty());
    assert_eq!(subst.mappings().get(&var.id), Some(&int_type));
}

#[test]
fn test_unify_variable_with_variable() {
    let checker = TypeChecker::new();
    let var1 = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var2 = TypeVar::new(ANOTHER_TEST_VAR_ID, "y".to_owned());
    let var1_type = CoreType::Variable(var1.clone());
    let var2_type = CoreType::Variable(var2.clone());

    let result = checker.unify(&var1_type, &var2_type, None, None);
    assert!(result.is_ok());

    let subst = result.unwrap();
    assert!(!subst.is_empty());
    // One variable should be mapped to the other
    assert!(subst.mappings().contains_key(&var1.id) || subst.mappings().contains_key(&var2.id));
}

#[test]
fn test_unify_arrays() {
    let checker = TypeChecker::new();
    let array_int = CoreType::Array(Box::new(CoreType::Int32));
    let array_string = CoreType::Array(Box::new(CoreType::String));

    // Arrays with same element type should unify
    let same_result = checker.unify(&array_int, &array_int, None, None);
    assert!(same_result.is_ok());
    assert!(same_result.unwrap().is_empty());

    // Arrays with different element types should not unify
    let different_result = checker.unify(&array_int, &array_string, None, None);
    assert!(different_result.is_err());
}

#[test]
fn test_unify_arrays_with_variables() {
    let checker = TypeChecker::new();
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());
    let array_var = CoreType::Array(Box::new(var_type));
    let array_int = CoreType::Array(Box::new(CoreType::Int32));

    let result = checker.unify(&array_var, &array_int, None, None);
    assert!(result.is_ok());

    let subst = result.unwrap();
    assert_eq!(subst.mappings().get(&var.id), Some(&CoreType::Int32));
}

#[test]
fn test_unify_functions() {
    let checker = TypeChecker::new();
    let func1 = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32],
        return_types: vec![CoreType::String],
        error_types: vec![],
    };
    let func2 = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32],
        return_types: vec![CoreType::String],
        error_types: vec![],
    };
    let func3 = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::String],
        return_types: vec![CoreType::Int32],
        error_types: vec![],
    };

    // Identical functions should unify
    let same_result = checker.unify(&func1, &func2, None, None);
    assert!(same_result.is_ok());
    assert!(same_result.unwrap().is_empty());

    // Different functions should not unify
    let different_result = checker.unify(&func1, &func3, None, None);
    assert!(different_result.is_err());
}

#[test]
fn test_occurs_check() {
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());

    // Variable should occur in itself
    assert!(TypeChecker::occurs_check(var.id, &var_type));

    // Variable should occur in array containing it
    let array_var = CoreType::Array(Box::new(var_type));
    assert!(TypeChecker::occurs_check(var.id, &array_var));

    // Variable should not occur in different type
    assert!(!TypeChecker::occurs_check(var.id, &CoreType::Int32));

    // Variable should not occur in array of different type
    let array_int = CoreType::Array(Box::new(CoreType::Int32));
    assert!(!TypeChecker::occurs_check(var.id, &array_int));
}

#[test]
fn test_occurs_check_prevents_infinite_types() {
    let checker = TypeChecker::new();
    let var = TypeVar::new(TEST_VAR_ID, "x".to_owned());
    let var_type = CoreType::Variable(var.clone());
    let array_var = CoreType::Array(Box::new(var_type));

    // Trying to unify x with Array<x> should fail
    let infinite_result = checker.unify(&CoreType::Variable(var.clone()), &array_var, None, None);
    assert!(infinite_result.is_err());

    if let Err(TypeError::OccursCheckFailed {
        var_name,
        type_name,
        ..
    }) = infinite_result
    {
        assert_eq!(var_name, var.name);
        assert!(type_name.contains('[') && type_name.contains('x'));
    } else {
        unreachable!("Expected OccursCheckFailed error");
    }
}

#[test]
fn test_symbol_table_scope_management() {
    let mut table = SymbolTable::new();

    // Register in global scope
    table.register(SymbolInfo {
        name: "global_var".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Public,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Should find global variable
    assert!(table.contains("global_var"));
    assert!(table.lookup("global_var").is_some());

    // Enter function scope
    let func_scope = table.enter_scope();
    assert_ne!(func_scope, ScopeId(0));

    // Register parameter in function scope
    table.register(SymbolInfo {
        name: "param".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::String,
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Should find both global and local
    assert!(
        table.contains("global_var"),
        "Should find global variable from nested scope"
    );
    assert!(table.contains("param"), "Should find local parameter");
    assert!(
        table.lookup_local("param").is_some(),
        "Should find param in current scope"
    );
    assert!(
        table.lookup_local("global_var").is_none(),
        "Should not find global_var in current scope only"
    );

    // Enter block scope
    let block_scope = table.enter_scope();
    assert_ne!(block_scope, func_scope);

    // Register local variable in block
    table.register(SymbolInfo {
        name: "local_var".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Boolean,
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Should find all three variables
    assert!(table.contains("global_var"));
    assert!(table.contains("param"));
    assert!(table.contains("local_var"));

    // Exit block scope
    table.exit_scope();

    // Should still find global and param, but not local_var
    assert!(table.contains("global_var"));
    assert!(table.contains("param"));
    assert!(
        !table.contains("local_var"),
        "local_var should not be accessible after exiting scope"
    );

    // Exit function scope
    table.exit_scope();

    // Should only find global
    assert!(table.contains("global_var"));
    assert!(
        !table.contains("param"),
        "param should not be accessible after exiting function scope"
    );
    assert!(!table.contains("local_var"));
}

#[test]
fn test_symbol_table_shadowing() {
    let mut table = SymbolTable::new();

    // Register in global scope
    table.register(SymbolInfo {
        name: "x".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let global_x = table.lookup("x").unwrap();
    assert_eq!(global_x.core_type, CoreType::Int32);

    // Enter scope and shadow x
    table.enter_scope();
    table.register(SymbolInfo {
        name: "x".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::String,
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Should find shadowed version
    let shadowed_x = table.lookup("x").unwrap();
    assert_eq!(
        shadowed_x.core_type,
        CoreType::String,
        "Should find shadowed version"
    );

    // Exit scope
    table.exit_scope();

    // Should find original version again
    let original_x = table.lookup("x").unwrap();
    assert_eq!(
        original_x.core_type,
        CoreType::Int32,
        "Should find original version after exiting scope"
    );
}

#[test]
fn test_symbol_table_exported_symbols() {
    let mut table = SymbolTable::new();

    // Register public symbol in global scope
    table.register(SymbolInfo {
        name: "public_func".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![],
            return_types: vec![CoreType::Unit],
            error_types: vec![],
        },
        visibility: Visibility::Public,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Register entry point in global scope
    table.register(SymbolInfo {
        name: "main".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![],
            return_types: vec![CoreType::Unit],
            error_types: vec![],
        },
        visibility: Visibility::Entry,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    // Register private symbol in global scope
    table.register(SymbolInfo {
        name: "private_func".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![],
            return_types: vec![CoreType::Unit],
            error_types: vec![],
        },
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let exported = table.exported_symbols();
    assert_eq!(exported.len(), 2, "Should have 2 exported symbols");
    assert!(exported.iter().any(|s| s.name == "public_func"));
    assert!(exported.iter().any(|s| s.name == "main"));
    assert!(!exported.iter().any(|s| s.name == "private_func"));
}

#[test]
fn test_type_check_literal_expression() {
    let mut checker = TypeChecker::new();
    let expr = literal_expr(LiteralValue::Integer(42), 10_000);
    let ty = checker
        .type_check_expr(&expr)
        .expect("literal expressions should type check");
    assert_eq!(ty, CoreType::Int64, "integer literals default to int64");
}

#[test]
fn test_type_check_array_literal_with_consistent_elements() {
    let mut checker = TypeChecker::new();
    let array_expr = Expr::Array {
        elements: vec![
            literal_expr(LiteralValue::Integer(1), 20_000),
            literal_expr(LiteralValue::Integer(2), 20_001),
        ],
        span: test_span(),
        id: node_id(20_002),
    };

    let ty = checker
        .type_check_expr(&array_expr)
        .expect("consistent element types should infer array type");

    assert_eq!(ty, CoreType::Array(Box::new(CoreType::Int64)));
}

#[test]
fn test_type_check_array_literal_detects_mismatched_elements() {
    let mut checker = TypeChecker::new();
    let array_expr = Expr::Array {
        elements: vec![
            literal_expr(LiteralValue::Integer(1), 20_010),
            literal_expr(LiteralValue::String("oops".to_owned()), 20_011),
        ],
        span: test_span(),
        id: node_id(20_012),
    };

    let result = checker.type_check_expr(&array_expr);
    assert!(
        matches!(result, Err(TypeError::TypeMismatch { .. })),
        "array literals must enforce uniform element types"
    );
}

#[test]
fn test_type_check_string_interpolation_rejects_non_displayable_expression() {
    let mut checker = TypeChecker::new();
    let array_expr = Expr::Array {
        elements: vec![literal_expr(LiteralValue::Integer(7), 20_020)],
        span: test_span(),
        id: node_id(20_021),
    };
    let interpolation = Expr::StringInterpolation {
        parts: vec![
            StringPart::Literal("value: ".to_owned()),
            StringPart::Expression(array_expr),
        ],
        span: test_span(),
        id: node_id(20_022),
    };

    let result = checker.type_check_expr(&interpolation);
    assert!(
        matches!(result, Err(TypeError::InvalidOperation { .. })),
        "string interpolation should reject non-displayable values"
    );
}

#[test]
fn test_type_check_let_statement_registers_symbol() {
    let mut checker = TypeChecker::new();
    let binding = LetBinding {
        name: "value".to_owned(),
        type_annotation: Some(Type::Basic {
            name: "int64".to_owned(),
            span: test_span(),
        }),
        is_mutable: false,
        span: test_span(),
        id: node_id(10_100),
    };

    let stmt = Stmt::Let {
        binding,
        initializer: Some(literal_expr(LiteralValue::Integer(1), 10_101)),
        span: test_span(),
        id: node_id(10_102),
    };

    checker
        .type_check_stmt(&stmt)
        .expect("let with matching initializer should type check");

    let symbol = checker
        .symbol_table()
        .lookup("value")
        .expect("binding should be registered");
    assert_eq!(symbol.core_type, CoreType::Int64);
}

#[test]
fn test_generic_constraint_inference_succeeds_for_matching_argument() {
    let source = "\
public identity = f<T: int32>(value: T): T => return value
entry main = f(): int32 => {
    let value: int32 = 1
    return identity(value)
}
";
    let program = parse_program_from_source_with_spaces(source);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "expected constrained generic inference to pass: {result:?}"
    );
}

#[test]
fn test_generic_constraint_violation_reports_error() {
    let source = "\
public identity = f<T: int32>(value: T): T => return value
entry main = f(): string => return identity<int32>('value')
";
    let program = parse_program_from_source_with_spaces(source);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(result.is_err(), "expected constraint violation to fail");
}

#[test]
fn test_explicit_generic_call_respects_constraints() {
    let source = "\
public identity = f<T: int32>(value: T): T => return value
entry main = f(): int32 => {
    let value: int32 = 1
    return identity<int32>(value)
}
";
    let program = parse_program_from_source_with_spaces(source);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "expected explicit generic call with matching constraints to pass: {result:?}"
    );
}

#[test]
fn test_multiple_generic_constraints_conflict_reports_error() {
    let source = "\
public identity = f<T: int32 + int64>(value: T): T => return value
entry main = f(): int32 => return identity(1)
";
    let program = parse_program_from_source_with_spaces(source);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_err(),
        "expected conflicting constraints to fail type checking"
    );
}

#[test]
fn test_type_check_assignment_type_mismatch() {
    let mut checker = TypeChecker::new();
    let binding = LetBinding {
        name: "value".to_owned(),
        type_annotation: Some(Type::Basic {
            name: "int64".to_owned(),
            span: test_span(),
        }),
        is_mutable: true,
        span: test_span(),
        id: node_id(10_110),
    };

    let let_stmt = Stmt::Let {
        binding,
        initializer: Some(literal_expr(LiteralValue::Integer(10), 10_111)),
        span: test_span(),
        id: node_id(10_112),
    };

    checker
        .type_check_stmt(&let_stmt)
        .expect("initial declaration should succeed");

    let assignment = Stmt::Assignment {
        target: identifier_expr("value", 10_113),
        value: literal_expr(LiteralValue::String("oops".to_owned()), 10_114),
        span: test_span(),
        id: node_id(10_115),
    };

    let result = checker.type_check_stmt(&assignment);
    assert!(
        matches!(result, Err(TypeError::TypeMismatch { .. })),
        "assignment should fail due to mismatched types"
    );
}

#[test]
fn test_type_check_for_loop_registers_loop_variable_in_body_scope() {
    let mut checker = TypeChecker::new();
    let for_stmt = Stmt::For {
        variable: "item".to_owned(),
        iterable: Expr::Array {
            elements: vec![literal_expr(LiteralValue::Integer(1), 20_100)],
            span: test_span(),
            id: node_id(20_101),
        },
        body: Box::new(Stmt::Expression {
            expr: identifier_expr("item", 20_102),
            span: test_span(),
            id: node_id(20_103),
        }),
        span: test_span(),
        id: node_id(20_104),
    };

    checker
        .type_check_stmt(&for_stmt)
        .expect("for loop over array should type check");

    assert!(
        checker.symbol_table().lookup("item").is_none(),
        "loop variable should not escape its scope"
    );
}

#[test]
fn test_type_check_for_loop_requires_iterable_array() {
    let mut checker = TypeChecker::new();
    let for_stmt = Stmt::For {
        variable: "value".to_owned(),
        iterable: literal_expr(LiteralValue::Integer(1), 20_110),
        body: Box::new(Stmt::Expression {
            expr: literal_expr(LiteralValue::Void, 20_111),
            span: test_span(),
            id: node_id(20_112),
        }),
        span: test_span(),
        id: node_id(20_113),
    };

    let result = checker.type_check_stmt(&for_stmt);
    assert!(
        matches!(result, Err(TypeError::InvalidOperation { .. })),
        "for loop should reject non-iterable types"
    );
}

#[test]
fn test_type_check_return_enforces_expected_type() {
    let mut checker = TypeChecker::new();
    let return_stmt = Stmt::Return {
        values: vec![LabeledValue {
            label: String::new(),
            value: literal_expr(LiteralValue::String("bad".to_owned()), 20_120),
            span: test_span(),
            id: node_id(20_121),
        }],
        span: test_span(),
        id: node_id(20_121),
    };

    let expected = CoreType::Int32;
    let result = checker.type_check_stmt_with_return(&return_stmt, Some(&[expected]));
    assert!(
        matches!(result, Err(TypeError::TypeMismatch { .. })),
        "return statements must match expected return type"
    );
}

#[test]
fn test_type_check_return_arity_mismatch_for_multiple_returns() {
    let mut checker = TypeChecker::new();
    let return_stmt = Stmt::Return {
        values: vec![LabeledValue {
            label: String::new(),
            value: literal_expr(LiteralValue::Integer(1), 20_125),
            span: test_span(),
            id: node_id(20_126),
        }],
        span: test_span(),
        id: node_id(20_127),
    };

    let expected = [CoreType::Int32, CoreType::Int32];
    let result = checker.type_check_stmt_with_return(&return_stmt, Some(&expected));

    assert!(
        matches!(
            result,
            Err(TypeError::ArityMismatch {
                expected: 2,
                found: 1,
                ..
            })
        ),
        "multi-return functions should enforce return arity"
    );
}

#[test]
fn test_type_check_single_return_backward_compatibility() {
    let mut checker = TypeChecker::new();

    let function = make_function_decl(
        "single_return",
        vec![],
        Some(int_type("int32")),
        return_stmt(literal_expr(LiteralValue::Integer(7), 20_130), 20_131),
        20_132,
    );

    let program = create_entry_program(vec![function]);
    let result = checker.type_check_program(&program);

    assert!(
        result.is_ok(),
        "single-return behavior should remain backward compatible"
    );
}

#[test]
fn test_type_check_let_destructure_loop_break_values() {
    let mut checker = TypeChecker::new();

    let destructure = Stmt::LetDestructure {
        bindings: vec![
            LetBinding {
                name: "user_input".to_owned(),
                type_annotation: Some(int_type("int64")),
                is_mutable: false,
                span: test_span(),
                id: node_id(20_133),
            },
            LetBinding {
                name: "user_number".to_owned(),
                type_annotation: Some(int_type("int64")),
                is_mutable: false,
                span: test_span(),
                id: node_id(20_134),
            },
        ],
        initializer: Expr::Loop {
            body: Box::new(Stmt::Block {
                statements: vec![Stmt::Break {
                    values: vec![
                        LabeledValue {
                            label: "user_input".to_owned(),
                            value: literal_expr(LiteralValue::Integer(1), 20_135),
                            span: test_span(),
                            id: node_id(20_136),
                        },
                        LabeledValue {
                            label: "user_number".to_owned(),
                            value: literal_expr(LiteralValue::Integer(2), 20_137),
                            span: test_span(),
                            id: node_id(20_138),
                        },
                    ],
                    span: test_span(),
                    id: node_id(20_139),
                }],
                span: test_span(),
                id: node_id(20_140),
            }),
            span: test_span(),
            id: node_id(20_141),
        },
        span: test_span(),
        id: node_id(20_142),
    };

    let function = Decl::Function {
        name: "loop_destructure".to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: vec![],
        return_types: Some(vec![int_type("int64")]),
        error_types: Vec::new(),
        body: Stmt::Block {
            statements: vec![
                destructure,
                return_stmt(identifier_expr("user_number", 20_143), 20_144),
            ],
            span: test_span(),
            id: node_id(20_145),
        },
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(20_146),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![function]);
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "let destructure with loop break payload should type-check"
    );
}

#[test]
fn test_type_check_labeled_returns_require_consistent_ordered_labels() {
    let mut checker = TypeChecker::new();

    let first_return = Stmt::Return {
        values: vec![
            LabeledValue {
                label: "left".to_owned(),
                value: literal_expr(LiteralValue::Integer(1), 20_140),
                span: test_span(),
                id: node_id(20_141),
            },
            LabeledValue {
                label: "right".to_owned(),
                value: literal_expr(LiteralValue::Integer(2), 20_142),
                span: test_span(),
                id: node_id(20_143),
            },
        ],
        span: test_span(),
        id: node_id(20_144),
    };

    let second_return = Stmt::Return {
        values: vec![
            LabeledValue {
                label: "right".to_owned(),
                value: literal_expr(LiteralValue::Integer(3), 20_145),
                span: test_span(),
                id: node_id(20_146),
            },
            LabeledValue {
                label: "left".to_owned(),
                value: literal_expr(LiteralValue::Integer(4), 20_147),
                span: test_span(),
                id: node_id(20_148),
            },
        ],
        span: test_span(),
        id: node_id(20_149),
    };

    let function = Decl::Function {
        name: "swap_like".to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: vec![],
        return_types: Some(vec![int_type("int32"), int_type("int32")]),
        error_types: Vec::new(),
        body: Stmt::Block {
            statements: vec![first_return, second_return],
            span: test_span(),
            id: node_id(20_150),
        },
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(20_151),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![function]);
    let result = checker.type_check_program(&program);

    assert!(
        matches!(
            result,
            Err(ref errors) if errors
                .iter()
                .any(|error| matches!(error, &TypeError::ReturnLabelMismatch { .. }))
        ),
        "labeled returns should require a consistent ordered label set"
    );
}

#[test]
fn test_type_check_labeled_and_unlabeled_returns_cannot_mix() {
    let mut checker = TypeChecker::new();

    let unlabeled_return = Stmt::Return {
        values: vec![
            LabeledValue {
                label: String::new(),
                value: literal_expr(LiteralValue::Integer(5), 20_160),
                span: test_span(),
                id: node_id(20_161),
            },
            LabeledValue {
                label: String::new(),
                value: literal_expr(LiteralValue::Integer(6), 20_162),
                span: test_span(),
                id: node_id(20_163),
            },
        ],
        span: test_span(),
        id: node_id(20_164),
    };

    let labeled_return = Stmt::Return {
        values: vec![
            LabeledValue {
                label: "left".to_owned(),
                value: literal_expr(LiteralValue::Integer(7), 20_165),
                span: test_span(),
                id: node_id(20_166),
            },
            LabeledValue {
                label: "right".to_owned(),
                value: literal_expr(LiteralValue::Integer(8), 20_167),
                span: test_span(),
                id: node_id(20_168),
            },
        ],
        span: test_span(),
        id: node_id(20_169),
    };

    let function = Decl::Function {
        name: "mixed_returns".to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: vec![],
        return_types: Some(vec![int_type("int32"), int_type("int32")]),
        error_types: Vec::new(),
        body: Stmt::Block {
            statements: vec![unlabeled_return, labeled_return],
            span: test_span(),
            id: node_id(20_170),
        },
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(20_171),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = create_entry_program(vec![function]);
    let result = checker.type_check_program(&program);

    assert!(
        matches!(
            result,
            Err(ref errors) if errors
                .iter()
                .any(|error| matches!(error, &TypeError::ReturnLabelMismatch { .. }))
        ),
        "mixing unlabeled and labeled returns should fail"
    );
}

#[test]
fn test_type_check_if_requires_boolean_condition() {
    let mut checker = TypeChecker::new();
    let condition = literal_expr(LiteralValue::Integer(1), 10_120);
    let then_branch = Stmt::Expression {
        expr: literal_expr(LiteralValue::Void, 10_121),
        span: test_span(),
        id: node_id(10_122),
    };

    let if_stmt = Stmt::If {
        condition,
        then_branch: Box::new(then_branch),
        else_branch: None,
        span: test_span(),
        id: node_id(10_123),
    };

    let result = checker.type_check_stmt(&if_stmt);
    assert!(
        matches!(result, Err(TypeError::InvalidOperation { .. })),
        "non-boolean conditions must be rejected"
    );
}

#[test]
fn test_if_expression_infers_branch_type_when_branches_match() {
    let mut checker = TypeChecker::new();
    let program =
        parse_program_from_source("entry main = f(): int64 => return if true { 1 } else { 2 }\n");

    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "matching if-expression branches should infer a concrete result type"
    );
}

#[test]
fn test_if_expression_branch_mismatch_reports_type_error() {
    let mut checker = TypeChecker::new();
    let program = parse_program_from_source("let value = if true { 1 } else { false }\n");

    let result = checker.type_check_program(&program);
    assert!(
        matches!(
            result,
            Err(ref errors) if errors
                .iter()
                .any(|error| matches!(*error, TypeError::TypeMismatch { .. }))
        ),
        "mismatched if-expression branches must fail with a TypeMismatch"
    );
}

#[test]
fn test_else_less_if_expression_defaults_to_unit_type() {
    let mut checker = TypeChecker::new();
    let program = parse_program_from_source("let value: int32 = if true { 1 }\n");

    let result = checker.type_check_program(&program);
    assert!(
        matches!(
            result,
            Err(ref errors) if errors
                .iter()
                .any(|error| matches!(*error, TypeError::MissingElseBranch { .. }))
        ),
        "else-less if expressions should type as unit and fail non-unit annotation checks"
    );
}

#[test]
fn test_type_check_program_collects_errors() {
    let mut checker = TypeChecker::new();
    let decl = Decl::Function {
        name: "bad".to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: vec![Parameter {
            name: "x".to_owned(),
            param_type: Type::Basic {
                name: "int32".to_owned(),
                span: test_span(),
            },
            span: test_span(),
        }],
        return_types: Some(vec![Type::Basic {
            name: "int32".to_owned(),
            span: test_span(),
        }]),
        error_types: Vec::new(),
        body: Stmt::Return {
            values: vec![LabeledValue {
                label: String::new(),
                value: literal_expr(LiteralValue::Boolean(true), 10_200),
                span: test_span(),
                id: node_id(10_201),
            }],
            span: test_span(),
            id: node_id(10_201),
        },
        visibility: AstVisibility::Private,
        is_entry: false,
        modifiers: vec![],
        doc_comment: None,
        span: test_span(),
        id: node_id(10_202),
        metadata: HotReloadMetadata::for_function(),
    };

    let program = Program {
        declarations: vec![decl],
        span: test_span(),
        id: node_id(10_203),
    };

    let result = checker.type_check_program(&program);
    assert!(result.is_err(), "program should fail type checking");
    let errors = result.unwrap_err();
    assert_eq!(errors.len(), 1, "exactly one type mismatch expected");
    assert!(matches!(errors[0], TypeError::TypeMismatch { .. }));
}

#[test]
#[should_panic(expected = "Cannot exit global scope")]
fn test_symbol_table_cannot_exit_global_scope() {
    let mut table = SymbolTable::new();
    table.exit_scope(); // Should panic
}

#[test]
fn test_type_check_program_handles_forward_function_reference() {
    let mut checker = TypeChecker::new();

    let call_expr = Expr::Call {
        callee: Box::new(identifier_expr("future_fn", 30_000)),
        generic_args: None,
        args: vec![],
        span: test_span(),
        id: node_id(30_001),
    };

    let let_decl = make_let_decl("value", Some(int_type("int32")), call_expr, 30_010);

    let fn_return = literal_expr(LiteralValue::Integer(42), 30_020);
    let function_body = return_stmt(fn_return, 30_021);
    let fn_decl = make_function_decl(
        "future_fn",
        vec![],
        Some(int_type("int32")),
        function_body,
        30_030,
    );

    let program = create_entry_program(vec![let_decl, fn_decl]);
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "forward references should resolve once declarations are registered"
    );
}

#[test]
fn test_type_check_program_accumulates_multiple_errors() {
    let mut checker = TypeChecker::new();

    let first_fn = make_function_decl(
        "bad_one",
        vec![make_parameter("x", int_type("int32"))],
        Some(int_type("int32")),
        return_stmt(literal_expr(LiteralValue::Boolean(true), 31_100), 31_101),
        31_102,
    );

    let second_fn = make_function_decl(
        "second_bad",
        vec![],
        Some(int_type("boolean")),
        return_stmt(literal_expr(LiteralValue::Integer(5), 31_110), 31_111),
        31_112,
    );

    let program = create_entry_program(vec![first_fn, second_fn]);
    let result = checker.type_check_program(&program);
    assert!(result.is_err(), "program should report collected errors");
    let errors = result.unwrap_err();
    assert!(
        errors.len() >= 2,
        "expected at least two independent errors"
    );
}

#[test]
fn test_type_check_program_reports_let_type_mismatch() {
    let mut checker = TypeChecker::new();

    let let_decl = make_let_decl(
        "value",
        Some(int_type("int32")),
        literal_expr(LiteralValue::Boolean(true), 31_200),
        31_201,
    );

    let program = create_entry_program(vec![let_decl]);
    let result = checker.type_check_program(&program);
    assert!(result.is_err(), "mismatched let annotations must fail");
    let errors = result.unwrap_err();
    assert!(
        errors
            .iter()
            .any(|err| matches!(*err, TypeError::TypeMismatch { .. })),
        "expected a type mismatch error for the let declaration"
    );
}

#[test]
fn test_lambda_expression_body_type_checking() {
    let mut checker = TypeChecker::new();
    let lambda = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: vec![make_parameter("x", int_type("int32"))],
        return_types: vec![int_type("int32")],
        error_types: Vec::new(),
        body: crate::ast::LambdaBody::Expression(Box::new(identifier_expr("x", 32_000))),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(32_001),
    };

    let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
    assert!(
        result.is_ok(),
        "lambda expression should type check successfully"
    );
    let core_type = result.unwrap();
    if let CoreType::Function {
        parameters,
        return_types,
        ..
    } = core_type
    {
        assert_eq!(parameters, vec![CoreType::Int32]);
        assert_eq!(return_types, vec![CoreType::Int32]);
    } else {
        unreachable!("lambda should yield a function type");
    }
}

#[test]
fn test_lambda_block_body_type_checking() {
    let mut checker = TypeChecker::new();
    let return_stmt = Stmt::Return {
        values: vec![LabeledValue {
            label: String::new(),
            value: identifier_expr("x", 32_100),
            span: test_span(),
            id: node_id(32_101),
        }],
        span: test_span(),
        id: node_id(32_101),
    };
    let body = Stmt::Block {
        statements: vec![return_stmt],
        span: test_span(),
        id: node_id(32_102),
    };
    let lambda = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: vec![make_parameter("x", int_type("int32"))],
        return_types: vec![int_type("int32")],
        error_types: Vec::new(),
        body: crate::ast::LambdaBody::Block(vec![body]),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(32_103),
    };

    let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
    assert!(
        result.is_ok(),
        "lambda block body should type check successfully"
    );
    let core_type = result.unwrap();
    if let CoreType::Function {
        parameters,
        return_types,
        ..
    } = core_type
    {
        assert_eq!(parameters, vec![CoreType::Int32]);
        assert_eq!(return_types, vec![CoreType::Int32]);
    } else {
        unreachable!("lambda should yield a function type");
    }
}

#[test]
fn test_lambda_return_type_mismatch_is_reported() {
    let mut checker = TypeChecker::new();
    let lambda = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: vec![make_parameter("x", int_type("int32"))],
        return_types: vec![int_type("int32")],
        error_types: Vec::new(),
        body: crate::ast::LambdaBody::Expression(Box::new(literal_expr(
            LiteralValue::Boolean(true),
            32_200,
        ))),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(32_201),
    };

    let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
    assert!(
        matches!(result, Err(TypeError::TypeMismatch { .. })),
        "lambda returning the wrong type should fail"
    );
}

#[test]
fn test_generic_lambda_populates_function_generic_params() {
    let mut checker = TypeChecker::new();
    let lambda = Expr::Lambda {
        generic_params: Some(vec!["T".to_owned()]),
        generic_constraints: Some(vec![TypeParameter {
            name: "T".to_owned(),
            constraints: Vec::new(),
            span: test_span(),
        }]),
        params: vec![make_parameter(
            "x",
            Type::Basic {
                name: "T".to_owned(),
                span: test_span(),
            },
        )],
        return_types: vec![Type::Basic {
            name: "T".to_owned(),
            span: test_span(),
        }],
        error_types: Vec::new(),
        body: LambdaBody::Expression(Box::new(identifier_expr("x", 32_300))),
        captured_variables: vec![],
        metadata: Box::new(HotReloadMetadata::for_expression()),
        span: test_span(),
        id: node_id(32_301),
    };

    let result = checker.within_new_scope(|inner| inner.type_check_expr(&lambda));
    assert!(
        result.is_ok(),
        "generic lambda should type check: {result:?}"
    );

    let core_type = result.expect("type checked above");
    assert!(
        matches!(core_type, CoreType::Function { .. }),
        "expected function type for generic lambda, got {core_type:?}"
    );

    if let CoreType::Function {
        generic_params,
        parameters,
        return_types,
        ..
    } = core_type
    {
        assert_eq!(
            generic_params.len(),
            1,
            "generic lambda function type should declare one generic parameter"
        );
        let expected_var = CoreType::Variable(generic_params[0].type_var.clone());
        assert_eq!(parameters, vec![expected_var.clone()]);
        assert_eq!(return_types, vec![expected_var]);
    }
}

#[test]
fn test_solve_constraints_unifies_equalities() {
    let mut checker = TypeChecker::new();
    let span = test_span();
    let var_a = checker
        .fresh_type_var_auto(span)
        .expect("should create type variable");
    let var_b = checker
        .fresh_type_var_auto(span)
        .expect("should create type variable");

    checker.add_constraint(TypeConstraint::equality(
        var_a.clone(),
        CoreType::Int32,
        None,
        None,
    ));
    checker.add_constraint(TypeConstraint::equality(
        var_b.clone(),
        var_a.clone(),
        None,
        None,
    ));

    let subst = checker
        .solve_constraints()
        .expect("constraints should solve successfully");

    assert_eq!(subst.apply(&var_a), CoreType::Int32);
    assert_eq!(subst.apply(&var_b), CoreType::Int32);
}

#[test]
fn test_solve_constraints_detects_conflicts() {
    let mut checker = TypeChecker::new();
    checker.add_constraint(TypeConstraint::equality(
        CoreType::Int32,
        CoreType::String,
        None,
        None,
    ));
    let result = checker.solve_constraints();
    assert!(
        matches!(result, Err(TypeError::UnificationFailed { .. })),
        "conflicting constraints must fail"
    );
}

#[test]
fn test_solve_constraints_conflict_reports_spans() {
    let mut checker = TypeChecker::new();
    let left_span = span_with_offset(820, 3);
    let right_span = span_with_offset(940, 4);
    checker.add_constraint(TypeConstraint::equality(
        CoreType::Int32,
        CoreType::String,
        Some(left_span),
        Some(right_span),
    ));

    let result = checker
        .solve_constraints()
        .expect_err("conflict should produce error");

    match result {
        TypeError::UnificationFailed {
            left_span: reported_left,
            right_span: reported_right,
            ..
        } => {
            assert_eq!(
                reported_left,
                TypeError::span_from_span(left_span),
                "left span should match constraint origin"
            );
            assert_eq!(
                reported_right,
                TypeError::span_from_span(right_span),
                "right span should match constraint origin"
            );
        }
        other => {
            assert!(
                matches!(other, TypeError::UnificationFailed { .. }),
                "expected UnificationFailed with spans, got {other:?}"
            );
        }
    }
}

#[test]
fn test_solve_constraints_composes_substitutions() {
    let mut checker = TypeChecker::new();
    let span = test_span();
    let var_a = checker
        .fresh_type_var_auto(span)
        .expect("should create type variable");
    let var_b = checker
        .fresh_type_var_auto(span)
        .expect("should create type variable");
    let var_c = checker
        .fresh_type_var_auto(span)
        .expect("should create type variable");

    checker.add_constraint(TypeConstraint::equality(
        var_a.clone(),
        CoreType::Int32,
        None,
        None,
    ));
    checker.add_constraint(TypeConstraint::equality(
        var_b.clone(),
        var_a.clone(),
        None,
        None,
    ));
    checker.add_constraint(TypeConstraint::equality(
        var_c.clone(),
        CoreType::Boolean,
        None,
        None,
    ));

    let subst = checker
        .solve_constraints()
        .expect("constraints should compose correctly");

    assert_eq!(subst.apply(&var_a), CoreType::Int32);
    assert_eq!(subst.apply(&var_b), CoreType::Int32);
    assert_eq!(subst.apply(&var_c), CoreType::Boolean);
}

#[test]
fn test_solve_constraints_occurs_check_reports_span() {
    let mut checker = TypeChecker::new();
    let var_span = span_with_offset(1000, 2);
    let var_type = checker
        .fresh_type_var_auto(var_span)
        .expect("should allocate type variable");

    let array_type = CoreType::Array(Box::new(var_type.clone()));
    checker.add_constraint(TypeConstraint::equality(
        var_type,
        array_type,
        Some(var_span),
        Some(var_span),
    ));

    let result = checker
        .solve_constraints()
        .expect_err("occurs check should fail");

    match result {
        TypeError::OccursCheckFailed { span, .. } => {
            assert_eq!(
                span,
                TypeError::span_from_span(var_span),
                "occurs check diagnostics should use variable span"
            );
        }
        other => {
            assert!(
                matches!(other, TypeError::OccursCheckFailed { .. }),
                "expected OccursCheckFailed with span, got {other:?}"
            );
        }
    }
}

// Note: HasField constraint tests are deferred to Phase 3 when ADT (Product/Sum types) are implemented
// HasField constraints require Product types with named fields, which are part of the advanced type features

#[test]
fn test_solve_constraints_callable_with_function_type() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    // Create a function type
    let function_type = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32, CoreType::String],
        return_types: vec![CoreType::Boolean],
        error_types: vec![],
    };

    // Add Callable constraint that matches the function signature
    checker.add_constraint(TypeConstraint::Callable {
        callee: function_type,
        arguments: vec![CoreType::Int32, CoreType::String],
        return_type: CoreType::Boolean,
        callee_span: Some(span),
        argument_spans: vec![Some(span), Some(span)],
        return_span: Some(span),
    });

    let result = checker.solve_constraints();
    assert!(
        result.is_ok(),
        "Callable constraint should succeed for matching function type: {result:?}"
    );
}

#[test]
fn test_solve_constraints_callable_wrong_arity() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    // Create a function type with 2 parameters
    let function_type = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32, CoreType::String],
        return_types: vec![CoreType::Boolean],
        error_types: vec![],
    };

    // Add Callable constraint with wrong number of arguments
    checker.add_constraint(TypeConstraint::Callable {
        callee: function_type,
        arguments: vec![CoreType::Int32], // Only 1 argument, but function expects 2
        return_type: CoreType::Boolean,
        callee_span: Some(span),
        argument_spans: vec![Some(span)],
        return_span: Some(span),
    });

    let result = checker.solve_constraints();
    assert!(
        result.is_err(),
        "Callable constraint should fail for wrong arity"
    );
}

#[test]
fn test_solve_constraints_callable_wrong_argument_type() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    // Create a function type
    let function_type = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32, CoreType::String],
        return_types: vec![CoreType::Boolean],
        error_types: vec![],
    };

    // Add Callable constraint with wrong argument type
    checker.add_constraint(TypeConstraint::Callable {
        callee: function_type,
        arguments: vec![CoreType::String, CoreType::String], // First arg should be Int32
        return_type: CoreType::Boolean,
        callee_span: Some(span),
        argument_spans: vec![Some(span), Some(span)],
        return_span: Some(span),
    });

    let result = checker.solve_constraints();
    assert!(
        result.is_err(),
        "Callable constraint should fail for wrong argument type"
    );
}

#[test]
fn test_constraint_solver_applies_substitution_to_inferred_top_level_bindings() {
    let program = parse_program_from_source_with_spaces(
        "
        public identity = f<T>(x: T): T =>
            return x

        let inferred = identity(42)

        entry main = f(): int64 =>
            return inferred
        ",
    );

    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "generic call-site inference should type check successfully: {result:?}"
    );

    let inferred_symbol = checker
        .symbol_table()
        .lookup("inferred")
        .expect("top-level inferred binding should be registered");
    assert_eq!(
        inferred_symbol.core_type,
        CoreType::Int64,
        "constraint solving should apply substitution to inferred top-level binding type"
    );
}

#[test]
fn test_constraint_solver_unifies_compatible_type_variables_in_sequence() {
    let mut checker = TypeChecker::new();
    let span = test_span();
    let var_a = checker
        .fresh_type_var_auto(span)
        .expect("should allocate type variable");
    let var_b = checker
        .fresh_type_var_auto(span)
        .expect("should allocate type variable");
    let var_c = checker
        .fresh_type_var_auto(span)
        .expect("should allocate type variable");

    checker.add_constraint(TypeConstraint::equality(
        var_a.clone(),
        var_b.clone(),
        Some(span),
        Some(span),
    ));
    checker.add_constraint(TypeConstraint::equality(
        var_b.clone(),
        var_c.clone(),
        Some(span),
        Some(span),
    ));
    checker.add_constraint(TypeConstraint::equality(
        var_c.clone(),
        CoreType::Int64,
        Some(span),
        Some(span),
    ));

    let substitution = checker
        .solve_constraints()
        .expect("compatible chained variables should unify");
    assert_eq!(substitution.apply(&var_a), CoreType::Int64);
    assert_eq!(substitution.apply(&var_b), CoreType::Int64);
    assert_eq!(substitution.apply(&var_c), CoreType::Int64);
}

#[test]
fn test_constraint_solver_reports_incompatible_types_for_generic_call() {
    let program = parse_program_from_source_with_spaces(
        "
        public identity = f<T>(x: T): T =>
            return x

        entry main = f(): int64 =>
            let bad: string = identity(42)
            return 0
        ",
    );

    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("incompatible generic call assignment should fail type checking");

    assert!(
        errors.iter().any(|error| {
            matches!(
                *error,
                TypeError::TypeMismatch { .. }
                    | TypeError::UnificationFailed { .. }
                    | TypeError::ConstraintSolvingFailed { .. }
            )
        }),
        "expected a proper type incompatibility diagnostic, got: {errors:?}"
    );
}

#[test]
fn test_solve_constraints_applies_substitution_to_registered_symbols() {
    let mut checker = TypeChecker::new();
    let span = test_span();
    let variable_type = checker
        .fresh_type_var("resolved_symbol_type".to_owned(), span)
        .expect("should allocate symbol type variable");

    checker.symbol_table_mut().register(SymbolInfo {
        name: "pending_value".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: variable_type.clone(),
        visibility: Visibility::Private,
        source_location: span,
        is_let_binding: true,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    checker.add_constraint(TypeConstraint::equality(
        variable_type,
        CoreType::Int32,
        Some(span),
        Some(span),
    ));

    checker
        .solve_constraints()
        .expect("symbol substitution constraints should solve");

    let pending_symbol = checker
        .symbol_table()
        .lookup("pending_value")
        .expect("symbol should still be registered after solving");
    assert_eq!(
        pending_symbol.core_type,
        CoreType::Int32,
        "solved substitution should be applied to registered symbols"
    );
}

// ============================================================================
// Cast Validation Tests
// ============================================================================

#[test]
fn test_safe_cast_widening_signed_integers() {
    let span = test_span();

    // Safe widening casts within signed integers
    assert!(
        TypeChecker::validate_cast(&CoreType::Int8, &CoreType::Int16, span).is_ok(),
        "int8 -> int16 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int8, &CoreType::Int32, span).is_ok(),
        "int8 -> int32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int8, &CoreType::Int64, span).is_ok(),
        "int8 -> int64 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int16, &CoreType::Int32, span).is_ok(),
        "int16 -> int32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int16, &CoreType::Int64, span).is_ok(),
        "int16 -> int64 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Int64, span).is_ok(),
        "int32 -> int64 should be a safe cast"
    );
}

#[test]
fn test_safe_cast_widening_unsigned_integers() {
    let span = test_span();

    // Safe widening casts within unsigned integers
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt8, &CoreType::UInt16, span).is_ok(),
        "uint8 -> uint16 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt8, &CoreType::UInt32, span).is_ok(),
        "uint8 -> uint32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt8, &CoreType::UInt64, span).is_ok(),
        "uint8 -> uint64 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt16, &CoreType::UInt32, span).is_ok(),
        "uint16 -> uint32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt16, &CoreType::UInt64, span).is_ok(),
        "uint16 -> uint64 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt32, &CoreType::UInt64, span).is_ok(),
        "uint32 -> uint64 should be a safe cast"
    );
}

#[test]
fn test_safe_cast_widening_floats() {
    let span = test_span();

    // Safe widening cast from float32 to float64
    assert!(
        TypeChecker::validate_cast(&CoreType::Float32, &CoreType::Float64, span).is_ok(),
        "float32 -> float64 should be a safe cast"
    );
}

#[test]
fn test_safe_cast_integer_to_float() {
    let span = test_span();

    // Safe casts from integer to float (may lose precision for very large integers, but no overflow)
    assert!(
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Float32, span).is_ok(),
        "int32 -> float32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Float64, span).is_ok(),
        "int32 -> float64 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt32, &CoreType::Float32, span).is_ok(),
        "uint32 -> float32 should be a safe cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt32, &CoreType::Float64, span).is_ok(),
        "uint32 -> float64 should be a safe cast"
    );
}

#[test]
fn test_safe_cast_identity() {
    let span = test_span();

    // Identity casts are always safe
    assert!(
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Int32, span).is_ok(),
        "int32 -> int32 (identity) should be safe"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Float64, &CoreType::Float64, span).is_ok(),
        "float64 -> float64 (identity) should be safe"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Boolean, &CoreType::Boolean, span).is_ok(),
        "boolean -> boolean (identity) should be safe"
    );
}

#[test]
fn test_warning_creation_for_unsafe_cast() {
    let warning = Warning::UnsafeCast {
        from_type: "int64".to_owned(),
        to_type: "int32".to_owned(),
        span: TypeError::span_from_span(test_span()),
        suppression_annotation: None,
    };

    assert_eq!(
        warning
            .code()
            .map(|diagnostic_code| diagnostic_code.to_string())
            .as_deref(),
        Some("opalescent::type_system::warning::unsafe_cast"),
        "unsafe cast warning should expose a stable diagnostic code"
    );
    assert!(
        warning.help().is_some(),
        "unsafe cast warning should provide actionable help text"
    );
}

#[test]
fn test_unsafe_cast_is_warning_not_type_error() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    let result = checker.validate_cast_with_warnings(&CoreType::Int64, &CoreType::Int32, span);
    assert!(
        result.is_ok(),
        "unsafe cast should no longer fail type checking"
    );
    assert!(
        !checker.warnings().is_empty(),
        "unsafe cast should be collected as a warning"
    );
}

#[test]
fn test_warning_collection_for_unsafe_cast() {
    let mut checker = TypeChecker::new();
    let span = test_span();

    let result_int64_to_int32 =
        checker.validate_cast_with_warnings(&CoreType::Int64, &CoreType::Int32, span);
    assert!(
        result_int64_to_int32.is_ok(),
        "int64 -> int32 should succeed with warning collection"
    );

    let result_int32_to_int16 =
        checker.validate_cast_with_warnings(&CoreType::Int32, &CoreType::Int16, span);
    assert!(
        result_int32_to_int16.is_ok(),
        "int32 -> int16 should succeed with warning collection"
    );

    let warnings = checker.warnings();
    assert_eq!(
        warnings.len(),
        2,
        "two unsafe casts should produce two collected warnings"
    );
    let first_warning = warnings
        .first()
        .expect("warning collection should contain at least one warning");
    assert!(
        matches!(
            *first_warning,
            Warning::UnsafeCast {
                ref from_type,
                ref to_type,
                ..
            } if from_type == "int64" && to_type == "int32"
        ),
        "first warning should describe int64 -> int32 cast"
    );
}

#[test]
fn test_unsafe_cast_narrowing_unsigned_integers() {
    let span = test_span();

    // Unsafe narrowing casts within unsigned integers are warning-level now
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt64, &CoreType::UInt32, span).is_ok(),
        "uint64 -> uint32 should be accepted as warning-level cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt32, &CoreType::UInt16, span).is_ok(),
        "uint32 -> uint16 should be accepted as warning-level cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt16, &CoreType::UInt8, span).is_ok(),
        "uint16 -> uint8 should be accepted as warning-level cast"
    );
}

#[test]
fn test_unsafe_cast_float_narrowing() {
    let span = test_span();

    assert!(
        TypeChecker::validate_cast(&CoreType::Float64, &CoreType::Float32, span).is_ok(),
        "float64 -> float32 should be accepted as warning-level cast"
    );
}

#[test]
fn test_unsafe_cast_float_to_integer() {
    let span = test_span();

    assert!(
        TypeChecker::validate_cast(&CoreType::Float32, &CoreType::Int32, span).is_ok(),
        "float32 -> int32 should be accepted as warning-level cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Float64, &CoreType::Int64, span).is_ok(),
        "float64 -> int64 should be accepted as warning-level cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::Float32, &CoreType::UInt32, span).is_ok(),
        "float32 -> uint32 should be accepted as warning-level cast"
    );
}

#[test]
fn test_unsafe_cast_signed_unsigned_conversion() {
    let span = test_span();

    assert!(
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::UInt32, span).is_ok(),
        "int32 -> uint32 should be accepted as warning-level cast"
    );
    assert!(
        TypeChecker::validate_cast(&CoreType::UInt32, &CoreType::Int32, span).is_ok(),
        "uint32 -> int32 should be accepted as warning-level cast"
    );
}

#[test]
fn test_invalid_cast_non_numeric_types() {
    let span = test_span();

    // Invalid casts involving non-numeric types
    let result_string_to_int32 =
        TypeChecker::validate_cast(&CoreType::String, &CoreType::Int32, span);
    assert!(
        matches!(result_string_to_int32, Err(TypeError::InvalidCast { .. })),
        "string -> int32 should be an invalid cast"
    );

    let result_int32_to_string =
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::String, span);
    assert!(
        matches!(result_int32_to_string, Err(TypeError::InvalidCast { .. })),
        "int32 -> string should be an invalid cast"
    );

    let result_boolean_to_int32 =
        TypeChecker::validate_cast(&CoreType::Boolean, &CoreType::Int32, span);
    assert!(
        matches!(result_boolean_to_int32, Err(TypeError::InvalidCast { .. })),
        "boolean -> int32 should be an invalid cast"
    );

    let result_int32_to_boolean =
        TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Boolean, span);
    assert!(
        matches!(result_int32_to_boolean, Err(TypeError::InvalidCast { .. })),
        "int32 -> boolean should be an invalid cast"
    );
}

#[test]
fn test_invalid_cast_array_types() {
    let span = test_span();

    // Invalid casts involving array types
    let array_type = CoreType::Array(Box::new(CoreType::Int32));
    let result_array_to_int32 = TypeChecker::validate_cast(&array_type, &CoreType::Int32, span);
    assert!(
        matches!(result_array_to_int32, Err(TypeError::InvalidCast { .. })),
        "[int32] -> int32 should be an invalid cast"
    );

    let result_int32_to_array = TypeChecker::validate_cast(&CoreType::Int32, &array_type, span);
    assert!(
        matches!(result_int32_to_array, Err(TypeError::InvalidCast { .. })),
        "int32 -> [int32] should be an invalid cast"
    );
}

#[test]
fn test_invalid_cast_function_types() {
    let span = test_span();

    // Invalid casts involving function types
    let function_type = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int32],
        return_types: vec![CoreType::Int32],
        error_types: vec![],
    };

    let result_fn_to_int32 = TypeChecker::validate_cast(&function_type, &CoreType::Int32, span);
    assert!(
        matches!(result_fn_to_int32, Err(TypeError::InvalidCast { .. })),
        "(int32) -> int32 function -> int32 should be an invalid cast"
    );

    let result_int32_to_fn = TypeChecker::validate_cast(&CoreType::Int32, &function_type, span);
    assert!(
        matches!(result_int32_to_fn, Err(TypeError::InvalidCast { .. })),
        "int32 -> (int32) -> int32 function should be an invalid cast"
    );
}

#[test]
fn test_invalid_cast_unit_type() {
    let span = test_span();
    let result_from_unit = TypeChecker::validate_cast(&CoreType::Unit, &CoreType::Int32, span);
    assert!(
        matches!(result_from_unit, Err(TypeError::InvalidCast { .. })),
        "unit -> int32 should be an invalid cast"
    );

    // Reverse direction
    let result_to_unit = TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Unit, span);
    assert!(
        matches!(result_to_unit, Err(TypeError::InvalidCast { .. })),
        "int32 -> unit should be an invalid cast"
    );
}

#[test]
fn test_invalid_cast_string_type() {
    let span = test_span();
    let result_from_string = TypeChecker::validate_cast(&CoreType::String, &CoreType::Int32, span);
    assert!(
        matches!(result_from_string, Err(TypeError::InvalidCast { .. })),
        "string -> int32 should be an invalid cast"
    );

    // Reverse direction
    let result_to_string = TypeChecker::validate_cast(&CoreType::Int32, &CoreType::String, span);
    assert!(
        matches!(result_to_string, Err(TypeError::InvalidCast { .. })),
        "int32 -> string should be an invalid cast"
    );
}

#[test]
fn test_invalid_cast_boolean_type() {
    let span = test_span();
    let result_from_bool = TypeChecker::validate_cast(&CoreType::Boolean, &CoreType::Int32, span);
    assert!(
        matches!(result_from_bool, Err(TypeError::InvalidCast { .. })),
        "boolean -> int32 should be an invalid cast"
    );

    // Reverse direction
    let result_to_bool = TypeChecker::validate_cast(&CoreType::Int32, &CoreType::Boolean, span);
    assert!(
        matches!(result_to_bool, Err(TypeError::InvalidCast { .. })),
        "int32 -> boolean should be an invalid cast"
    );
}

#[test]
fn test_cast_with_type_variable() {
    let span = test_span();
    let var = TypeVar::new(0, "T".to_owned());
    let var_type = CoreType::Variable(var);

    // Type variables cannot be cast to concrete types during inference
    let result_from_var = TypeChecker::validate_cast(&var_type, &CoreType::Int32, span);
    assert!(
        matches!(result_from_var, Err(TypeError::InvalidCast { .. })),
        "type variable -> int32 should be an invalid cast during inference"
    );

    // Reverse direction
    let result_to_var = TypeChecker::validate_cast(&CoreType::Int32, &var_type, span);
    assert!(
        matches!(result_to_var, Err(TypeError::InvalidCast { .. })),
        "int32 -> type variable should be an invalid cast during inference"
    );
}

#[test]
fn test_cast_generic_type() {
    let span = test_span();
    let generic_type = CoreType::Generic {
        name: "Option".to_owned(),
        type_args: Vec::from([CoreType::Int32]),
    };

    // Generic types cannot be cast to primitives
    let result = TypeChecker::validate_cast(&generic_type, &CoreType::Int32, span);
    assert!(
        matches!(result, Err(TypeError::InvalidCast { .. })),
        "Option<int32> -> int32 should be an invalid cast"
    );
}

/// Ensure the guard/propagate integration sample type checks end to end.
#[test]
fn test_type_check_error_handling_sample_program() {
    let parsed_program = parse_error_handling_sample_program();
    let program = create_entry_program(parsed_program.declarations);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "integration sample should type check successfully: {result:?}",
    );
}

#[test]
fn test_builtin_print_supports_generic_arguments() {
    const SOURCE: &str = "
entry demo = f(): unit =>
    print('hello')
    print(42)
    print(true)
    return void
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "print<T> should accept different argument types: {result:?}"
    );
}

#[test]
fn test_builtin_take_input_returns_string() {
    const SOURCE: &str = "
entry demo = f(): string =>
    let input: string = take_input()
    return input
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "take_input() should type check as string: {result:?}"
    );
}

#[test]
fn test_builtin_string_to_int32_signature_type_checks() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): int32 => {
let n: int32 = string_to_int32(input)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    let errors =
        result.expect_err("bare call to string_to_int32 without guard/propagate must fail");
    assert!(
        errors.iter().any(
            |e| matches!(*e, TypeError::UnhandledCallError { ref name, .. } if name == "string_to_int32")
        ),
        "expected UnhandledCallError for string_to_int32, got: {errors:?}"
    );
}

#[test]
fn test_builtin_string_to_int64_is_not_registered() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): int64 => {
let n: int64 = string_to_int64(input)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("string_to_int64 should not be available in this runtime-aligned phase");
    assert!(
        errors.iter().any(
            |error| matches!(*error, TypeError::SymbolNotFound { ref name, .. } if name == "string_to_int64")
        ),
        "expected SymbolNotFound for string_to_int64, got: {errors:?}"
    );
}

#[test]
fn test_builtin_random_int32_signature_type_checks() {
    const SOURCE: &str = "
entry quiz_num = f(): int32 => {
let n: int32 = random_int32(1, 5)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "random_int32(min, max) should type check and return int32: {result:?}"
    );
}

#[test]
fn test_builtin_random_int64_is_not_registered() {
    const SOURCE: &str = "
entry quiz_num = f(): int64 => {
let n: int64 = random_int64(1, 5)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("random_int64 should not be available in this runtime-aligned phase");
    assert!(
        errors.iter().any(|error| {
            matches!(*error, TypeError::SymbolNotFound { ref name, .. } if name == "random_int64")
        }),
        "expected SymbolNotFound for random_int64, got: {errors:?}"
    );
}

#[test]
fn test_builtin_print_declares_generic_parameter() {
    let checker = TypeChecker::new();
    let signature = checker
        .environment()
        .lookup_builtin("print")
        .cloned()
        .expect("print builtin should be registered");

    let signature_is_expected = match signature {
        CoreType::Function {
            generic_params,
            parameters,
            return_types,
            ..
        } => {
            assert_eq!(
                generic_params.len(),
                1,
                "print should declare exactly one generic type parameter"
            );
            assert_eq!(
                parameters,
                vec![CoreType::Variable(generic_params[0].type_var.clone())],
                "print parameter type should reuse the declared generic type variable"
            );
            assert_eq!(
                return_types,
                vec![CoreType::Unit],
                "print should return unit"
            );
            true
        }
        _ => false,
    };
    assert!(
        signature_is_expected,
        "print should be registered as a function type"
    );
}

#[test]
fn test_builtin_calls_report_wrong_arity() {
    const SOURCE: &str = "
let invalid = f(): unit =>
    print()
    return void
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let errors = checker
        .type_check_program(&program)
        .expect_err("print() without arguments should fail arity checking");

    assert!(
        errors
            .into_iter()
            .any(|error| matches!(error, TypeError::InvalidOperation { .. })),
        "expected InvalidOperation arity diagnostic for print()"
    );
}

#[test]
fn test_hello_world_spec_file_type_checks_with_builtins() {
    const HELLO_WORLD_SOURCE: &str = include_str!("../../language-spec/hello_world.op");
    let program = parse_program_from_source_with_spaces(HELLO_WORLD_SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "language-spec/hello_world.op should type check once built-ins are registered: {result:?}"
    );
}

#[test]
fn test_member_access_module_member_resolves_type() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "math".to_owned(),
        symbol_type: SymbolType::Constant,
        core_type: CoreType::Generic {
            name: "math".to_owned(),
            type_args: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    checker.register_symbol(SymbolInfo {
        name: "math.sqrt".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![CoreType::Int32],
            return_types: vec![CoreType::Int32],
            error_types: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let expr = Expr::Member {
        object: Box::new(identifier_expr("math", 81_000)),
        member: "sqrt".to_owned(),
        span: test_span(),
        id: node_id(81_001),
    };

    let result = checker.type_check_expr(&expr);
    assert!(result.is_ok(), "module member access should type check");
    assert!(
        matches!(result, Ok(CoreType::Function { .. })),
        "math.sqrt should resolve to function type"
    );
}

#[test]
fn test_member_access_struct_like_field_resolves_type() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "person".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Generic {
            name: "Person".to_owned(),
            type_args: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    checker.register_symbol(SymbolInfo {
        name: "Person.name".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::String,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let expr = Expr::Member {
        object: Box::new(identifier_expr("person", 82_000)),
        member: "name".to_owned(),
        span: test_span(),
        id: node_id(82_001),
    };

    let result = checker.type_check_expr(&expr);
    assert_eq!(
        result,
        Ok(CoreType::String),
        "field should resolve to string"
    );
}

#[test]
fn test_member_access_missing_member_reports_symbol_error_with_span() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "math".to_owned(),
        symbol_type: SymbolType::Constant,
        core_type: CoreType::Generic {
            name: "math".to_owned(),
            type_args: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let member_span = span_with_offset(210, 9);
    let expr = Expr::Member {
        object: Box::new(Expr::Identifier {
            name: "math".to_owned(),
            span: member_span,
            id: node_id(83_000),
        }),
        member: "does_not_exist".to_owned(),
        span: member_span,
        id: node_id(83_001),
    };

    let result = checker.type_check_expr(&expr);
    match result {
        Err(TypeError::SymbolNotFound { name, span, .. }) => {
            assert_eq!(
                name, "math.does_not_exist",
                "missing member should be qualified"
            );
            assert_eq!(
                span,
                TypeError::span_from_span(member_span),
                "error should preserve source span"
            );
        }
        other => {
            assert!(
                matches!(other, Err(TypeError::SymbolNotFound { .. })),
                "expected SymbolNotFound for missing member, got {other:?}"
            );
        }
    }
}

#[test]
fn test_member_access_chained_member_resolves_type() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "pkg".to_owned(),
        symbol_type: SymbolType::Constant,
        core_type: CoreType::Generic {
            name: "pkg".to_owned(),
            type_args: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    checker.register_symbol(SymbolInfo {
        name: "pkg.math".to_owned(),
        symbol_type: SymbolType::Constant,
        core_type: CoreType::Generic {
            name: "MathModule".to_owned(),
            type_args: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    checker.register_symbol(SymbolInfo {
        name: "MathModule.sqrt".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![CoreType::Int32],
            return_types: vec![CoreType::Int32],
            error_types: Vec::new(),
        },
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let expr = Expr::Member {
        object: Box::new(Expr::Member {
            object: Box::new(identifier_expr("pkg", 84_000)),
            member: "math".to_owned(),
            span: test_span(),
            id: node_id(84_001),
        }),
        member: "sqrt".to_owned(),
        span: test_span(),
        id: node_id(84_002),
    };

    let result = checker.type_check_expr(&expr);
    assert!(
        matches!(result, Ok(CoreType::Function { .. })),
        "chained member access should resolve to final member type"
    );
}

#[test]
fn test_constant_i32_addition_overflow_emits_warning() {
    let mut checker = TypeChecker::new();
    let expr = Expr::Binary {
        left: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(0x7FFF_FFFF), 90_000)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_001),
        }),
        operator: crate::ast::BinaryOp::Add,
        right: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(1), 90_002)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_003),
        }),
        span: test_span(),
        id: node_id(90_004),
    };
    let result = checker.type_check_expr(&expr);
    assert!(
        result.is_ok(),
        "constant overflow should remain non-fatal: {result:?}"
    );

    assert!(
        checker
            .warnings()
            .iter()
            .any(|warning| matches!(*warning, Warning::ArithmeticOverflow { ref operation, .. } if operation == "addition")),
        "expected arithmetic-overflow warning for i32 max + 1"
    );
}

#[test]
fn test_constant_i32_subtraction_overflow_emits_warning() {
    let mut checker = TypeChecker::new();
    let expr = Expr::Binary {
        left: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(-0x8000_0000), 90_100)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_101),
        }),
        operator: crate::ast::BinaryOp::Subtract,
        right: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(1), 90_102)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_103),
        }),
        span: test_span(),
        id: node_id(90_104),
    };
    let result = checker.type_check_expr(&expr);
    assert!(
        result.is_ok(),
        "constant overflow should remain non-fatal: {result:?}"
    );

    assert!(
        checker
            .warnings()
            .iter()
            .any(|warning| matches!(*warning, Warning::ArithmeticOverflow { ref operation, .. } if operation == "subtraction")),
        "expected arithmetic-overflow warning for i32 min - 1"
    );
}

#[test]
fn test_constant_i32_multiplication_overflow_emits_warning() {
    let mut checker = TypeChecker::new();
    let expr = Expr::Binary {
        left: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(0x7FFF_FFFF), 90_200)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_201),
        }),
        operator: crate::ast::BinaryOp::Multiply,
        right: Box::new(Expr::Cast {
            expr: Box::new(literal_expr(LiteralValue::Integer(2), 90_202)),
            target_type: int_type("int32"),
            span: test_span(),
            id: node_id(90_203),
        }),
        span: test_span(),
        id: node_id(90_204),
    };
    let result = checker.type_check_expr(&expr);
    assert!(
        result.is_ok(),
        "constant overflow should remain non-fatal: {result:?}"
    );

    assert!(
        checker
            .warnings()
            .iter()
            .any(|warning| matches!(*warning, Warning::ArithmeticOverflow { ref operation, .. } if operation == "multiplication")),
        "expected arithmetic-overflow warning for i32 max * 2"
    );
}

#[test]
fn test_constant_shift_count_at_i32_upper_bound_is_valid() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "value".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("value", 90_300)),
        operator: crate::ast::BinaryOp::BitShiftLeft,
        right: Box::new(literal_expr(LiteralValue::Integer(31), 90_301)),
        span: test_span(),
        id: node_id(90_302),
    };
    let result = checker.type_check_expr(&expr);
    assert!(
        result.is_ok(),
        "shift count equal to bit-width-1 should be valid: {result:?}"
    );
}

#[test]
fn test_constant_shift_count_at_i32_bit_width_is_rejected() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "value".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("value", 90_400)),
        operator: crate::ast::BinaryOp::BitShiftLeft,
        right: Box::new(literal_expr(LiteralValue::Integer(32), 90_401)),
        span: test_span(),
        id: node_id(90_402),
    };
    let error = checker
        .type_check_expr(&expr)
        .expect_err("shift count >= bit width should be rejected at compile time");

    assert!(
        matches!(
            error,
            TypeError::InvalidShiftCount {
                shift_count: 32,
                bit_width: 32,
                ..
            }
        ),
        "expected InvalidShiftCount diagnostic for out-of-range shift count"
    );
}

#[test]
fn test_constant_negative_shift_count_is_rejected() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "value".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("value", 90_500)),
        operator: crate::ast::BinaryOp::BitShiftLeft,
        right: Box::new(literal_expr(LiteralValue::Integer(-1), 90_501)),
        span: test_span(),
        id: node_id(90_502),
    };
    let error = checker
        .type_check_expr(&expr)
        .expect_err("negative shift count should be rejected at compile time");

    assert!(
        matches!(
            error,
            TypeError::InvalidShiftCount {
                shift_count: -1,
                bit_width: 32,
                ..
            }
        ),
        "expected InvalidShiftCount diagnostic for negative shift count"
    );
}

#[test]
fn test_constant_division_by_zero_is_rejected_with_rhs_span() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "numerator".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int64,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let rhs_span = span_with_offset(2_100, 1);
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("numerator", 90_600)),
        operator: crate::ast::BinaryOp::Divide,
        right: Box::new(Expr::Literal {
            value: LiteralValue::Integer(0),
            span: rhs_span,
            id: node_id(90_601),
        }),
        span: test_span(),
        id: node_id(90_602),
    };

    let error = checker
        .type_check_expr(&expr)
        .expect_err("division by a constant zero divisor should be rejected");

    match error {
        TypeError::DivisionByZero { operation, span } => {
            assert_eq!(
                operation, "division",
                "division should report operation name"
            );
            assert_eq!(
                span,
                TypeError::span_from_span(rhs_span),
                "division-by-zero diagnostic should highlight divisor expression"
            );
        }
        other => {
            assert!(
                matches!(other, TypeError::DivisionByZero { .. }),
                "expected DivisionByZero diagnostic, got {other:?}"
            );
        }
    }
}

#[test]
fn test_constant_modulo_by_zero_is_rejected_with_rhs_span() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "numerator".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int64,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let rhs_span = span_with_offset(2_200, 1);
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("numerator", 90_610)),
        operator: crate::ast::BinaryOp::Modulo,
        right: Box::new(Expr::Literal {
            value: LiteralValue::Integer(0),
            span: rhs_span,
            id: node_id(90_611),
        }),
        span: test_span(),
        id: node_id(90_612),
    };

    let error = checker
        .type_check_expr(&expr)
        .expect_err("modulo by a constant zero divisor should be rejected");

    match error {
        TypeError::DivisionByZero { operation, span } => {
            assert_eq!(operation, "modulo", "modulo should report operation name");
            assert_eq!(
                span,
                TypeError::span_from_span(rhs_span),
                "modulo-by-zero diagnostic should highlight divisor expression"
            );
        }
        other => {
            assert!(
                matches!(other, TypeError::DivisionByZero { .. }),
                "expected DivisionByZero diagnostic, got {other:?}"
            );
        }
    }
}

#[test]
fn test_non_constant_lhs_with_literal_zero_divisor_is_still_rejected() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "a".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int64,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("a", 90_620)),
        operator: crate::ast::BinaryOp::Divide,
        right: Box::new(literal_expr(LiteralValue::Integer(0), 90_621)),
        span: test_span(),
        id: node_id(90_622),
    };

    let error = checker
        .type_check_expr(&expr)
        .expect_err("literal zero divisor must be rejected even when lhs is non-constant");
    assert!(
        matches!(error, TypeError::DivisionByZero { ref operation, .. } if operation == "division"),
        "expected DivisionByZero for non-constant lhs with literal zero divisor"
    );
}

#[test]
fn test_non_constant_divisor_does_not_emit_compile_time_division_by_zero() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "numerator".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int64,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    checker.register_symbol(SymbolInfo {
        name: "denominator".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int64,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });
    let expr = Expr::Binary {
        left: Box::new(identifier_expr("numerator", 90_630)),
        operator: crate::ast::BinaryOp::Divide,
        right: Box::new(identifier_expr("denominator", 90_631)),
        span: test_span(),
        id: node_id(90_632),
    };

    let result = checker.type_check_expr(&expr);
    assert!(
        result.is_ok(),
        "non-constant divisor should not trigger compile-time division-by-zero error: {result:?}"
    );
}

#[test]
fn test_non_constant_integer_addition_does_not_emit_overflow_warning() {
    const SOURCE: &str = "
entry sum_values = f(a: int32, b: int32): int32 =>
    return a + b
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "non-constant addition should type check: {result:?}"
    );
    assert!(
        checker
            .warnings()
            .iter()
            .all(|warning| !matches!(*warning, Warning::ArithmeticOverflow { .. })),
        "non-constant addition must not emit compile-time overflow warning"
    );
}

#[test]
fn test_integer_intrinsic_member_calls_type_check() {
    let mut checker = TypeChecker::new();
    checker.register_symbol(SymbolInfo {
        name: "value".to_owned(),
        symbol_type: SymbolType::Variable,
        core_type: CoreType::Int32,
        visibility: Visibility::Private,
        source_location: test_span(),
        is_let_binding: false,
        is_mutable: false,
        read_count: 0,
        is_pure: false,
    });

    let checked_add_expr = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(identifier_expr("value", 95_000)),
            member: "checked_add".to_owned(),
            span: test_span(),
            id: node_id(95_001),
        }),
        generic_args: None,
        args: vec![literal_expr(LiteralValue::Integer(1), 95_002)],
        span: test_span(),
        id: node_id(95_003),
    };

    let wrapping_sub_expr = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(identifier_expr("value", 95_010)),
            member: "wrapping_sub".to_owned(),
            span: test_span(),
            id: node_id(95_011),
        }),
        generic_args: None,
        args: vec![literal_expr(LiteralValue::Integer(1), 95_012)],
        span: test_span(),
        id: node_id(95_013),
    };

    let saturating_mul_expr = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(identifier_expr("value", 95_020)),
            member: "saturating_mul".to_owned(),
            span: test_span(),
            id: node_id(95_021),
        }),
        generic_args: None,
        args: vec![literal_expr(LiteralValue::Integer(2), 95_022)],
        span: test_span(),
        id: node_id(95_023),
    };

    let checked_result = checker.type_check_expr(&checked_add_expr);
    let wrapping_result = checker.type_check_expr(&wrapping_sub_expr);
    let saturating_result = checker.type_check_expr(&saturating_mul_expr);

    let result = checked_result.and(wrapping_result).and(saturating_result);
    assert!(
        result.is_ok(),
        "checked/wrapping/saturating integer intrinsics should type check: {result:?}"
    );
}

#[test]
fn test_guard_with_string_to_int32_type_checks() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): int32 errors ParseError => {
guard string_to_int32(input) into n else _e =>
    let _handled: ParseError = _e
    propagate _e
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "guard string_to_int32(input) into n else _e => ... should type check: {result:?}"
    );
}

#[test]
fn test_propagate_string_to_int32_in_error_function_type_checks() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): int32 errors ParseError => {
let n: int32 = propagate string_to_int32(input)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "propagate string_to_int32(input) in errors ParseError function should type check: {result:?}"
    );
}

#[test]
fn test_bare_call_to_string_to_uint32_produces_unhandled_call_error() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): uint32 => {
let n: uint32 = string_to_uint32(input)
return n
}
";

    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    let errors =
        result.expect_err("bare call to string_to_uint32 without guard/propagate must fail");
    assert!(
        errors.iter().any(
            |e| matches!(*e, TypeError::UnhandledCallError { ref name, .. } if name == "string_to_uint32")
        ),
        "expected UnhandledCallError for string_to_uint32, got: {errors:?}"
    );
}

#[test]
fn test_int32_to_string_type_checks() {
    const SOURCE: &str = "
entry show_number = f(n: int32): string => {
let s: string = int32_to_string(n)
return s
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "int32_to_string(n) should type check as string: {result:?}"
    );
}

#[test]
fn test_int32_to_string_does_not_require_error_handling() {
    const SOURCE: &str = "
entry show_number = f(n: int32): string => {
let s: string = int32_to_string(n)
return s
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "int32_to_string should be callable without guard/propagate (infallible): {result:?}"
    );
}

#[test]
fn test_bare_call_error_message_mentions_function_name() {
    const SOURCE: &str = "
entry parse_user_number = f(input: string): int32 => {
let n: int32 = string_to_int32(input)
return n
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    let errors = result.expect_err("bare call to string_to_int32 must fail");
    let error_text = format!("{errors:?}");
    assert!(
        error_text.contains("string_to_int32"),
        "error message should mention the function name 'string_to_int32': {error_text}"
    );
}

#[test]
fn test_float64_to_string_type_checks() {
    const SOURCE: &str = "
entry show_float = f(x: float64): string => {
let s: string = float64_to_string(x)
return s
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "float64_to_string(x) should type check as string: {result:?}"
    );
}

#[test]
fn test_int32_to_string_then_string_to_int32_roundtrip_type_checks() {
    const SOURCE: &str = "
entry roundtrip = f(n: int32): int32 errors ParseError => {
let s: string = int32_to_string(n)
let result: int32 = propagate string_to_int32(s)
return result
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "int32_to_string then string_to_int32 roundtrip should type check: {result:?}"
    );
}

#[test]
fn test_purity_violation_variant_exists() {
    let err = TypeError::PurityViolation {
        callee_name: String::from("print"),
        reason: String::from("this function performs I/O or has side effects"),
        span: TypeError::unknown_span(),
    };
    let msg = alloc::format!("{err}");
    assert!(
        msg.contains("print"),
        "PurityViolation Display should include callee name, got: {msg}"
    );
}

// -----------------------------------------------------------------------------
// Bytes stdlib built-in type-checking tests.
//
// These tests pin down the externally visible signatures registered for the
// `Bytes` standard-library surface. They are the RED phase of the
// Opalescent-language-level integration of `stdlib::bytes`.
// -----------------------------------------------------------------------------

#[test]
fn test_builtin_bytes_new_returns_bytes() {
    const SOURCE: &str = "
entry demo = f(): int32 => {
let buffer: Bytes = bytes_new()
let length: int32 = buffer.length
return length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "bytes_new() should produce a Bytes value exposing .length: {result:?}"
    );
}

#[test]
fn test_builtin_bytes_to_hex_returns_string() {
    const SOURCE: &str = "
entry demo = f(): string => {
let buffer: Bytes = bytes_new()
let hex: string = bytes_to_hex(buffer)
return hex
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "bytes_to_hex(Bytes) should produce a string: {result:?}"
    );
}

#[test]
fn test_builtin_bytes_concatenate_returns_bytes() {
    const SOURCE: &str = "
entry demo = f(): int32 => {
let a: Bytes = bytes_new()
let b: Bytes = bytes_new()
let joined: Bytes = bytes_concatenate(a, b)
let length: int32 = joined.length
return length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "bytes_concatenate(Bytes, Bytes) should produce a Bytes: {result:?}"
    );
}

#[test]
fn test_builtin_bytes_from_hex_requires_error_handling() {
    const SOURCE: &str = "
entry demo = f(hex: string): int32 => {
let buffer: Bytes = bytes_from_hex(hex)
return bytes_length(buffer)
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    let errors = result
        .expect_err("bare call to bytes_from_hex without guard/propagate must fail type-check");
    assert!(
        errors.iter().any(
            |e| matches!(*e, TypeError::UnhandledCallError { ref name, .. } if name == "bytes_from_hex"),
        ),
        "expected UnhandledCallError for bytes_from_hex, got: {errors:?}"
    );
}

#[test]
fn test_builtin_bytes_from_hex_type_checks_under_propagate() {
    const SOURCE: &str = "
entry demo = f(hex: string): int32 errors HexDecodeError => {
let buffer: Bytes = propagate bytes_from_hex(hex)
return buffer.length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "bytes_from_hex with propagate + declared errors HexDecodeError should type check: {result:?}"
    );
}

#[test]
fn test_builtin_bytes_slice_type_checks_under_propagate() {
    const SOURCE: &str = "
entry demo = f(source: Bytes, start: int32, end: int32): int32 errors SliceRangeError => {
let sub: Bytes = propagate bytes_slice(source, start, end)
return sub.length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "bytes_slice with propagate + declared errors SliceRangeError should type check: {result:?}"
    );
}

#[test]
fn test_string_length_member_type_checks_as_int64() {
    const SOURCE: &str = "
entry demo = f(): int64 => {
let message = 'hello'
return message.length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "string .length should type check as int64: {result:?}"
    );
}

#[test]
fn test_array_length_member_type_checks_as_int64() {
    const SOURCE: &str = "
entry demo = f(): int64 => {
let values = ['a', 'b', 'c']
return values.length
}
";
    let program = parse_program_from_source(SOURCE);
    let mut checker = TypeChecker::new();
    let result = checker.type_check_program(&program);
    assert!(
        result.is_ok(),
        "array .length should type check as int64: {result:?}"
    );
}
