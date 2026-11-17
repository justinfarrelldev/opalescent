//! Typed setup/example values through the scalar lifecycle ABI.
use super::binding_store::release_binding_value_if_needed;
use super::context::CodegenContext;
use super::error::CodegenError;
use super::native_callback_boundary::{CallbackBoundary, c_function, declare_boundary};
use super::native_callbacks::{NativeCallbackArgument, arguments};
use super::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use inkwell::AddressSpace;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue, StructValue};

/// Construct one typed value slot only after the source callback succeeds.
///
/// Storage is reserved before invoking source: allocation failure cannot strand
/// an already-returned source value. Failed setup never publishes partial state.
///
/// # Errors
/// Rejects incompatible source signatures, argument plans and runtime ABIs.
pub fn emit_value_callback_wrapper<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    callback: FunctionValue<'context>,
    core: &CoreType,
    application_errors: bool,
    plan: &[NativeCallbackArgument],
) -> Result<FunctionValue<'context>, CodegenError> {
    let outcome = codegen
        .native_function_outcome(callback)
        .ok_or_else(|| CodegenError::new("value wrapper requires a compiled native callback"))?;
    let value_type = core_type_to_llvm(codegen.context, core);
    let expected = if application_errors {
        super::error_abi::build_error_return_type(codegen.context, Some(value_type)).into()
    } else {
        value_type
    };
    if codegen.application_return_type(callback) != Some(expected)
        || usize::try_from(callback.count_params()).ok() != Some(plan.len())
    {
        return Err(CodegenError::new(
            "typed value callback does not match its checked source contract",
        ));
    }
    let boundary = declare_boundary(codegen, name)?;
    let lowered = arguments::lower_arguments(codegen, boundary.function, callback, plan)?;
    let storage = reserve_storage(codegen, &boundary, value_type)?;
    let result = codegen
        .builder
        .build_call(callback, &lowered, "value.callback.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("value callback omitted its native outcome"))?
        .into_struct_value();
    let branches = outcome.branch_after_call(codegen, result)?;
    let application = outcome
        .completed_application_value(codegen, result)?
        .ok_or_else(|| {
            CodegenError::new("value callback omitted its completed application value")
        })?;
    let value = if application_errors {
        extract_application_success(
            codegen,
            &boundary,
            storage,
            application.into_struct_value(),
            value_type,
        )?
    } else {
        application
    };
    let typed = codegen.builder.build_pointer_cast(
        storage,
        value_type.ptr_type(AddressSpace::default()),
        "value.slot",
    )?;
    codegen.builder.build_store(typed, value)?;
    codegen.builder.build_store(boundary.created, storage)?;
    return_status(codegen, 0)?;
    codegen.builder.position_at_end(branches.aborted);
    free_storage(codegen, storage)?;
    codegen
        .builder
        .build_store(boundary.failure, branches.failure_identity)?;
    return_status(codegen, 2)?;
    Ok(boundary.function)
}

/// Release only ordinary typed value memory and the compiler's value slot.
///
/// This does not discover, close or recover external resources. User-declared
/// cleanup runs separately, and is never synthesized by this drop boundary.
///
/// # Errors
/// Rejects duplicate wrapper names and incompatible runtime function ABIs.
pub fn emit_value_drop_wrapper<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    core: &CoreType,
) -> Result<FunctionValue<'context>, CodegenError> {
    if codegen.module.get_function(name).is_some() {
        return Err(CodegenError::new("value drop wrapper name already exists"));
    }
    let pointer = codegen.context.i8_type().ptr_type(AddressSpace::default());
    let function = codegen.module.add_function(
        name,
        codegen
            .context
            .void_type()
            .fn_type(&[pointer.into()], false),
        None,
    );
    codegen
        .builder
        .position_at_end(codegen.context.append_basic_block(function, "entry"));
    let storage = function
        .get_first_param()
        .ok_or_else(|| CodegenError::new("value drop boundary omitted its slot"))?
        .into_pointer_value();
    let typed = codegen.builder.build_pointer_cast(
        storage,
        core_type_to_llvm(codegen.context, core).ptr_type(AddressSpace::default()),
        "drop.value.slot",
    )?;
    let value = codegen.builder.build_load(typed, "drop.value")?;
    release_binding_value_if_needed(codegen, core, value, "lifecycle value drop")?;
    free_storage(codegen, storage)?;
    codegen.builder.build_return(None)?;
    Ok(function)
}

