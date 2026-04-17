use crate::codegen::adts::{
    codegen_field_access_expression, codegen_match_expression, instantiate_generic_adt_name,
};
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_expression, codegen_if_statement, codegen_loop_statement, codegen_return_statement,
};
use crate::codegen::expressions::{codegen_expression, CodegenEnv, VariableBinding};
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
        modifiers: vec![],
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
        modifiers: vec![],
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
        ir.contains("snprintf"),
        "interpolation with variable should emit snprintf call in LLVM IR: {ir}"
    );
}

#[test]
fn codegen_string_interpolation_frees_to_string_temporary_arguments() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "interp_with_to_string_temp");
    let _function = create_codegen_function(&codegen_context, "interp_with_to_string_temp_fn");
    let mut env = CodegenEnv::new(true);

    let interpolation_expr = Expr::StringInterpolation {
        parts: vec![
            StringPart::Literal(String::from("value: ")),
            StringPart::Expression(call_expr(
                8_060,
                ident(8_061, "int64_to_string"),
                vec![int_lit(8_062, 42)],
            )),
        ],
        span: test_span(),
        id: test_node_id(8_063),
    };

    let interpolation_result = codegen_expression(
        &codegen_context,
        &mut env,
        &interpolation_expr,
        Some(&CoreType::String),
    );
    assert!(
        interpolation_result.is_ok(),
        "interpolation with int64_to_string temporary should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("@int64_to_string"),
        "interpolation should call int64_to_string for temporary string expression: {ir}"
    );
    assert!(
        ir.contains("call void @free(i8*"),
        "interpolation should free temporary string returned by *_to_string after snprintf: {ir}"
    );
}

#[test]
fn codegen_nested_string_interpolation_frees_inner_temporary_buffer() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "interp_nested_temp");
    let _function = create_codegen_function(&codegen_context, "interp_nested_temp_fn");
    let mut env = CodegenEnv::new(true);

    let nested_expr = Expr::StringInterpolation {
        parts: vec![
            StringPart::Literal(String::from("inner ")),
            StringPart::Expression(int_lit(8_070, 7)),
        ],
        span: test_span(),
        id: test_node_id(8_071),
    };
    let interpolation_expr = Expr::StringInterpolation {
        parts: vec![
            StringPart::Literal(String::from("outer ")),
            StringPart::Expression(nested_expr),
        ],
        span: test_span(),
        id: test_node_id(8_072),
    };

    let interpolation_result = codegen_expression(
        &codegen_context,
        &mut env,
        &interpolation_expr,
        Some(&CoreType::String),
    );
    assert!(
        interpolation_result.is_ok(),
        "nested interpolation should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.matches("call i8* @malloc(i64").count() >= 2,
        "nested interpolation should allocate separate outer and inner buffers: {ir}"
    );
    assert!(
        ir.contains("call void @free(i8*"),
        "outer interpolation should free inner temporary interpolation buffer: {ir}"
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
fn test_codegen_unsigned_int_to_float_cast_uses_uitofp() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "uint_to_float_cast");
    let _function = create_codegen_function(&codegen_context, "uint_to_float_cast_fn");
    let mut env = CodegenEnv::new(true);

    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("u"),
            type_annotation: Some(Type::Basic {
                name: String::from("uint64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_300),
        },
        initializer: Some(int_lit(7_301, 7)),
        span: test_span(),
        id: test_node_id(7_302),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "unsigned integer let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("casted"),
            type_annotation: Some(Type::Basic {
                name: String::from("float64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_303),
        },
        initializer: Some(cast(7_304, ident(7_305, "u"), "float64")),
        span: test_span(),
        id: test_node_id(7_306),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "uint64-to-float64 cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("uitofp"),
        "unsigned int-to-float cast should emit uitofp: {ir}"
    );
}

#[test]
fn test_codegen_narrowing_signed_int_cast_emits_runtime_range_trap() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "narrowing_i64_to_i8_cast");
    let _function = create_codegen_function(&codegen_context, "narrowing_i64_to_i8_cast_fn");
    let mut env = CodegenEnv::new(true);

    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_320),
        },
        initializer: Some(int_lit(7_321, 300)),
        span: test_span(),
        id: test_node_id(7_322),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "source int64 let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("narrowed"),
            type_annotation: Some(Type::Basic {
                name: String::from("int8"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_323),
        },
        initializer: Some(cast(7_324, ident(7_325, "x"), "int8")),
        span: test_span(),
        id: test_node_id(7_326),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "int64-to-int8 cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("call void @opal_runtime_error"),
        "narrowing int cast should emit runtime trap call: {ir}"
    );
    assert!(
        ir.contains("cast out of range: int64 to int8"),
        "narrowing int cast trap should contain source/target message: {ir}"
    );
}

