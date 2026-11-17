//! C-compatible lifecycle wrappers around typed native Opalescent callbacks.
use super::context::CodegenContext;
use super::error::CodegenError;
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::types::BasicTypeEnum;
use inkwell::values::FunctionValue;

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
    let outcome = codegen.native_function_outcome(callback).ok_or_else(|| {
        CodegenError::new("callback wrapper requires an explicitly compiled native function")
    })?;
    if callback.count_params() != 0 {
        return Err(CodegenError::new(
            "callback parameters require a typed injection wrapper plan",
        ));
    }
    validate_void_result(
        codegen.application_return_type(callback),
        application_errors,
    )?;
    if codegen.module.get_function(name).is_some() {
        return Err(CodegenError::new(
            "native callback wrapper name conflicts with an existing function",
        ));
    }
    let llvm = codegen.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let pointer_slot = pointer.ptr_type(AddressSpace::default());
    let failure_slot = llvm.i64_type().ptr_type(AddressSpace::default());
    let signature = llvm.i32_type().fn_type(
        &[
            pointer_slot.into(),
            pointer.into(),
            pointer_slot.into(),
            failure_slot.into(),
            pointer_slot.into(),
        ],
        false,
    );
    let wrapper = codegen
        .module
        .add_function(name, signature, Some(Linkage::External));
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(wrapper, "entry"));
    let created = wrapper
        .get_nth_param(2)
        .ok_or_else(|| CodegenError::new("wrapper created-value output missing"))?
        .into_pointer_value();
    let failure = wrapper
        .get_nth_param(3)
        .ok_or_else(|| CodegenError::new("wrapper failure output missing"))?
        .into_pointer_value();
    let error = wrapper
        .get_nth_param(4)
        .ok_or_else(|| CodegenError::new("wrapper error output missing"))?
        .into_pointer_value();
    codegen.builder.build_store(created, pointer.const_null())?;
    codegen
        .builder
        .build_store(failure, llvm.i64_type().const_zero())?;
    codegen.builder.build_store(error, pointer.const_null())?;
    let result = codegen
        .builder
        .build_call(callback, &[], "callback.outcome")?
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
