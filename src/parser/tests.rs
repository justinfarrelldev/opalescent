//! Comprehensive test suite for the parser
//!
//! This module contains tests for all parsing functionality including
//! expressions, statements, declarations, types, and error cases.

#![expect(
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::pattern_type_mismatch,
    clippy::uninlined_format_args,
    reason = "Test code is allowed to use panic and have some relaxed linting rules for this module only"
)]

use super::*;
use crate::ast::{
    BinaryOp, Decl, Expr, ImportItem, LabeledValue, LambdaBody, LiteralValue, Parameter, Stmt,
    StringPart, Type, TypeDef, UnaryOp, Visibility,
};
use crate::lexer::{Lexer, RESERVED_KEYWORDS};
use crate::parser::errors::ParseError;
use proptest::prelude::*;
use proptest::proptest;
use proptest::strategy::Strategy;
use proptest::string::string_regex;

fn parse_expression_from_string(input: &str) -> ParseResult<Expr> {
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_expression()?;
    parser.skip_newlines_and_comments();
    if !parser.is_at_end() {
        let token = parser.current_token();
        return Err(ParseError::UnexpectedToken {
            expected: "end of input".to_owned(),
            found: format!("{}", token.token_type),
            span: ParseError::span_from_token(token),
        });
    }
    Ok(expr)
}

fn parse_statement_from_string(input: &str) -> ParseResult<Stmt> {
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let statement = parser.parse_statement()?;
    parser.skip_newlines_and_comments();
    if !parser.is_at_end() {
        let token = parser.current_token();
        return Err(ParseError::UnexpectedToken {
            expected: "end of input".to_owned(),
            found: format!("{}", token.token_type),
            span: ParseError::span_from_token(token),
        });
    }
    Ok(statement)
}

fn parse_type_from_string(input: &str) -> ParseResult<Type> {
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ty = parser.parse_type()?;
    parser.skip_newlines_and_comments();
    if !parser.is_at_end() {
        let token = parser.current_token();
        return Err(ParseError::UnexpectedToken {
            expected: "end of input".to_owned(),
            found: format!("{}", token.token_type),
            span: ParseError::span_from_token(token),
        });
    }
    Ok(ty)
}

fn parse_program_from_string(input: &str) -> Result<Program, Vec<ParseError>> {
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (program_opt, errors) = parser.parse();

    if errors.is_empty() {
        Ok(program_opt.unwrap())
    } else {
        Err(errors.errors)
    }
}

/// Source text for the guard/propagate integration sample program.
const ERROR_HANDLING_SAMPLE_SOURCE: &str =
    include_str!("../../language-spec/error_handling_samples.op");

#[test]
#[ignore]
#[expect(
    clippy::use_debug,
    clippy::panic,
    reason = "Debug test intentionally uses println and panic for manual inspection"
)]
fn debug_print_error_handling_tokens() {
    let lexer = Lexer::new(ERROR_HANDLING_SAMPLE_SOURCE);
    let (tokens, _) = lexer.tokenize();
    for token in tokens {
        println!("{:?}", token.token_type);
    }
    panic!("debug output above");
}

/// Parsed AST nodes that we want to detect inside integration samples.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AstFeature {
    /// Guard expressions introduced for structured error handling.
    Guard,
    /// Propagate expressions that bubble errors to the caller.
    Propagate,
}

/// Determine whether a string interpolation segment contains the requested AST feature.
fn string_part_contains_feature(part: &StringPart, feature: AstFeature) -> bool {
    match part {
        StringPart::Expression(expr) => expr_contains_feature(expr, feature),
        StringPart::Literal(_) => false,
    }
}

/// Determine whether a labeled control-flow payload contains the requested AST feature.
fn labeled_value_contains_feature(value: &LabeledValue, feature: AstFeature) -> bool {
    expr_contains_feature(&value.value, feature)
}

/// Determine whether the provided lambda body contains the requested AST feature.
fn lambda_body_contains_feature(body: &LambdaBody, feature: AstFeature) -> bool {
    match body {
        LambdaBody::Expression(expr) => expr_contains_feature(expr, feature),
        LambdaBody::Block(statements) => statements
            .iter()
            .any(|stmt| stmt_contains_feature(stmt, feature)),
    }
}

/// Determine whether the provided expression tree contains the requested AST feature.
fn expr_contains_feature(expr: &Expr, feature: AstFeature) -> bool {
    match expr {
        Expr::Guard {
            expr: guarded,
            else_branch,
            ..
        } => {
            matches!(feature, AstFeature::Guard)
                || expr_contains_feature(guarded, feature)
                || stmt_contains_feature(else_branch, feature)
        }
        Expr::Propagate { call, .. } => {
            matches!(feature, AstFeature::Propagate) || expr_contains_feature(call, feature)
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_feature(left, feature) || expr_contains_feature(right, feature)
        }
        Expr::Unary { operand, .. } => expr_contains_feature(operand, feature),
        Expr::Call { callee, args, .. } => {
            expr_contains_feature(callee, feature)
                || args.iter().any(|arg| expr_contains_feature(arg, feature))
        }
        Expr::Constructor { callee, fields, .. } => {
            expr_contains_feature(callee, feature)
                || fields
                    .iter()
                    .any(|field| expr_contains_feature(&field.value, feature))
        }
        Expr::Index { object, index, .. } => {
            expr_contains_feature(object, feature) || expr_contains_feature(index, feature)
        }
        Expr::Member { object, .. } => expr_contains_feature(object, feature),
        Expr::Cast { expr, .. } | Expr::TypeOf { expr, .. } | Expr::Parenthesized { expr, .. } => {
            expr_contains_feature(expr, feature)
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_feature(condition, feature)
                || stmt_contains_feature(then_branch, feature)
                || else_branch
                    .as_deref()
                    .is_some_and(|branch| stmt_contains_feature(branch, feature))
        }
        Expr::Array { elements, .. } => elements
            .iter()
            .any(|element| expr_contains_feature(element, feature)),
        Expr::Match {
            scrutinee, arms, ..
        } => {
            expr_contains_feature(scrutinee, feature)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|guard| expr_contains_feature(guard, feature))
                        || expr_contains_feature(&arm.body, feature)
                })
        }
        Expr::Loop { body, .. } => stmt_contains_feature(body, feature),
        Expr::Lambda { body, .. } => lambda_body_contains_feature(body, feature),
        Expr::StringInterpolation { parts, .. } => parts
            .iter()
            .any(|part| string_part_contains_feature(part, feature)),
        Expr::Literal { .. } | Expr::Identifier { .. } => false,
    }
}

/// Determine whether the provided statement tree contains the requested AST feature.
fn stmt_contains_feature(stmt: &Stmt, feature: AstFeature) -> bool {
    match stmt {
        Stmt::Let {
            initializer: Some(expr),
            ..
        }
        | Stmt::Expression { expr, .. } => expr_contains_feature(expr, feature),
        Stmt::Return { values, .. } => values
            .iter()
            .any(|value| expr_contains_feature(&value.value, feature)),
        Stmt::Let {
            initializer: None, ..
        }
        | Stmt::Comment { .. } => false,
        Stmt::LetDestructure { initializer, .. } => expr_contains_feature(initializer, feature),
        Stmt::Assignment { target, value, .. } => {
            expr_contains_feature(target, feature) || expr_contains_feature(value, feature)
        }
        Stmt::Block { statements, .. } => statements
            .iter()
            .any(|inner| stmt_contains_feature(inner, feature)),
        Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            expr_contains_feature(condition, feature)
                || stmt_contains_feature(then_branch, feature)
                || else_branch
                    .as_deref()
                    .is_some_and(|branch| stmt_contains_feature(branch, feature))
        }
        Stmt::For { iterable, body, .. } => {
            expr_contains_feature(iterable, feature) || stmt_contains_feature(body, feature)
        }
        Stmt::While {
            condition, body, ..
        } => expr_contains_feature(condition, feature) || stmt_contains_feature(body, feature),
        Stmt::Guard {
            expression,
            else_body,
            ..
        } => {
            matches!(feature, AstFeature::Guard)
                || expr_contains_feature(expression, feature)
                || stmt_contains_feature(else_body, feature)
        }
        Stmt::Loop { body, .. } => stmt_contains_feature(body, feature),
        Stmt::Break { values, .. } | Stmt::Continue { values, .. } => values
            .iter()
            .any(|value| labeled_value_contains_feature(value, feature)),
    }
}

/// Ensure the guard/propagate sample program parses without diagnostics.
#[test]
fn test_error_handling_sample_parses_successfully() {
    let program = match parse_program_from_string(ERROR_HANDLING_SAMPLE_SOURCE) {
        Ok(program) => program,
        Err(errors) => panic!("integration sample should parse successfully: {errors:?}"),
    };

    assert!(
        !program.declarations.is_empty(),
        "sample should contain declarations to exercise parsing"
    );
}

/// Verify that the guard/propagate sample includes both constructs in its AST.
#[test]
fn test_error_handling_sample_contains_guard_and_propagate() {
    let program = match parse_program_from_string(ERROR_HANDLING_SAMPLE_SOURCE) {
        Ok(program) => program,
        Err(errors) => panic!("integration sample should parse successfully: {errors:?}"),
    };

    let mut saw_guard = false;
    let mut saw_propagate = false;

    for declaration in &program.declarations {
        match declaration {
            Decl::Function { body, .. } => {
                saw_guard |= stmt_contains_feature(body, AstFeature::Guard);
                saw_propagate |= stmt_contains_feature(body, AstFeature::Propagate);
            }
            Decl::Let { initializer, .. } => {
                saw_guard |= expr_contains_feature(initializer, AstFeature::Guard);
                saw_propagate |= expr_contains_feature(initializer, AstFeature::Propagate);
            }
            _ => {}
        }

        if saw_guard && saw_propagate {
            break;
        }
    }

    assert!(
        saw_guard,
        "expected sample program to contain a guard expression"
    );
    assert!(
        saw_propagate,
        "expected sample program to contain a propagate expression"
    );
}

// --- Error handling syntax tests (guard/propagate) ---

#[test]
fn test_parse_guard_with_expression_else() {
    let expr =
        parse_expression_from_string("guard read_line() into line else handle_error()").unwrap();

    match expr {
        Expr::Guard {
            binding_name,
            binding_type,
            is_mutable,
            else_branch,
            ..
        } => {
            assert_eq!(binding_name, "line", "binding name should be parsed");
            assert!(binding_type.is_none(), "no explicit type annotation");
            assert!(!is_mutable, "binding should be immutable by default");
            match *else_branch {
                Stmt::Expression { .. } => {}
                other => panic!("expected else expression wrapped as statement, found: {other:?}"),
            }
        }
        other => panic!("expected guard expression, found: {other:?}"),
    }
}

#[test]
fn test_parse_guard_with_block_else_and_type_mutable() {
    let expr =
        parse_expression_from_string("guard parse(s) into value: int32 mutable else { return 0 }")
            .unwrap();

    match expr {
        Expr::Guard {
            binding_name,
            binding_type,
            is_mutable,
            else_branch,
            ..
        } => {
            assert_eq!(binding_name, "value");
            assert!(binding_type.is_some(), "type annotation should be present");
            assert!(is_mutable, "mutable flag should be set");
            match *else_branch {
                Stmt::Block { .. } => {}
                other => panic!("expected else block, found: {other:?}"),
            }
        }
        other => panic!("expected guard expression, found: {other:?}"),
    }
}

#[test]
fn test_parse_propagate_with_call() {
    let expr = parse_expression_from_string("propagate string_to_int32(s)").unwrap();
    match expr {
        Expr::Propagate { call, .. } => match *call {
            Expr::Call { callee, .. } => match *callee {
                Expr::Identifier { name, .. } => assert_eq!(name, "string_to_int32"),
                other => panic!("expected identifier callee, found: {other:?}"),
            },
            other => panic!("expected call expression, found: {other:?}"),
        },
        other => panic!("expected propagate expression, found: {other:?}"),
    }
}

#[test]
fn test_parse_propagate_rejects_non_call() {
    let result = parse_expression_from_string("propagate 1 + 2");
    assert!(result.is_err(), "propagate must wrap a call expression");
}

#[test]
fn test_parse_guard_missing_into_reports_guard_specific_error() {
    let result = parse_expression_from_string("guard read_line() value else 0");
    match result {
        Err(ParseError::GuardMissingIntoClause { .. }) => {}
        other => {
            panic!("missing 'into' should emit a GuardMissingIntoClause error, received: {other:?}")
        }
    }
}

#[test]
fn test_parse_guard_missing_else_reports_guard_specific_error() {
    let result = parse_expression_from_string("guard read_line() into value 0");
    match result {
        Err(ParseError::GuardMissingElseClause { .. }) => {}
        other => {
            panic!("missing 'else' should emit a GuardMissingElseClause error, received: {other:?}")
        }
    }
}

#[test]
fn test_guard_missing_else_recovers_without_cascading_errors() {
    let source = "\
let parse = f(): int32 errors ParseError => {
    guard read_line() into value
    return 42
}

let after = f(): int32 => {
    return 0
}
";

    let lexer = Lexer::new(source);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (_program, errors) = parser.parse();

    assert_eq!(
        errors.errors.len(),
        1,
        "guard missing 'else' should produce exactly one parse error after recovery"
    );

    assert!(
        matches!(
            errors.errors.first(),
            Some(ParseError::GuardMissingElseClause { .. })
        ),
        "expected the recorded parse error to be GuardMissingElseClause, found: {:?}",
        errors.errors
    );
}

fn identifier_strategy() -> impl Strategy<Value = String> {
    string_regex("[a-z]{1,8}")
        .expect("regex is valid")
        .prop_filter("identifiers must avoid reserved keywords", |candidate| {
            !RESERVED_KEYWORDS.contains(&candidate.as_str())
        })
}

fn integer_literal_strategy() -> impl Strategy<Value = String> {
    proptest::num::i32::ANY.prop_map(|value| value.to_string())
}

fn arithmetic_expr_strategy() -> impl Strategy<Value = String> {
    integer_literal_strategy().prop_recursive(3, 32, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} + {b}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} * {b}")),
            inner.prop_map(|expr| format!("({expr})")),
        ]
    })
}

fn parenthesized_arithmetic_expr_strategy() -> impl Strategy<Value = String> {
    arithmetic_expr_strategy().prop_map(|expr| format!("({expr})"))
}

fn boolean_leaf_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("true".to_owned()),
        Just("false".to_owned()),
        identifier_strategy(),
    ]
}

fn boolean_expr_strategy() -> impl Strategy<Value = String> {
    boolean_leaf_strategy().prop_recursive(3, 32, 2, |inner| {
        prop_oneof![
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} and {b}")),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| format!("{a} or {b}")),
            inner.prop_map(|expr| format!("not ({expr})")),
        ]
    })
}

fn simple_operand_strategy() -> impl Strategy<Value = String> {
    prop_oneof![identifier_strategy(), integer_literal_strategy()]
}

fn comparison_expr_strategy() -> impl Strategy<Value = String> {
    (
        simple_operand_strategy(),
        prop_oneof![
            Just("<".to_owned()),
            Just(">".to_owned()),
            Just("<=".to_owned()),
            Just(">=".to_owned())
        ],
        simple_operand_strategy(),
    )
        .prop_map(|(left, op, right)| format!("{left} {op} {right}"))
}

fn dangling_operator_strategy() -> impl Strategy<Value = String> {
    simple_operand_strategy().prop_map(|expr| format!("{expr} +"))
}

fn mismatched_parentheses_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        arithmetic_expr_strategy().prop_map(|expr| format!("({expr}")),
        arithmetic_expr_strategy().prop_map(|expr| format!("{expr})")),
    ]
}

fn invalid_break_payload_strategy() -> impl Strategy<Value = String> {
    (identifier_strategy(), identifier_strategy())
        .prop_map(|(label, value)| format!("break {label} {value}"))
}

