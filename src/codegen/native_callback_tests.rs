//! Compiled Opalescent callbacks execute through the native lifecycle ABI.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_assertion_tests::callback_statements;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::type_system::types::CoreType;
use inkwell::context::Context;

#[test]
fn source_assertion_failure_crosses_compiled_wrapper_and_runs_lifecycle_after_hook() {
    exercise(
        "scenario lifecycle:\n    it should fail natively: f(): void =>\n        expect false to be true\n        return void\n",
        false,
    );
}

#[test]
fn ordinary_application_error_crosses_wrapper_as_a_distinct_completed_result() {
    exercise(
        "scenario lifecycle:\n    it should return an application error: f(): void errors CallbackError =>\n        propagate error_from_application()\n        return void\n",
        true,
    );
}

/// Compile the actual source callback and connect its wrapper to the runtime.
fn exercise(source: &str, application_errors: bool) {
    let statements = callback_statements(source);
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_callback_boundary");
    let original = if application_errors {
        super::error_abi::build_error_return_type(&llvm, None).fn_type(&[], false)
    } else {
        llvm.void_type().fn_type(&[], false)
    };
    codegen.module.add_function(
        "error_from_application",
        super::error_abi::build_error_return_type(&llvm, None).fn_type(&[], false),
        None,
    );
    let callback = codegen
        .declare_native_test_function("source_body", original, None)
        .expect("source callback");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(callback, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment.current_source_path = "bridge.test.op".to_owned();
    environment.current_source_text = source.to_owned();
    environment
        .current_function_return_types
        .push(vec![CoreType::Unit]);
    environment.imported_signatures.insert(
        "error_from_application".to_owned(),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Unit],
            error_types: vec![CoreType::Generic {
                name: "CallbackError".to_owned(),
                type_args: Vec::new(),
            }],
        },
    );
    for statement in &statements {
        super::statements::codegen_statement(&codegen, &mut environment, statement)
            .expect("source lowering");
    }
    super::native_callbacks::emit_void_callback_wrapper(
        &codegen,
        "compiled_body_wrapper",
        callback,
        application_errors,
    )
    .expect("compiled callback wrapper");
    codegen
        .module
        .verify()
        .expect("verified native lifecycle wrapper");
    let directory = tempfile::tempdir().expect("fixture");
    let driver = directory.path().join("driver.c");
    let expected_kind = if application_errors {
        "OPAL_TEST_FAILURE_APPLICATION_ERROR"
    } else {
        "OPAL_TEST_FAILURE_ASSERTION"
    };
    std::fs::write(
        &driver,
        format!("#define EXPECTED_PRIMARY_KIND {expected_kind}\n{DRIVER}"),
    )
    .expect("driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    execute_module_with_sources(
        &codegen,
        &[
            driver,
            runtime.join("opal_testing_runtime.c"),
            runtime.join("opal_testing_lifecycle.c"),
            runtime.join("opal_error.c"),
        ],
    );
}

/// The lifecycle calls the generated wrapper, never a Rust callback substitute.
const DRIVER: &str = r#"
#include "opal_testing_lifecycle.h"
int compiled_body_wrapper(void **, const void *, void **, uint64_t *, const char **);
typedef struct { void *unused; const char *error; } ApplicationVoidResult;
ApplicationVoidResult error_from_application(void) { ApplicationVoidResult value = {NULL, "CallbackError.Failed"}; return value; }
static int after_called;
static int after(void **worlds, const void *example, void **created, uint64_t *failure, const char **error) {
    (void)worlds; (void)example; (void)created; (void)failure;
    if (!opal_test_attempt_failed()) return 99;
    ++after_called;
    *error = "AfterHookError";
    return OPAL_TEST_CALLBACK_APPLICATION_ERROR;
}
int main(void) {
    if (!opal_test_attempt_begin("compiled source lifecycle", 1)) return 1;
    OpalTestCallback after_hook = {after, "after.test.op", 0, 1};
    OpalTestHookScope scope = {NULL, 0, &after_hook, 1};
    OpalTestCaseDescriptor descriptor = {NULL, 0, &scope, 1, {compiled_body_wrapper, "bridge.test.op", 0, 1}, {NULL, "", 0, 0}, NULL};
    OpalTestLifecycleResult result;
    if (!opal_test_run_case(&descriptor, &result)) return 2;
    if (!result.body_started || result.body_passed || after_called != 1) return 3;
    if (opal_test_failure_count() != 2) return 4;
    const OpalTestFailure *first = opal_test_failure_at(0);
    const OpalTestFailure *second = opal_test_failure_at(1);
    if (first->kind != EXPECTED_PRIMARY_KIND || first->phase != OPAL_TEST_PHASE_BODY) return 5;
    if (second->phase != OPAL_TEST_PHASE_AFTER_HOOK || second->primary_failure_id != first->id) return 6;
    opal_test_attempt_end();
    return 0;
}
"#;