#[test]
fn test_codegen_widening_signed_int_cast_emits_no_range_trap() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "widening_i8_to_i64_cast");
    let _function = create_codegen_function(&codegen_context, "widening_i8_to_i64_cast_fn");
    let mut env = CodegenEnv::new(true);

    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("small"),
            type_annotation: Some(Type::Basic {
                name: String::from("int8"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_330),
        },
        initializer: Some(int_lit(7_331, 7)),
        span: test_span(),
        id: test_node_id(7_332),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "source int8 let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("widened"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_333),
        },
        initializer: Some(cast(7_334, ident(7_335, "small"), "int64")),
        span: test_span(),
        id: test_node_id(7_336),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "int8-to-int64 cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        !ir.contains("cast out of range:"),
        "widening int cast should not emit cast range trap message: {ir}"
    );
}

#[test]
fn test_codegen_same_width_signed_to_unsigned_cast_emits_runtime_range_trap() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "reinterpret_i64_to_u64_cast");
    let _function = create_codegen_function(&codegen_context, "reinterpret_i64_to_u64_cast_fn");
    let mut env = CodegenEnv::new(true);

    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_340),
        },
        initializer: Some(int_lit(7_341, -1)),
        span: test_span(),
        id: test_node_id(7_342),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "source int64 let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("as_unsigned"),
            type_annotation: Some(Type::Basic {
                name: String::from("uint64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_343),
        },
        initializer: Some(cast(7_344, ident(7_345, "x"), "uint64")),
        span: test_span(),
        id: test_node_id(7_346),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "int64-to-uint64 cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("call void @opal_runtime_error"),
        "same-width signed-to-unsigned cast should emit runtime trap call: {ir}"
    );
    assert!(
        ir.contains("cast out of range: int64 to uint64"),
        "same-width signed-to-unsigned cast trap should contain source/target message: {ir}"
    );
}

#[test]
fn test_codegen_same_width_unsigned_to_signed_cast_emits_runtime_range_trap() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "reinterpret_u64_to_i64_cast");
    let _function = create_codegen_function(&codegen_context, "reinterpret_u64_to_i64_cast_fn");
    let mut env = CodegenEnv::new(true);

    let source_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("uint64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_350),
        },
        initializer: Some(int_lit(7_351, 1)),
        span: test_span(),
        id: test_node_id(7_352),
    };
    let source_result = codegen_statement(&codegen_context, &mut env, &source_stmt);
    assert!(source_result.is_ok(), "source uint64 let should codegen");

    let cast_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("as_signed"),
            type_annotation: Some(Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_353),
        },
        initializer: Some(cast(7_354, ident(7_355, "x"), "int64")),
        span: test_span(),
        id: test_node_id(7_356),
    };
    let cast_result = codegen_statement(&codegen_context, &mut env, &cast_stmt);
    assert!(cast_result.is_ok(), "uint64-to-int64 cast should codegen");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("call void @opal_runtime_error"),
        "same-width unsigned-to-signed cast should emit runtime trap call: {ir}"
    );
    assert!(
        ir.contains("cast out of range: uint64 to int64"),
        "same-width unsigned-to-signed cast trap should contain source/target message: {ir}"
    );
}

