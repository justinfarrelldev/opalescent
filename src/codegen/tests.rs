use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{codegen_expression, CodegenEnv};
use crate::codegen::statements::codegen_statement;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::{CoreType, GenericTypeParameter, TypeVar};
use crate::{
    ast::{BinaryOp, Expr, LetBinding, LiteralValue, NodeId, Stmt, Type, UnaryOp},
    token::{Position, Span},
};
extern crate alloc;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::context::Context;
use inkwell::types::AnyType;
use inkwell::values::AnyValue;

const TEST_LINE: usize = 1;
const TEST_COLUMN: usize = 1;

fn test_span() -> Span {
    Span::single(Position::new(TEST_LINE, TEST_COLUMN, 0))
}

fn test_node_id(id: usize) -> NodeId {
    NodeId(id)
}

fn int_lit(id: usize, value: i64) -> Expr {
    Expr::Literal {
        value: LiteralValue::Integer(value),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn float_lit(id: usize, value: f64) -> Expr {
    Expr::Literal {
        value: LiteralValue::Float(value),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn bool_lit(id: usize, value: bool) -> Expr {
    Expr::Literal {
        value: LiteralValue::Boolean(value),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn string_lit(id: usize, value: &str) -> Expr {
    Expr::Literal {
        value: LiteralValue::String(value.to_owned()),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn void_lit(id: usize) -> Expr {
    Expr::Literal {
        value: LiteralValue::Void,
        span: test_span(),
        id: test_node_id(id),
    }
}

fn ident(id: usize, name: &str) -> Expr {
    Expr::Identifier {
        name: name.to_owned(),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn binary(id: usize, left: Expr, op: BinaryOp, right: Expr) -> Expr {
    Expr::Binary {
        left: Box::new(left),
        operator: op,
        right: Box::new(right),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn unary(id: usize, op: UnaryOp, operand: Expr) -> Expr {
    Expr::Unary {
        operator: op,
        operand: Box::new(operand),
        span: test_span(),
        id: test_node_id(id),
    }
}

fn cast(id: usize, expr: Expr, target_name: &str) -> Expr {
    Expr::Cast {
        expr: Box::new(expr),
        target_type: Type::Basic {
            name: target_name.to_owned(),
            span: test_span(),
        },
        span: test_span(),
        id: test_node_id(id),
    }
}

fn create_codegen_function<'context>(
    codegen_context: &CodegenContext<'context>,
    function_name: &str,
) -> inkwell::values::FunctionValue<'context> {
    let function_type = codegen_context.context.void_type().fn_type(&[], false);
    let function = codegen_context
        .module
        .add_function(function_name, function_type, None);
    let entry_block = codegen_context
        .context
        .append_basic_block(function, "entry");
    codegen_context.builder.position_at_end(entry_block);
    function
}

#[test]
fn test_core_type_mapping_covers_all_variants() {
    let context = Context::create();

    let type_variable = TypeVar {
        id: 1,
        name: "T".to_owned(),
    };
    let generic_param = GenericTypeParameter {
        name: "T".to_owned(),
        type_var: type_variable.clone(),
        constraints: Vec::new(),
    };

    let cases = [
        (CoreType::Int8, "i8"),
        (CoreType::Int16, "i16"),
        (CoreType::Int32, "i32"),
        (CoreType::Int64, "i64"),
        (CoreType::UInt8, "i8"),
        (CoreType::UInt16, "i16"),
        (CoreType::UInt32, "i32"),
        (CoreType::UInt64, "i64"),
        (CoreType::Float32, "float"),
        (CoreType::Float64, "double"),
        (CoreType::Boolean, "i1"),
        (CoreType::String, "i8*"),
        (CoreType::Array(Box::new(CoreType::Int32)), "[0 x i32]"),
        (CoreType::Unit, "{}"),
        (CoreType::Variable(type_variable), "i8*"),
        (
            CoreType::Function {
                generic_params: vec![generic_param],
                parameters: vec![CoreType::Int32],
                return_types: vec![CoreType::Int32],
                error_types: Vec::new(),
            },
            "i8*",
        ),
        (
            CoreType::Generic {
                name: "List".to_owned(),
                type_args: vec![CoreType::Int32],
            },
            "i8*",
        ),
    ];

    for (core_type, expected_llvm_text) in cases {
        let llvm_type = core_type_to_llvm(&context, &core_type);
        let llvm_type_text = llvm_type.as_any_type_enum().print_to_string().to_string();
        assert_eq!(
            llvm_type_text, expected_llvm_text,
            "unexpected LLVM type mapping for {core_type}"
        );
    }
}

#[test]
fn test_codegen_context_new_creates_module_and_builder() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "task21_module");

    assert_eq!(
        codegen_context.module.get_name().to_str(),
        Ok("task21_module"),
        "module name should match constructor input"
    );
    assert!(
        codegen_context.target_machine.is_some(),
        "target machine should be created for the default target triple"
    );
}

#[test]
fn test_codegen_context_sets_target_triple() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "triple_module");
    let configured_triple = codegen_context.target_triple();
    let default_triple = inkwell::targets::TargetMachine::get_default_triple();

    assert_eq!(
        configured_triple, default_triple,
        "module target triple must match LLVM default triple"
    );
}

#[test]
fn test_codegen_integer_and_float_literals_with_type_hints() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "expr_literals");
    let _function = create_codegen_function(&codegen_context, "expr_literals_fn");
    let mut env = CodegenEnv::new(true);

    let int_expr = int_lit(1, 42);
    let int_result = codegen_expression(
        &codegen_context,
        &mut env,
        &int_expr,
        Some(&CoreType::UInt8),
    );
    assert!(
        int_result.is_ok(),
        "integer literal codegen should succeed for uint8 hint"
    );

    let float_expr = float_lit(2, 3.5);
    let float_result = codegen_expression(
        &codegen_context,
        &mut env,
        &float_expr,
        Some(&CoreType::Float32),
    );
    assert!(
        float_result.is_ok(),
        "float literal codegen should succeed for float32 hint"
    );

    let int_bits = int_result
        .ok()
        .map(|value| value.into_int_value().print_to_string().to_string())
        .unwrap_or_default();
    assert!(
        int_bits.contains("i8 42"),
        "integer literal should lower to i8 constant when hinted: {int_bits}"
    );

    let float_bits = float_result
        .ok()
        .map(|value| value.into_float_value().print_to_string().to_string())
        .unwrap_or_default();
    assert!(
        float_bits.contains("float 0x") || float_bits.contains("float 3.500000e+00"),
        "float literal should lower to float constant when hinted: {float_bits}"
    );
}

#[test]
fn test_codegen_boolean_string_and_unit_literals() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "misc_literals");
    let _function = create_codegen_function(&codegen_context, "misc_literals_fn");
    let mut env = CodegenEnv::new(true);

    let bool_result = codegen_expression(
        &codegen_context,
        &mut env,
        &bool_lit(3, true),
        Some(&CoreType::Boolean),
    );
    assert!(
        bool_result.is_ok(),
        "boolean literal codegen should succeed"
    );

    let string_result = codegen_expression(
        &codegen_context,
        &mut env,
        &string_lit(4, "hello"),
        Some(&CoreType::String),
    );
    assert!(
        string_result.is_ok(),
        "string literal codegen should succeed"
    );

    let unit_result = codegen_expression(
        &codegen_context,
        &mut env,
        &void_lit(5),
        Some(&CoreType::Unit),
    );
    assert!(unit_result.is_ok(), "unit literal codegen should succeed");

    let ir = codegen_context.module.print_to_string().to_string();
    let bool_bits = bool_result
        .ok()
        .map(|value| value.into_int_value().print_to_string().to_string())
        .unwrap_or_default();
    assert!(
        bool_bits.contains("i1 true") || bool_bits.contains("i1 1"),
        "boolean should lower to i1 true constant: {bool_bits}"
    );
    assert!(
        ir.contains("@str.") || ir.contains("c\"hello\\00\""),
        "string literal should materialize as global constant: {ir}"
    );
}

#[test]
fn test_codegen_arithmetic_overflow_and_division_traps() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "trap_ops");
    let _function = create_codegen_function(&codegen_context, "trap_ops_fn");
    let mut env = CodegenEnv::new(true);

    let overflow_expr = binary(10, int_lit(11, i64::MAX), BinaryOp::Add, int_lit(12, 1));
    let overflow_result = codegen_expression(
        &codegen_context,
        &mut env,
        &overflow_expr,
        Some(&CoreType::Int64),
    );
    assert!(
        overflow_result.is_ok(),
        "overflow-checking add codegen should succeed"
    );

    let division_expr = binary(13, int_lit(14, 42), BinaryOp::Divide, int_lit(15, 0));
    let division_result = codegen_expression(
        &codegen_context,
        &mut env,
        &division_expr,
        Some(&CoreType::Int64),
    );
    assert!(
        division_result.is_ok(),
        "division expression with runtime zero-check should codegen"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("llvm.sadd.with.overflow.i64"),
        "debug overflow path should call LLVM overflow intrinsic: {ir}"
    );
    assert!(
        ir.contains("llvm.trap"),
        "division by zero checks should emit llvm.trap path: {ir}"
    );
}