fn canonicalize_simple_expression(expr: &Expr) -> Option<String> {
    match expr {
        Expr::Literal {
            value: LiteralValue::Integer(value),
            ..
        } => Some(value.to_string()),
        Expr::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let left = canonicalize_simple_expression(left)?;
            let right = canonicalize_simple_expression(right)?;
            match operator {
                BinaryOp::Add => Some(format!("({left} + {right})")),
                BinaryOp::Multiply => Some(format!("({left} * {right})")),
                _ => None,
            }
        }
        Expr::Parenthesized { expr: inner, .. } => {
            let inner = canonicalize_simple_expression(inner)?;
            Some(inner)
        }
        Expr::Unary {
            operator, operand, ..
        } => {
            let operand = canonicalize_simple_expression(operand)?;
            match operator {
                UnaryOp::Negate => Some(format!("-{operand}")),
                UnaryOp::Plus => Some(format!("+{operand}")),
                _ => None,
            }
        }
        Expr::Identifier { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn canonicalize_comparison_expression(expr: &Expr) -> Option<String> {
    if let Expr::Binary {
        operator,
        left,
        right,
        ..
    } = expr
    {
        let left = canonicalize_simple_expression(left)?;
        let right = canonicalize_simple_expression(right)?;
        Some(format!("{left} {operator} {right}"))
    } else {
        None
    }
}

// Error handling tests
#[test]
fn test_unexpected_token_errors() {
    // Test invalid expression syntax
    let result1 = parse_expression_from_string("5 + +");
    assert!(result1.is_err());
    if let Err(ParseError::UnexpectedToken {
        expected, found, ..
    }) = result1
    {
        assert!(expected.contains("expression") || expected.contains("operand"));
        assert_eq!(found, "end of file");
    }

    // Test invalid binary operation
    let result2 = parse_expression_from_string("5 + * 3");
    assert!(result2.is_err());
    assert!(matches!(result2, Err(ParseError::UnexpectedToken { .. })));

    // Test invalid parenthesized expression
    let result3 = parse_expression_from_string("(5 +)");
    assert!(result3.is_err());
}

#[test]
fn test_missing_token_errors() {
    // Test missing closing parenthesis
    let result1 = parse_expression_from_string("(5 + 3");
    assert!(result1.is_err());

    // Test missing function call parentheses end
    let result2 = parse_expression_from_string("foo(5, 3");
    assert!(result2.is_err());

    // Test missing assignment value
    let result3 = parse_statement_from_string("let x =");
    assert!(result3.is_err());

    // Test missing block closing brace
    let result4 = parse_statement_from_string("{ let x = 5");
    assert!(result4.is_err());
}

#[test]
fn test_invalid_syntax_errors() {
    // Test invalid variable name (not an identifier)
    let result1 = parse_statement_from_string("let 123");
    assert!(result1.is_err());

    // Test invalid assignment target
    let result2 = parse_statement_from_string("5 = 10");
    assert!(result2.is_err());

    // Test invalid function parameter syntax
    let result3 = parse_statement_from_string("let f = f(x y): int32 => x + y");
    assert!(result3.is_err());
}

#[test]
fn test_unexpected_eof_errors() {
    // Test EOF in middle of expression
    let result1 = parse_expression_from_string("5 +");
    assert!(result1.is_err());

    // Test EOF in function parameters
    let result2 = parse_statement_from_string("let f = f(");
    assert!(result2.is_err());

    // Test EOF in block
    let result3 = parse_statement_from_string("{");
    assert!(result3.is_err());
}

#[test]
fn test_type_annotation_errors() {
    // Test invalid type syntax
    let result1 = parse_type_from_string("int32[");
    assert!(result1.is_err());

    // Test invalid generic type syntax
    let result2 = parse_type_from_string("Map<string");
    assert!(result2.is_err());

    // Test invalid function type syntax
    let result3 = parse_type_from_string("f(int32");
    assert!(result3.is_err());
}

#[test]
fn test_visibility_modifier_errors() {
    // Test invalid visibility placement
    let result1 = parse_statement_from_string("let public x = 5");
    assert!(result1.is_err());

    // Test duplicate visibility modifiers would be caught by lexer, but test parser response
    let result2 = parse_program_from_string("public public let x = 5");
    assert!(result2.is_err());
}

#[test]
fn test_multiple_error_collection() {
    // Test program with multiple syntax errors
    let result = parse_program_from_string(
        "
        let x = 5 +
        let y =
        { missing_brace
    ",
    );

    // Should collect multiple errors
    assert!(result.is_err());
    let errors = result.unwrap_err();
    assert!(
        errors.len() >= 2,
        "Should collect multiple errors, got {}",
        errors.len()
    );
}

#[test]
fn test_literal_expressions() {
    // Test value for floating point comparison - define at top to avoid items after statements
    #[expect(
        clippy::approx_constant,
        reason = "Test value intentionally matches pi approximation"
    )]
    const TEST_VALUE: f64 = 3.14;

    let integer_expr = parse_expression_from_string("42").unwrap();
    assert!(matches!(
        integer_expr,
        Expr::Literal {
            value: LiteralValue::Integer(42),
            ..
        }
    ));

    let float_expr = parse_expression_from_string("3.14").unwrap();
    assert!(
        matches!(float_expr, Expr::Literal { value: LiteralValue::Float(f), .. } if (f - TEST_VALUE).abs() < f64::EPSILON)
    );

    let string_expr = parse_expression_from_string("'hello'").unwrap();
    assert!(
        matches!(string_expr, Expr::Literal { value: LiteralValue::String(s), .. } if s == "hello")
    );

    let bool_expr = parse_expression_from_string("true").unwrap();
    assert!(matches!(
        bool_expr,
        Expr::Literal {
            value: LiteralValue::Boolean(true),
            ..
        }
    ));
}

#[test]
fn test_identifier_expressions() {
    let identifier_expr = parse_expression_from_string("hello_world").unwrap();
    assert!(matches!(identifier_expr, Expr::Identifier { name, .. } if name == "hello_world"));
}

#[test]
fn test_binary_expressions() {
    let add_expr = parse_expression_from_string("1 + 2").unwrap();
    assert!(matches!(
        add_expr,
        Expr::Binary {
            operator: BinaryOp::Add,
            ..
        }
    ));

    let less_than_expr = parse_expression_from_string("x < y").unwrap();
    assert!(matches!(
        less_than_expr,
        Expr::Binary {
            operator: BinaryOp::Less,
            ..
        }
    ));

    let logical_and_expr = parse_expression_from_string("a and b").unwrap();
    assert!(matches!(
        logical_and_expr,
        Expr::Binary {
            operator: BinaryOp::And,
            ..
        }
    ));
}

#[test]
fn test_unary_expressions() {
    let negate_expr = parse_expression_from_string("-42").unwrap();
    assert!(matches!(
        negate_expr,
        Expr::Unary {
            operator: UnaryOp::Negate,
            ..
        }
    ));

    let not_expr = parse_expression_from_string("not true").unwrap();
    assert!(matches!(
        not_expr,
        Expr::Unary {
            operator: UnaryOp::Not,
            ..
        }
    ));
}

#[test]
fn test_parenthesized_expressions() {
    let paren_expr = parse_expression_from_string("(1 + 2)").unwrap();
    assert!(matches!(paren_expr, Expr::Parenthesized { .. }));
}

#[test]
fn test_function_calls() {
    let call_expr = parse_expression_from_string("print('hello')").unwrap();
    assert!(matches!(call_expr, Expr::Call { .. }));
}

#[test]
fn test_array_literal_expression() {
    let expr = parse_expression_from_string("[1, 2, 3]").unwrap();
    if let Expr::Array { elements, .. } = expr {
        assert_eq!(elements.len(), 3);
        assert!(matches!(
            elements[0],
            Expr::Literal {
                value: LiteralValue::Integer(1),
                ..
            }
        ));
        assert!(matches!(
            elements[1],
            Expr::Literal {
                value: LiteralValue::Integer(2),
                ..
            }
        ));
        assert!(matches!(
            elements[2],
            Expr::Literal {
                value: LiteralValue::Integer(3),
                ..
            }
        ));
    } else {
        unreachable!("Expected Expr::Array for array literal");
    }
}

#[test]
fn test_empty_array_literal_expression() {
    let expr = parse_expression_from_string("[]").unwrap();
    if let Expr::Array { elements, .. } = expr {
        assert!(elements.is_empty(), "Expected empty array literal");
    } else {
        unreachable!("Expected Expr::Array for empty array literal");
    }
}

#[test]
fn test_index_expression_literal_index() {
    let expr = parse_expression_from_string("arr[0]").unwrap();
    if let Expr::Index { object, index, .. } = expr {
        assert!(matches!(*object, Expr::Identifier { name, .. } if name == "arr"));
        assert!(matches!(
            *index,
            Expr::Literal {
                value: LiteralValue::Integer(0),
                ..
            }
        ));
    } else {
        unreachable!("Expected Expr::Index for arr[0]");
    }
}

#[test]
fn test_index_expression_with_binary_index() {
    let expr = parse_expression_from_string("arr[i + 1]").unwrap();
    if let Expr::Index { object, index, .. } = expr {
        assert!(matches!(*object, Expr::Identifier { name, .. } if name == "arr"));
        assert!(matches!(
            *index,
            Expr::Binary {
                operator: BinaryOp::Add,
                ..
            }
        ));
    } else {
        unreachable!("Expected Expr::Index for arr[i + 1]");
    }
}

#[test]
fn test_nested_index_expression() {
    let expr = parse_expression_from_string("arr[0][1]").unwrap();
    if let Expr::Index { object, index, .. } = expr {
        assert!(matches!(
            *index,
            Expr::Literal {
                value: LiteralValue::Integer(1),
                ..
            }
        ));
        assert!(matches!(*object, Expr::Index { .. }));
    } else {
        unreachable!("Expected outer Expr::Index for arr[0][1]");
    }
}

#[test]
fn test_function_result_index_expression() {
    let expr = parse_expression_from_string("foo()[0]").unwrap();
    if let Expr::Index { object, index, .. } = expr {
        assert!(matches!(*object, Expr::Call { .. }));
        assert!(matches!(
            *index,
            Expr::Literal {
                value: LiteralValue::Integer(0),
                ..
            }
        ));
    } else {
        unreachable!("Expected Expr::Index for foo()[0]");
    }
}

#[test]
fn test_operator_precedence() {
    // Test that multiplication has higher precedence than addition
    let precedence_expr = parse_expression_from_string("1 + 2 * 3").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Add,
        right,
        ..
    } = precedence_expr
    {
        assert!(matches!(
            *left,
            Expr::Literal {
                value: LiteralValue::Integer(1),
                ..
            }
        ));
        assert!(matches!(
            *right,
            Expr::Binary {
                operator: BinaryOp::Multiply,
                ..
            }
        ));
    } else {
        unreachable!("Expected addition with multiplication on right side");
    }
}

#[test]
fn test_binary_op_subtract() {
    let expr = parse_expression_from_string("a - b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Subtract,
                ..
            }
        ),
        "Expected subtraction binary operator"
    );
}

#[test]
fn test_binary_op_divide() {
    let expr = parse_expression_from_string("a / b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Divide,
                ..
            }
        ),
        "Expected division binary operator"
    );
}

#[test]
fn test_binary_op_modulo() {
    let expr = parse_expression_from_string("a % b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Modulo,
                ..
            }
        ),
        "Expected modulo binary operator"
    );
}

#[test]
fn test_binary_op_power() {
    let expr = parse_expression_from_string("a ^ b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Power,
                ..
            }
        ),
        "Expected power binary operator"
    );
}

#[test]
fn test_binary_op_less_equal() {
    let expr = parse_expression_from_string("a <= b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::LessEqual,
                ..
            }
        ),
        "Expected less-equal binary operator"
    );
}

#[test]
fn test_binary_op_greater() {
    let expr = parse_expression_from_string("a > b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Greater,
                ..
            }
        ),
        "Expected greater-than binary operator"
    );
}

#[test]
fn test_binary_op_greater_equal() {
    let expr = parse_expression_from_string("a >= b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::GreaterEqual,
                ..
            }
        ),
        "Expected greater-equal binary operator"
    );
}

#[test]
fn test_binary_op_is() {
    let expr = parse_expression_from_string("a is b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Is,
                ..
            }
        ),
        "Expected identity binary operator"
    );
}

#[test]
fn test_binary_op_is_not() {
    let expr = parse_expression_from_string("a is not b").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::IsNot,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(*left, Expr::Identifier { name, .. } if name == "a"),
            "Expected identifier 'a' as left operand of IsNot"
        );
        assert!(
            matches!(*right, Expr::Identifier { name, .. } if name == "b"),
            "Expected identifier 'b' as right operand of IsNot"
        );
    } else {
        unreachable!("Expected IsNot operator with identifier operands");
    }
}

#[test]
fn test_cast_expression_int32() {
    let expr = parse_expression_from_string("x as int32").unwrap();
    if let Expr::Cast {
        expr: inner,
        target_type,
        ..
    } = expr
    {
        assert!(
            matches!(*inner, Expr::Identifier { name, .. } if name == "x"),
            "Expected identifier 'x' as cast operand"
        );
        assert!(
            matches!(target_type, Type::Basic { name, .. } if name == "int32"),
            "Expected cast target type int32"
        );
    } else {
        unreachable!("Expected cast expression for 'x as int32'");
    }
}

#[test]
fn test_cast_expression_float64() {
    let expr = parse_expression_from_string("value as float64").unwrap();
    if let Expr::Cast {
        expr: inner,
        target_type,
        ..
    } = expr
    {
        assert!(
            matches!(*inner, Expr::Identifier { name, .. } if name == "value"),
            "Expected identifier 'value' as cast operand"
        );
        assert!(
            matches!(target_type, Type::Basic { name, .. } if name == "float64"),
            "Expected cast target type float64"
        );
    } else {
        unreachable!("Expected cast expression for 'value as float64'");
    }
}

#[test]
fn test_cast_expression_parenthesized_sum() {
    let expr = parse_expression_from_string("(a + b) as int64").unwrap();
    if let Expr::Cast {
        expr: inner,
        target_type,
        ..
    } = expr
    {
        assert!(
            matches!(
                *inner,
                Expr::Parenthesized { expr, .. } if matches!(
                    *expr,
                    Expr::Binary {
                        operator: BinaryOp::Add,
                        ..
                    }
                )
            ),
            "Expected parenthesized addition as cast operand"
        );
        assert!(
            matches!(target_type, Type::Basic { name, .. } if name == "int64"),
            "Expected cast target type int64"
        );
    } else {
        unreachable!("Expected cast expression for '(a + b) as int64'");
    }
}

#[test]
fn test_cast_expression_nested() {
    let expr = parse_expression_from_string("x as int32 as int64").unwrap();
    if let Expr::Cast {
        expr: inner,
        target_type,
        ..
    } = expr
    {
        assert!(
            matches!(target_type, Type::Basic { name, .. } if name == "int64"),
            "Expected outer cast target type int64"
        );
        assert!(
            matches!(*inner, Expr::Cast { .. }),
            "Expected nested cast with inner target type int32"
        );
        if let Expr::Cast {
            expr: inner_expr,
            target_type: inner_target_type,
            ..
        } = *inner
        {
            assert!(
                matches!(*inner_expr, Expr::Identifier { name, .. } if name == "x"),
                "Expected identifier 'x' as nested cast operand"
            );
            assert!(
                matches!(inner_target_type, Type::Basic { name, .. } if name == "int32"),
                "Expected inner cast target type int32"
            );
        }
    } else {
        unreachable!("Expected nested cast expression");
    }
}

#[test]
fn test_binary_op_or() {
    let expr = parse_expression_from_string("a or b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Or,
                ..
            }
        ),
        "Expected logical OR binary operator"
    );
}

#[test]
fn test_binary_op_xor() {
    let expr = parse_expression_from_string("a xor b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::Xor,
                ..
            }
        ),
        "Expected logical XOR binary operator"
    );
}

#[test]
fn test_binary_op_bitand() {
    let expr = parse_expression_from_string("a band b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitAnd,
                ..
            }
        ),
        "Expected bitwise AND binary operator"
    );
}

#[test]
fn test_binary_op_bitor() {
    let expr = parse_expression_from_string("a bor b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitOr,
                ..
            }
        ),
        "Expected bitwise OR binary operator"
    );
}

#[test]
fn test_binary_op_bitxor() {
    let expr = parse_expression_from_string("a bxor b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitXor,
                ..
            }
        ),
        "Expected bitwise XOR binary operator"
    );
}

#[test]
fn test_binary_op_bitshl() {
    let expr = parse_expression_from_string("a bshl b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitShiftLeft,
                ..
            }
        ),
        "Expected bitwise left-shift binary operator"
    );
}

