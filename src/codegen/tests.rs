use crate::codegen::adts::{
    codegen_field_access_expression, codegen_match_expression, instantiate_generic_adt_name,
};
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_expression, codegen_if_statement, codegen_loop_statement, codegen_return_statement,
};
use crate::codegen::expressions::{codegen_expression, CodegenEnv};
use crate::codegen::functions::{
    codegen_call_expression, codegen_function_declaration, codegen_guard_expression,
    codegen_propagate_expression,
};
use crate::codegen::monomorphization::monomorphized_function_name;
use crate::codegen::statements::codegen_statement;
use crate::codegen::types::core_type_to_llvm;
use crate::compiler::compile_to_module;
use crate::type_system::types::{CoreType, GenericTypeParameter, TypeVar};
use crate::{
    ast::{
        BinaryOp, Decl, Expr, HotReloadMetadata, LabeledValue, LambdaBody, LetBinding,
        LiteralValue, NodeId, Parameter, Stmt, StringPart, Type, UnaryOp, Visibility,
    },
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

fn call_expr(id: usize, callee: Expr, args: Vec<Expr>) -> Expr {
    Expr::Call {
        callee: Box::new(callee),
        generic_args: None,
        args,
        span: test_span(),
        id: test_node_id(id),
    }
}

fn return_stmt(id: usize, values: Vec<LabeledValue>) -> Stmt {
    Stmt::Return {
        values,
        span: test_span(),
        id: test_node_id(id),
    }
}

fn labeled_value(id: usize, label: &str, value: Expr) -> LabeledValue {
    LabeledValue {
        label: label.to_owned(),
        value,
        span: test_span(),
        id: test_node_id(id),
    }
}

fn simple_void_function_decl(id: usize, name: &str, body: Stmt, is_entry: bool) -> Decl {
    Decl::Function {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: Vec::new(),
        return_types: Some(vec![Type::Basic {
            name: String::from("void"),
            span: test_span(),
        }]),
        error_types: Vec::new(),
        body,
        visibility: Visibility::Public,
        is_entry,
        doc_comment: None,
        span: test_span(),
        id: test_node_id(id),
        metadata: HotReloadMetadata::default(),
    }
}

fn simple_i64_function_decl(id: usize, name: &str, param: &str, body: Stmt) -> Decl {
    Decl::Function {
        name: name.to_owned(),
        generic_params: None,
        generic_constraints: None,
        parameters: vec![Parameter {
            name: param.to_owned(),
            param_type: Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            },
            span: test_span(),
        }],
        return_types: Some(vec![Type::Basic {
            name: String::from("int64"),
            span: test_span(),
        }]),
        error_types: Vec::new(),
        body,
        visibility: Visibility::Public,
        is_entry: false,
        doc_comment: None,
        span: test_span(),
        id: test_node_id(id),
        metadata: HotReloadMetadata::default(),
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
fn codegen_string_interpolation_pure_literal() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "interp_pure_literal");
    let _function = create_codegen_function(&codegen_context, "interp_pure_literal_fn");
    let mut env = CodegenEnv::new(true);

    let interpolation_expr = Expr::StringInterpolation {
        parts: vec![StringPart::Literal(String::from("Hello world"))],
        span: test_span(),
        id: test_node_id(800),
    };

    let interpolation_result = codegen_expression(
        &codegen_context,
        &mut env,
        &interpolation_expr,
        Some(&CoreType::String),
    );
    assert!(
        interpolation_result.is_ok(),
        "pure literal interpolation should lower successfully"
    );

    let Ok(lowered_value) = interpolation_result else {
        return;
    };
    assert!(
        lowered_value.is_pointer_value(),
        "pure literal interpolation should lower to i8*"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("Hello world") || ir.contains("c\"Hello world\\00\""),
        "pure literal interpolation should materialize global constant text: {ir}"
    );
}

