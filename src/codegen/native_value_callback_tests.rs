//! Native setup/example values cross C boundaries only after successful completion.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_declaration_tests::function;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::type_system::types::CoreType;
use inkwell::context::Context;

#[test]
fn compiled_value_callback_creates_a_typed_state_slot() {
    exercise(false, false);
}

#[test]
fn aborted_value_callback_never_publishes_a_fabricated_state() {
    exercise(true, false);
}

#[test]
fn allocation_failure_aborts_before_source_setup_can_execute() {
    exercise(true, true);
}

#[test]
fn ordinary_setup_error_does_not_publish_success_fields_or_abort() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "fallible_native_setup");
    let application_type =
        super::error_abi::build_error_return_type(&llvm, Some(llvm.i64_type().into()));
    let callback = codegen
        .declare_native_test_function("source_setup", application_type.fn_type(&[], false), None)
        .expect("fallible setup");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(callback, "entry"));
    let error = codegen
        .builder
        .build_global_string_ptr("setup application error", "ordinary.error")
        .expect("error")
        .as_pointer_value();
    // Ordinary application error ABI has no valid success payload on this path.
    let application = codegen
        .builder
        .build_insert_value(application_type.get_undef(), error, 1, "application.error")
        .expect("application error")
        .into_struct_value();
    let outcome = codegen
        .native_function_outcome(callback)
        .expect("native ABI");
    let completed = outcome
        .build_completed(&codegen, Some(application.into()))
        .expect("completed ordinary error");
    codegen
        .builder
        .build_return(Some(&completed))
        .expect("source return");
    super::native_value_callbacks::emit_value_callback_wrapper(
        &codegen,
        "compiled_setup",
        callback,
        &CoreType::Int64,
        true,
        &[],
    )
    .expect("fallible value wrapper");
    super::native_value_callbacks::emit_value_drop_wrapper(
        &codegen,
        "drop_value",
        &CoreType::Int64,
    )
    .expect("drop");
    codegen
        .module
        .verify()
        .expect("valid ordinary-error boundary");
    execute_driver(
        &codegen,
        "#define EXPECTED_ABORT 0\n#define EXPECTED_ALLOCATION_FAILURE 0\n#define EXPECTED_APPLICATION_ERROR 1\n",
    );
}

/// Execute the same reusable callback ABI in separate native processes.
fn exercise(aborted: bool, allocation_failure: bool) {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_value_callbacks");
    let source = if aborted {
        "let setup = f(): int64 =>\n    expect false to be true\n    return 41\n"
    } else {
        "let setup = f(): int64 =>\n    return 41\n"
    };
    let (parameters, statements) = function(source);
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    let callback = super::native_declarations::emit_source_function(
        &codegen,
        &mut CodegenEnv::new(false),
        "source_setup",
        &parameters,
        &signature,
        &statements,
    )
    .expect("source setup");
    super::native_value_callbacks::emit_value_callback_wrapper(
        &codegen,
        "compiled_setup",
        callback,
        &CoreType::Int64,
        false,
        &[],
    )
    .expect("typed construction wrapper");
    super::native_value_callbacks::emit_value_drop_wrapper(
        &codegen,
        "drop_value",
        &CoreType::Int64,
    )
    .expect("ordinary typed value drop");
    if allocation_failure {
        codegen
            .module
            .get_function("malloc")
            .expect("slot allocator")
            .as_global_value()
            .set_name("fixture_malloc");
    }
    codegen
        .module
        .verify()
        .expect("valid typed construction ABI");
    let abort = u8::from(aborted);
    let allocation = u8::from(allocation_failure);
    execute_driver(
        &codegen,
        &format!(
            "#define EXPECTED_ABORT {abort}\n#define EXPECTED_ALLOCATION_FAILURE {allocation}\n#define EXPECTED_APPLICATION_ERROR 0\n"
        ),
    );
}

/// Link generated boundaries against the real test runtime, not Rust callbacks.
fn execute_driver(codegen: &CodegenContext<'_>, defines: &str) {
    let directory = tempfile::tempdir().expect("fixture");
    let driver = directory.path().join("driver.c");
    std::fs::write(&driver, format!("{defines}{DRIVER}")).expect("driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    execute_module_with_sources(codegen, &[driver, runtime.join("opal_testing_runtime.c")]);
}

/// A failed setup has no initialized state; successful slots carry exact values.
const DRIVER: &str = r#"
#include "opal_testing_lifecycle.h"
#include <stdlib.h>
#include <string.h>
void *fixture_malloc(uint64_t size) { return EXPECTED_ALLOCATION_FAILURE ? NULL : malloc((size_t)size); }
int compiled_setup(void **, const void *, void **, uint64_t *, const char **);
void drop_value(void *);
int main(void) {
    if (!opal_test_attempt_begin("typed source setup", 1)) return 1;
    void *created = (void *)1;
    uint64_t failure = 999;
    const char *error = "sentinel";
    int status = compiled_setup(NULL, NULL, &created, &failure, &error);
    if (EXPECTED_APPLICATION_ERROR) {
        if (status != OPAL_TEST_CALLBACK_APPLICATION_ERROR || created || failure || !error || strcmp(error, "setup application error") || opal_test_failure_count()) return 7;
        opal_test_attempt_end(); return 0;
    }
    if (error) return 2;
    if (EXPECTED_ABORT) {
        if (status != OPAL_TEST_CALLBACK_ABORTED || created || !failure || failure != opal_test_primary_failure_id()) return 3;
        const OpalTestFailure *diagnostic = opal_test_failure_at(0);
        if (!diagnostic || opal_test_failure_count() != 1) return 5;
        if (diagnostic->kind != (EXPECTED_ALLOCATION_FAILURE ? OPAL_TEST_FAILURE_INFRASTRUCTURE : OPAL_TEST_FAILURE_ASSERTION)) return 6;
    } else {
        if (status != OPAL_TEST_CALLBACK_COMPLETED || !created || failure || *(int64_t *)created != 41) return 4;
        drop_value(created);
    }
    opal_test_attempt_end(); return 0;
}
"#;
