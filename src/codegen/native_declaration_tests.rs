//! Checked source function definitions use the private native ABI end to end.
#![cfg(target_os = "linux")]
extern crate alloc;
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::ast::{Decl, Expr, LambdaBody, Parameter, Stmt};
use crate::type_system::types::CoreType;
use inkwell::IntPredicate;
use inkwell::context::Context;

#[test]
fn compiled_native_reference_parameters_update_the_callers_original_binding() {
    exercise(
        "let caller = f(): int64 =>\n    let mutable original: int64 = 41\n    increment(mutable ref original)\n    return original\n",
    );
}

#[test]
fn native_import_alias_can_call_a_forward_declared_source_function() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "forward_native_source");
    let (parameters, body) = function(
        "let helper = f(mutable ref value: int64): void =>\n    value = value + 1\n    return void\n",
    );
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int64],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    let header = super::native_declarations::declare_source_function(
        &codegen,
        "__opal_source_helper",
        &parameters,
        &signature,
    )
    .expect("native source header");
    let (caller_parameters, caller_body) = function(
        "let caller = f(mutable ref original: int64): void =>\n    step(mutable ref original)\n    return void\n",
    );
    let mut environment = CodegenEnv::new(false);
    environment
        .imported_functions
        .insert("step".to_owned(), "__opal_source_helper".to_owned());
    environment
        .imported_signatures
        .insert("step".to_owned(), signature.clone());
    let caller = super::native_declarations::emit_source_function(
        &codegen,
        &mut environment,
        "caller",
        &caller_parameters,
        &signature,
        &caller_body,
    )
    .expect("forward aliased call");
    let emitted = super::native_declarations::emit_source_function(
        &codegen,
        &mut environment,
        "__opal_source_helper",
        &parameters,
        &signature,
        &body,
    )
    .expect("deferred source body");
    assert_eq!(header, emitted);
    emit_reference_driver(&codegen, caller);
    codegen
        .module
        .verify()
        .expect("forward call has the native reference ABI");
    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("call { i1, i64 } @__opal_source_helper(i64*"));
    execute_module_with_sources(
        &codegen,
        &[std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/opal_error.c")],
    );
}

/// Execute a forward source reference call and validate the original scalar.
fn emit_reference_driver<'context>(
    codegen: &CodegenContext<'context>,
    caller: inkwell::values::FunctionValue<'context>,
) {
    let llvm = codegen.context;
    let main = codegen
        .module
        .add_function("main", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(main, "entry"));
    let original = codegen
        .builder
        .build_alloca(llvm.i64_type(), "original")
        .expect("original slot");
    codegen
        .builder
        .build_store(original, llvm.i64_type().const_int(41, false))
        .expect("initialize");
    let result = codegen
        .builder
        .build_call(caller, &[original.into()], "forward.outcome")
        .expect("forward call")
        .try_as_basic_value()
        .basic()
        .expect("native outcome")
        .into_struct_value();
    let branches = codegen
        .native_function_outcome(caller)
        .expect("source ABI")
        .branch_after_call(codegen, result)
        .expect("native branch");
    let actual = codegen
        .builder
        .build_load(original, "updated.original")
        .expect("original storage")
        .into_int_value();
    let failed = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            actual,
            llvm.i64_type().const_int(42, false),
            "wrong.value",
        )
        .expect("value comparison");
    let status = codegen
        .builder
        .build_int_z_extend(failed, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("completed");
    codegen.builder.position_at_end(branches.aborted);
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(2, false)))
        .expect("aborted");
}

#[test]
fn projected_native_borrows_alias_original_product_field_storage() {
    exercise(
        "let caller = f(): int64 =>\n    let mutable holder: Holder = new Holder:\n        value: 41\n    increment(mutable ref holder.value)\n    return holder.value\n",
    );
}

/// Compile the same reference helper against identifier and field lvalues.
fn exercise(caller_source: &str) {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_reference_definition");
    let (increment_parameters, increment_body) = function(
        "let increment = f(mutable ref value: int64): void =>\n    value = value + 1\n    return void\n",
    );
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int64],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    super::native_declarations::emit_source_function(
        &codegen,
        &mut CodegenEnv::new(false),
        "increment",
        &increment_parameters,
        &signature,
        &increment_body,
    )
    .expect("native mutable-reference function");
    let (parameters, statements) = function(caller_source);
    let caller_signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    let mut environment = CodegenEnv::new(false);
    environment.adt_field_layouts.insert(
        "Holder".to_owned(),
        vec![("value".to_owned(), CoreType::Int64)],
    );
    environment.adt_field_indices.insert(
        "Holder".to_owned(),
        alloc::collections::BTreeMap::from([("value".to_owned(), 0)]),
    );
    environment
        .imported_signatures
        .insert("increment".to_owned(), signature);
    let caller = super::native_declarations::emit_source_function(
        &codegen,
        &mut environment,
        "caller",
        &parameters,
        &caller_signature,
        &statements,
    )
    .expect("native reference call");
    let main = codegen
        .module
        .add_function("main", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(main, "entry"));
    let value = codegen
        .builder
        .build_call(caller, &[], "caller.outcome")
        .expect("call")
        .try_as_basic_value()
        .basic()
        .expect("outcome")
        .into_struct_value();
    let outcome = codegen.native_function_outcome(caller).expect("metadata");
    let branches = outcome.branch_after_call(&codegen, value).expect("branch");
    let actual = outcome
        .completed_application_value(&codegen, value)
        .expect("completed")
        .expect("value")
        .into_int_value();
    let wrong = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            actual,
            llvm.i64_type().const_int(42, false),
            "wrong",
        )
        .expect("comparison");
    let status = codegen
        .builder
        .build_int_z_extend(wrong, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("completed status");
    codegen.builder.position_at_end(branches.aborted);
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(1, false)))
        .expect("unexpected abort");
    codegen.module.verify().expect("valid native reference ABI");
    execute_module_with_sources(
        &codegen,
        &[
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/opal_error.c"),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/opal_rc.c"),
        ],
    );
}

/// Read ordinary AST function parameters and body without rewriting source.
pub(super) fn function(source: &str) -> (Vec<Parameter>, Stmt) {
    let (tokens, lex_errors) = crate::lexer::Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse_test_module();
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    program
        .expect("source")
        .items
        .into_iter()
        .filter_map(|item| match item {
            crate::ast::testing::TestItem::Application(declaration) => Some(*declaration),
            _ => None,
        })
        .find_map(|declaration| match declaration {
            Decl::Let {
                initializer:
                    Expr::Lambda {
                        params,
                        body: LambdaBody::Block(statements),
                        span,
                        id,
                        ..
                    },
                ..
            } => Some((
                params,
                Stmt::Block {
                    statements,
                    span,
                    id,
                },
            )),
            _ => None,
        })
        .expect("function")
}
