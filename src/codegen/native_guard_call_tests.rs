//! Source-level application guards must not consume internal native test aborts.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_function_tests::body;
use super::native_outcome_call_tests::execute_module;
use crate::type_system::types::CoreType;
use inkwell::IntPredicate;
use inkwell::context::Context;
use inkwell::values::{BasicValue, FunctionValue};

#[test]
fn compiled_source_guard_cannot_catch_a_native_test_abort() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "guard_cannot_catch_abort");
    emit_operation(&codegen, true);
    let guarded = emit_guarded_source(&codegen);
    emit_entry(&codegen, guarded, true);
    codegen.module.verify().expect("valid separate channels");
    execute_module(&codegen);
}

#[test]
fn non_fallible_source_calls_propagate_abort_without_a_default_value() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "non_fallible_abort");
    let operation = codegen
        .declare_native_test_function("operation", llvm.i64_type().fn_type(&[], false), None)
        .expect("non-fallible operation");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(operation, "entry"));
    let outcome = codegen.native_function_outcome(operation).expect("outcome");
    let aborted = outcome
        .build_aborted(&codegen, llvm.i64_type().const_int(41, false))
        .expect("abort");
    codegen
        .builder
        .build_return(Some(&aborted))
        .expect("return");
    let caller = emit_source_caller(
        &codegen,
        "let guarded = f(): int64 =>\n    return operation()\n",
        Vec::new(),
    );
    emit_entry(&codegen, caller, true);
    codegen
        .module
        .verify()
        .expect("non-fallible abort has no source error or default value");
    execute_module(&codegen);
}

#[test]
fn compiled_source_guard_can_handle_a_deliberate_application_error() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "guard_handles_application_error");
    emit_operation(&codegen, false);
    let guarded = emit_guarded_source(&codegen);
    emit_entry(&codegen, guarded, false);
    codegen
        .module
        .verify()
        .expect("valid deliberate application error");
    execute_module(&codegen);
}

/// Produce either a test abort or an ordinary completed application error.
fn emit_operation(codegen: &CodegenContext<'_>, abort: bool) {
    let llvm = codegen.context;
    let application = super::error_abi::build_error_return_type(llvm, Some(llvm.i64_type().into()));
    let operation = codegen
        .declare_native_test_function("operation", application.fn_type(&[], false), None)
        .expect("operation");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(operation, "entry"));
    if abort {
        let transport = codegen
            .native_function_outcome(operation)
            .expect("native ABI");
        let aborted = transport
            .build_aborted(codegen, llvm.i64_type().const_int(41, false))
            .expect("abort");
        codegen
            .builder
            .build_return(Some(&aborted))
            .expect("operation abort");
    } else {
        let error = codegen
            .builder
            .build_global_string_ptr("ParseError", "expected_error")
            .expect("error")
            .as_pointer_value();
        let value =
            super::error_abi::build_error_aggregate_for_return_type(codegen, application, error)
                .expect("application error");
        codegen
            .emit_application_return(Some(value.as_basic_value_enum()))
            .expect("completed application error");
    }
}

/// This is an ordinary parsed Opalescent guard, not a Rust guard simulation.
fn emit_guarded_source<'context>(codegen: &CodegenContext<'context>) -> FunctionValue<'context> {
    emit_source_caller(
        codegen,
        "let guarded = f(): int64 =>\n    return guard operation() into value else 7\n",
        vec![CoreType::Generic {
            name: "ParseError".to_owned(),
            type_args: Vec::new(),
        }],
    )
}

/// Lower an ordinary parsed caller with its original application signature.
fn emit_source_caller<'context>(
    codegen: &CodegenContext<'context>,
    source: &str,
    error_types: Vec<CoreType>,
) -> FunctionValue<'context> {
    let llvm = codegen.context;
    let guarded = codegen
        .declare_native_test_function("guarded", llvm.i64_type().fn_type(&[], false), None)
        .expect("caller");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(guarded, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Int64]);
    environment.imported_signatures.insert(
        "operation".to_owned(),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Int64],
            error_types,
        },
    );
    super::statements::codegen_statement(codegen, &mut environment, &body(source))
        .expect("source call lowering");
    guarded
}

/// Assert the selected result using an ordinary host-native entry point.
fn emit_entry<'context>(
    codegen: &CodegenContext<'context>,
    guarded: FunctionValue<'context>,
    expect_abort: bool,
) {
    let llvm = codegen.context;
    let outcome = codegen
        .native_function_outcome(guarded)
        .expect("caller ABI");
    let main = codegen
        .module
        .add_function("main", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(main, "entry"));
    let result = codegen
        .builder
        .build_call(guarded, &[], "guarded_result")
        .expect("native caller")
        .try_as_basic_value()
        .basic()
        .expect("result")
        .into_struct_value();
    let branch = outcome
        .branch_after_call(codegen, result)
        .expect("entry branch");
    if expect_abort {
        codegen
            .builder
            .build_return(Some(&llvm.i32_type().const_int(1, false)))
            .expect("guard swallowed failure");
        codegen.builder.position_at_end(branch.aborted);
        emit_comparison_status(codegen, branch.failure_identity, 41);
    } else {
        let value = outcome
            .completed_application_value(codegen, result)
            .expect("completed edge")
            .expect("value")
            .into_int_value();
        emit_comparison_status(codegen, value, 7);
        codegen.builder.position_at_end(branch.aborted);
        codegen
            .builder
            .build_return(Some(&llvm.i32_type().const_int(1, false)))
            .expect("unexpected test abort");
    }
}

/// Return a native process failure if a result value or identity differs.
fn emit_comparison_status<'context>(
    codegen: &CodegenContext<'context>,
    value: inkwell::values::IntValue<'context>,
    expected: u64,
) {
    let llvm = codegen.context;
    let wrong = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            value,
            llvm.i64_type().const_int(expected, false),
            "wrong_result",
        )
        .expect("comparison");
    let status = codegen
        .builder
        .build_int_z_extend(wrong, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("main return");
}