#[test]
fn test_codegen_assignment_to_immutable_variable_returns_error() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "immutable_assignment");
    let _function = create_codegen_function(&codegen_context, "immutable_assignment_fn");
    let mut env = CodegenEnv::new(true);

    let let_stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("int32"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(7_310),
        },
        initializer: Some(int_lit(7_311, 1)),
        span: test_span(),
        id: test_node_id(7_312),
    };
    let let_result = codegen_statement(&codegen_context, &mut env, &let_stmt);
    assert!(let_result.is_ok(), "immutable let binding should codegen");

    let assign_stmt = Stmt::Assignment {
        target: ident(7_313, "x"),
        value: int_lit(7_314, 2),
        span: test_span(),
        id: test_node_id(7_315),
    };
    let assign_result = codegen_statement(&codegen_context, &mut env, &assign_stmt);
    assert!(
        assign_result.is_err(),
        "assignment to immutable variable should return CodegenError"
    );
    let error_text = assign_result
        .err()
        .map_or_else(String::new, |error| error.to_string());
    assert!(
        error_text.contains("cannot assign to immutable variable: x"),
        "error should clearly describe immutable assignment failure: {error_text}"
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
fn test_entry_function_with_string_array_parameter_emits_callable_main_wrapper() {
    let source = "
entry main = f(args: string[]): void => {
    return void
}
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "entry main(args: string[]) should compile successfully"
    );

    let Ok(module) = module_result else {
        return;
    };

    let verification = module.verify();
    assert!(
        verification.is_ok(),
        "module containing entry args parameter should verify: {verification:?}"
    );

    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("define i32 @main(i32"),
        "codegen should emit C ABI main wrapper with argc param: {ir}"
    );
    assert!(
        ir.contains("call void @__opalescent_entry_main("),
        "main wrapper should call lowered entry function with args: {ir}"
    );
}

#[test]
fn test_entry_main_wrapper_has_argc_argv_params() {
    let source = "
entry main = f(args: string[]): void => {
    return void
}
";
    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "entry main(args: string[]) should compile: {:?}",
        module_result.err()
    );
    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("define i32 @main(i32"),
        "C main wrapper must declare argc (i32) as first param: {ir}"
    );
    assert!(
        ir.contains("i8**") || ir.contains("ptr"),
        "C main wrapper must declare argv (i8** or ptr) as second param: {ir}"
    );
}

#[test]
fn test_import_take_input_emits_take_input_declaration() {
    let source = "
import take_input from standard

entry main = f(): void => {
    let user_input = take_input()
    print(user_input)
    return void
}
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "imported take_input should compile and be callable in subsequent code"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("declare i8* @take_input()"),
        "import take_input from standard should emit declare i8* @take_input(): {ir}"
    );
}

#[test]
fn test_import_random_int32_emits_random_int32_declaration() {
    let source = "
import random_int32 from math

entry main = f(): void => {
    let roll = random_int32(1, 10)
    print('roll: {roll}')
    return void
}
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "imported random_int32 should compile and be callable in subsequent code"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("declare i32 @random_int32(i32, i32)"),
        "import random_int32 from math should emit declare i32 @random_int32(i32, i32): {ir}"
    );
}

#[test]
fn test_import_random_int64_emits_correct_declaration() {
    let source = "
import random_int64 from math

entry main = f(): void => {
    let roll = random_int64(1, 100)
    print('roll: {roll}')
    return void
}
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "imported random_int64 should compile and be callable"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("declare i64 @random_int64(i64, i64)"),
        "import random_int64 from math should emit declare i64 @random_int64(i64, i64): {ir}"
    );
}

#[test]
fn test_import_string_to_int64_emits_correct_declaration() {
    let source = "
import string_to_int64 from standard

entry main = f(): void =>
    guard string_to_int64('42') into n else _e =>
        return void
    print('n: {n}')
    return void
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "imported string_to_int64 should compile and be callable"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("@string_to_int64"),
        "import string_to_int64 from standard should emit string_to_int64 declaration: {ir}"
    );
}