/// Reserve ordinary compiler-owned slot memory before executing user setup.
fn reserve_storage<'context>(
    codegen: &CodegenContext<'context>,
    boundary: &CallbackBoundary<'context>,
    value_type: BasicTypeEnum<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let malloc = c_function(
        codegen,
        "malloc",
        pointer.fn_type(&[llvm.i64_type().into()], false),
    )?;
    let size = value_type
        .size_of()
        .ok_or_else(|| CodegenError::new("native state value has an unsized representation"))?;
    let empty = codegen.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        size,
        size.get_type().const_zero(),
        "value.empty",
    )?;
    let size = codegen.builder.build_select(
        empty,
        size.get_type().const_int(1, false),
        size,
        "value.allocation.size",
    )?;
    let storage = codegen
        .builder
        .build_call(malloc, &[size.into()], "value.storage")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("malloc returned no pointer"))?
        .into_pointer_value();
    let failed = llvm.append_basic_block(boundary.function, "value.allocation.failed");
    let ready = llvm.append_basic_block(boundary.function, "value.allocation.ready");
    let is_null = codegen
        .builder
        .build_is_null(storage, "value.allocation.null")?;
    codegen
        .builder
        .build_conditional_branch(is_null, failed, ready)?;
    codegen.builder.position_at_end(failed);
    let record = c_function(
        codegen,
        "opal_test_record_failure",
        llvm.i64_type().fn_type(
            &[
                llvm.i32_type().into(),
                pointer.into(),
                pointer.into(),
                llvm.i64_type().into(),
                llvm.i64_type().into(),
            ],
            false,
        ),
    )?;
    let message = codegen
        .builder
        .build_global_string_ptr(
            "native test value allocation failed",
            "value.allocation.message",
        )?
        .as_pointer_value();
    let file = codegen
        .builder
        .build_global_string_ptr("", "value.allocation.file")?
        .as_pointer_value();
    let identity = codegen
        .builder
        .build_call(
            record,
            &[
                llvm.i32_type().const_int(5, false).into(),
                message.into(),
                file.into(),
                llvm.i64_type().const_zero().into(),
                llvm.i64_type().const_zero().into(),
            ],
            "value.allocation.failure",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("failure recorder omitted its identity"))?;
    codegen.builder.build_store(boundary.failure, identity)?;
    return_status(codegen, 2)?;
    codegen.builder.position_at_end(ready);
    Ok(storage)
}

/// Application errors branch before any source success field is extracted.
fn extract_application_success<'context>(
    codegen: &CodegenContext<'context>,
    boundary: &CallbackBoundary<'context>,
    storage: PointerValue<'context>,
    application: StructValue<'context>,
    value_type: BasicTypeEnum<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let index = super::error_abi::error_field_index(application.get_type().count_fields());
    let error = codegen
        .builder
        .build_extract_value(application, index, "value.application.error")?
        .into_pointer_value();
    let failed = codegen
        .context
        .append_basic_block(boundary.function, "value.application.failed");
    let ready = codegen
        .context
        .append_basic_block(boundary.function, "value.application.ready");
    let has_error = codegen
        .builder
        .build_is_not_null(error, "value.application.has_error")?;
    codegen
        .builder
        .build_conditional_branch(has_error, failed, ready)?;
    codegen.builder.position_at_end(failed);
    free_storage(codegen, storage)?;
    codegen.builder.build_store(boundary.error, error)?;
    return_status(codegen, 1)?;
    codegen.builder.position_at_end(ready);
    if let BasicTypeEnum::StructType(structure) = value_type {
        let mut value = structure.get_undef();
        for field in 0..structure.count_fields() {
            let element =
                codegen
                    .builder
                    .build_extract_value(application, field, "value.success.field")?;
            value = codegen
                .builder
                .build_insert_value(value, element, field, "value.success")?
                .into_struct_value();
        }
        Ok(value.into())
    } else {
        Ok(codegen
            .builder
            .build_extract_value(application, 0, "value.success")?)
    }
}

/// Free slot memory only, never state resources owned by user code.
fn free_storage<'context>(
    codegen: &CodegenContext<'context>,
    pointer: PointerValue<'context>,
) -> Result<(), CodegenError> {
    let function = c_function(
        codegen,
        "free",
        codegen
            .context
            .void_type()
            .fn_type(&[pointer.get_type().into()], false),
    )?;
    codegen
        .builder
        .build_call(function, &[pointer.into()], "")?;
    Ok(())
}

/// Scalar lifecycle status avoids platform-dependent C aggregate returns.
fn return_status(codegen: &CodegenContext<'_>, status: u64) -> Result<(), CodegenError> {
    codegen
        .builder
        .build_return(Some(&codegen.context.i32_type().const_int(status, false)))?;
    Ok(())
}
