//! Typed World injection wrappers preserve one native state identity.
#![cfg(target_os = "linux")]
extern crate alloc;
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_declaration_tests::function;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::type_system::types::CoreType;
use inkwell::context::Context;

#[test]
fn world_reference_and_cleanup_state_wrappers_share_original_typed_storage() {
    exercise(false);
}

#[test]
fn after_hook_context_observes_primary_failure_without_erasing_it() {
    exercise(true);
}

#[test]
fn context_getters_reject_incompatible_source_symbols() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "context_symbol_collision");
    codegen
        .declare_native_test_function(
            "opal_test_attempt_failed",
            llvm.i64_type().fn_type(&[], false),
            None,
        )
        .expect("source declaration");
    let function =
        codegen
            .module
            .add_function("wrapper", llvm.void_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    assert!(super::native_context::reference_address(&codegen).is_err());
}

/// Run in fresh processes, both before and after a retained source failure.
fn exercise(previous_failure: bool) {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "typed_world_injection");
    let (parameters, body) = function(
        "let increment = f(mutable ref value: int64): void =>\n    value = value + 1\n    return void\n",
    );
    let increment_signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int64],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    let increment = super::native_declarations::emit_source_function(
        &codegen,
        &mut CodegenEnv::new(false),
        "increment",
        &parameters,
        &increment_signature,
        &body,
    )
    .expect("helper");
    let world = CoreType::Generic {
        name: "FixtureWorld".to_owned(),
        type_args: Vec::new(),
    };
    let failed_literal = if previous_failure { "true" } else { "false" };
    let source = format!(
        "let body = f(mutable ref world: FixtureWorld, ref row: int64, mutable ref test: TestContext): void =>\n    increment(mutable ref world.state)\n    expect row to be 7\n    expect test.failed to be {failed_literal}\n    expect test.attempt to be 1\n    return void\n"
    );
    let (world_parameters, world_body) = function(&source);
    let mut environment = CodegenEnv::new(false);
    environment
        .imported_signatures
        .insert("increment".to_owned(), increment_signature);
    environment.adt_field_layouts.insert(
        "FixtureWorld".to_owned(),
        vec![("state".to_owned(), CoreType::Int64)],
    );
    environment.adt_field_indices.insert(
        "FixtureWorld".to_owned(),
        alloc::collections::BTreeMap::from([("state".to_owned(), 0)]),
    );
    super::native_context::install_layout(&mut environment);
    let body_signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![
            world,
            CoreType::Int64,
            CoreType::Generic {
                name: "__opal_testing::TestContext".to_owned(),
                type_args: Vec::new(),
            },
        ],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    let callback = super::native_declarations::emit_source_function(
        &codegen,
        &mut environment,
        "source_body",
        &world_parameters,
        &body_signature,
        &world_body,
    )
    .expect("World callback");
    super::native_callbacks::emit_void_callback_wrapper_with_arguments(
        &codegen,
        "compiled_body",
        callback,
        false,
        &[
            super::native_callbacks::NativeCallbackArgument::WorldReference(0),
            super::native_callbacks::NativeCallbackArgument::ExampleReference,
            super::native_callbacks::NativeCallbackArgument::ContextReference,
        ],
    )
    .expect("World wrapper");
    // The same checked scalar-reference helper is a declared state cleanup.
    super::native_callbacks::emit_void_callback_wrapper_with_arguments(
        &codegen,
        "compiled_cleanup",
        increment,
        false,
        &[super::native_callbacks::NativeCallbackArgument::StateReference(0)],
    )
    .expect("state wrapper");
    emit_constructors(&codegen);
    codegen
        .module
        .verify()
        .expect("valid typed World/state ABI");
    execute_driver(&codegen, previous_failure);
}