#[test]
fn test_codegen_unary_and_cast_operations() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "unary_cast");
    let _function = create_codegen_function(&codegen_context, "unary_cast_fn");
    let mut env = CodegenEnv::new(true);

    let unary_result = codegen_expression(
        &codegen_context,
        &mut env,
        &unary(21, UnaryOp::Not, bool_lit(22, false)),
        Some(&CoreType::Boolean),
    );
    assert!(unary_result.is_ok(), "unary boolean not should codegen");

    let cast_expr = cast(23, ident(24, "x"), "float64");
    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(25),
        },
        initializer: Some(int_lit(26, 7)),
        span: test_span(),
        id: test_node_id(27),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "source integer let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("casted"),
            type_annotation: Some(Type::Basic {
                name: String::from("float64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(28),
        },
        initializer: Some(cast_expr),
        span: test_span(),
        id: test_node_id(29),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "int-to-float cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("sitofp") || ir.contains("uitofp"),
        "cast should emit numeric conversion instruction: {ir}"
    );
}

#[test]
fn test_codegen_let_assignment_array_and_access_statements() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "stmt_codegen");
    let _function = create_codegen_function(&codegen_context, "stmt_codegen_fn");
    let mut env = CodegenEnv::new(true);

    let let_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int32"),
                span: test_span(),
            }),
            is_mutable: true,
            span: test_span(),
            id: test_node_id(30),
        },
        initializer: Some(int_lit(31, 5)),
        span: test_span(),
        id: test_node_id(32),
    };
    let let_result = codegen_statement(&codegen_context, &mut env, &let_stmt);
    assert!(let_result.is_ok(), "let statement codegen should succeed");

    let assign_stmt = Stmt::Assignment {
        target: ident(33, "x"),
        value: int_lit(34, 9),
        span: test_span(),
        id: test_node_id(35),
    };
    let assign_result = codegen_statement(&codegen_context, &mut env, &assign_stmt);
    assert!(
        assign_result.is_ok(),
        "assignment statement codegen should succeed"
    );

    let array_expr = Expr::Array {
        elements: vec![int_lit(36, 1), int_lit(37, 2), int_lit(38, 3)],
        span: test_span(),
        id: test_node_id(39),
    };
    let array_result = codegen_expression(
        &codegen_context,
        &mut env,
        &array_expr,
        Some(&CoreType::Array(Box::new(CoreType::Int64))),
    );
    assert!(array_result.is_ok(), "array literal codegen should succeed");

    let array_let = Stmt::Let {
        binding: LetBinding {
            name: String::from("arr"),
            type_annotation: Some(Type::Array {
                element_type: Box::new(Type::Basic {
                    name: String::from("int64"),
                    span: test_span(),
                }),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(40),
        },
        initializer: Some(array_expr),
        span: test_span(),
        id: test_node_id(41),
    };
    let array_let_result = codegen_statement(&codegen_context, &mut env, &array_let);
    assert!(
        array_let_result.is_ok(),
        "array let statement codegen should succeed"
    );

    let access_expr = Expr::Index {
        object: Box::new(ident(42, "arr")),
        index: Box::new(int_lit(43, 1)),
        span: test_span(),
        id: test_node_id(44),
    };
    let access_result = codegen_expression(
        &codegen_context,
        &mut env,
        &access_expr,
        Some(&CoreType::Int64),
    );
    assert!(access_result.is_ok(), "array access codegen should succeed");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("alloca") && ir.contains("store"),
        "let and assignment should lower to alloca/store: {ir}"
    );
    assert!(
        ir.contains("getelementptr"),
        "array literal/access should emit gep instructions: {ir}"
    );
}