#[test]
fn test_import_standard_multiple_symbols_emit_all_runtime_declarations() {
    let source = "
import take_input, string_to_int32 from standard

entry main = f(): void =>
    let text = take_input()
    guard string_to_int32(text) into value else _e =>
        return void
    print('value: {value}')
    return void
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "multiple imports from the same module should compile and be callable"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("declare i8* @take_input()"),
        "take_input declaration should exist when imported from standard: {ir}"
    );
    assert!(
        ir.contains("@string_to_int32"),
        "string_to_int32 declaration should exist when imported from standard: {ir}"
    );
}

#[test]
fn test_guard_statement_compiles_to_valid_llvm_ir() {
    let source = "
entry main = f(): void =>
    guard string_to_int32('5') into n else e =>
        continue
    print('number: {n}')
    return void
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "guard statement source should compile successfully"
    );

    let Ok(module) = module_result else {
        return;
    };

    let verification = module.verify();
    assert!(
        verification.is_ok(),
        "module containing guard statement should verify: {verification:?}"
    );

    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("extractvalue"),
        "guard statement should emit extractvalue for struct-return parse function: {ir}"
    );
    assert!(
        ir.contains("declare { i32, i8* } @string_to_int32"),
        "guard statement should emit struct-return declaration signature for string_to_int32: {ir}"
    );
}

#[test]
fn test_builtin_calls_emit_runtime_declarations_without_imports() {
    let source = "
entry main = f(): void =>
    let raw = take_input()
    guard string_to_int32(raw) into parsed else _e =>
        return void
    let roll = random_int32(1, 6)
    print('parsed: {parsed}, roll: {roll}')
    return void
";

    let context = Context::create();
    let module_result = compile_to_module(&context, source);
    assert!(
        module_result.is_ok(),
        "builtin calls should compile and emit runtime declarations without import statements"
    );

    let Ok(module) = module_result else {
        return;
    };
    let ir = module.print_to_string().to_string();
    assert!(
        ir.contains("declare i8* @take_input()"),
        "take_input builtin should emit take_input declaration: {ir}"
    );
    assert!(
        ir.contains("@string_to_int32"),
        "string_to_int32 builtin should emit string_to_int32 declaration: {ir}"
    );
    assert!(
        ir.contains("declare i32 @random_int32(i32, i32)"),
        "random_int32 builtin should emit random_int32 declaration: {ir}"
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
        None,
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

    let destructured_loop = Stmt::LetDestructure {
        bindings: vec![
            LetBinding {
                name: String::from("user_input"),
                type_annotation: Some(Type::Basic {
                    name: String::from("int64"),
                    span: test_span(),
                }),
                is_mutable: false,
                span: test_span(),
                id: test_node_id(651),
            },
            LetBinding {
                name: String::from("user_number"),
                type_annotation: Some(Type::Basic {
                    name: String::from("int64"),
                    span: test_span(),
                }),
                is_mutable: false,
                span: test_span(),
                id: test_node_id(652),
            },
        ],
        initializer: Expr::Loop {
            body: Box::new(Stmt::Block {
                statements: vec![Stmt::Break {
                    values: vec![
                        labeled_value(653, "user_input", int_lit(654, 11)),
                        labeled_value(655, "user_number", int_lit(656, 22)),
                    ],
                    span: test_span(),
                    id: test_node_id(657),
                }],
                span: test_span(),
                id: test_node_id(658),
            }),
            span: test_span(),
            id: test_node_id(659),
        },
        span: test_span(),
        id: test_node_id(660),
    };
    let destructure_result = codegen_statement(&codegen_context, &mut env, &destructured_loop);
    assert!(
        destructure_result.is_ok(),
        "let destructure from loop expression should lower with break payload slots"
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
        None,
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

#[test]
fn test_codegen_power_operator_int_computes_correct_value() {
    // Verify that the `^` (power) binary operator lowers to correct LLVM IR for integers.
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "power_int_test");
    let _function = create_codegen_function(&codegen_context, "power_int_fn");
    let mut env = CodegenEnv::new(false);

    // 2 ^ 10 — must produce a value (not an error)
    let expr = binary(9001, int_lit(9002, 2), BinaryOp::Power, int_lit(9003, 10));
    let result = codegen_expression(&codegen_context, &mut env, &expr, Some(&CoreType::Int64));
    assert!(
        result.is_ok(),
        "integer power expression 2^10 should lower successfully, got: {result:?}"
    );
}

#[test]
fn test_codegen_power_operator_float_computes_correct_value() {
    // Verify that the `^` operator lowers to `pow` intrinsic for float types.
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "power_float_test");
    let _function = create_codegen_function(&codegen_context, "power_float_fn");
    let mut env = CodegenEnv::new(false);

    // 2.0 ^ 3.0
    let expr = binary(
        9010,
        float_lit(9011, 2.0),
        BinaryOp::Power,
        float_lit(9012, 3.0),
    );
    let result = codegen_expression(&codegen_context, &mut env, &expr, Some(&CoreType::Float64));
    assert!(
        result.is_ok(),
        "float power expression 2.0^3.0 should lower successfully, got: {result:?}"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("llvm.pow"),
        "float power should emit llvm.pow intrinsic call in LLVM IR: {ir}"
    );
}

#[test]
fn test_codegen_div_euclid_operator() {
    // Verify that `div_euclid` binary operator lowers to correct LLVM IR.
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "div_euclid_test");
    let _function = create_codegen_function(&codegen_context, "div_euclid_fn");
    let mut env = CodegenEnv::new(false);

    // -7 div_euclid 2 should produce floor division (result = -4)
    let expr = binary(
        9020,
        int_lit(9021, -7),
        BinaryOp::DivEuclid,
        int_lit(9022, 2),
    );
    let result = codegen_expression(&codegen_context, &mut env, &expr, Some(&CoreType::Int64));
    assert!(
        result.is_ok(),
        "div_euclid expression should lower successfully, got: {result:?}"
    );
}

