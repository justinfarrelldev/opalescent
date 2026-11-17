//! Existing lexical cleanup remains secondary during native abort propagation.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::{CodegenEnv, VariableBinding};
use super::native_function_tests::body;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::type_system::types::CoreType;
use inkwell::context::Context;
use inkwell::values::FunctionValue;
use inkwell::{AddressSpace, IntPredicate};

#[test]
fn native_abort_runs_declared_lexical_cleanup_and_retains_primary_failure() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_cleanup_error");
    emit_failure(&codegen);
    let caller = emit_caller_with_existing_scope(&codegen);
    emit_bridge(&codegen, caller);
    codegen
        .module
        .verify()
        .expect("valid lexical abort scope exits");
    let directory = tempfile::tempdir().expect("fixture directory");
    let driver = directory.path().join("driver.c");
    std::fs::write(
        &driver,
        r#"
#include "opal_testing_runtime.h"
static int cleanup_count;
const char *__opal_using_cleanup_fixture(void *resource) {
    (void)resource;
    ++cleanup_count;
    return "CleanupError";
}
int run_fixture(void);
int main(void) {
    if (!opal_test_attempt_begin("lexical cleanup", 1)) return 1;
    opal_test_set_phase(OPAL_TEST_PHASE_BODY);
    if (run_fixture()) return 2;
    if (cleanup_count != 1 || opal_test_failure_count() != 2) return 3;
    const OpalTestFailure *first = opal_test_failure_at(0);
    const OpalTestFailure *second = opal_test_failure_at(1);
    if (first->kind != OPAL_TEST_FAILURE_ASSERTION) return 4;
    if (second->kind != OPAL_TEST_FAILURE_APPLICATION_ERROR) return 5;
    if (second->primary_failure_id != first->id || second->parent_failure_id != first->id) return 6;
    opal_test_attempt_end();
    return 0;
}
"#,
    )
    .expect("C fixture");
    execute_module_with_sources(
        &codegen,
        &[
            driver,
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/opal_testing_runtime.c"),
        ],
    );
}

/// A compiled source callee records a real runtime failure identity.
fn emit_failure(codegen: &CodegenContext<'_>) {
    let llvm = codegen.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let record_type = llvm.i64_type().fn_type(
        &[
            llvm.i32_type().into(),
            pointer.into(),
            pointer.into(),
            llvm.i64_type().into(),
            llvm.i64_type().into(),
        ],
        false,
    );
    let record = codegen
        .module
        .add_function("opal_test_record_failure", record_type, None);
    let operation = codegen
        .declare_native_test_function("operation", llvm.i64_type().fn_type(&[], false), None)
        .expect("operation");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(operation, "entry"));
    let message = codegen
        .builder
        .build_global_string_ptr("primary assertion", "message")
        .expect("message");
    let file = codegen
        .builder
        .build_global_string_ptr("fixture.test.op", "file")
        .expect("file");
    let identity = codegen
        .builder
        .build_call(
            record,
            &[
                llvm.i32_type().const_int(1, false).into(),
                message.as_pointer_value().into(),
                file.as_pointer_value().into(),
                llvm.i64_type().const_zero().into(),
                llvm.i64_type().const_int(1, false).into(),
            ],
            "failure",
        )
        .expect("record")
        .try_as_basic_value()
        .basic()
        .expect("identity")
        .into_int_value();
    let aborted = codegen
        .native_function_outcome(operation)
        .expect("outcome")
        .build_aborted(codegen, identity)
        .expect("abort");
    codegen
        .builder
        .build_return(Some(&aborted))
        .expect("operation return");
}

/// Reuse the existing compiler using-obligation model, not a new resource ledger.
fn emit_caller_with_existing_scope<'context>(
    codegen: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let llvm = codegen.context;
    let caller = codegen
        .declare_native_test_function("caller", llvm.i64_type().fn_type(&[], false), None)
        .expect("caller");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(caller, "entry"));
    let token = codegen
        .builder
        .build_global_string_ptr("opaque fixture token", "resource")
        .expect("resource")
        .as_pointer_value();
    let slot = codegen
        .builder
        .build_alloca(token.get_type(), "resource.slot")
        .expect("slot");
    codegen
        .builder
        .build_store(slot, token)
        .expect("resource slot");
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
            error_types: Vec::new(),
        },
    );
    let _scope = environment.enter_scope();
    environment.variables.insert(
        "resource".to_owned(),
        VariableBinding {
            alloca: slot,
            core_type: CoreType::Generic {
                name: "TerminalSession".to_owned(),
                type_args: Vec::new(),
            },
            length: None,
            capacity: None,
            is_mutable: true,
        },
    );
    environment.register_scope_binding("resource");
    environment.register_using_cleanup_obligation("resource", "fixture", &["CleanupError"], None);
    super::statements::codegen_statement(
        codegen,
        &mut environment,
        &body("let caller = f(): int64 =>\n    return operation()\n"),
    )
    .expect("ordinary source call");
    caller
}

/// A C-compatible bridge inspects only the internal outcome, not its payload.
fn emit_bridge<'context>(codegen: &CodegenContext<'context>, caller: FunctionValue<'context>) {
    let llvm = codegen.context;
    let bridge =
        codegen
            .module
            .add_function("run_fixture", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(bridge, "entry"));
    let result = codegen
        .builder
        .build_call(caller, &[], "caller.result")
        .expect("call")
        .try_as_basic_value()
        .basic()
        .expect("outcome")
        .into_struct_value();
    let branches = codegen
        .native_function_outcome(caller)
        .expect("native ABI")
        .branch_after_call(codegen, result)
        .expect("branch");
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(1, false)))
        .expect("unexpected completion");
    codegen.builder.position_at_end(branches.aborted);
    let primary = codegen.module.add_function(
        "opal_test_primary_failure_id",
        llvm.i64_type().fn_type(&[], false),
        None,
    );
    let identity = codegen
        .builder
        .build_call(primary, &[], "primary")
        .expect("primary")
        .try_as_basic_value()
        .basic()
        .expect("identity")
        .into_int_value();
    let wrong = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            identity,
            branches.failure_identity,
            "wrong",
        )
        .expect("identity equality");
    let status = codegen
        .builder
        .build_int_z_extend(wrong, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("bridge return");
}
