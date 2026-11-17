//! Native test abort transport is distinct from application error aggregates.
use super::context::CodegenContext;
use super::native_outcome::NativeOutcomeType;
use inkwell::context::Context;
use inkwell::types::BasicType;
use inkwell::values::BasicValue;

#[test]
fn aborted_non_fallible_results_leave_the_application_slot_undefined() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "abort_channel");
    let outcome = NativeOutcomeType::new(&llvm, Some(llvm.i64_type().into()));
    let function =
        codegen
            .module
            .add_function("aborted", outcome.llvm_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let aborted = outcome
        .build_aborted(&codegen, llvm.i64_type().const_int(41, false))
        .expect("aborted outcome");
    codegen
        .builder
        .build_return(Some(&aborted))
        .expect("return");
    codegen.module.verify().expect("valid outcome IR");
    let ir = codegen.module.print_to_string().to_string();
    assert!(ir.contains("i64 undef"), "{ir}");
    assert!(
        !ir.contains("i64 0"),
        "an abort must not synthesize an application value: {ir}"
    );
}

#[test]
fn completed_application_errors_are_not_internal_test_aborts() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "application_error_channel");
    let application =
        super::error_abi::build_error_return_type(&llvm, Some(llvm.i64_type().into()));
    let outcome = NativeOutcomeType::new(&llvm, Some(application.into()));
    let function = codegen.module.add_function(
        "expected_error",
        outcome.llvm_type().fn_type(&[], false),
        None,
    );
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let error = codegen
        .builder
        .build_global_string_ptr("FilesystemFullError", "application_error")
        .expect("error string")
        .as_pointer_value();
    let application_value =
        super::error_abi::build_error_aggregate_for_return_type(&codegen, application, error)
            .expect("ordinary application error");
    let completed = outcome
        .build_completed(&codegen, Some(application_value.as_basic_value_enum()))
        .expect("completed error");
    assert_eq!(
        outcome
            .aborted_flag(&codegen, completed)
            .expect("tag")
            .get_zero_extended_constant(),
        Some(0)
    );
    assert_eq!(
        outcome
            .completed_application_value(&codegen, completed)
            .expect("application payload")
            .expect("result")
            .get_type(),
        application.as_basic_type_enum()
    );
    codegen
        .builder
        .build_return(Some(&completed))
        .expect("return");
    codegen.module.verify().expect("valid separate channels");
}

#[test]
fn void_results_have_no_dummy_application_slot_and_invalid_shapes_fail() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "void_channel");
    let outcome = NativeOutcomeType::new(&llvm, None);
    assert_eq!(outcome.llvm_type().count_fields(), 2);
    let function = codegen.module.add_function(
        "completed_void",
        outcome.llvm_type().fn_type(&[], false),
        None,
    );
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let completed = outcome
        .build_completed(&codegen, None)
        .expect("completed void");
    assert!(
        outcome
            .completed_application_value(&codegen, completed)
            .expect("no payload")
            .is_none()
    );
    assert!(
        outcome
            .build_aborted(&codegen, llvm.i64_type().const_zero())
            .is_err()
    );
    assert!(
        outcome
            .build_completed(&codegen, Some(llvm.i64_type().const_zero().into()))
            .is_err()
    );
    codegen
        .builder
        .build_return(Some(&completed))
        .expect("return");
    codegen.module.verify().expect("valid void outcome");
}