#[test]
fn test_codegen_mod_euclid_operator() {
    // Verify that `mod_euclid` binary operator lowers to always-positive remainder LLVM IR.
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "mod_euclid_test");
    let _function = create_codegen_function(&codegen_context, "mod_euclid_fn");
    let mut env = CodegenEnv::new(false);

    // -7 mod_euclid 2 should produce positive remainder (result = 1, since -7 = (-4)*2 + 1)
    let expr = binary(
        9030,
        int_lit(9031, -7),
        BinaryOp::ModEuclid,
        int_lit(9032, 2),
    );
    let result = codegen_expression(&codegen_context, &mut env, &expr, Some(&CoreType::Int64));
    assert!(
        result.is_ok(),
        "mod_euclid expression should lower successfully, got: {result:?}"
    );
}

#[test]
fn test_codegen_unsupported_expression_kind_error_message() {
    // Verify that unsupported expression kinds produce the correct error message (no "task 22").
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "unsupported_expr_test");
    let _function = create_codegen_function(&codegen_context, "unsupported_expr_fn");
    let mut env = CodegenEnv::new(false);

    // Expr::TypeOf is currently unsupported — hits the catch-all arm.
    let expr = Expr::TypeOf {
        expr: Box::new(int_lit(9100, 42)),
        span: test_span(),
        id: test_node_id(9101),
    };
    let result = codegen_expression(&codegen_context, &mut env, &expr, None);
    assert!(result.is_err(), "TypeOf expression should produce an error");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unsupported expression kind"),
        "error should say 'unsupported expression kind', got: {err_msg}"
    );
    assert!(
        !err_msg.contains("task 22"),
        "error must not reference 'task 22', got: {err_msg}"
    );
}

#[test]
fn test_codegen_cast_function_type_error_message() {
    // Verify that casting to a function type produces the correct error (no "task 22").
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "cast_fn_type_test");
    let _function = create_codegen_function(&codegen_context, "cast_fn_type_fn");
    let mut env = CodegenEnv::new(false);

    // Cast to a function type — unsupported as a cast target.
    let expr = Expr::Cast {
        expr: Box::new(int_lit(9110, 1)),
        target_type: Type::Function {
            parameters: Vec::new(),
            return_types: vec![Type::Basic {
                name: String::from("int64"),
                span: test_span(),
            }],
            errors: None,
            span: test_span(),
        },
        span: test_span(),
        id: test_node_id(9111),
    };
    let result = codegen_expression(&codegen_context, &mut env, &expr, None);
    assert!(
        result.is_err(),
        "cast to function type should produce an error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("function and generic types cannot be cast targets"),
        "error should say 'function and generic types cannot be cast targets', got: {err_msg}"
    );
    assert!(
        !err_msg.contains("task 22"),
        "error must not reference 'task 22', got: {err_msg}"
    );
}

