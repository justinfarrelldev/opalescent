//! C-compatible lifecycle wrappers around typed native Opalescent callbacks.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::native_callback_boundary::declare_boundary;
use inkwell::types::BasicTypeEnum;
use inkwell::values::FunctionValue;
#[path = "native_callback_arguments.rs"]
pub(super) mod arguments;

/// Compiler-resolved injection positions, never source-name fixture lookup.
#[derive(Clone, Copy, Debug)]
pub enum NativeCallbackArgument {
    /// Borrow a nominal World handle from the ordered initialized state slots.
    WorldReference(usize),
    /// Borrow that World's actual state for its declared cleanup callback.
    StateReference(usize),
    /// Borrow the independently constructed typed example value.
    ExampleReference,
    /// Borrow a fresh runner-owned view of this attempt's metadata.
    ContextReference,
}

/// Emit an argument-free void callback boundary for a compiled body/hook.
///
/// The C ABI uses scalar status and explicit outputs, not target-dependent
/// aggregate return conventions. Native aborts never read application payloads.
///
/// # Errors
/// Rejects non-native functions, non-void signatures and missing argument plans.
pub fn emit_void_callback_wrapper<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    callback: FunctionValue<'context>,
    application_errors: bool,
) -> Result<FunctionValue<'context>, CodegenError> {
    emit_void_callback_wrapper_with_arguments(codegen, name, callback, application_errors, &[])
}

/// Emit a void callback with statically resolved nominal injection positions.
///
/// # Errors
/// Rejects mismatched callback signatures or invalid typed argument plans.
pub fn emit_void_callback_wrapper_with_arguments<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    callback: FunctionValue<'context>,
    application_errors: bool,
    plan: &[NativeCallbackArgument],
) -> Result<FunctionValue<'context>, CodegenError> {
    let outcome = codegen.native_function_outcome(callback).ok_or_else(|| {
        CodegenError::new("callback wrapper requires an explicitly compiled native function")
    })?;
    if usize::try_from(callback.count_params()).ok() != Some(plan.len()) {
        return Err(CodegenError::new(
            "callback parameters require a typed injection wrapper plan",
        ));
    }
    validate_void_result(
        codegen.application_return_type(callback),
        application_errors,
    )?;
    let boundary = declare_boundary(codegen, name)?;
    let wrapper = boundary.function;
    let failure = boundary.failure;
    let error = boundary.error;
    let llvm = codegen.context;
    let lowered_arguments = arguments::lower_arguments(codegen, wrapper, callback, plan)?;
    let result = codegen
        .builder
        .build_call(callback, &lowered_arguments, "callback.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("callback returned no internal outcome"))?
        .into_struct_value();
    let branches = outcome.branch_after_call(codegen, result)?;
    if application_errors {
        let application = outcome
            .completed_application_value(codegen, result)?
            .ok_or_else(|| {
                CodegenError::new("fallible callback omitted its application error ABI")
            })?
            .into_struct_value();
        let application_error = codegen
            .builder
            .build_extract_value(application, 1, "callback.application.error")?
            .into_pointer_value();
        let failed = llvm.append_basic_block(wrapper, "callback.application.failed");
        let completed = llvm.append_basic_block(wrapper, "callback.application.completed");
        let has_error = codegen
            .builder
            .build_is_not_null(application_error, "callback.has_error")?;
        codegen
            .builder
            .build_conditional_branch(has_error, failed, completed)?;
        codegen.builder.position_at_end(failed);
        codegen.builder.build_store(error, application_error)?;
        codegen
            .builder
            .build_return(Some(&llvm.i32_type().const_int(1, false)))?;
        codegen.builder.position_at_end(completed);
    }
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_zero()))?;
    codegen.builder.position_at_end(branches.aborted);
    codegen
        .builder
        .build_store(failure, branches.failure_identity)?;
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(2, false)))?;
    Ok(wrapper)
}

/// Void callbacks either have no result or the existing two-pointer void error ABI.
fn validate_void_result(
    result: Option<BasicTypeEnum<'_>>,
    application_errors: bool,
) -> Result<(), CodegenError> {
    let valid = if application_errors {
        result.is_some_and(|result| {
            if let BasicTypeEnum::StructType(structure) = result {
                structure.count_fields() == 2
                    && structure
                        .get_field_types()
                        .iter()
                        .all(|field| match *field {
                            BasicTypeEnum::PointerType(pointer) => {
                                pointer.get_element_type().is_int_type()
                                    && pointer.get_element_type().into_int_type().get_bit_width()
                                        == 8
                            }
                            _ => false,
                        })
            } else {
                false
            }
        })
    } else {
        result.is_none()
    };
    if valid {
        Ok(())
    } else {
        Err(CodegenError::new(
            "void callback wrapper does not match its original application signature",
        ))
    }
}