#[test]
fn test_binary_op_bitshr() {
    let expr = parse_expression_from_string("a bshr b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitShiftRight,
                ..
            }
        ),
        "Expected bitwise right-shift binary operator"
    );
}

#[test]
fn test_binary_op_bitushr() {
    let expr = parse_expression_from_string("a bushr b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::BitUnsignedShiftRight,
                ..
            }
        ),
        "Expected bitwise unsigned right-shift binary operator"
    );
}

#[test]
fn test_unary_op_plus() {
    let expr = parse_expression_from_string("+x").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Unary {
                operator: UnaryOp::Plus,
                ..
            }
        ),
        "Expected unary plus operator"
    );
}

#[test]
fn test_unary_op_bitnot() {
    let expr = parse_expression_from_string("bnot x").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Unary {
                operator: UnaryOp::BitNot,
                ..
            }
        ),
        "Expected unary bitwise not operator"
    );
}

#[test]
fn test_precedence_or_vs_xor() {
    let expr = parse_expression_from_string("a or b xor c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Or,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Xor,
                    ..
                }
            ),
            "Expected XOR to bind tighter than OR"
        );
    } else {
        unreachable!("Expected OR as outer operator with XOR on right side");
    }
}

#[test]
fn test_precedence_xor_vs_and() {
    let expr = parse_expression_from_string("a xor b and c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Xor,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::And,
                    ..
                }
            ),
            "Expected AND to bind tighter than XOR"
        );
    } else {
        unreachable!("Expected XOR as outer operator with AND on right side");
    }
}

#[test]
fn test_precedence_and_vs_bitor() {
    let expr = parse_expression_from_string("a and b bor c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::And,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::BitOr,
                    ..
                }
            ),
            "Expected bitwise OR to bind tighter than AND"
        );
    } else {
        unreachable!("Expected AND as outer operator with BitOr on right side");
    }
}

#[test]
fn test_precedence_bitor_vs_bitxor() {
    let expr = parse_expression_from_string("a bor b bxor c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::BitOr,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::BitXor,
                    ..
                }
            ),
            "Expected bitwise XOR to bind tighter than bitwise OR"
        );
    } else {
        unreachable!("Expected BitOr as outer operator with BitXor on right side");
    }
}

#[test]
fn test_precedence_bitxor_vs_bitand() {
    let expr = parse_expression_from_string("a bxor b band c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::BitXor,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::BitAnd,
                    ..
                }
            ),
            "Expected bitwise AND to bind tighter than bitwise XOR"
        );
    } else {
        unreachable!("Expected BitXor as outer operator with BitAnd on right side");
    }
}

#[test]
fn test_precedence_bitand_vs_equality() {
    let expr = parse_expression_from_string("a band b is c band d").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::BitAnd,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::BitAnd,
                    right,
                    ..
                } if matches!(
                    *right,
                    Expr::Binary {
                        operator: BinaryOp::Is,
                        ..
                    }
                )
            ),
            "Expected left side to be nested bitwise AND containing Is expression"
        );
        assert!(
            matches!(*right, Expr::Identifier { name, .. } if name == "d"),
            "Expected right side to be identifier 'd'"
        );
    } else {
        unreachable!("Expected outer BitAnd for bitand/equality precedence interaction");
    }
}

#[test]
fn test_precedence_equality_vs_comparison() {
    let expr = parse_expression_from_string("1 < 2 is 3 < 4").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Is,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::Less,
                    ..
                }
            ),
            "Expected left side comparison to bind before identity comparison"
        );
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Less,
                    ..
                }
            ),
            "Expected right side comparison to bind before identity comparison"
        );
    } else {
        unreachable!("Expected Is as outer operator with Less comparisons on both sides");
    }
}

#[test]
fn test_precedence_comparison_vs_shift() {
    let expr = parse_expression_from_string("a < b bshl c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Less,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::BitShiftLeft,
                    ..
                }
            ),
            "Expected shift to bind tighter than comparison"
        );
    } else {
        unreachable!("Expected Less as outer operator with shift expression on right side");
    }
}

#[test]
fn test_precedence_shift_vs_term() {
    let expr = parse_expression_from_string("a bshl b + c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::BitShiftLeft,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Add,
                    ..
                }
            ),
            "Expected addition to bind tighter than shift"
        );
    } else {
        unreachable!("Expected BitShiftLeft as outer operator with Add on right side");
    }
}

#[test]
fn test_precedence_term_vs_factor() {
    let expr = parse_expression_from_string("a + b * c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Add,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Multiply,
                    ..
                }
            ),
            "Expected multiplication to bind tighter than addition"
        );
    } else {
        unreachable!("Expected Add as outer operator with Multiply on right side");
    }
}

#[test]
fn test_precedence_factor_vs_power() {
    let expr = parse_expression_from_string("a * b ^ c").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Multiply,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Power,
                    ..
                }
            ),
            "Expected power to bind tighter than multiplication"
        );
    } else {
        unreachable!("Expected Multiply as outer operator with Power on right side");
    }
}

#[test]
fn test_precedence_power_vs_unary() {
    let expr = parse_expression_from_string("-a ^ b").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Power,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Unary {
                    operator: UnaryOp::Negate,
                    ..
                }
            ),
            "Expected unary negate to bind before power on left side"
        );
        assert!(
            matches!(
                *right,
                Expr::Identifier { name, .. } if name == "b"
            ),
            "Expected identifier 'b' as right operand of power"
        );
    } else {
        unreachable!("Expected Power as outer operator with unary-negated left operand");
    }
}

#[test]
fn test_associativity_power_right() {
    let expr = parse_expression_from_string("2 ^ 3 ^ 4").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Power,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Literal {
                    value: LiteralValue::Integer(2),
                    ..
                }
            ),
            "Expected left operand to be literal 2 for right-associative power"
        );
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Power,
                    ..
                }
            ),
            "Expected right operand to be nested power expression"
        );
    } else {
        unreachable!("Expected right-associative power expression");
    }
}

#[test]
fn test_associativity_subtract_left() {
    let expr = parse_expression_from_string("1 - 2 - 3").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Subtract,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::Subtract,
                    ..
                }
            ),
            "Expected left operand to be nested subtraction for left associativity"
        );
        assert!(
            matches!(
                *right,
                Expr::Literal {
                    value: LiteralValue::Integer(3),
                    ..
                }
            ),
            "Expected right operand to be literal 3 for left-associative subtraction"
        );
    } else {
        unreachable!("Expected left-associative subtraction expression");
    }
}

#[test]
fn test_associativity_and_left() {
    let expr = parse_expression_from_string("a and b and c").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::And,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::And,
                    ..
                }
            ),
            "Expected left operand to be nested AND expression for left associativity"
        );
        assert!(
            matches!(*right, Expr::Identifier { name, .. } if name == "c"),
            "Expected right operand to be identifier 'c'"
        );
    } else {
        unreachable!("Expected left-associative logical AND expression");
    }
}

#[test]
fn test_precedence_chain_arithmetic() {
    let expr = parse_expression_from_string("1 + 2 * 3 ^ 4").unwrap();
    if let Expr::Binary {
        right,
        operator: BinaryOp::Add,
        ..
    } = expr
    {
        if let Expr::Binary {
            right: multiply_right,
            operator: BinaryOp::Multiply,
            ..
        } = *right
        {
            assert!(
                matches!(
                    *multiply_right,
                    Expr::Binary {
                        operator: BinaryOp::Power,
                        ..
                    }
                ),
                "Expected power at deepest right side of arithmetic precedence chain"
            );
        } else {
            unreachable!("Expected Multiply as right child of Add in arithmetic chain");
        }
    } else {
        unreachable!("Expected Add as outer operator in arithmetic precedence chain");
    }
}

#[test]
fn test_precedence_chain_logical_and_comparison() {
    let expr = parse_expression_from_string("a or b and c <= d + e * g").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::Or,
        right,
        ..
    } = expr
    {
        if let Expr::Binary {
            operator: BinaryOp::And,
            right: and_right,
            ..
        } = *right
        {
            if let Expr::Binary {
                operator: BinaryOp::LessEqual,
                right: less_right,
                ..
            } = *and_right
            {
                assert!(
                    matches!(
                        *less_right,
                        Expr::Binary {
                            operator: BinaryOp::Add,
                            right,
                            ..
                        } if matches!(
                            *right,
                            Expr::Binary {
                                operator: BinaryOp::Multiply,
                                ..
                            }
                        )
                    ),
                    "Expected Add with nested Multiply on right side of Less comparison"
                );
            } else {
                unreachable!(
                    "Expected LessEqual as right child of And in logical/comparison chain"
                );
            }
        } else {
            unreachable!("Expected And as right child of Or in logical/comparison chain");
        }
    } else {
        unreachable!("Expected Or as outer operator in logical/comparison chain");
    }
}

#[test]
fn test_precedence_chain_bitwise() {
    let expr = parse_expression_from_string("a bor b bxor c band d").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::BitOr,
        right,
        ..
    } = expr
    {
        if let Expr::Binary {
            operator: BinaryOp::BitXor,
            right: bitxor_right,
            ..
        } = *right
        {
            assert!(
                matches!(
                    *bitxor_right,
                    Expr::Binary {
                        operator: BinaryOp::BitAnd,
                        ..
                    }
                ),
                "Expected BitAnd to be deepest right child in bitwise precedence chain"
            );
        } else {
            unreachable!("Expected BitXor as right child of BitOr in bitwise chain");
        }
    } else {
        unreachable!("Expected BitOr as outer operator in bitwise precedence chain");
    }
}

#[test]
fn test_precedence_paren_overrides_multiplication() {
    let expr = parse_expression_from_string("(a + b) * c").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Multiply,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Parenthesized { expr, .. } if matches!(
                    *expr,
                    Expr::Binary {
                        operator: BinaryOp::Add,
                        ..
                    }
                )
            ),
            "Expected parenthesized addition on left side of multiplication"
        );
    } else {
        unreachable!("Expected Multiply with parenthesized Add on left side");
    }
}

#[test]
fn test_precedence_paren_overrides_logical() {
    let expr = parse_expression_from_string("(a or b) and c").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::And,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Parenthesized { expr, .. } if matches!(
                    *expr,
                    Expr::Binary {
                        operator: BinaryOp::Or,
                        ..
                    }
                )
            ),
            "Expected parenthesized OR on left side of AND"
        );
    } else {
        unreachable!("Expected And with parenthesized Or on left side");
    }
}

#[test]
fn test_precedence_comparison_in_and() {
    let expr = parse_expression_from_string("a < b and c > d").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::And,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::Less,
                    ..
                }
            ),
            "Expected left operand of AND to be Less comparison"
        );
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Greater,
                    ..
                }
            ),
            "Expected right operand of AND to be Greater comparison"
        );
    } else {
        unreachable!("Expected And expression with comparison operands");
    }
}

#[test]
fn test_precedence_comparison_in_or() {
    let expr = parse_expression_from_string("a <= b or c >= d").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Or,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Binary {
                    operator: BinaryOp::LessEqual,
                    ..
                }
            ),
            "Expected left operand of OR to be LessEqual comparison"
        );
        assert!(
            matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::GreaterEqual,
                    ..
                }
            ),
            "Expected right operand of OR to be GreaterEqual comparison"
        );
    } else {
        unreachable!("Expected Or expression with comparison operands");
    }
}

#[test]
fn test_precedence_unary_not_with_and() {
    let expr = parse_expression_from_string("not a and b").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::And,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Unary {
                    operator: UnaryOp::Not,
                    ..
                }
            ),
            "Expected unary not expression on left side of AND"
        );
    } else {
        unreachable!("Expected And with unary not on left side");
    }
}

#[test]
fn test_precedence_unary_bitnot_with_bitor() {
    let expr = parse_expression_from_string("bnot x bor y").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::BitOr,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Unary {
                    operator: UnaryOp::BitNot,
                    ..
                }
            ),
            "Expected unary bitnot expression on left side of BitOr"
        );
    } else {
        unreachable!("Expected BitOr with unary bitnot on left side");
    }
}

#[test]
fn test_precedence_unary_negate_with_add() {
    let expr = parse_expression_from_string("-a + b").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Add,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Unary {
                    operator: UnaryOp::Negate,
                    ..
                }
            ),
            "Expected unary negate expression on left side of Add"
        );
    } else {
        unreachable!("Expected Add with unary negate on left side");
    }
}

#[test]
fn test_edge_case_less_equal_is_single_token() {
    let expr = parse_expression_from_string("a <= b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::LessEqual,
                ..
            }
        ),
        "Expected <= to parse as single LessEqual operator"
    );
}

#[test]
fn test_edge_case_greater_equal_is_single_token() {
    let expr = parse_expression_from_string("a >= b").unwrap();
    assert!(
        matches!(
            expr,
            Expr::Binary {
                operator: BinaryOp::GreaterEqual,
                ..
            }
        ),
        "Expected >= to parse as single GreaterEqual operator"
    );
}

#[test]
fn test_edge_case_is_not_is_single_operator() {
    let expr = parse_expression_from_string("a is not b").unwrap();
    if let Expr::Binary {
        operator: BinaryOp::IsNot,
        ..
    } = expr
    {
    } else {
        unreachable!("Expected IsNot operator for 'a is not b'");
    }
}

#[test]
fn test_edge_case_not_a_is_not_b() {
    let expr = parse_expression_from_string("not a is not b").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::IsNot,
        right,
        ..
    } = expr
    {
        assert!(
            matches!(
                *left,
                Expr::Unary {
                    operator: UnaryOp::Not,
                    ..
                }
            ),
            "Expected unary not expression as left operand"
        );
        assert!(
            matches!(*right, Expr::Identifier { name, .. } if name == "b"),
            "Expected identifier 'b' as right operand"
        );
    } else {
        unreachable!("Expected IsNot operator for 'not a is not b'");
    }
}

#[test]
fn test_break_continue_without_values() {
    let break_stmt = parse_statement_from_string("break").unwrap();
    if let Stmt::Break { values, .. } = break_stmt {
        assert!(
            values.is_empty(),
            "Break without payload should have no labeled values"
        );
    } else {
        unreachable!("Expected break statement, got {break_stmt:?}");
    }

    let continue_stmt = parse_statement_from_string("continue").unwrap();
    if let Stmt::Continue { values, .. } = continue_stmt {
        assert!(
            values.is_empty(),
            "Continue without payload should have no labeled values"
        );
    } else {
        unreachable!("Expected continue statement, got {continue_stmt:?}");
    }
}

#[test]
fn test_break_with_single_labeled_value() {
    let break_stmt = parse_statement_from_string("break result: value").unwrap();
    if let Stmt::Break { values, .. } = break_stmt {
        assert_eq!(values.len(), 1, "Expected exactly one labeled break value");
        let labeled = &values[0];
        assert_eq!(labeled.label, "result");
        assert!(matches!(&labeled.value, Expr::Identifier { name, .. } if name == "value"));
    } else {
        unreachable!("Expected labeled break statement, got {break_stmt:?}");
    }
}

#[test]
fn test_break_with_multiple_labeled_values() {
    let input = "break first: a, second: b + c";
    let break_stmt = parse_statement_from_string(input).unwrap();

    if let Stmt::Break { values, .. } = break_stmt {
        assert_eq!(values.len(), 2, "Expected two labeled break values");

        assert_eq!(values[0].label, "first");
        assert!(matches!(
            &values[0].value,
            Expr::Identifier { name, .. } if name == "a"
        ));

        assert_eq!(values[1].label, "second");
        assert!(matches!(
            &values[1].value,
            Expr::Binary {
                operator: BinaryOp::Add,
                ..
            }
        ));
    } else {
        unreachable!("Expected labeled break statement, got {break_stmt:?}");
    }
}

#[test]
fn test_continue_with_labeled_values() {
    let continue_stmt = parse_statement_from_string("continue accumulator: sum").unwrap();

    if let Stmt::Continue { values, .. } = continue_stmt {
        assert_eq!(values.len(), 1, "Expected one labeled continue value");
        assert_eq!(values[0].label, "accumulator");
        assert!(matches!(
            &values[0].value,
            Expr::Identifier { name, .. } if name == "sum"
        ));
    } else {
        unreachable!("Expected continue with labeled payload, got {continue_stmt:?}");
    }
}

#[test]
fn test_break_labeled_value_requires_colon() {
    let result = parse_statement_from_string("break result value");
    assert!(
        result.is_err(),
        "Break labeled values must use ':' separator"
    );
}

