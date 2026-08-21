//! Tests for transactional affine aggregate constructor lowering.

extern crate alloc;

use crate::ast::{ConstructorField, Expr, NodeId};
use crate::codegen::adts::codegen_product_constructor;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::token::{Position, Span};
use crate::type_system::types::CoreType;
use alloc::vec;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::context::Context;

fn test_span() -> Span {
    Span::single(Position::start())
}

fn ident(id: usize, name: &str) -> Expr {
    Expr::Identifier {
        name: name.to_owned(),
        span: test_span(),
        id: NodeId(id),
    }
}

fn call(id: usize, name: &str) -> Expr {
    Expr::Call {
        callee: Box::new(ident(id.saturating_add(1), name)),
        generic_args: None,
        args: Vec::new(),
        span: test_span(),
        id: NodeId(id),
    }
}

fn propagate(id: usize, name: &str) -> Expr {
    Expr::Propagate {
        call: Box::new(call(id.saturating_add(1), name)),
        cause: None,
        span: test_span(),
        id: NodeId(id),
    }
}

fn aggregate_fields(first: Expr, second: Expr) -> Vec<ConstructorField> {
    vec![
        ConstructorField {
            name: "first".to_owned(),
            value: first,
            span: test_span(),
        },
        ConstructorField {
            name: "second".to_owned(),
            value: second,
            span: test_span(),
        },
    ]
}

fn member_core_type() -> CoreType {
    CoreType::Generic {
        name: "TerminalAggregateMember".to_owned(),
        type_args: Vec::new(),
    }
}

fn aggregate_core_type() -> CoreType {
    CoreType::Generic {
        name: "TerminalAggregateFixture".to_owned(),
        type_args: Vec::new(),
    }
}

fn seed_aggregate_layout(env: &mut CodegenEnv<'_>) {
    env.adt_field_layouts.insert(
        "TerminalAggregateFixture".to_owned(),
        vec![
            ("first".to_owned(), member_core_type()),
            ("second".to_owned(), member_core_type()),
        ],
    );
}

fn create_pointer_return_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> inkwell::values::FunctionValue<'context> {
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function = codegen_context
        .module
        .add_function(name, i8_ptr.fn_type(&[], false), None);
    let entry = codegen_context
        .context
        .append_basic_block(function, "entry");
    codegen_context.builder.position_at_end(entry);
    function
}

fn create_error_return_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> inkwell::values::FunctionValue<'context> {
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let result_type = codegen_context
        .context
        .struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let function = codegen_context
        .module
        .add_function(name, result_type.fn_type(&[], false), None);
    let entry = codegen_context
        .context
        .append_basic_block(function, "entry");
    codegen_context.builder.position_at_end(entry);
    function
}

fn declare_member_new(codegen_context: &CodegenContext<'_>, name: &str) {
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr.fn_type(&[], false);
    let _function = codegen_context
        .module
        .add_function(name, function_type, None);
}

fn declare_fallible_member_new(codegen_context: &CodegenContext<'_>, name: &str) {
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let result_type = codegen_context
        .context
        .struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let function_type = result_type.fn_type(&[], false);
    let _function = codegen_context
        .module
        .add_function(name, function_type, None);
}

#[test]
fn terminal_aggregate_constructor_path_returns_nominal_payload_pointer() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "terminal_aggregate_constructor_seal");
    let function = create_pointer_return_function(&codegen_context, "aggregate_constructor_seal");
    declare_member_new(&codegen_context, "terminal_aggregate_first_new");
    declare_member_new(&codegen_context, "terminal_aggregate_second_new");
    let mut env = CodegenEnv::new(true);
    seed_aggregate_layout(&mut env);

    let lowered = codegen_product_constructor(
        &codegen_context,
        &mut env,
        aggregate_fields(
            call(100, "terminal_aggregate_first_new"),
            call(200, "terminal_aggregate_second_new"),
        )
        .as_slice(),
        Some(&aggregate_core_type()),
    )
    .expect("aggregate constructor should lower through transactional path");

    assert!(
        lowered.is_pointer_value(),
        "transactional aggregate constructor should return nominal i8* payload"
    );
    let _return = codegen_context
        .builder
        .build_return(Some(&lowered))
        .expect("nominal aggregate payload should be return-compatible with i8*");
    codegen_context
        .module
        .verify()
        .expect("aggregate constructor return should verify as nominal i8*");
    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("aggregate.seal.insert"),
        "constructor path should still use allocation-free aggregate seal before payload store: {ir}"
    );
    assert!(
        ir.contains("aggregate.payload.cast"),
        "transactional aggregate should store the sealed value into a nominal payload: {ir}"
    );
    assert!(
        !ir.contains("product.payload.cast"),
        "transactional aggregate path should use its aggregate payload cast, not ordinary product lowering: {ir}"
    );
    assert!(
        ir.contains("ret i8*"),
        "transactional aggregate result should be return-compatible with nominal generic i8*: {ir}"
    );
    assert_eq!(
        function.count_basic_blocks(),
        1,
        "non-fallible aggregate constructor should not create rollback blocks"
    );
}

#[test]
fn terminal_aggregate_constructor_path_rolls_back_initialized_members() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "terminal_aggregate_constructor_rollback");
    let _function =
        create_error_return_function(&codegen_context, "aggregate_constructor_rollback");
    declare_member_new(&codegen_context, "terminal_aggregate_first_new");
    declare_fallible_member_new(&codegen_context, "terminal_aggregate_second_try_new");
    let mut env = CodegenEnv::new(true);
    seed_aggregate_layout(&mut env);

    let _lowered = codegen_product_constructor(
        &codegen_context,
        &mut env,
        aggregate_fields(
            call(300, "terminal_aggregate_first_new"),
            propagate(400, "terminal_aggregate_second_try_new"),
        )
        .as_slice(),
        Some(&aggregate_core_type()),
    )
    .expect("aggregate constructor should lower rollback path");

    let ir = codegen_context.module.print_to_string().to_string();
    assert!(
        ir.contains("aggregate.constructor.rollback"),
        "fallible aggregate field should create rollback edge: {ir}"
    );
    assert!(
        ir.contains("call void @__opal_aggregate_cleanup_terminal_aggregate_member_drop"),
        "rollback edge should clean initialized provisional members: {ir}"
    );
    assert!(
        ir.contains("cleanup.body"),
        "rollback should return the original field acquisition error: {ir}"
    );
}
