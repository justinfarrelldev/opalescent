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
    let (world_parameters, world_body) = function(
        "let body = f(mutable ref world: FixtureWorld, ref row: int64): void =>\n    increment(mutable ref world.state)\n    expect row to be 7\n    return void\n",
    );
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
    let body_signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![world, CoreType::Int64],
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
    codegen
        .module
        .verify()
        .expect("valid typed World/state ABI");
    let directory = tempfile::tempdir().expect("fixture");
    let driver = directory.path().join("driver.c");
    std::fs::write(&driver, DRIVER).expect("driver");
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

/// Native state exists once; body and cleanup observe the same original scalar.
const DRIVER: &str = r#"
#include "opal_testing_lifecycle.h"
static int64_t state;
static const int64_t example_row = 7;
int compiled_body(void **, const void *, void **, uint64_t *, const char **);
int compiled_cleanup(void **, const void *, void **, uint64_t *, const char **);
static int setup(void **worlds, const void *example, void **created, uint64_t *failure, const char **error) {
    (void)worlds; (void)example; (void)failure; (void)error;
    state = 41; *created = &state; return OPAL_TEST_CALLBACK_COMPLETED;
}
static int construct_example(void **worlds, const void *example, void **created, uint64_t *failure, const char **error) {
    (void)worlds; (void)example; (void)failure; (void)error;
    *created = (void *)&example_row; return OPAL_TEST_CALLBACK_COMPLETED;
}
int main(void) {
    if (!opal_test_attempt_begin("typed World identity", 1)) return 1;
    OpalTestWorldDescriptor world = {"World", {setup, "", 0, 0}, {compiled_cleanup, "", 0, 0}, NULL};
    OpalTestCaseDescriptor descriptor = {&world, 1, NULL, 0, {compiled_body, "", 0, 0}, {construct_example, "", 0, 0}, NULL};
    OpalTestLifecycleResult result;
    if (opal_test_run_case(&descriptor, &result) || state != 43) return 2;
    opal_test_attempt_end(); return 0;
}
"#;