#[test]
fn test_codegen_assignment_non_identifier_target_error_message() {
    // Verify that assigning to a non-identifier produces the correct error (no "task 22").
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "assign_non_ident_test");
    let _function = create_codegen_function(&codegen_context, "assign_non_ident_fn");
    let mut env = CodegenEnv::new(false);

    // Assignment where the target is a literal (not an identifier) — must error.
    let stmt = Stmt::Assignment {
        target: int_lit(9120, 99),
        value: int_lit(9121, 1),
        span: test_span(),
        id: test_node_id(9122),
    };
    let result = codegen_statement(&codegen_context, &mut env, &stmt);
    assert!(
        result.is_err(),
        "assignment to non-identifier should produce an error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("assignment target must be an identifier"),
        "error should say 'assignment target must be an identifier', got: {err_msg}"
    );
    assert!(
        !err_msg.contains("task 22"),
        "error must not reference 'task 22', got: {err_msg}"
    );
}

#[test]
fn test_codegen_unsupported_let_type_annotation_error_message() {
    // Verify that a let binding with a function-type annotation produces the correct error.
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "unsupported_let_type_test");
    let _function = create_codegen_function(&codegen_context, "unsupported_let_type_fn");
    let mut env = CodegenEnv::new(false);

    // Let binding with a function-type annotation — not supported for let bindings.
    let stmt = Stmt::Let {
        binding: LetBinding {
            name: String::from("callback"),
            type_annotation: Some(Type::Function {
                parameters: vec![Type::Basic {
                    name: String::from("int64"),
                    span: test_span(),
                }],
                return_types: vec![Type::Basic {
                    name: String::from("int64"),
                    span: test_span(),
                }],
                errors: None,
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(9130),
        },
        initializer: Some(int_lit(9131, 0)),
        span: test_span(),
        id: test_node_id(9132),
    };
    let result = codegen_statement(&codegen_context, &mut env, &stmt);
    assert!(
        result.is_err(),
        "let with function-type annotation should produce an error"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("unsupported type annotation in let binding"),
        "error should say 'unsupported type annotation in let binding', got: {err_msg}"
    );
    assert!(
        !err_msg.contains("task 22"),
        "error must not reference 'task 22', got: {err_msg}"
    );
}

#[test]
fn test_codegen_string_is_comparison_emits_strcmp() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "str_is_cmp");
    let _function = create_codegen_function(&codegen_context, "str_is_cmp_fn");
    let mut env = CodegenEnv::new(true);

    let x_binding = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("string"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(9200),
        },
        initializer: Some(string_lit(9201, "hello")),
        span: test_span(),
        id: test_node_id(9202),
    };
    let binding_result = codegen_statement(&codegen_context, &mut env, &x_binding);
    assert!(
        binding_result.is_ok(),
        "string let binding should lower before is comparison"
    );

    let is_expr = binary(
        9203,
        ident(9204, "x"),
        BinaryOp::Is,
        string_lit(9205, "hello"),
    );
    let result = codegen_expression(
        &codegen_context,
        &mut env,
        &is_expr,
        Some(&CoreType::String),
    );
    assert!(
        result.is_ok(),
        "string is comparison should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("strcmp"),
        "is operator on string should lower to strcmp in LLVM IR: {ir}"
    );
}