#[test]
fn codegen_string_interpolation_with_variable() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "interp_with_variable");
    let _function = create_codegen_function(&codegen_context, "interp_with_variable_fn");
    let mut env = CodegenEnv::new(true);

    let name_binding = Stmt::Let {
        binding: LetBinding {
            name: String::from("name"),
            type_annotation: Some(Type::Basic {
                name: String::from("string"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(801),
        },
        initializer: Some(string_lit(802, "world")),
        span: test_span(),
        id: test_node_id(803),
    };
    let binding_result = codegen_statement(&codegen_context, &mut env, &name_binding);
    assert!(
        binding_result.is_ok(),
        "string variable binding should lower before interpolation"
    );

    let interpolation_expr = Expr::StringInterpolation {
        parts: vec![
            StringPart::Literal(String::from("Hello ")),
            StringPart::Expression(ident(804, "name")),
        ],
        span: test_span(),
        id: test_node_id(805),
    };

    let interpolation_result = codegen_expression(
        &codegen_context,
        &mut env,
        &interpolation_expr,
        Some(&CoreType::String),
    );
    assert!(
        interpolation_result.is_ok(),
        "interpolation with variable should lower successfully"
    );

    let Ok(lowered_value) = interpolation_result else {
        return;
    };
    assert!(
        lowered_value.is_pointer_value(),
        "interpolation result should lower to i8* for puts"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("sprintf"),
        "interpolation with variable should emit sprintf call in LLVM IR: {ir}"
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
fn test_codegen_is_operator_on_int64_emits_icmp_eq() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "is_cmp_int64");
    let _function = create_codegen_function(&codegen_context, "is_cmp_int64_fn");
    let mut env = CodegenEnv::new(true);

    let x_binding = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(8001),
        },
        initializer: Some(int_lit(8002, 5)),
        span: test_span(),
        id: test_node_id(8003),
    };
    let binding_result = codegen_statement(&codegen_context, &mut env, &x_binding);
    assert!(
        binding_result.is_ok(),
        "int64 let binding should lower before is comparison"
    );

    let is_expr = binary(8004, ident(8005, "x"), BinaryOp::Is, int_lit(8006, 5));
    let result = codegen_expression(&codegen_context, &mut env, &is_expr, Some(&CoreType::Int64));
    assert!(
        result.is_ok(),
        "int64 is comparison should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("icmp eq i64"),
        "is operator on int64 should lower to icmp eq i64 in LLVM IR: {ir}"
    );
}

#[test]
fn test_fibonacci_if_n_is_zero_compiles_to_valid_llvm_ir() {
    let source = "
public fib_recursive = f(n: int64): int64 =>
    if n is 0 { return 0 }
    if n is 1 { return 1 }
    return fib_recursive(n - 1) + fib_recursive(n - 2)

entry main = f(): void =>
    let n: int64 = 10
    let _result: int64 = fib_recursive(n)
    return void
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "fib recursion source using 'if n is 0' should compile to LLVM module"
    );

    let Ok(module) = module_result else {
        return;
    };

    let verification = module.verify();
    assert!(
        verification.is_ok(),
        "generated LLVM module should verify for fib recursion with 'is' guard: {verification:?}"
    );

    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("icmp eq i64"),
        "fib recursion source should emit integer equality compare for 'n is 0': {ir}"
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

#[test]
fn test_codegen_function_declaration_lowers_function_definition() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "fn_decl");
    let mut env = CodegenEnv::new(true);
    let body = Stmt::Block {
        statements: vec![return_stmt(
            601,
            vec![labeled_value(602, "", void_lit(603))],
        )],
        span: test_span(),
        id: test_node_id(604),
    };
    let decl = simple_void_function_decl(605, "main", body, true);

    let result = codegen_function_declaration(&codegen_context, &mut env, &decl);
    assert!(
        result.is_ok(),
        "function declaration codegen should succeed for simple entry function"
    );
}

#[test]
fn test_codegen_call_expression_lowers_function_call() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "fn_call");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let callee_decl = simple_i64_function_decl(
        610,
        "inc",
        "value",
        return_stmt(611, vec![labeled_value(612, "", ident(613, "value"))]),
    );
    let decl_result = codegen_function_declaration(&codegen_context, &mut env, &callee_decl);
    assert!(
        decl_result.is_ok(),
        "callee declaration codegen should succeed"
    );

    let result = codegen_call_expression(
        &codegen_context,
        &mut env,
        &ident(614, "inc"),
        None,
        &[int_lit(615, 41)],
    );
    assert!(
        result.is_ok(),
        "call expression codegen should succeed for known callee"
    );
}