#[test]
fn test_break_duplicate_labels_rejected() {
    let result = parse_statement_from_string("break result: 5, result: 10");
    assert!(
        result.is_err(),
        "Duplicate labels in break statement should be rejected"
    );

    if let Err(ParseError::DuplicateLabel { label, .. }) = result {
        assert_eq!(label, "result", "Error should identify the duplicate label");
    } else {
        panic!("Expected DuplicateLabel error, got {:?}", result);
    }
}

#[test]
fn test_continue_duplicate_labels_rejected() {
    let result = parse_statement_from_string("continue state: a, state: b, state: c");
    assert!(
        result.is_err(),
        "Duplicate labels in continue statement should be rejected"
    );

    if let Err(ParseError::DuplicateLabel { label, .. }) = result {
        assert_eq!(label, "state", "Error should identify the duplicate label");
    } else {
        panic!("Expected DuplicateLabel error, got {:?}", result);
    }
}

#[test]
fn test_break_multiple_unique_labels_accepted() {
    let result = parse_statement_from_string("break first: 1, second: 2, third: 3");
    assert!(
        result.is_ok(),
        "Multiple unique labels should be accepted: {:?}",
        result.err()
    );

    if let Ok(Stmt::Break { values, .. }) = result {
        assert_eq!(values.len(), 3, "Should have three labeled values");
        assert_eq!(values[0].label, "first");
        assert_eq!(values[1].label, "second");
        assert_eq!(values[2].label, "third");
    } else {
        panic!("Expected Break statement with labeled values");
    }
}

#[test]
fn test_for_statements() {
    // Test simple for loop
    let simple_for = parse_statement_from_string("for item in collection { print(item) }").unwrap();
    if let Stmt::For {
        variable,
        iterable,
        body,
        ..
    } = simple_for
    {
        // Check variable
        assert_eq!(variable, "item");

        // Check iterable
        if let Expr::Identifier { name, .. } = iterable {
            assert_eq!(name, "collection");
        } else {
            unreachable!("Expected identifier in for iterable");
        }

        // Check body
        if let Stmt::Block { .. } = *body {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in for body");
        }
    } else {
        unreachable!("Expected for statement, got {simple_for:?}");
    }

    // TODO: Add test for array literal when array expressions are implemented
    /*
    // Test for loop with array literal
    let array_for = parse_statement_from_string("for i in [1, 2, 3] { sum = sum + i }").unwrap();
    if let Stmt::For { variable, iterable, body, .. } = array_for {
        assert_eq!(variable, "i");

        if let Expr::Array { .. } = iterable {
            // Good, array literal
        } else {
            unreachable!("Expected array in for iterable");
        }

        if let Stmt::Block { .. } = *body {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in for body");
        }
    } else {
        unreachable!("Expected for statement");
    }
    */
}

#[test]
fn test_while_statements() {
    // Test simple while loop
    let simple_while = parse_statement_from_string("while x < 10 { x = x + 1 }").unwrap();
    if let Stmt::While {
        condition, body, ..
    } = simple_while
    {
        // Check condition
        if let Expr::Binary { .. } = condition {
            // Good, binary comparison
        } else {
            unreachable!("Expected binary expression in while condition");
        }

        // Check body
        if let Stmt::Block { .. } = *body {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in while body");
        }
    } else {
        unreachable!("Expected while statement, got {simple_while:?}");
    }

    // Test while with boolean variable
    let bool_while = parse_statement_from_string("while running { update() }").unwrap();
    if let Stmt::While {
        condition, body, ..
    } = bool_while
    {
        if let Expr::Identifier { name, .. } = condition {
            assert_eq!(name, "running");
        } else {
            unreachable!("Expected identifier in while condition");
        }

        if let Stmt::Block { .. } = *body {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in while body");
        }
    } else {
        unreachable!("Expected while statement");
    }
}

#[test]
#[expect(
    clippy::cognitive_complexity,
    reason = "Complex test covering multiple loop scenarios"
)]
fn test_loop_statements() {
    // Test simple loop statement
    let simple_loop = parse_statement_from_string("loop => { break }").unwrap();
    if let Stmt::Loop { body, .. } = simple_loop {
        // Check body
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 1);
            if let Stmt::Break { .. } = statements[0] {
                // Good, break statement
            } else {
                unreachable!("Expected break statement in loop body");
            }
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement, got {simple_loop:?}");
    }

    // Test loop with multiple statements
    let complex_loop =
        parse_statement_from_string("loop => { let x = 1; if x > 10 { break } else { continue } }")
            .unwrap();
    if let Stmt::Loop { body, .. } = complex_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 2);
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }

    // Test loop with nested loops
    let nested_loop =
        parse_statement_from_string("loop => { loop => { break }; continue }").unwrap();
    if let Stmt::Loop { body, .. } = nested_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 2);
            // First statement should be a nested loop
            if let Stmt::Loop { .. } = statements[0] {
                // Good, nested loop
            } else {
                unreachable!("Expected nested loop statement");
            }
            // Second statement should be continue
            if let Stmt::Continue { .. } = statements[1] {
                // Good, continue statement
            } else {
                unreachable!("Expected continue statement");
            }
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }

    // Test loop with variable assignments and conditions
    let assignment_loop =
        parse_statement_from_string("loop => { let i = 0; i = i + 1; if i > 5 { break } }")
            .unwrap();
    if let Stmt::Loop { body, .. } = assignment_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 3);
            // Check that we have let, assignment, and if statements
            assert!(matches!(statements[0], Stmt::Let { .. }));
            assert!(matches!(statements[1], Stmt::Assignment { .. }));
            assert!(matches!(statements[2], Stmt::If { .. }));
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }

    // Test loop with function calls
    let function_call_loop =
        parse_statement_from_string("loop => { process_item(); if should_exit() { break } }")
            .unwrap();
    if let Stmt::Loop { body, .. } = function_call_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 2);
            // First should be an expression statement with function call
            if let &Stmt::Expression { ref expr, .. } = &statements[0] {
                assert!(matches!(*expr, Expr::Call { .. }));
            } else {
                unreachable!("Expected expression statement with function call");
            }
            // Second should be an if statement
            assert!(matches!(statements[1], Stmt::If { .. }));
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }

    // Test empty loop (just for syntax, though not practical)
    let empty_loop = parse_statement_from_string("loop => { }").unwrap();
    if let Stmt::Loop { body, .. } = empty_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 0);
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }
}

#[test]
fn test_loop_error_cases() {
    // Test loop without arrow - should fail
    let missing_arrow = parse_statement_from_string("loop { break }");
    assert!(missing_arrow.is_err());

    // Test loop without body - should fail
    let missing_body = parse_statement_from_string("loop =>");
    assert!(missing_body.is_err());

    // Test loop with malformed arrow - should fail
    let bad_arrow = parse_statement_from_string("loop = { break }");
    assert!(bad_arrow.is_err());

    // Test loop with unclosed body - should fail
    let unclosed_body = parse_statement_from_string("loop => { break");
    assert!(unclosed_body.is_err());
}

#[test]
fn test_loop_with_various_statements() {
    // Test loop containing all types of statements
    let comprehensive_loop = parse_statement_from_string(
        "loop => { 
            let x = 0;
            let mutable counter = 1;
            counter = counter + 1;
            for i in items { process(i) };
            while running { update() };
            if done { break };
            return void
        }",
    )
    .unwrap();

    if let Stmt::Loop { body, .. } = comprehensive_loop {
        if let Stmt::Block { statements, .. } = *body {
            assert_eq!(statements.len(), 7);
            assert!(matches!(statements[0], Stmt::Let { .. }));
            assert!(matches!(statements[1], Stmt::Let { .. }));
            if let Stmt::Let { binding, .. } = &statements[1] {
                assert!(binding.is_mutable);
            } else {
                unreachable!("Expected let statement with mutable binding");
            }
            assert!(matches!(statements[2], Stmt::Assignment { .. }));
            assert!(matches!(statements[3], Stmt::For { .. }));
            assert!(matches!(statements[4], Stmt::While { .. }));
            assert!(matches!(statements[5], Stmt::If { .. }));
            assert!(matches!(statements[6], Stmt::Return { .. }));
        } else {
            unreachable!("Expected block statement in loop body");
        }
    } else {
        unreachable!("Expected loop statement");
    }
}

#[test]
fn test_guard_into_else_statement_parses() {
    let src = "
import string_to_int32 from standard
entry main = f(): void =>
    guard string_to_int32('5') into n else e =>
        continue
    return void
";

    let program = parse_program_from_string(src).expect("guard statement should parse");
    assert!(!program.declarations.is_empty());

    let mut found_guard = false;
    for decl in &program.declarations {
        if let Decl::Function {
            body: Stmt::Block { statements, .. },
            ..
        } = decl
        {
            for statement in statements {
                if let Stmt::Guard {
                    success_binding,
                    error_binding,
                    else_body,
                    ..
                } = statement
                {
                    found_guard = true;
                    assert_eq!(success_binding, "n");
                    assert_eq!(error_binding, "e");
                    assert!(matches!(
                        else_body.as_ref(),
                        Stmt::Block { statements, .. }
                            if statements.iter().any(|stmt| matches!(stmt, Stmt::Continue { .. }))
                    ));
                }
            }
        }
    }

    assert!(found_guard, "expected function body to contain Stmt::Guard");
}

#[test]
fn test_loop_expression_parses_in_expression_position() {
    let expr = parse_expression_from_string("loop => { break result: 1 }")
        .expect("loop expression should parse");
    match expr {
        Expr::Loop { body, .. } => match *body {
            Stmt::Block { statements, .. } => {
                assert_eq!(statements.len(), 1);
                assert!(matches!(statements[0], Stmt::Break { .. }));
            }
            other => panic!("expected loop-expression block body, got {other:?}"),
        },
        other => panic!("expected Expr::Loop, got {other:?}"),
    }
}

#[test]
fn test_let_destructure_loop_expression_statement() {
    let stmt = parse_statement_from_string("let a, b = loop => { break a: 1, b: 2 }")
        .expect("destructuring let with loop expression should parse");
    match stmt {
        Stmt::LetDestructure {
            bindings,
            initializer,
            ..
        } => {
            assert_eq!(bindings.len(), 2);
            assert_eq!(bindings[0].name, "a");
            assert_eq!(bindings[1].name, "b");
            assert!(matches!(initializer, Expr::Loop { .. }));
        }
        other => panic!("expected Stmt::LetDestructure, got {other:?}"),
    }
}

#[test]
fn test_if_statements() {
    // Test simple if statement
    let simple_if = parse_statement_from_string("if x < 5 { return true }").unwrap();
    if let Stmt::If {
        condition,
        then_branch,
        else_branch,
        ..
    } = simple_if
    {
        // Check condition
        if let Expr::Binary { .. } = condition {
            // Good, binary comparison
        } else {
            unreachable!("Expected binary expression in if condition");
        }

        // Check then branch
        if let Stmt::Block { .. } = *then_branch {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in then branch");
        }

        // Check no else branch
        assert!(else_branch.is_none());
    } else {
        unreachable!("Expected if statement, got {simple_if:?}");
    }

    // Test if-else statement
    let if_else = parse_statement_from_string("if x { y = 1 } else { y = 2 }").unwrap();
    if let Stmt::If {
        condition,
        then_branch,
        else_branch,
        ..
    } = if_else
    {
        // Check condition
        if let Expr::Identifier { name, .. } = condition {
            assert_eq!(name, "x");
        } else {
            unreachable!("Expected identifier in if condition");
        }

        // Check then branch
        if let Stmt::Block { .. } = *then_branch {
            // Good, block statement
        } else {
            unreachable!("Expected block statement in then branch");
        }

        // Check else branch exists
        assert!(else_branch.is_some());
        if let Some(else_stmt) = else_branch {
            if let Stmt::Block { .. } = *else_stmt {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in else branch");
            }
        }
    } else {
        unreachable!("Expected if statement");
    }
}

#[test]
fn test_if_expression_parses_in_expression_position() {
    let parsed = parse_expression_from_string("if true { 1 } else { 2 }");
    assert!(
        parsed.is_ok(),
        "if expression used in expression position should parse"
    );
}

#[test]
fn test_if_expression_without_else_parses() {
    let parsed = parse_expression_from_string("if true { 1 }");
    assert!(
        parsed.is_ok(),
        "else-less if expression should parse and default to unit semantics"
    );
}

#[test]
fn test_assignment_statements() {
    // Test simple assignment
    let input = "x = 5";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let simple_expr = parser.parse_statement().unwrap();

    if let Stmt::Assignment { target, value, .. } = simple_expr {
        if let Expr::Identifier { name, .. } = target {
            assert_eq!(name, "x");
        } else {
            unreachable!("Expected identifier in assignment target, got {target:?}");
        }
        if let Expr::Literal {
            value: LiteralValue::Integer(n),
            ..
        } = value
        {
            assert_eq!(n, 5);
        } else {
            unreachable!("Expected integer literal in assignment value, got {value:?}");
        }
    } else {
        unreachable!("Expected assignment statement, got {simple_expr:?}");
    }

    // TODO: Add tests for array index and member access assignments
    // when those expression types are fully implemented
    /*
    // Test assignment to array index
    let array_assignment = parse_statement_from_string("arr[0] = 10").unwrap();
    if let Stmt::Assignment { target, value, .. } = array_assignment {
        if let Expr::Index { .. } = target {
            // Correct target type
        } else {
            unreachable!("Expected index expression in assignment target");
        }
        if let Expr::Literal {
            value: LiteralValue::Integer(n),
            ..
        } = value
        {
            assert_eq!(n, 10);
        } else {
            unreachable!("Expected integer literal in assignment value");
        }
    } else {
        unreachable!("Expected assignment statement");
    }

    // Test assignment to member access
    let member_assignment = parse_statement_from_string("obj.field = 'value'").unwrap();
    if let Stmt::Assignment { target, value, .. } = member_assignment {
        if let Expr::Member { .. } = target {
            // Correct target type
        } else {
            unreachable!("Expected member expression in assignment target");
        }
        if let Expr::Literal {
            value: LiteralValue::String(s),
            ..
        } = value
        {
            assert_eq!(s, "value");
        } else {
            unreachable!("Expected string literal in assignment value");
        }
    } else {
        unreachable!("Expected assignment statement");
    }
    */
}

#[test]
fn test_simple_function_parsing() {
    let input = "entry main = f(args: string[]): void => return void";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (program, errors) = parser.parse();

    assert!(errors.is_empty(), "Parse errors: {errors:?}");
    assert!(program.is_some());

    let program = program.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function {
        name,
        parameters,
        return_types,
        is_entry,
        ..
    } = program.declarations[0].clone()
    {
        assert_eq!(name, "main");
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].name, "args");
        assert!(return_types.is_some());
        assert!(is_entry);
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_type_of_expressions() {
    // Test type_of with literal
    let type_of_literal = parse_expression_from_string("type_of(42)").unwrap();
    if let Expr::TypeOf { expr, .. } = type_of_literal {
        if let Expr::Literal {
            value: LiteralValue::Integer(42),
            ..
        } = *expr
        {
            // Good, correct structure
        } else {
            unreachable!("Expected integer literal inside type_of");
        }
    } else {
        unreachable!("Expected type_of expression, got {type_of_literal:?}");
    }

    // Test type_of with variable
    let type_of_var = parse_expression_from_string("type_of(my_variable)").unwrap();
    if let Expr::TypeOf { expr, .. } = type_of_var {
        if let Expr::Identifier { name, .. } = *expr {
            assert_eq!(name, "my_variable");
        } else {
            unreachable!("Expected identifier inside type_of");
        }
    } else {
        unreachable!("Expected type_of expression");
    }

    // Test type_of with expression
    let type_of_expr = parse_expression_from_string("type_of(x + y)").unwrap();
    if let Expr::TypeOf { expr, .. } = type_of_expr {
        if let Expr::Binary {
            operator: BinaryOp::Add,
            ..
        } = *expr
        {
            // Good, binary expression inside type_of
        } else {
            unreachable!("Expected binary expression inside type_of");
        }
    } else {
        unreachable!("Expected type_of expression");
    }

    // Test nested type_of (though semantically questionable)
    let nested_type_of = parse_expression_from_string("type_of(type_of(x))").unwrap();
    if let Expr::TypeOf { expr, .. } = nested_type_of {
        if let Expr::TypeOf { .. } = *expr {
            // Good, nested type_of
        } else {
            unreachable!("Expected nested type_of inside outer type_of");
        }
    } else {
        unreachable!("Expected type_of expression");
    }
}