/// World setup and example construction are actual compiled source callbacks.
fn emit_constructors(codegen: &CodegenContext<'_>) {
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    for (name, literal) in [("setup", 41_i64), ("example", 7_i64)] {
        let (parameters, statements) = function(&format!(
            "let constructor = f(): int64 =>\n    return {literal}\n"
        ));
        let source = super::native_declarations::emit_source_function(
            codegen,
            &mut CodegenEnv::new(false),
            &format!("source_{name}"),
            &parameters,
            &signature,
            &statements,
        )
        .expect("source constructor");
        super::native_value_callbacks::emit_value_callback_wrapper(
            codegen,
            &format!("compiled_{name}"),
            source,
            &CoreType::Int64,
            false,
            &[],
        )
        .expect("typed constructor boundary");
    }
    super::native_value_callbacks::emit_value_drop_wrapper(codegen, "drop_value", &CoreType::Int64)
        .expect("ordinary value drop");
}

/// Link the same reusable native functions to an explicit lifecycle descriptor.
fn execute_driver(codegen: &CodegenContext<'_>, previous_failure: bool) {
    let directory = tempfile::tempdir().expect("fixture");
    let driver = directory.path().join("driver.c");
    let fail = u8::from(previous_failure);
    std::fs::write(
        &driver,
        format!("#define PREVIOUS_FAILURE {fail}\n{DRIVER}"),
    )
    .expect("driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    execute_module_with_sources(
        codegen,
        &[
            driver,
            runtime.join("opal_testing_runtime.c"),
            runtime.join("opal_testing_lifecycle.c"),
            runtime.join("opal_error.c"),
        ],
    );
}

/// Native state exists once; body and cleanup observe the same original scalar.
const DRIVER: &str = r#"
#include "opal_testing_lifecycle.h"
#include <stdio.h>
static int64_t state;
int compiled_body(void **, const void *, void **, uint64_t *, const char **);
int compiled_cleanup(void **, const void *, void **, uint64_t *, const char **);
int compiled_setup(void **, const void *, void **, uint64_t *, const char **);
int compiled_example(void **, const void *, void **, uint64_t *, const char **);
void drop_value(void *);
static void observe_world_drop(void *value) { state = *(int64_t *)value; drop_value(value); }
#if PREVIOUS_FAILURE
static int failed_body(void **worlds, const void *example, void **created, uint64_t *failure, const char **error) {
    (void)worlds; (void)example; (void)created; (void)error;
    *failure = opal_test_record_failure(OPAL_TEST_FAILURE_ASSERTION, "earlier body failure", "", 0, 0);
    return OPAL_TEST_CALLBACK_ABORTED;
}
#endif
int main(void) {
    if (!opal_test_attempt_begin("typed World identity", 1)) return 1;
    OpalTestWorldDescriptor world = {"World", {compiled_setup, "", 0, 0}, {compiled_cleanup, "", 0, 0}, observe_world_drop};
#if PREVIOUS_FAILURE
    OpalTestCallback after = {compiled_body, "", 0, 0};
    OpalTestHookScope scope = {NULL, 0, &after, 1};
    OpalTestCaseDescriptor descriptor = {&world, 1, &scope, 1, {failed_body, "", 0, 0}, {compiled_example, "", 0, 0}, drop_value};
#else
    OpalTestCaseDescriptor descriptor = {&world, 1, NULL, 0, {compiled_body, "", 0, 0}, {compiled_example, "", 0, 0}, drop_value};
#endif
    OpalTestLifecycleResult result;
    int failed = opal_test_run_case(&descriptor, &result);
    if (failed != PREVIOUS_FAILURE || state != 43 || opal_test_failure_count() != PREVIOUS_FAILURE) {
        const OpalTestFailure *failure = opal_test_failure_at(0);
        if (failure) fprintf(stderr, "typed bridge failure at %llu: %s\n", (unsigned long long)failure->offset, failure->message);
        fprintf(stderr, "state=%lld\n", (long long)state);
        return 2;
    }
    opal_test_attempt_end(); return 0;
}
"#;