#[test]
fn test_codegen_propagate_and_guard_expressions_lower_error_flow() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "guard_propagate");
    let _host = create_codegen_function(&codegen_context, "host");
    let fallible_type = codegen_context
        .context
        .i64_type()
        .fn_type(&[codegen_context.context.i64_type().into()], false);
    let _fallible = codegen_context
        .module
        .add_function("fallible", fallible_type, None);
    let mut env = CodegenEnv::new(true);

    let guard_result = codegen_guard_expression(
        &codegen_context,
        &mut env,
        &call_expr(620, ident(621, "fallible"), vec![int_lit(622, 1)]),
        "ok_value",
    );
    assert!(
        guard_result.is_ok(),
        "guard codegen should lower to branch-based error handling"
    );

    let propagate_result = codegen_propagate_expression(
        &codegen_context,
        &mut env,
        &call_expr(623, ident(624, "fallible"), vec![int_lit(625, 2)]),
    );
    assert!(
        propagate_result.is_ok(),
        "propagate codegen should lower to early-return error path"
    );
}

#[test]
fn test_codegen_if_statement_and_if_expression_lower_control_flow() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "if_codegen");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let if_stmt_result = codegen_if_statement(
        &codegen_context,
        &mut env,
        &bool_lit(630, true),
        &Stmt::Expression {
            expr: int_lit(631, 1),
            span: test_span(),
            id: test_node_id(632),
        },
        Some(&Stmt::Expression {
            expr: int_lit(633, 2),
            span: test_span(),
            id: test_node_id(634),
        }),
    );
    assert!(
        if_stmt_result.is_ok(),
        "if statement codegen should emit conditional branches"
    );

    let if_expr_result = codegen_if_expression(
        &codegen_context,
        &mut env,
        &bool_lit(635, true),
        &Stmt::Expression {
            expr: int_lit(636, 10),
            span: test_span(),
            id: test_node_id(637),
        },
        Some(&Stmt::Expression {
            expr: int_lit(638, 20),
            span: test_span(),
            id: test_node_id(639),
        }),
    );
    assert!(
        if_expr_result.is_ok(),
        "if expression codegen should emit phi-backed merged value"
    );
}

#[test]
fn test_codegen_loop_forms_and_return_multi_value() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "loop_return");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let while_stmt = Stmt::While {
        condition: bool_lit(640, true),
        body: Box::new(Stmt::Block {
            statements: vec![Stmt::Break {
                values: Vec::new(),
                span: test_span(),
                id: test_node_id(641),
            }],
            span: test_span(),
            id: test_node_id(642),
        }),
        span: test_span(),
        id: test_node_id(643),
    };

    let while_result = codegen_loop_statement(&codegen_context, &mut env, &while_stmt);
    assert!(
        while_result.is_ok(),
        "while loop codegen should emit condition and back-edge blocks"
    );

    let loop_stmt = Stmt::Loop {
        body: Box::new(Stmt::Block {
            statements: vec![Stmt::Continue {
                values: Vec::new(),
                span: test_span(),
                id: test_node_id(644),
            }],
            span: test_span(),
            id: test_node_id(645),
        }),
        span: test_span(),
        id: test_node_id(646),
    };

    let loop_result = codegen_loop_statement(&codegen_context, &mut env, &loop_stmt);
    assert!(
        loop_result.is_ok(),
        "loop codegen should emit unconditional back-edge and break target"
    );

    let return_result = codegen_return_statement(
        &codegen_context,
        &mut env,
        &[
            labeled_value(647, "lhs", int_lit(648, 1)),
            labeled_value(649, "rhs", int_lit(650, 2)),
        ],
    );
    assert!(
        return_result.is_ok(),
        "return statement codegen should support aggregate multi-value return"
    );
}

#[test]
fn test_codegen_lambda_closure_as_function_value() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "lambda_codegen");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let lambda_expr = Expr::Lambda {
        generic_params: None,
        generic_constraints: None,
        params: vec![Parameter {
            name: String::from("value"),
            param_type: Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            },
            span: test_span(),
        }],
        return_types: vec![Type::Basic {
            name: String::from("int64"),
            span: test_span(),
        }],
        error_types: Vec::new(),
        body: LambdaBody::Expression(Box::new(ident(661, "value"))),
        captured_variables: vec![String::from("capture")],
        metadata: Box::new(HotReloadMetadata::default()),
        span: test_span(),
        id: test_node_id(662),
    };

    let call_result = codegen_call_expression(
        &codegen_context,
        &mut env,
        &lambda_expr,
        None,
        &[int_lit(663, 3)],
    );
    assert!(
        call_result.is_ok(),
        "lambda/closure codegen should lower captured lambda as callable function value"
    );
}