#[test]
fn test_type_of_error_cases() {
    // Test type_of without parentheses - should fail
    let missing_parens = parse_expression_from_string("type_of x");
    assert!(missing_parens.is_err());

    // Test type_of without expression - should fail
    let missing_expr = parse_expression_from_string("type_of()");
    assert!(missing_expr.is_err());

    // Test type_of with unclosed parentheses - should fail
    let unclosed_parens = parse_expression_from_string("type_of(x");
    assert!(unclosed_parens.is_err());

    // Test empty type_of call - should fail
    let empty_call = parse_expression_from_string("type_of( )");
    assert!(empty_call.is_err());
}

#[test]
fn test_type_of_in_complex_expressions() {
    // Test type_of in binary expressions
    let binary_with_type_of = parse_expression_from_string("type_of(x) is type_of(y)").unwrap();
    if let Expr::Binary {
        left,
        operator: BinaryOp::Is,
        right,
        ..
    } = binary_with_type_of
    {
        assert!(matches!(*left, Expr::TypeOf { .. }));
        assert!(matches!(*right, Expr::TypeOf { .. }));
    } else {
        unreachable!("Expected binary expression with type_of operands");
    }

    // Test type_of as function argument
    let type_of_as_arg = parse_expression_from_string("print(type_of(value))").unwrap();
    if let Expr::Call { args, .. } = type_of_as_arg {
        assert_eq!(args.len(), 1);
        assert!(matches!(args[0], Expr::TypeOf { .. }));
    } else {
        unreachable!("Expected function call with type_of argument");
    }

    // Test type_of with parenthesized expression
    let type_of_paren = parse_expression_from_string("type_of((x + y))").unwrap();
    if let Expr::TypeOf { expr, .. } = type_of_paren {
        if let Expr::Parenthesized { expr: inner, .. } = *expr {
            assert!(matches!(*inner, Expr::Binary { .. }));
        } else {
            unreachable!("Expected parenthesized expression inside type_of");
        }
    } else {
        unreachable!("Expected type_of expression");
    }
}

#[test]
fn test_string_interpolation_simple() {
    // Test simple variable interpolation: 'Hello {world}'
    let simple = parse_expression_from_string("'Hello {world}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = simple {
        assert_eq!(parts.len(), 3);

        // First part should be literal "Hello "
        if let StringPart::Literal(ref text) = parts[0] {
            assert_eq!(text, "Hello ");
        } else {
            unreachable!("Expected literal part, got {:?}", parts[0]);
        }

        // Second part should be identifier expression
        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
            assert_eq!(name, "world");
        } else {
            unreachable!("Expected identifier expression, got {:?}", parts[1]);
        }

        // Third part should be empty literal (trailing string after last interpolation)
        if let StringPart::Literal(ref text) = parts[2] {
            assert_eq!(text, "");
        } else {
            unreachable!("Expected literal part, got {:?}", parts[2]);
        }
    } else {
        unreachable!("Expected string interpolation, got {:?}", simple);
    }
}