#[test]
fn test_codegen_string_is_not_comparison_emits_strcmp() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "str_is_not_cmp");
    let _function = create_codegen_function(&codegen_context, "str_is_not_cmp_fn");
    let mut env = CodegenEnv::new(true);

    let x_binding = Stmt::Let {
        binding: LetBinding {
            name: String::from("x"),
            type_annotation: Some(Type::Basic {
                name: String::from("string"),
                span: test_span(),
            }),
            is_mutable: false,
            span: test_span(),
            id: test_node_id(9210),
        },
        initializer: Some(string_lit(9211, "hello")),
        span: test_span(),
        id: test_node_id(9212),
    };
    let binding_result = codegen_statement(&codegen_context, &mut env, &x_binding);
    assert!(
        binding_result.is_ok(),
        "string let binding should lower before is not comparison"
    );

    let is_not_expr = binary(
        9213,
        ident(9214, "x"),
        BinaryOp::IsNot,
        string_lit(9215, "hello"),
    );
    let result = codegen_expression(
        &codegen_context,
        &mut env,
        &is_not_expr,
        Some(&CoreType::String),
    );
    assert!(
        result.is_ok(),
        "string is not comparison should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("strcmp"),
        "is not operator on string should lower to strcmp in LLVM IR: {ir}"
    );
    assert!(
        ir.contains("icmp ne"),
        "is not operator on string should lower to icmp ne in LLVM IR: {ir}"
    );
}

#[test]
fn test_codegen_function_pointer_is_comparison_emits_icmp() {
    use inkwell::AddressSpace;

    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "fn_ptr_is_cmp");
    let _function = create_codegen_function(&codegen_context, "fn_ptr_is_cmp_fn");
    let mut env = CodegenEnv::new(true);

    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
    let f1_alloca = codegen_context
        .builder
        .build_alloca(i8_ptr_type, "f1")
        .unwrap();
    env.variables.insert(
        String::from("f1"),
        VariableBinding {
            alloca: f1_alloca,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            length: None,
            is_mutable: false,
        },
    );

    let f2_alloca = codegen_context
        .builder
        .build_alloca(i8_ptr_type, "f2")
        .unwrap();
    env.variables.insert(
        String::from("f2"),
        VariableBinding {
            alloca: f2_alloca,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            length: None,
            is_mutable: false,
        },
    );

    let is_expr = binary(9220, ident(9221, "f1"), BinaryOp::Is, ident(9222, "f2"));
    let result = codegen_expression(
        &codegen_context,
        &mut env,
        &is_expr,
        Some(&CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Unit],
            error_types: Vec::new(),
        }),
    );
    assert!(
        result.is_ok(),
        "function pointer is comparison should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("icmp eq"),
        "is operator on function pointer should lower to icmp eq in LLVM IR: {ir}"
    );
    assert!(
        !ir.contains("strcmp"),
        "is operator on function pointer should NOT use strcmp: {ir}"
    );
}

#[test]
fn test_codegen_function_pointer_is_not_comparison_emits_icmp() {
    use inkwell::AddressSpace;

    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "fn_ptr_is_not_cmp");
    let _function = create_codegen_function(&codegen_context, "fn_ptr_is_not_cmp_fn");
    let mut env = CodegenEnv::new(true);

    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
    let f1_alloca = codegen_context
        .builder
        .build_alloca(i8_ptr_type, "f1")
        .unwrap();
    env.variables.insert(
        String::from("f1"),
        VariableBinding {
            alloca: f1_alloca,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            length: None,
            is_mutable: false,
        },
    );

    let f2_alloca = codegen_context
        .builder
        .build_alloca(i8_ptr_type, "f2")
        .unwrap();
    env.variables.insert(
        String::from("f2"),
        VariableBinding {
            alloca: f2_alloca,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
            length: None,
            is_mutable: false,
        },
    );

    let is_not_expr = binary(9230, ident(9231, "f1"), BinaryOp::IsNot, ident(9232, "f2"));
    let result = codegen_expression(
        &codegen_context,
        &mut env,
        &is_not_expr,
        Some(&CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Unit],
            error_types: Vec::new(),
        }),
    );
    assert!(
        result.is_ok(),
        "function pointer is not comparison should lower successfully"
    );

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("icmp ne"),
        "is not operator on function pointer should lower to icmp ne in LLVM IR: {ir}"
    );
    assert!(
        !ir.contains("strcmp"),
        "is not operator on function pointer should NOT use strcmp: {ir}"
    );
}