#[test]
fn test_codegen_adt_constructor_emits_tagged_union_layout_for_sum_variant() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "adt_constructor_sum");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let constructor_expr = Expr::Constructor {
        callee: Box::new(Expr::Member {
            object: Box::new(ident(700, "Result")),
            member: String::from("Ok"),
            span: test_span(),
            id: test_node_id(701),
        }),
        fields: vec![crate::ast::ConstructorField {
            name: String::from("value"),
            value: int_lit(702, 42),
            span: test_span(),
        }],
        span: test_span(),
        id: test_node_id(703),
    };

    let result = codegen_expression(&codegen_context, &mut env, &constructor_expr, None);
    assert!(result.is_ok(), "sum constructor codegen should succeed");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("{ i64, [64 x i8] }") || ir.contains("{i64, [64 x i8]}"),
        "sum constructor should lower to tagged-union style struct layout: {ir}"
    );
}

#[test]
fn test_codegen_match_expression_lowers_to_switch() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "match_lowering");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let match_expr = Expr::Match {
        scrutinee: Box::new(int_lit(710, 2)),
        arms: vec![
            crate::ast::MatchArm {
                pattern: crate::ast::Pattern::Literal {
                    value: LiteralValue::Integer(1),
                    span: test_span(),
                },
                guard: None,
                body: int_lit(711, 10),
                span: test_span(),
            },
            crate::ast::MatchArm {
                pattern: crate::ast::Pattern::Wildcard { span: test_span() },
                guard: None,
                body: int_lit(712, 20),
                span: test_span(),
            },
        ],
        span: test_span(),
        id: test_node_id(713),
    };

    let lowered = codegen_match_expression(&codegen_context, &mut env, &match_expr);
    assert!(lowered.is_ok(), "match lowering should succeed");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("switch i64"),
        "match lowering should emit switch-based decision tree: {ir}"
    );
}

#[test]
fn test_codegen_monomorphization_name_generation_and_generic_adt_instantiation() {
    let mono_name = monomorphized_function_name(
        "identity",
        &[
            CoreType::Int64,
            CoreType::Generic {
                name: String::from("Result"),
                type_args: vec![CoreType::Int32, CoreType::String],
            },
        ],
    );
    assert_eq!(
        mono_name, "identity__int64__Result_int32_string",
        "generic function calls should dispatch using deterministic monomorphized symbol names"
    );

    let instantiated = instantiate_generic_adt_name("Pair", &[CoreType::Int32, CoreType::Boolean]);
    assert_eq!(
        instantiated, "Pair__int32__boolean",
        "generic ADT instantiation should produce deterministic concrete type name"
    );
}

#[test]
fn test_codegen_product_field_access_loads_named_field() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "product_field_access");
    let _host = create_codegen_function(&codegen_context, "host");
    let mut env = CodegenEnv::new(true);

    let point_constructor = Expr::Constructor {
        callee: Box::new(ident(720, "Point")),
        fields: vec![
            crate::ast::ConstructorField {
                name: String::from("x"),
                value: int_lit(721, 5),
                span: test_span(),
            },
            crate::ast::ConstructorField {
                name: String::from("y"),
                value: int_lit(722, 6),
                span: test_span(),
            },
        ],
        span: test_span(),
        id: test_node_id(723),
    };

    let point_decl = Stmt::Let {
        binding: LetBinding {
            name: String::from("point"),
            type_annotation: None,
            is_mutable: false,
            span: test_span(),
            id: test_node_id(724),
        },
        initializer: Some(point_constructor),
        span: test_span(),
        id: test_node_id(725),
    };
    let decl_result = codegen_statement(&codegen_context, &mut env, &point_decl);
    assert!(
        decl_result.is_ok(),
        "product constructor let should codegen"
    );

    let field_expr = Expr::Member {
        object: Box::new(ident(726, "point")),
        member: String::from("y"),
        span: test_span(),
        id: test_node_id(727),
    };
    let field_result = codegen_field_access_expression(&codegen_context, &mut env, &field_expr);
    assert!(
        field_result.is_ok(),
        "field access on product should codegen"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("getelementptr"),
        "field access should emit gep into product struct: {ir}"
    );
}