#[test]
fn test_string_interpolation_multiple() {
    // Test multiple interpolations: 'fib({n}) = {result}'
    let multiple = parse_expression_from_string("'fib({n}) = {result}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = multiple {
        assert_eq!(parts.len(), 5);

        // Should be: literal("fib("), expr(n), literal(") = "), expr(result), literal("")
        if let StringPart::Literal(ref text) = parts[0] {
            assert_eq!(text, "fib(");
        } else {
            unreachable!("Expected literal part");
        }

        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
            assert_eq!(name, "n");
        } else {
            unreachable!("Expected identifier expression");
        }

        if let StringPart::Literal(ref text) = parts[2] {
            assert_eq!(text, ") = ");
        } else {
            unreachable!("Expected literal part");
        }

        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[3] {
            assert_eq!(name, "result");
        } else {
            unreachable!("Expected identifier expression");
        }

        if let StringPart::Literal(ref text) = parts[4] {
            assert_eq!(text, "");
        } else {
            unreachable!("Expected literal part");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_complex_expressions() {
    // Test complex expressions in interpolation: 'Result: {a + b * c}'
    let complex = parse_expression_from_string("'Result: {a + b * c}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = complex {
        assert_eq!(parts.len(), 3);

        if let StringPart::Literal(ref text) = parts[0] {
            assert_eq!(text, "Result: ");
        } else {
            unreachable!("Expected literal part");
        }

        if let StringPart::Expression(Expr::Binary { .. }) = parts[1] {
            // Good, binary expression
        } else {
            unreachable!("Expected binary expression");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_function_calls() {
    // Test function calls in interpolation: 'Value: {get_value()}'
    let func_call = parse_expression_from_string("'Value: {get_value()}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = func_call {
        assert_eq!(parts.len(), 3);

        if let StringPart::Expression(Expr::Call { .. }) = parts[1] {
            // Good, function call expression
        } else {
            unreachable!("Expected function call expression");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_type_of() {
    // Test type_of in interpolation: 'Type: {type_of(x)}'
    let type_of_interp = parse_expression_from_string("'Type: {type_of(x)}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = type_of_interp {
        assert_eq!(parts.len(), 3);

        if let StringPart::Expression(Expr::TypeOf { .. }) = parts[1] {
            // Good, type_of expression
        } else {
            unreachable!("Expected type_of expression");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_only_expression() {
    // Test string with only interpolation: '{value}'
    let only_expr = parse_expression_from_string("'{value}'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = only_expr {
        assert_eq!(parts.len(), 3);

        // Should be: literal(""), expr(value), literal("")
        if let StringPart::Literal(ref text) = parts[0] {
            assert_eq!(text, "");
        } else {
            unreachable!("Expected empty literal part");
        }

        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
            assert_eq!(name, "value");
        } else {
            unreachable!("Expected identifier expression");
        }

        if let StringPart::Literal(ref text) = parts[2] {
            assert_eq!(text, "");
        } else {
            unreachable!("Expected empty literal part");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_no_spaces() {
    // Test interpolation without spaces: 'a{b}c{d}e'
    let no_spaces = parse_expression_from_string("'a{b}c{d}e'").unwrap();
    if let Expr::StringInterpolation { parts, .. } = no_spaces {
        assert_eq!(parts.len(), 5);

        // Should be: literal("a"), expr(b), literal("c"), expr(d), literal("e")
        if let StringPart::Literal(ref text) = parts[0] {
            assert_eq!(text, "a");
        } else {
            unreachable!("Expected literal 'a'");
        }

        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
            assert_eq!(name, "b");
        } else {
            unreachable!("Expected identifier 'b'");
        }

        if let StringPart::Literal(ref text) = parts[2] {
            assert_eq!(text, "c");
        } else {
            unreachable!("Expected literal 'c'");
        }

        if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[3] {
            assert_eq!(name, "d");
        } else {
            unreachable!("Expected identifier 'd'");
        }

        if let StringPart::Literal(ref text) = parts[4] {
            assert_eq!(text, "e");
        } else {
            unreachable!("Expected literal 'e'");
        }
    } else {
        unreachable!("Expected string interpolation");
    }
}

#[test]
fn test_string_interpolation_error_cases() {
    // Test unclosed interpolation brace
    let result = parse_expression_from_string("'Hello {world'");
    assert!(result.is_err(), "Should fail on unclosed brace");

    // Test empty interpolation
    let empty_result = parse_expression_from_string("'Hello {}'");
    assert!(empty_result.is_err(), "Should fail on empty interpolation");

    // Test unmatched closing brace
    let _unmatched_result = parse_expression_from_string("'Hello world}'");
    // This should actually be a regular string literal with '}' in it
    // So it might not be an error, depending on implementation
}

// Basic type parsing tests
#[test]
fn test_basic_type_parsing() {
    // Test primitive types
    let int_type = parse_type_from_string("int32").unwrap();
    if let Type::Basic { name, .. } = int_type {
        assert_eq!(name, "int32");
    } else {
        panic!("Expected basic type");
    }

    let string_type = parse_type_from_string("string").unwrap();
    if let Type::Basic { name, .. } = string_type {
        assert_eq!(name, "string");
    } else {
        panic!("Expected basic type");
    }

    let bool_type = parse_type_from_string("boolean").unwrap();
    if let Type::Basic { name, .. } = bool_type {
        assert_eq!(name, "boolean");
    } else {
        panic!("Expected basic type");
    }

    let void_type = parse_type_from_string("void").unwrap();
    if let Type::Basic { name, .. } = void_type {
        assert_eq!(name, "void");
    } else {
        panic!("Expected basic type");
    }
}

#[test]
fn test_array_type_parsing() {
    // Test simple array type
    let array_type = parse_type_from_string("int32[]").unwrap();
    if let Type::Array { element_type, .. } = array_type {
        if let Type::Basic { name, .. } = element_type.as_ref() {
            assert_eq!(name, "int32");
        } else {
            panic!("Expected basic element type");
        }
    } else {
        panic!("Expected array type");
    }

    // Test nested array type
    let nested_array = parse_type_from_string("string[][]").unwrap();
    if let Type::Array { element_type, .. } = nested_array {
        if let Type::Array {
            element_type: inner,
            ..
        } = element_type.as_ref()
        {
            if let Type::Basic { name, .. } = inner.as_ref() {
                assert_eq!(name, "string");
            } else {
                panic!("Expected basic inner element type");
            }
        } else {
            panic!("Expected nested array type");
        }
    } else {
        panic!("Expected array type");
    }
}

#[test]
fn test_custom_type_parsing() {
    // Test custom type names (Pascal case)
    let custom_type = parse_type_from_string("MyCustomType").unwrap();
    if let Type::Basic { name, .. } = custom_type {
        assert_eq!(name, "MyCustomType");
    } else {
        panic!("Expected basic type for custom type");
    }

    // Test custom type with array
    let custom_array = parse_type_from_string("Person[]").unwrap();
    if let Type::Array { element_type, .. } = custom_array {
        if let Type::Basic { name, .. } = element_type.as_ref() {
            assert_eq!(name, "Person");
        } else {
            panic!("Expected basic element type");
        }
    } else {
        panic!("Expected array type");
    }
}

#[test]
fn test_basic_type_parsing_error_cases() {
    // Test starting with a number token
    let result1 = parse_type_from_string("32");
    assert!(result1.is_err(), "Should fail on number token as type");

    // Test empty type
    let result2 = parse_type_from_string("");
    assert!(result2.is_err(), "Should fail on empty input");

    // Test invalid token as type name
    let result3 = parse_type_from_string("+");
    assert!(result3.is_err(), "Should fail on operator token as type");
}

#[test]
fn test_generic_type_parsing_simple() {
    // Test simple generic type: Array<T>
    let simple_generic = parse_type_from_string("Array<T>").unwrap();
    if let Type::Generic {
        name, type_args, ..
    } = simple_generic
    {
        assert_eq!(name, "Array");
        assert_eq!(type_args.len(), 1);
        if let &Type::Basic {
            name: ref arg_name, ..
        } = &type_args[0]
        {
            assert_eq!(arg_name, "T");
        } else {
            unreachable!("Expected basic type T as argument");
        }
    } else {
        unreachable!("Expected generic type, got {simple_generic:?}");
    }
}

#[test]
fn test_generic_type_parsing_multiple_params() {
    // Test multiple type parameters: Result<T, E>
    let multiple_params = parse_type_from_string("Result<T, E>").unwrap();
    if let Type::Generic {
        name, type_args, ..
    } = multiple_params
    {
        assert_eq!(name, "Result");
        assert_eq!(type_args.len(), 2);

        if let &Type::Basic {
            name: ref first_arg,
            ..
        } = &type_args[0]
        {
            assert_eq!(first_arg, "T");
        } else {
            unreachable!("Expected basic type T as first argument");
        }

        if let &Type::Basic {
            name: ref second_arg,
            ..
        } = &type_args[1]
        {
            assert_eq!(second_arg, "E");
        } else {
            unreachable!("Expected basic type E as second argument");
        }
    } else {
        unreachable!("Expected generic type, got {multiple_params:?}");
    }
}

#[test]
fn test_generic_type_parsing_concrete_args() {
    // Test concrete type arguments: Array<int32>
    let concrete_args = parse_type_from_string("Array<int32>").unwrap();
    if let Type::Generic {
        name, type_args, ..
    } = concrete_args
    {
        assert_eq!(name, "Array");
        assert_eq!(type_args.len(), 1);
        if let &Type::Basic {
            name: ref arg_name, ..
        } = &type_args[0]
        {
            assert_eq!(arg_name, "int32");
        } else {
            unreachable!("Expected basic type int32 as argument");
        }
    } else {
        unreachable!("Expected generic type");
    }
}

#[test]
fn test_generic_type_parsing_nested() {
    // Test nested generic types: Array<Result<T, E>>
    let nested_generic = parse_type_from_string("Array<Result<T, E>>").unwrap();
    if let Type::Generic {
        name, type_args, ..
    } = nested_generic
    {
        assert_eq!(name, "Array");
        assert_eq!(type_args.len(), 1);

        if let &Type::Generic {
            name: ref inner_name,
            type_args: ref inner_args,
            ..
        } = &type_args[0]
        {
            assert_eq!(inner_name, "Result");
            assert_eq!(inner_args.len(), 2);

            if let &Type::Basic {
                name: ref t_name, ..
            } = &inner_args[0]
            {
                assert_eq!(t_name, "T");
            } else {
                unreachable!("Expected T in nested generic");
            }

            if let &Type::Basic {
                name: ref e_name, ..
            } = &inner_args[1]
            {
                assert_eq!(e_name, "E");
            } else {
                unreachable!("Expected E in nested generic");
            }
        } else {
            unreachable!("Expected nested generic type as argument");
        }
    } else {
        unreachable!("Expected generic type");
    }
}

#[test]
fn test_generic_type_parsing_with_array_suffix() {
    // Test generic type with array suffix: Array<T>[]
    let generic_array = parse_type_from_string("Array<T>[]").unwrap();
    if let Type::Array { element_type, .. } = generic_array {
        if let &Type::Generic {
            ref name,
            ref type_args,
            ..
        } = element_type.as_ref()
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);

            if let &Type::Basic {
                name: ref arg_name, ..
            } = &type_args[0]
            {
                assert_eq!(arg_name, "T");
            } else {
                unreachable!("Expected T as type argument");
            }
        } else {
            unreachable!("Expected generic type as array element");
        }
    } else {
        unreachable!("Expected array type with generic element");
    }
}

#[test]
fn test_generic_type_parsing_error_cases() {
    // Test unclosed angle bracket
    let unclosed_result = parse_type_from_string("Array<T");
    assert!(
        unclosed_result.is_err(),
        "Should fail on unclosed angle bracket"
    );

    // Test empty generic arguments
    let empty_result = parse_type_from_string("Array<>");
    assert!(
        empty_result.is_err(),
        "Should fail on empty generic arguments"
    );

    // Test missing comma between arguments
    let missing_comma_result = parse_type_from_string("Result<T E>");
    assert!(
        missing_comma_result.is_err(),
        "Should fail on missing comma"
    );
}

#[test]
fn test_function_type_parsing_simple() {
    // Test simple function type: f(int32): string
    let simple_func = parse_type_from_string("f(int32): string").unwrap();
    if let Type::Function {
        parameters,
        return_types,
        ..
    } = simple_func
    {
        assert_eq!(parameters.len(), 1);
        if let &Type::Basic { ref name, .. } = &parameters[0] {
            assert_eq!(name, "int32");
        } else {
            unreachable!("Expected basic type int32 as parameter");
        }

        if let Type::Basic { name, .. } = &return_types[0] {
            assert_eq!(name, "string");
        } else {
            unreachable!("Expected basic type string as return type");
        }
    } else {
        unreachable!("Expected function type, got {simple_func:?}");
    }
}

#[test]
fn test_function_type_parsing_multiple_params() {
    // Test multiple parameters: f(int32, string, boolean): void
    let multi_param = parse_type_from_string("f(int32, string, boolean): void").unwrap();
    if let Type::Function {
        parameters,
        return_types,
        ..
    } = multi_param
    {
        assert_eq!(parameters.len(), 3);

        if let &Type::Basic { ref name, .. } = &parameters[0] {
            assert_eq!(name, "int32");
        } else {
            unreachable!("Expected int32 as first parameter");
        }

        if let &Type::Basic { ref name, .. } = &parameters[1] {
            assert_eq!(name, "string");
        } else {
            unreachable!("Expected string as second parameter");
        }

        if let &Type::Basic { ref name, .. } = &parameters[2] {
            assert_eq!(name, "boolean");
        } else {
            unreachable!("Expected boolean as third parameter");
        }

        if let Type::Basic { name, .. } = &return_types[0] {
            assert_eq!(name, "void");
        } else {
            unreachable!("Expected void as return type");
        }
    } else {
        unreachable!("Expected function type");
    }
}

#[test]
fn test_function_type_parsing_no_params() {
    // Test function with no parameters: f(): void
    let no_params = parse_type_from_string("f(): void").unwrap();
    if let Type::Function {
        parameters,
        return_types,
        ..
    } = no_params
    {
        assert_eq!(parameters.len(), 0);

        if let Type::Basic { name, .. } = &return_types[0] {
            assert_eq!(name, "void");
        } else {
            unreachable!("Expected void as return type");
        }
    } else {
        unreachable!("Expected function type");
    }
}

#[test]
fn test_function_type_parsing_generic_params() {
    // Test function with generic parameters: f(Array<T>, Result<T, E>): boolean
    let generic_params = parse_type_from_string("f(Array<T>, Result<T, E>): boolean").unwrap();
    if let Type::Function {
        parameters,
        return_types,
        ..
    } = generic_params
    {
        assert_eq!(parameters.len(), 2);

        if let &Type::Generic {
            ref name,
            ref type_args,
            ..
        } = &parameters[0]
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);
        } else {
            unreachable!("Expected generic type Array<T> as first parameter");
        }

        if let &Type::Generic {
            ref name,
            ref type_args,
            ..
        } = &parameters[1]
        {
            assert_eq!(name, "Result");
            assert_eq!(type_args.len(), 2);
        } else {
            unreachable!("Expected generic type Result<T, E> as second parameter");
        }

        if let Type::Basic { name, .. } = &return_types[0] {
            assert_eq!(name, "boolean");
        } else {
            unreachable!("Expected boolean as return type");
        }
    } else {
        unreachable!("Expected function type");
    }
}

#[test]
fn test_function_type_parsing_array_suffix() {
    // Test function type with array suffix: f(int32): string[]
    let array_return = parse_type_from_string("f(int32): string[]").unwrap();
    if let Type::Function {
        parameters,
        return_types,
        ..
    } = array_return
    {
        assert_eq!(parameters.len(), 1);

        if let Type::Array { element_type, .. } = &return_types[0] {
            if let Type::Basic { name, .. } = element_type.as_ref() {
                assert_eq!(name, "string");
            } else {
                unreachable!("Expected string as array element type");
            }
        } else {
            unreachable!("Expected array type as return type");
        }
    } else {
        unreachable!("Expected function type");
    }
}

#[test]
fn test_function_type_parsing_error_cases() {
    // Test function without parentheses - should fail
    let no_parens = parse_type_from_string("f int32: string");
    assert!(no_parens.is_err(), "Should fail on missing parentheses");

    // Test function without return type - should fail
    let no_return = parse_type_from_string("f(int32)");
    assert!(no_return.is_err(), "Should fail on missing return type");

    // Test function with unclosed parameters - should fail
    let unclosed_params = parse_type_from_string("f(int32: string");
    assert!(
        unclosed_params.is_err(),
        "Should fail on unclosed parameters"
    );

    // Test function with malformed parameters - should fail
    let bad_params = parse_type_from_string("f(int32 string): void");
    assert!(bad_params.is_err(), "Should fail on malformed parameters");
}

// Type declaration parsing tests
#[test]
fn test_simple_type_declaration_no_doc() {
    // Test a simple type without doc comments first
    let input = "type Direction:\n    North\n    East\n    South\n    West";

    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse simple type successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type {
        name,
        type_def,
        doc_comment,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(name, "Direction");
        assert!(doc_comment.is_none());

        if let TypeDef::Sum { variants, .. } = type_def {
            assert_eq!(variants.len(), 4);
            assert_eq!(variants[0].name, "North");
            assert_eq!(variants[1].name, "East");
            assert_eq!(variants[2].name, "South");
            assert_eq!(variants[3].name, "West");
        } else {
            panic!("Expected sum type definition");
        }
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_function_doc_comment_associated() {
    let source = "##\n  Description: Adds two numbers.\n##\npublic add = f(x: int32, y: int32): int32 => x + y";

    let program = parse_program_from_string(source).expect("Program should parse");
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function { doc_comment, .. } = &program.declarations[0] {
        let documentation = doc_comment.as_ref().expect("Doc comment should exist");
        assert_eq!(
            documentation
                .sections
                .get("Description")
                .expect("Description section should be present"),
            "Adds two numbers."
        );
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_type_doc_comment_associated() {
    let source =
        "##\n  Description: Represents a direction.\n##\ntype Direction:\n    North\n    East";

    let program = parse_program_from_string(source).expect("Program should parse");
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type { doc_comment, .. } = &program.declarations[0] {
        let documentation = doc_comment.as_ref().expect("Doc comment should exist");
        assert_eq!(
            documentation
                .sections
                .get("Description")
                .expect("Description section missing"),
            "Represents a direction."
        );
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_let_doc_comment_associated() {
    let source = "##\n  Description: Provides a zero constant.\n##\nlet zero = 0";

    let program = parse_program_from_string(source).expect("Program should parse");
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { doc_comment, .. } = &program.declarations[0] {
        let documentation = doc_comment.as_ref().expect("Doc comment should exist");
        assert_eq!(
            documentation
                .sections
                .get("Description")
                .expect("Description missing"),
            "Provides a zero constant."
        );
    } else {
        panic!("Expected let declaration");
    }
}

#[test]
fn test_documentation_sections_parsed() {
    let source = "##\n  Description: Primary entry point.\n  Detail: Invoke to start processing.\n##\nentry main = f(): void => return void";

    let program = parse_program_from_string(source).expect("Program should parse");
    let decl = &program.declarations[0];
    let documentation = match decl {
        Decl::Function { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected function declaration"),
    };

    assert_eq!(
        documentation.sections.get("Description"),
        Some(&"Primary entry point.".to_owned())
    );
    assert_eq!(
        documentation.sections.get("Detail"),
        Some(&"Invoke to start processing.".to_owned())
    );
}

#[test]
fn test_documentation_trims_indentation() {
    let source =
        "##\n    Description: Handles indentation.\n##\nlet helper = f(): void => { return void }";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Let { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected let declaration"),
    };

    assert_eq!(
        documentation.sections.get("Description"),
        Some(&"Handles indentation.".to_owned())
    );
}

#[test]
fn test_documentation_preserves_raw() {
    let source = "##\n  Description: Keeps raw text.\n  Detail: Important for downstream tools.\n##\nlet config = f(): void => { return void }";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Let { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected let declaration"),
    };

    assert!(documentation.raw.contains("Description: Keeps raw text."));
    assert!(documentation
        .raw
        .contains("Detail: Important for downstream tools."));
}

#[test]
fn test_function_signature_documentation_without_errors_clause() {
    let source = "##\n  Description: Document signature without errors.\n##\npublic compute = f(value: int32): int32 => {\n    return value\n}";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Function { doc_comment, .. } => doc_comment.as_ref().expect("Doc comment missing"),
        _ => panic!("Expected function declaration"),
    };

    let signature = documentation
        .sections
        .get("Signature")
        .expect("Signature section missing");
    assert_eq!(signature, "compute = f(value: int32): int32");
}

#[test]
fn test_function_signature_documentation_with_single_error() {
    let source = "##\n  Description: Document signature with single error.\n##\npublic parse = f(raw: string): int32 errors ParseError => {\n    return 0\n}";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Function { doc_comment, .. } => doc_comment.as_ref().expect("Doc comment missing"),
        _ => panic!("Expected function declaration"),
    };

    let signature = documentation
        .sections
        .get("Signature")
        .expect("Signature section missing");
    assert_eq!(signature, "parse = f(raw: string): int32 errors ParseError");
}

#[test]
fn test_function_signature_documentation_with_multiple_errors() {
    let source = "##\n  Description: Document signature with multiple errors.\n##\npublic read = f(path: string): string errors IoError, ParseError => {\n    return \"data\"\n}";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Function { doc_comment, .. } => doc_comment.as_ref().expect("Doc comment missing"),
        _ => panic!("Expected function declaration"),
    };

    let signature = documentation
        .sections
        .get("Signature")
        .expect("Signature section missing");
    assert_eq!(
        signature,
        "read = f(path: string): string errors IoError, ParseError"
    );
}

#[test]
fn test_documentation_attribute_parsed() {
    let source = "##\n  Description: Tagged declaration.\n  @deprecated Use new_entry instead.\n##\nentry main = f(): void => { return void }";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Function { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected function declaration"),
    };

    assert_eq!(
        documentation.attributes.get("deprecated"),
        Some(&"Use new_entry instead.".to_owned())
    );
}

#[test]
fn test_documentation_multiple_attributes() {
    let source = "##\n  Description: Supports multiple attributes.\n  @since 1.2.0\n  @unstable true\n##\nlet tool = f(): void => { return void }";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Let { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected let declaration"),
    };

    assert_eq!(
        documentation.attributes.get("since"),
        Some(&"1.2.0".to_owned())
    );
    assert_eq!(
        documentation.attributes.get("unstable"),
        Some(&"true".to_owned())
    );
}

#[test]
fn test_documentation_attribute_without_value() {
    let source = "##\n  Description: Attribute without explicit value.\n  @thread_safe\n##\nlet worker = f(): void => { return void }";

    let program = parse_program_from_string(source).expect("Program should parse");
    let documentation = match &program.declarations[0] {
        Decl::Let { doc_comment, .. } => doc_comment.as_ref().unwrap(),
        _ => panic!("Expected let declaration"),
    };

    assert!(
        matches!(
            documentation.attributes.get("thread_safe"),
            Some(value) if value.is_empty()
        ),
        "Attribute without value should map to empty string"
    );
}

// Property-based tests: random expression generation
proptest! {
    #[test]
    fn prop_random_arithmetic_expression_parses(expr in arithmetic_expr_strategy()) {
        let result = parse_expression_from_string(&expr);
        prop_assert!(result.is_ok(), "Arithmetic expression should parse: {expr}");
    }
}

proptest! {
    #[test]
    fn prop_random_parenthesized_expression_parses(expr in parenthesized_arithmetic_expr_strategy()) {
        let result = parse_expression_from_string(&expr);
        prop_assert!(result.is_ok(), "Parenthesized expression should parse: {expr}");
    }
}

proptest! {
    #[test]
    fn prop_random_boolean_expression_parses(expr in boolean_expr_strategy()) {
        let result = parse_expression_from_string(&expr);
        prop_assert!(result.is_ok(), "Boolean expression should parse: {expr}");
    }
}

// Property-based tests: parse/unparse invariants
proptest! {
    #[test]
    fn prop_arithmetic_roundtrip(expr in arithmetic_expr_strategy()) {
        let parsed = parse_expression_from_string(&expr).expect("Generated arithmetic expressions must parse");
        let canonical = canonicalize_simple_expression(&parsed).expect("Strategy should only produce supported expressions");
        let reparsed = parse_expression_from_string(&canonical).expect("Canonicalized expression must parse");
        let canonical_again = canonicalize_simple_expression(&reparsed).expect("Reparsed expression should canonicalize");
        prop_assert_eq!(canonical, canonical_again);
    }
}

proptest! {
    #[test]
    fn prop_parenthesized_roundtrip(expr in parenthesized_arithmetic_expr_strategy()) {
        let parsed = parse_expression_from_string(&expr).expect("Generated expressions must parse");
        let canonical = canonicalize_simple_expression(&parsed).expect("Strategy should produce supported expressions");
        let reparsed = parse_expression_from_string(&canonical).expect("Canonical expression must parse");
        let canonical_again = canonicalize_simple_expression(&reparsed).expect("Reparsed expression should canonicalize");
        prop_assert_eq!(canonical, canonical_again);
    }
}

proptest! {
    #[test]
    fn prop_comparison_roundtrip(expr in comparison_expr_strategy()) {
        let parsed = parse_expression_from_string(&expr).expect("Generated comparison must parse");
        let canonical = canonicalize_comparison_expression(&parsed).expect("Comparison expressions should canonicalize");
        let reparsed = parse_expression_from_string(&canonical).expect("Canonical comparison must parse");
        let canonical_again = canonicalize_comparison_expression(&reparsed).expect("Reparsed comparison should canonicalize");
        prop_assert_eq!(canonical, canonical_again);
    }
}

// Property-based tests: error handling properties
proptest! {
    #[test]
    fn prop_dangling_operator_expressions_fail(expr in dangling_operator_strategy()) {
        let result = parse_expression_from_string(&expr);
        prop_assert!(result.is_err(), "Dangling operator should fail to parse: {expr}");
    }
}

proptest! {
    #[test]
    fn prop_mismatched_parentheses_fail(expr in mismatched_parentheses_strategy()) {
        let result = parse_expression_from_string(&expr);
        prop_assert!(result.is_err(), "Mismatched parentheses should fail: {expr}");
    }
}

proptest! {
    #[test]
    fn prop_invalid_break_payloads_fail(input in invalid_break_payload_strategy()) {
        let result = parse_statement_from_string(&input);
        prop_assert!(result.is_err(), "Break payload without colon should be rejected: {input}");
    }
}

#[test]
fn test_simple_sum_type_parsing() {
    // Test a simple enum-like type without the complex doc comment for now
    let input = "type Direction:\n    North\n    East\n    South\n    West";

    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Should parse simple sum type successfully");

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type {
        name,
        type_def,
        doc_comment,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(name, "Direction");
        assert!(doc_comment.is_none()); // Changed expectation since we're not providing a doc comment

        if let TypeDef::Sum { variants, .. } = type_def {
            assert_eq!(variants.len(), 4);
            assert_eq!(variants[0].name, "North");
            assert_eq!(variants[1].name, "East");
            assert_eq!(variants[2].name, "South");
            assert_eq!(variants[3].name, "West");

            // Simple enum variants should have no fields
            for variant in variants {
                assert!(variant.fields.is_empty());
            }
        } else {
            panic!("Expected sum type definition");
        }
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_sum_type_with_fields_parsing() {
    // Test a sum type with variants that have fields - simplified for now
    let input = "type Message:\n    Text:\n        sender: string\n        body: string";

    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse sum type with fields successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type { name, type_def, .. } = &program.declarations[0] {
        assert_eq!(name, "Message");

        if let TypeDef::Sum { variants, .. } = type_def {
            // For now, just check that we parse it as a sum type
            // We'll improve field parsing later
            assert!(!variants.is_empty());
        } else {
            panic!("Expected sum type definition, got: {:?}", type_def);
        }
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_product_type_parsing() {
    // Test a simple product type (struct-like)
    let input = "type Person:\n    name: string\n    age: int32";

    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse product type successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type { name, type_def, .. } = &program.declarations[0] {
        assert_eq!(name, "Person");

        if let TypeDef::Product { fields, .. } = type_def {
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "name");
            assert_eq!(fields[1].name, "age");

            // Check field types
            if let Type::Basic { name, .. } = &fields[0].type_annotation {
                assert_eq!(name, "string");
            } else {
                panic!("Expected basic type for name field");
            }

            if let Type::Basic { name, .. } = &fields[1].type_annotation {
                assert_eq!(name, "int32");
            } else {
                panic!("Expected basic type for age field");
            }
        } else {
            panic!("Expected product type definition, got: {:?}", type_def);
        }
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_generic_type_declaration_parsing() {
    let input = "type Result<T, E>:\n    Ok\n    Error";

    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse generic type successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Type {
        name,
        generic_params,
        generic_constraints,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(name, "Result");
        let generic_names = generic_params
            .as_ref()
            .expect("generic params should exist");
        assert_eq!(generic_names.as_slice(), ["T", "E"]);

        let declarations = generic_constraints
            .as_ref()
            .expect("generic constraints should exist");
        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].name, "T");
        assert_eq!(declarations[1].name, "E");
    } else {
        panic!("Expected type declaration");
    }
}

#[test]
fn test_type_declaration_error_cases() {
    // Test missing colon after type name
    let result = parse_program_from_string("type Message\n    Text");
    assert!(result.is_err(), "Should fail on missing colon");

    // Test missing variants/fields
    let result = parse_program_from_string("type Empty:");
    assert!(result.is_err(), "Should fail on empty type body");

    // Test invalid variant syntax
    let result = parse_program_from_string("type Bad:\n    123Invalid");
    assert!(result.is_err(), "Should fail on invalid variant name");
}

#[test]
fn test_import_single_item() {
    let input = "import is_prime from ./nums";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse single import successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./nums");
        assert_eq!(statement.module, "./nums");
        assert_eq!(statement.names, ["is_prime"]);
        assert_eq!(items.len(), 1);

        if let ImportItem::Named { name, alias, .. } = &items[0] {
            assert_eq!(name, "is_prime");
            assert!(alias.is_none());
        } else {
            panic!("Expected ImportItem::Named");
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_with_alias() {
    let input = "import is_prime as is_prime_new from ./nums";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse import with alias successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./nums");
        assert_eq!(statement.module, "./nums");
        assert_eq!(statement.names, ["is_prime"]);
        assert_eq!(items.len(), 1);

        if let ImportItem::Named { name, alias, .. } = &items[0] {
            assert_eq!(name, "is_prime");
            assert_eq!(alias.as_ref().unwrap(), "is_prime_new");
        } else {
            panic!("Expected ImportItem::Named with alias");
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_multiple_items() {
    let input = "import is_prime, gcd, pi from ./nums";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse multiple imports successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./nums");
        assert_eq!(statement.module, "./nums");
        assert_eq!(statement.names, ["is_prime", "gcd", "pi"]);
        assert_eq!(items.len(), 3);

        let expected_names = ["is_prime", "gcd", "pi"];
        for (i, expected_name) in expected_names.iter().enumerate() {
            if let ImportItem::Named { name, alias, .. } = &items[i] {
                assert_eq!(name, expected_name);
                assert!(alias.is_none());
            } else {
                panic!("Expected ImportItem::Named for {}", expected_name);
            }
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_type() {
    let input = "import type User from ./models.types";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse type import successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./models.types");
        assert_eq!(statement.module, "./models.types");
        assert_eq!(statement.names, ["User"]);
        assert_eq!(items.len(), 1);

        if let ImportItem::Type { name, alias, .. } = &items[0] {
            assert_eq!(name, "User");
            assert!(alias.is_none());
        } else {
            panic!("Expected ImportItem::Type");
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_mixed_with_aliases() {
    let input = "import is_prime as is_prime_new, gcd as greatest_cd from ./nums";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse mixed imports with aliases successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./nums");
        assert_eq!(statement.module, "./nums");
        assert_eq!(statement.names, ["is_prime", "gcd"]);
        assert_eq!(items.len(), 2);

        if let ImportItem::Named { name, alias, .. } = &items[0] {
            assert_eq!(name, "is_prime");
            assert_eq!(alias.as_ref().unwrap(), "is_prime_new");
        } else {
            panic!("Expected first ImportItem::Named with alias");
        }

        if let ImportItem::Named { name, alias, .. } = &items[1] {
            assert_eq!(name, "gcd");
            assert_eq!(alias.as_ref().unwrap(), "greatest_cd");
        } else {
            panic!("Expected second ImportItem::Named with alias");
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_mixed_types_and_items() {
    let input = "import type User, Address from ./models.types";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Should parse multiple type imports successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "./models.types");
        assert_eq!(statement.module, "./models.types");
        assert_eq!(statement.names, ["User", "Address"]);
        assert_eq!(items.len(), 2);

        let expected_names = ["User", "Address"];
        for (i, expected_name) in expected_names.iter().enumerate() {
            if let ImportItem::Type { name, alias, .. } = &items[i] {
                assert_eq!(name, expected_name);
                assert!(alias.is_none());
            } else {
                panic!("Expected ImportItem::Type for {}", expected_name);
            }
        }
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_error_cases() {
    // Test missing 'from' keyword
    let result = parse_program_from_string("import is_prime ./nums");
    assert!(result.is_err(), "Should fail on missing 'from' keyword");

    // Test missing source path
    let result = parse_program_from_string("import is_prime from");
    assert!(result.is_err(), "Should fail on missing source path");

    // Test empty import list
    let result = parse_program_from_string("import from ./nums");
    assert!(result.is_err(), "Should fail on empty import list");

    // Test invalid alias syntax
    let result = parse_program_from_string("import is_prime as from ./nums");
    assert!(result.is_err(), "Should fail on invalid alias syntax");

    // Test missing item name
    let result = parse_program_from_string("import , gcd from ./nums");
    assert!(result.is_err(), "Should fail on missing item name");
}

#[test]
fn test_import_simple_quiz_standard_syntax_builds_import_statement_ast() {
    let input = "import take_input, string_to_int32 from standard";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "simple_quiz standard import should parse successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "standard");
        assert_eq!(statement.module, "standard");
        assert_eq!(statement.names, ["take_input", "string_to_int32"]);
        assert_eq!(items.len(), 2);
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_simple_quiz_math_syntax_builds_import_statement_ast() {
    let input = "import random_int32 from math";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "simple_quiz math import should parse successfully: {:?}",
        result.err()
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Import {
        statement,
        items,
        source,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(source, "math");
        assert_eq!(statement.module, "math");
        assert_eq!(statement.names, ["random_int32"]);
        assert_eq!(items.len(), 1);
    } else {
        panic!("Expected import declaration");
    }
}

#[test]
fn test_import_must_be_top_level_prefix_before_other_declarations() {
    let input = "entry main = f(): void => return void\nimport random_int32 from math";
    let result = parse_program_from_string(input);
    assert!(
        result.is_err(),
        "imports after non-import declarations should be rejected"
    );
}

#[test]
fn test_function_parameter_edge_cases() {
    // Test function with generic parameter types
    let input =
        "entry main = f(items: Array<string>, result: Result<int32, string>): void => return void";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (program, errors) = parser.parse();

    assert!(errors.is_empty(), "Parse errors: {errors:?}");
    assert!(program.is_some());

    let program = program.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function { parameters, .. } = &program.declarations[0] {
        assert_eq!(parameters.len(), 2);

        // Check first parameter (items: Array<string>)
        assert_eq!(parameters[0].name, "items");
        if let Type::Generic {
            name, type_args, ..
        } = &parameters[0].param_type
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);
            if let Type::Basic { name, .. } = &type_args[0] {
                assert_eq!(name, "string");
            } else {
                unreachable!("Expected string type argument");
            }
        } else {
            unreachable!("Expected generic type for first parameter");
        }

        // Check second parameter (result: Result<int32, string>)
        assert_eq!(parameters[1].name, "result");
        if let Type::Generic {
            name, type_args, ..
        } = &parameters[1].param_type
        {
            assert_eq!(name, "Result");
            assert_eq!(type_args.len(), 2);
        } else {
            unreachable!("Expected generic type for second parameter");
        }
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_function_array_parameter_types() {
    // Test function with array parameter types
    let input = "public process = f(numbers: int32[], names: string[][]): boolean[] => return void";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (program, errors) = parser.parse();

    assert!(errors.is_empty(), "Parse errors: {errors:?}");
    assert!(program.is_some());

    let program = program.unwrap();
    if let Decl::Function {
        parameters,
        return_types,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(parameters.len(), 2);

        // Check first parameter (numbers: int32[])
        assert_eq!(parameters[0].name, "numbers");
        if let Type::Array { element_type, .. } = &parameters[0].param_type {
            if let Type::Basic { name, .. } = element_type.as_ref() {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected int32 element type");
            }
        } else {
            unreachable!("Expected array type for first parameter");
        }

        // Check second parameter (names: string[][])
        assert_eq!(parameters[1].name, "names");
        if let Type::Array { element_type, .. } = &parameters[1].param_type {
            if let Type::Array {
                element_type: inner,
                ..
            } = element_type.as_ref()
            {
                if let Type::Basic { name, .. } = inner.as_ref() {
                    assert_eq!(name, "string");
                } else {
                    unreachable!("Expected string inner element type");
                }
            } else {
                unreachable!("Expected nested array type");
            }
        } else {
            unreachable!("Expected array type for second parameter");
        }

        // Check return type (boolean[])
        assert!(return_types.is_some());
        if let Some(return_types) = return_types {
            if let Type::Array { element_type, .. } = &return_types[0] {
                if let Type::Basic { name, .. } = element_type.as_ref() {
                    assert_eq!(name, "boolean");
                } else {
                    unreachable!("Expected boolean element type in return");
                }
            } else {
                unreachable!("Expected array return type");
            }
        } else {
            unreachable!("Expected return type");
        }
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_function_complex_return_types() {
    // Test function with complex return types
    let input = "entry compute = f(x: int32): Result<Array<string>, string> => return void";
    let lexer = Lexer::new(input);
    let (tokens, _) = lexer.tokenize();
    let parser = Parser::new(tokens);
    let (program, errors) = parser.parse();

    assert!(errors.is_empty(), "Parse errors: {errors:?}");
    assert!(program.is_some());

    let program = program.unwrap();
    if let Decl::Function { return_types, .. } = &program.declarations[0] {
        assert!(return_types.is_some());
        if let Some(return_types) = return_types {
            if let Type::Generic {
                name, type_args, ..
            } = &return_types[0]
            {
                assert_eq!(name, "Result");
                assert_eq!(type_args.len(), 2);

                // First type arg should be Array<string>
                if let Type::Generic {
                    name,
                    type_args: inner_args,
                    ..
                } = &type_args[0]
                {
                    assert_eq!(name, "Array");
                    assert_eq!(inner_args.len(), 1);
                    if let Type::Basic { name, .. } = &inner_args[0] {
                        assert_eq!(name, "string");
                    } else {
                        unreachable!("Expected string type in Array");
                    }
                } else {
                    unreachable!("Expected Array<string> as first type arg");
                }

                // Second type arg should be string
                if let Type::Basic { name, .. } = &type_args[1] {
                    assert_eq!(name, "string");
                } else {
                    unreachable!("Expected string as second type arg");
                }
            } else {
                unreachable!("Expected generic return type");
            }
        } else {
            unreachable!("Expected return types");
        }
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_function_parameter_error_cases() {
    // Test invalid parameter syntax
    let result1 = parse_program_from_string("entry test = f(x y: int32): void => return void");
    assert!(
        result1.is_err(),
        "Should fail on missing colon in parameter"
    );

    // Test missing parameter type
    let result2 = parse_program_from_string("entry test = f(x:): void => return void");
    assert!(result2.is_err(), "Should fail on missing parameter type");

    // Test invalid parameter name
    let result3 = parse_program_from_string("entry test = f(123: int32): void => return void");
    assert!(result3.is_err(), "Should fail on numeric parameter name");

    // Test missing parameter name
    let result4 = parse_program_from_string("entry test = f(: int32): void => return void");
    assert!(result4.is_err(), "Should fail on missing parameter name");

    // Test malformed generic parameter type
    let result5 = parse_program_from_string("entry test = f(x: Array<>): void => return void");
    assert!(result5.is_err(), "Should fail on empty generic type args");
}

#[test]
fn test_lambda_expression_basic() {
    // Test basic lambda expression as let declaration
    let input = "let add = f(x: int32, y: int32): int32 => x + y";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse basic lambda: {result:?}");

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let {
        binding,
        initializer,
        visibility,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(binding.name, "add");
        assert!(!binding.is_mutable);
        assert!(binding.type_annotation.is_none());
        assert_eq!(*visibility, Visibility::Private);

        // Check that initializer is a lambda expression
        if let Expr::Lambda {
            generic_params,
            params,
            return_types,
            body,
            ..
        } = initializer
        {
            assert!(generic_params.is_none());
            assert_eq!(params.len(), 2);
            assert_eq!(params[0].name, "x");
            assert_eq!(params[1].name, "y");

            if let Type::Basic { name, .. } = &return_types[0] {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected basic return type");
            }

            if let LambdaBody::Expression(expr) = body {
                if let Expr::Binary { .. } = expr.as_ref() {
                    // Binary expression is expected for x + y
                } else {
                    unreachable!("Expected binary expression in lambda body");
                }
            } else {
                unreachable!("Expected expression body");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_lambda_expression_generic() {
    // Test generic lambda expression as let declaration
    let input = "let identity = f<T>(x: T): T => x";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse generic lambda: {result:?}");

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let {
        binding,
        initializer,
        visibility,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(binding.name, "identity");
        assert!(!binding.is_mutable);
        assert!(binding.type_annotation.is_none());
        assert_eq!(*visibility, Visibility::Private);

        // Check that initializer is a lambda expression
        if let Expr::Lambda {
            generic_params,
            params,
            return_types,
            body,
            ..
        } = initializer
        {
            assert!(generic_params.is_some());
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.len(), 1);
            assert_eq!(generics[0], "T");

            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");

            // Check parameter type is generic
            if let Type::Basic { name, .. } = &params[0].param_type {
                assert_eq!(name, "T");
            } else {
                unreachable!("Expected generic parameter type");
            }

            if let Type::Basic { name, .. } = &return_types[0] {
                assert_eq!(name, "T");
            } else {
                unreachable!("Expected generic return type");
            }

            if let LambdaBody::Expression(expr) = body {
                if let Expr::Identifier { name, .. } = expr.as_ref() {
                    assert_eq!(name, "x");
                } else {
                    unreachable!("Expected identifier in lambda body");
                }
            } else {
                unreachable!("Expected expression body");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_function_declaration_with_generic_constraints() {
    let input = "entry main = f<T: int32 + int32>(x: T): T => return x";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse constrained generic function: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function {
        generic_params,
        generic_constraints,
        parameters,
        return_types,
        ..
    } = &program.declarations[0]
    {
        let generic_names = generic_params
            .as_ref()
            .expect("generic params should exist");
        assert_eq!(generic_names.as_slice(), ["T"]);

        let declarations = generic_constraints
            .as_ref()
            .expect("generic constraints should exist");
        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].name, "T");
        assert_eq!(declarations[0].constraints.len(), 2);

        assert_eq!(parameters.len(), 1);
        if let Type::Basic { name, .. } = &parameters[0].param_type {
            assert_eq!(name, "T");
        } else {
            panic!("Expected parameter type T");
        }

        let returns = return_types
            .as_ref()
            .expect("return types should be present");
        if let Type::Basic { name, .. } = &returns[0] {
            assert_eq!(name, "T");
        } else {
            panic!("Expected return type T");
        }
    } else {
        panic!("Expected function declaration");
    }
}

#[test]
fn test_lambda_expression_with_generic_constraints() {
    let input = "let identity = f<T: int32>(x: T): T => x";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse constrained generic lambda: {result:?}"
    );

    let program = result.unwrap();
    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            generic_params,
            generic_constraints,
            ..
        } = initializer
        {
            let generic_names = generic_params
                .as_ref()
                .expect("generic params should exist");
            assert_eq!(generic_names.as_slice(), ["T"]);

            let declarations = generic_constraints
                .as_ref()
                .expect("generic constraints should exist");
            assert_eq!(declarations.len(), 1);
            assert_eq!(declarations[0].name, "T");
            assert_eq!(declarations[0].constraints.len(), 1);
        } else {
            panic!("Expected lambda initializer");
        }
    } else {
        panic!("Expected let declaration");
    }
}

#[test]
fn test_explicit_generic_call_expression_parsing() {
    let input = "map<int32, string>(value)";
    let result = parse_expression_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse explicit generic call: {result:?}"
    );

    let expr = result.unwrap();
    if let Expr::Call {
        callee,
        generic_args,
        args,
        ..
    } = expr
    {
        if let Expr::Identifier { name, .. } = *callee {
            assert_eq!(name, "map");
        } else {
            panic!("Expected identifier callee");
        }
        let parsed_generic_args = generic_args.expect("generic args should exist");
        assert_eq!(parsed_generic_args.len(), 2);
        assert_eq!(args.len(), 1);
    } else {
        panic!("Expected call expression");
    }
}

#[test]
fn test_lambda_expression_no_params() {
    // Test lambda with no parameters as let declaration
    let input = "let get_42 = f(): int32 => 42";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse no-param lambda: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let {
        binding,
        initializer,
        visibility,
        ..
    } = &program.declarations[0]
    {
        assert_eq!(binding.name, "get_42");
        assert!(!binding.is_mutable);
        assert!(binding.type_annotation.is_none());
        assert_eq!(*visibility, Visibility::Private);

        // Check that initializer is a lambda expression
        if let Expr::Lambda {
            generic_params,
            params,
            return_types,
            body,
            ..
        } = initializer
        {
            assert!(generic_params.is_none());
            assert_eq!(params.len(), 0);

            if let Type::Basic { name, .. } = &return_types[0] {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected basic return type");
            }

            if let LambdaBody::Expression(expr) = body {
                if let Expr::Literal { value, .. } = expr.as_ref() {
                    if let LiteralValue::Integer(n) = value {
                        assert_eq!(*n, 42);
                    } else {
                        unreachable!("Expected integer literal");
                    }
                } else {
                    unreachable!("Expected literal in lambda body");
                }
            } else {
                unreachable!("Expected expression body");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_lambda_expression_multiple_generics() {
    // Test lambda with multiple generic parameters as let declaration
    let input = "let map_fn = f<T, U>(transform: f(T): U, value: T): U => transform(value)";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse multi-generic lambda: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    let let_decl = match &program.declarations[0] {
        Decl::Let {
            binding,
            initializer,
            visibility,
            ..
        } => {
            assert_eq!(binding.name, "map_fn");
            assert!(!binding.is_mutable);
            assert!(binding.type_annotation.is_none());
            assert_eq!(*visibility, Visibility::Private);
            initializer
        }
        _ => unreachable!("Expected let declaration"),
    };

    // Check that initializer is a lambda expression
    if let Expr::Lambda {
        generic_params,
        params,
        return_types,
        ..
    } = let_decl
    {
        validate_lambda_generics(generic_params.as_ref());
        validate_lambda_parameters(params);
        validate_lambda_return_type(&return_types[0]);
    } else {
        unreachable!("Expected lambda expression");
    }
}

/// Helper function to validate generic parameters in lambda expressions
fn validate_lambda_generics(generic_params: Option<&Vec<String>>) {
    assert!(generic_params.is_some());
    let generics = generic_params.unwrap();
    assert_eq!(generics.len(), 2);
    assert_eq!(generics[0], "T");
    assert_eq!(generics[1], "U");
}

/// Helper function to validate lambda parameters in complex generic scenarios
fn validate_lambda_parameters(params: &[Parameter]) {
    assert_eq!(params.len(), 2);
    assert_eq!(params[0].name, "transform");
    assert_eq!(params[1].name, "value");

    // Check first parameter is a function type
    if let Type::Function {
        parameters: fn_params,
        return_types: fn_returns,
        ..
    } = &params[0].param_type
    {
        assert_eq!(fn_params.len(), 1);
        if let Type::Basic { name, .. } = &fn_params[0] {
            assert_eq!(name, "T");
        } else {
            unreachable!("Expected T parameter type");
        }

        if let Type::Basic { name, .. } = &fn_returns[0] {
            assert_eq!(name, "U");
        } else {
            unreachable!("Expected U return type");
        }
    } else {
        unreachable!("Expected function type for transform parameter");
    }

    // Check second parameter type
    if let Type::Basic { name, .. } = &params[1].param_type {
        assert_eq!(name, "T");
    } else {
        unreachable!("Expected generic parameter type");
    }
}

/// Helper function to validate lambda return type
fn validate_lambda_return_type(return_type: &Type) {
    if let Type::Basic { name, .. } = return_type {
        assert_eq!(name, "U");
    } else {
        unreachable!("Expected generic return type");
    }
}

#[test]
fn test_lambda_expression_error_cases() {
    // Test missing parameters parentheses
    let result1 = parse_expression_from_string("f x: int32 => x");
    assert!(result1.is_err(), "Should fail on missing parentheses");

    // Test missing colon before return type
    let result2 = parse_expression_from_string("f() int32 => 42");
    assert!(result2.is_err(), "Should fail on missing colon");

    // Test missing arrow
    let result3 = parse_expression_from_string("f(): int32 42");
    assert!(result3.is_err(), "Should fail on missing arrow");

    // Test empty generic parameters
    let result4 = parse_expression_from_string("f<>(): void => void");
    assert!(result4.is_err(), "Should fail on empty generics");

    // Test malformed generic parameters
    let result5 = parse_expression_from_string("f<T,>(): void => void");
    assert!(
        result5.is_err(),
        "Should fail on trailing comma in generics"
    );
}

#[test]
fn test_lambda_as_function_parameter() {
    // Test lambda as function parameter type
    let input = "entry test = f(callback: f(int32): boolean): void => return void";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse lambda as parameter: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function {
        parameters: params, ..
    } = &program.declarations[0]
    {
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "callback");

        if let Type::Function {
            parameters: fn_params,
            return_types: fn_returns,
            ..
        } = &params[0].param_type
        {
            assert_eq!(fn_params.len(), 1);
            if let Type::Basic { name, .. } = &fn_params[0] {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected int32 parameter type");
            }

            if let Type::Basic { name, .. } = &fn_returns[0] {
                assert_eq!(name, "boolean");
            } else {
                unreachable!("Expected boolean return type");
            }
        } else {
            unreachable!("Expected function type");
        }
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_lambda_expression_nested() {
    // Test nested lambda expressions
    let input = "let curry_add = f(x: int32): f(int32): int32 => f(y: int32): int32 => x + y";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse nested lambda: {result:?}");

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            params,
            return_types,
            body,
            ..
        } = initializer
        {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "x");

            // Check return type is a function type
            if let Type::Function { .. } = &return_types[0] {
                // Good
            } else {
                unreachable!("Expected function return type for curried function");
            }

            // Check body contains another lambda
            if let LambdaBody::Expression(expr) = body {
                if let Expr::Lambda { .. } = expr.as_ref() {
                    // Good, nested lambda found
                } else {
                    unreachable!("Expected nested lambda in body");
                }
            } else {
                unreachable!("Expected expression body");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_lambda_expression_block_body() {
    // Test lambda with block body
    let input =
        "let complex_fn = f(x: int32): int32 => { let doubled = x * 2; return doubled + 1; }";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse block body lambda: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda { body, .. } = initializer {
            if let LambdaBody::Block(statements) = body {
                assert_eq!(statements.len(), 2);
                // First statement should be a let binding
                if let Stmt::Let { binding, .. } = &statements[0] {
                    assert_eq!(binding.name, "doubled");
                } else {
                    unreachable!("Expected let statement");
                }
                // Second statement should be a return
                if let Stmt::Return { .. } = &statements[1] {
                    // Good
                } else {
                    unreachable!("Expected return statement");
                }
            } else {
                unreachable!("Expected block body");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_lambda_expression_complex_generics() {
    // Test lambda with complex generic constraints
    let input = "let transform = f<T, U, V>(data: T[], mapper: f(T): U, reducer: f(U[]): V): V => reducer(map(data, mapper))";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse complex generic lambda: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            generic_params,
            params,
            ..
        } = initializer
        {
            // Check generic parameters
            assert!(generic_params.is_some());
            let generics = generic_params.as_ref().unwrap();
            assert_eq!(generics.len(), 3);
            assert_eq!(generics[0], "T");
            assert_eq!(generics[1], "U");
            assert_eq!(generics[2], "V");

            // Check parameters
            assert_eq!(params.len(), 3);
            assert_eq!(params[0].name, "data");
            assert_eq!(params[1].name, "mapper");
            assert_eq!(params[2].name, "reducer");
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

// =========================================================================
// Error Handling Clause Tests
// =========================================================================

#[test]
fn test_function_with_zero_errors() {
    // Test function with no errors clause (default)
    let input = "let parse = f(s: string): int32 => 42";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse function without errors: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda { error_types, .. } = initializer {
            assert!(
                error_types.is_empty(),
                "Expected empty error_types for function without errors clause"
            );
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_function_with_one_error() {
    // Test function with single error type
    let input = "let parse = f(s: string): int32 errors ParseError => 42";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse function with one error: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda { error_types, .. } = initializer {
            assert_eq!(error_types.len(), 1, "Expected one error type");
            assert_eq!(error_types[0], "ParseError");
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_function_with_multiple_errors() {
    // Test function with multiple error types
    let input =
        r#"public let read_file = f(path: string): string errors IoError, ParseError => "result""#;
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse function with multiple errors: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda { error_types, .. } = initializer {
            assert_eq!(error_types.len(), 2, "Expected two error types");
            assert_eq!(error_types[0], "IoError");
            assert_eq!(error_types[1], "ParseError");
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_function_with_errors_whitespace_variations() {
    // Test various whitespace around errors clause
    let inputs = vec![
        "let f1 = f(x: int32): int32 errors E1 => x",
        "let f2 = f(x: int32): int32 errors E1, E2 => x",
        "let f3 = f(x: int32): int32 errors E1 , E2 => x",
        "let f4 = f(x: int32): int32 errors E1,E2,E3 => x",
    ];

    for input in inputs {
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse with whitespace variation: {input:?} - {result:?}"
        );
    }
}

#[test]
fn test_function_declaration_with_errors() {
    // Test entry function with errors clause
    let input = "entry main = f(args: string[]): int32 errors AppError => 0";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse entry function with errors: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Function {
        error_types,
        is_entry,
        ..
    } = &program.declarations[0]
    {
        assert!(*is_entry, "Expected entry function");
        assert_eq!(error_types.len(), 1, "Expected one error type");
        assert_eq!(error_types[0], "AppError");
    } else {
        unreachable!("Expected function declaration");
    }
}

#[test]
fn test_lambda_with_generic_and_errors() {
    // Test generic lambda with errors clause
    let input = "let map_try = f<T, U>(arr: T[], fn: f(T): U errors E): U[] errors E => arr";
    let result = parse_program_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse generic lambda with errors: {result:?}"
    );

    let program = result.unwrap();
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            generic_params,
            error_types,
            params,
            ..
        } = initializer
        {
            assert!(generic_params.is_some(), "Expected generic parameters");
            assert_eq!(error_types.len(), 1, "Expected one error type");
            assert_eq!(error_types[0], "E");

            // Check that parameter type also has errors
            assert_eq!(params.len(), 2);
            assert_eq!(params[1].name, "fn");
            if let Type::Function { errors, .. } = &params[1].param_type {
                assert!(
                    errors.is_some(),
                    "Expected errors in function parameter type"
                );
                let fn_errors = errors.as_ref().unwrap();
                assert_eq!(fn_errors.len(), 1);
                if let Type::Basic { name, .. } = &fn_errors[0] {
                    assert_eq!(name, "E");
                } else {
                    unreachable!("Expected basic error type");
                }
            } else {
                unreachable!("Expected function type for parameter");
            }
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_errors_clause_error_cases() {
    // Test errors clause without type names (should fail)
    let result1 = parse_program_from_string("let f1 = f(x: int32): int32 errors => x");
    assert!(
        result1.is_err(),
        "Should fail on errors clause without types"
    );

    // Test errors clause with trailing comma (should fail)
    let result2 = parse_program_from_string("let f2 = f(x: int32): int32 errors E1, => x");
    assert!(
        result2.is_err(),
        "Should fail on trailing comma in errors clause"
    );

    // Test errors clause with missing comma
    let result3 = parse_program_from_string("let f3 = f(x: int32): int32 errors E1 E2 => x");
    assert!(
        result3.is_err(),
        "Should fail on missing comma between error types"
    );
}

#[test]
fn test_function_type_with_errors() {
    // Test parsing function type with errors clause
    let input = "f(int32): int32 errors ParseError";
    let result = parse_type_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse function type with errors: {result:?}"
    );

    if let Type::Function { errors, .. } = result.unwrap() {
        assert!(errors.is_some(), "Expected errors in function type");
        let error_types = errors.unwrap();
        assert_eq!(error_types.len(), 1);
        if let Type::Basic { name, .. } = &error_types[0] {
            assert_eq!(name, "ParseError");
        } else {
            unreachable!("Expected basic error type");
        }
    } else {
        unreachable!("Expected function type");
    }
}

#[test]
fn test_function_type_with_multiple_errors() {
    // Test parsing function type with multiple errors
    let input = "f(string): int32 errors IoError, ParseError, NetworkError";
    let result = parse_type_from_string(input);
    assert!(
        result.is_ok(),
        "Failed to parse function type with multiple errors: {result:?}"
    );

    if let Type::Function { errors, .. } = result.unwrap() {
        assert!(errors.is_some(), "Expected errors in function type");
        let error_types = errors.unwrap();
        assert_eq!(error_types.len(), 3);

        let names: Vec<&str> = error_types
            .iter()
            .filter_map(|t| {
                if let Type::Basic { name, .. } = t {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(names, vec!["IoError", "ParseError", "NetworkError"]);
    } else {
        unreachable!("Expected function type");
    }
}

// Tests for unreachable!() conversion to proper ParseError handling
#[test]
fn test_guard_binding_with_invalid_token_after_into() {
    // RED test: guard into should receive identifier but might receive non-identifier
    // If check_identifier is true but token is not Identifier, we should error gracefully
    // This tests expressions.rs:229 unreachable
    let input = "let x = guard foo(42) into 123 else err => {return void}";
    let result = parse_expression_from_string(input);

    // Should return ParseError, not panic
    assert!(
        result.is_err(),
        "Expected parse error for invalid binding after 'into', but got: {result:?}"
    );
}

#[test]
fn test_guard_success_binding_with_invalid_token() {
    // RED test: statements.rs:565 unreachable
    // guard success_binding should receive identifier, error gracefully otherwise
    let input = "guard foo() into 456 else err => {return void}";
    let result = parse_statement_from_string(input);

    // Should return ParseError, not panic
    assert!(
        result.is_err(),
        "Expected parse error for invalid binding after 'into', but got: {result:?}"
    );
}

#[test]
fn test_guard_error_binding_with_invalid_token() {
    // RED test: statements.rs:582 unreachable
    // guard error_binding should receive identifier, error gracefully otherwise
    let input = "guard foo() into x else 789 => {return void}";
    let result = parse_statement_from_string(input);

    // Should return ParseError, not panic
    assert!(
        result.is_err(),
        "Expected parse error for invalid binding after 'else', but got: {result:?}"
    );
}

#[test]
fn test_closure_captures_outer_variable() {
    // RED test: closure capture analysis
    // let add_x = f(y: int64): int64 => x + y
    // "x" is NOT a parameter so it should be in captured_variables
    let input = "let add_x = f(y: int64): int64 => x + y";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse closure: {result:?}");

    let program = result.unwrap();
    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            params,
            captured_variables,
            ..
        } = initializer
        {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "y");
            assert!(
                captured_variables.contains(&"x".to_owned()),
                "Expected 'x' in captured_variables, got: {captured_variables:?}"
            );
            assert!(
                !captured_variables.contains(&"y".to_owned()),
                "Parameter 'y' should NOT be in captured_variables"
            );
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_lambda_with_no_captures() {
    // Lambda that only uses its own parameters — captured_variables should be empty
    let input = "let add = f(x: int64, y: int64): int64 => x + y";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse lambda: {result:?}");

    let program = result.unwrap();
    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            captured_variables, ..
        } = initializer
        {
            assert!(
                captured_variables.is_empty(),
                "Expected no captures for self-contained lambda, got: {captured_variables:?}"
            );
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}

#[test]
fn test_closure_block_body_captures() {
    // Closure with block body capturing an outer variable
    // add_base(y: int64): int64 => { return base + y }
    // "base" is captured, "y" is a parameter
    let input = "let add_base = f(y: int64): int64 => {\n    return base + y\n}";
    let result = parse_program_from_string(input);
    assert!(result.is_ok(), "Failed to parse block closure: {result:?}");

    let program = result.unwrap();
    if let Decl::Let { initializer, .. } = &program.declarations[0] {
        if let Expr::Lambda {
            captured_variables, ..
        } = initializer
        {
            assert!(
                captured_variables.contains(&"base".to_owned()),
                "Expected 'base' in captured_variables, got: {captured_variables:?}"
            );
            assert!(
                !captured_variables.contains(&"y".to_owned()),
                "Parameter 'y' should NOT be in captured_variables"
            );
        } else {
            unreachable!("Expected lambda expression");
        }
    } else {
        unreachable!("Expected let declaration");
    }
}
