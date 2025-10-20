//! Tests for the type system

extern crate alloc;
use alloc::collections::BTreeMap;

use super::checker::TypeChecker;
use super::constraints::TypeConstraint;
use super::environment::TypeEnvironment;
use super::errors::TypeError;
use super::substitution::Substitution;
use super::symbol_table::{ScopeId, SymbolInfo, SymbolTable, SymbolType, Visibility};
use super::types::{CoreType, TypeVar};
use crate::ast::{
    Decl, Expr, Field, HotReloadMetadata, LetBinding, LiteralValue, NodeId, Parameter, Program,
    Stmt, StringPart, Type, TypeDef, Variant, Visibility as AstVisibility,
};
use crate::token::{Position, Span};

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
        parameters: params,
        return_type,
        body,
        visibility: AstVisibility::Private,
        is_entry: false,
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
    Stmt::Return {
        value: Some(value),
        span: test_span(),
        id: node_id(id),
    }
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
    let core_type = TypeChecker::ast_type_to_core_type(&ast_type).unwrap();
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
        TypeChecker::ast_type_to_core_type(&int32_type).unwrap(),
        CoreType::Int32
    );

    let string_type = Type::Basic {
        name: "string".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&string_type).unwrap(),
        CoreType::String
    );

    let invalid_type = Type::Basic {
        name: "nonexistent".to_owned(),
        span,
    };
    assert!(TypeChecker::ast_type_to_core_type(&invalid_type).is_err());
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
        TypeChecker::ast_type_to_core_type(&int8_type).unwrap(),
        CoreType::Int8
    );

    let int16_type = Type::Basic {
        name: "int16".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&int16_type).unwrap(),
        CoreType::Int16
    );

    let uint8_type = Type::Basic {
        name: "uint8".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&uint8_type).unwrap(),
        CoreType::UInt8
    );

    let uint16_type = Type::Basic {
        name: "uint16".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&uint16_type).unwrap(),
        CoreType::UInt16
    );

    let uint32_type = Type::Basic {
        name: "uint32".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&uint32_type).unwrap(),
        CoreType::UInt32
    );

    let uint64_type = Type::Basic {
        name: "uint64".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&uint64_type).unwrap(),
        CoreType::UInt64
    );

    let int64_type = Type::Basic {
        name: "int64".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&int64_type).unwrap(),
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
        TypeChecker::ast_type_to_core_type(&float32_type).unwrap(),
        CoreType::Float32
    );

    let float64_type = Type::Basic {
        name: "float64".to_owned(),
        span,
    };
    assert_eq!(
        TypeChecker::ast_type_to_core_type(&float64_type).unwrap(),
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
    let array_result = TypeChecker::ast_type_to_core_type(&array_type);
    assert!(array_result.is_ok());
    assert_eq!(
        array_result.unwrap(),
        CoreType::Array(Box::new(CoreType::Int32))
    );

    let function_type = Type::Function {
        parameters: vec![],
        return_type: Box::new(Type::Basic {
            name: "unit".to_owned(),
            span,
        }),
        span,
    };
    let function_result = TypeChecker::ast_type_to_core_type(&function_type);
    assert!(function_result.is_ok());
    assert_eq!(
        function_result.unwrap(),
        CoreType::Function {
            parameters: vec![],
            return_type: Box::new(CoreType::Unit),
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
    let generic_result = TypeChecker::ast_type_to_core_type(&generic_type);
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
        parameters: vec![var1_type],
        return_type: Box::new(var2_type),
    };

    let mut mappings = BTreeMap::new();
    mappings.insert(var1.id, CoreType::Int32);
    mappings.insert(var2.id, CoreType::String);
    let subst = Substitution { mappings };

    let expected = CoreType::Function {
        parameters: vec![CoreType::Int32],
        return_type: Box::new(CoreType::String),
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
        parameters: vec![CoreType::Int32],
        return_type: Box::new(CoreType::String),
    };
    let func2 = CoreType::Function {
        parameters: vec![CoreType::Int32],
        return_type: Box::new(CoreType::String),
    };
    let func3 = CoreType::Function {
        parameters: vec![CoreType::String],
        return_type: Box::new(CoreType::Int32),
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
            parameters: vec![],
            return_type: Box::new(CoreType::Unit),
        },
        visibility: Visibility::Public,
        source_location: Span::single(Position::start()),
    });

    // Register entry point in global scope
    table.register(SymbolInfo {
        name: "main".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            parameters: vec![],
            return_type: Box::new(CoreType::Unit),
        },
        visibility: Visibility::Entry,
        source_location: Span::single(Position::start()),
    });

    // Register private symbol in global scope
    table.register(SymbolInfo {
        name: "private_func".to_owned(),
        symbol_type: SymbolType::Function,
        core_type: CoreType::Function {
            parameters: vec![],
            return_type: Box::new(CoreType::Unit),
        },
        visibility: Visibility::Private,
        source_location: Span::single(Position::start()),
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
        value: Some(literal_expr(LiteralValue::String("bad".to_owned()), 20_120)),
        span: test_span(),
        id: node_id(20_121),
    };

    let expected = CoreType::Int32;
    let result = checker.type_check_stmt_with_return(&return_stmt, Some(&expected));
    assert!(
        matches!(result, Err(TypeError::TypeMismatch { .. })),
        "return statements must match expected return type"
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
fn test_type_check_program_collects_errors() {
    let mut checker = TypeChecker::new();
    let decl = Decl::Function {
        name: "bad".to_owned(),
        parameters: vec![Parameter {
            name: "x".to_owned(),
            param_type: Type::Basic {
                name: "int32".to_owned(),
                span: test_span(),
            },
            span: test_span(),
        }],
        return_type: Some(Type::Basic {
            name: "int32".to_owned(),
            span: test_span(),
        }),
        body: Stmt::Return {
            value: Some(literal_expr(LiteralValue::Boolean(true), 10_200)),
            span: test_span(),
            id: node_id(10_201),
        },
        visibility: AstVisibility::Private,
        is_entry: false,
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

    let program = create_program(vec![let_decl, fn_decl]);
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

    let program = create_program(vec![first_fn, second_fn]);
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

    let program = create_program(vec![let_decl]);
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
        params: vec![make_parameter("x", int_type("int32"))],
        return_type: int_type("int32"),
        body: crate::ast::LambdaBody::Expression(Box::new(identifier_expr("x", 32_000))),
        captured_variables: vec![],
        metadata: HotReloadMetadata::for_expression(),
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
        return_type,
    } = core_type
    {
        assert_eq!(parameters, vec![CoreType::Int32]);
        assert_eq!(*return_type, CoreType::Int32);
    } else {
        unreachable!("lambda should yield a function type");
    }
}

#[test]
fn test_lambda_block_body_type_checking() {
    let mut checker = TypeChecker::new();
    let return_stmt = Stmt::Return {
        value: Some(identifier_expr("x", 32_100)),
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
        params: vec![make_parameter("x", int_type("int32"))],
        return_type: int_type("int32"),
        body: crate::ast::LambdaBody::Block(vec![body]),
        captured_variables: vec![],
        metadata: HotReloadMetadata::for_expression(),
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
        return_type,
    } = core_type
    {
        assert_eq!(parameters, vec![CoreType::Int32]);
        assert_eq!(*return_type, CoreType::Int32);
    } else {
        unreachable!("lambda should yield a function type");
    }
}

#[test]
fn test_lambda_return_type_mismatch_is_reported() {
    let mut checker = TypeChecker::new();
    let lambda = Expr::Lambda {
        generic_params: None,
        params: vec![make_parameter("x", int_type("int32"))],
        return_type: int_type("int32"),
        body: crate::ast::LambdaBody::Expression(Box::new(literal_expr(
            LiteralValue::Boolean(true),
            32_200,
        ))),
        captured_variables: vec![],
        metadata: HotReloadMetadata::for_expression(),
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
        parameters: vec![CoreType::Int32, CoreType::String],
        return_type: Box::new(CoreType::Boolean),
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
        parameters: vec![CoreType::Int32, CoreType::String],
        return_type: Box::new(CoreType::Boolean),
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
        parameters: vec![CoreType::Int32, CoreType::String],
        return_type: Box::new(CoreType::Boolean),
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
