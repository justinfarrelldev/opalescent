#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::missing_const_for_fn,
    clippy::pattern_type_mismatch,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::super::ast_type_to_core_type_for_signature;
use super::super::current_function;
use crate::ast::Expr;
use crate::codegen::binding_store::store_binding_overwrite_rc_safe;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding};
use crate::codegen::expressions_array::{
    allocate_array_payload, is_rc_bearing_element_type, load_array_capacity_from_value,
    load_array_data_ptr_for_element_type, load_array_length_from_value,
    load_array_payload_ptr_from_binding,
};
use crate::codegen::rc_emitter::RcEmitter;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, FunctionValue, IntValue, PointerValue};

pub(super) fn infer_array_callback_return_core_type(
    env: &CodegenEnv<'_>,
    callback: &Expr,
) -> Option<CoreType> {
    match *callback {
        Expr::Lambda {
            ref return_types, ..
        } => return_types
            .first()
            .and_then(|return_type| ast_type_to_core_type_for_signature(return_type).ok()),
        Expr::Identifier { ref name, .. } => env
            .variables
            .get(name.as_str())
            .and_then(|binding| match &binding.core_type {
                &CoreType::Function {
                    ref return_types, ..
                } => return_types.first().cloned(),
                _ => None,
            })
            .or_else(|| {
                env.imported_signatures
                    .get(name.as_str())
                    .and_then(|signature| match signature {
                        &CoreType::Function {
                            ref return_types, ..
                        } => return_types.first().cloned(),
                        _ => None,
                    })
            }),
        _ => None,
    }
}

pub(super) fn infer_map_callback_return_core_type(
    env: &CodegenEnv<'_>,
    callback: &Expr,
) -> Option<CoreType> {
    infer_array_callback_return_core_type(env, callback)
}

pub(super) fn store_array_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    array_value: PointerValue<'context>,
    operation: &str,
) -> Result<(), CodegenError> {
    let Some(binding_snapshot) = env.variables.get(binding_name).cloned() else {
        return Err(CodegenError::new(format!(
            "{operation} receiver '{binding_name}' not found"
        )));
    };
    if !binding_snapshot.is_mutable {
        return Err(CodegenError::new(format!(
            "array method '{operation}' requires mutable receiver '{binding_name}' during code generation"
        )));
    }

    store_binding_overwrite_rc_safe(
        codegen_context,
        env,
        binding_name,
        array_value.as_basic_value_enum(),
        operation,
    )
}

pub(super) fn retain_rc_element_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    value: inkwell::values::BasicValueEnum<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    if !is_rc_bearing_element_type(element_core_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "{name_prefix} expected pointer value for RC-bearing element type '{element_core_type}'"
        )));
    }

    let retain_fn = declare_or_get_opal_rc_inc(codegen_context);
    let casted = codegen_context.builder.build_pointer_cast(
        value.into_pointer_value(),
        codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default()),
        &env.next_name(format!("{name_prefix}.retain.cast").as_str()),
    )?;
    let args: [BasicMetadataValueEnum<'context>; 1] = [casted.into()];
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        retain_fn,
        &args,
        &env.next_name(format!("{name_prefix}.retain").as_str()),
    )?;
    Ok(())
}

fn declare_or_get_opal_rc_inc<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_rc_inc") {
        return function;
    }

    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = codegen_context
        .context
        .void_type()
        .fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_rc_inc", function_type, None)
}

pub(super) fn resolve_array_identifier_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    array_expr: &Expr,
) -> Result<
    (
        String,
        VariableBinding<'context>,
        PointerValue<'context>,
        IntValue<'context>,
        IntValue<'context>,
    ),
    CodegenError,
> {
    let Expr::Identifier { ref name, .. } = *array_expr else {
        return Err(CodegenError::new(format!(
            "{operation} currently requires an identifier array receiver"
        )));
    };
    let Some(binding) = env.variables.get(name).cloned() else {
        return Err(CodegenError::new(format!(
            "{operation} array receiver '{name}' not found"
        )));
    };
    if !matches!(binding.core_type, CoreType::Array(_)) {
        return Err(CodegenError::new(format!(
            "{operation} expects array receiver '{name}' to have an array type"
        )));
    }

    let array_value =
        load_array_payload_ptr_from_binding(codegen_context, env, name, binding.clone())?;
    let element_core_type = match binding.core_type {
        CoreType::Array(ref element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "{operation} expects array receiver '{name}' to have an array type"
            )));
        }
    };
    let base_ptr = load_array_data_ptr_for_element_type(
        codegen_context,
        env,
        array_value,
        &element_core_type,
        operation,
    )?;

    let length_value = load_array_length_from_value(codegen_context, env, array_value, operation)?;
    let capacity_value =
        load_array_capacity_from_value(codegen_context, env, array_value, operation)?;
    Ok((
        name.clone(),
        binding,
        base_ptr,
        length_value,
        capacity_value,
    ))
}

pub(super) fn rc_object_is_unique<'context>(
    codegen_context: &CodegenContext<'context>,
    obj: PointerValue<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_is_unique(obj)
}

pub(super) fn rc_object_is_reuse_eligible<'context>(
    codegen_context: &CodegenContext<'context>,
    obj: PointerValue<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_is_reuse_eligible(obj)
}

pub(super) fn validate_array_operation_metadata<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    base_ptr: PointerValue<'context>,
    length: IntValue<'context>,
    capacity: IntValue<'context>,
) -> Result<(), CodegenError> {
    let metadata_overflow = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        length,
        capacity,
        &env.next_name("append.meta.len_gt_cap"),
    )?;
    let metadata_overflow_message = format!("{operation} array metadata length exceeds capacity");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        metadata_overflow,
        metadata_overflow_message.as_str(),
        "append.meta.len_gt_cap",
    )?;

    let length_non_zero = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        length,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("append.meta.len_non_zero"),
    )?;
    let base_ptr_is_null = codegen_context
        .builder
        .build_is_null(base_ptr, &env.next_name("append.meta.ptr_null"))?;
    let missing_storage = codegen_context.builder.build_and(
        length_non_zero,
        base_ptr_is_null,
        &env.next_name("append.meta.missing_storage"),
    )?;
    let missing_storage_message =
        format!("{operation} array metadata requires storage for non-empty arrays");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        missing_storage,
        missing_storage_message.as_str(),
        "append.meta.missing_storage",
    )
}

pub(super) fn trap_on_invalid_array_state<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    should_trap: IntValue<'context>,
    message: &str,
    block_prefix: &str,
) -> Result<(), CodegenError> {
    let current_function = current_function(codegen_context)?;
    let trap_block_name = env.next_name(format!("{block_prefix}.trap").as_str());
    let continue_block_name = env.next_name(format!("{block_prefix}.ok").as_str());
    let trap_block = codegen_context
        .context
        .append_basic_block(current_function, trap_block_name.as_str());
    let continue_block = codegen_context
        .context
        .append_basic_block(current_function, continue_block_name.as_str());
    codegen_context
        .builder
        .build_conditional_branch(should_trap, trap_block, continue_block)?;

    codegen_context.builder.position_at_end(trap_block);
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let message_name = env.next_name(format!("{block_prefix}.msg").as_str());
    let trap_call_name = env.next_name(format!("{block_prefix}.call").as_str());
    let msg = codegen_context
        .builder
        .build_global_string_ptr(message, message_name.as_str())?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue =
        codegen_context
            .builder
            .build_call(runtime_fn, &[msg.into()], trap_call_name.as_str())?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(continue_block);
    Ok(())
}

pub(super) fn compute_next_array_capacity<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    current_capacity: IntValue<'context>,
    required_length: IntValue<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    let fits_existing = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::UGE,
        current_capacity,
        required_length,
        &env.next_name("append.cap.fits"),
    )?;
    let needs_growth = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        current_capacity,
        required_length,
        &env.next_name("append.cap.grow"),
    )?;
    let doubled_capacity = codegen_context.builder.build_int_mul(
        current_capacity,
        codegen_context.context.i64_type().const_int(2, false),
        &env.next_name("append.cap.double"),
    )?;
    let has_capacity = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        current_capacity,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("append.cap.has_capacity"),
    )?;
    let doubled_overflow = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        doubled_capacity,
        current_capacity,
        &env.next_name("append.cap.double_overflow"),
    )?;
    let growth_overflow_candidate = codegen_context.builder.build_and(
        has_capacity,
        doubled_overflow,
        &env.next_name("append.cap.growth_overflow_candidate"),
    )?;
    let growth_overflow = codegen_context.builder.build_and(
        needs_growth,
        growth_overflow_candidate,
        &env.next_name("append.cap.growth_overflow"),
    )?;
    let growth_overflow_message = format!("{operation} array capacity overflow");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        growth_overflow,
        growth_overflow_message.as_str(),
        "append.cap.growth_overflow",
    )?;
    let grown_capacity = codegen_context
        .builder
        .build_select(
            has_capacity,
            doubled_capacity,
            required_length,
            &env.next_name("append.cap.grown"),
        )?
        .into_int_value();
    Ok(codegen_context
        .builder
        .build_select(
            fits_existing,
            current_capacity,
            grown_capacity,
            &env.next_name("append.cap.next"),
        )?
        .into_int_value())
}

pub(super) fn allocate_array_with_capacity<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    element_core_type: &CoreType,
    capacity: IntValue<'context>,
) -> Result<(PointerValue<'context>, PointerValue<'context>), CodegenError> {
    allocate_array_payload(
        codegen_context,
        env,
        element_core_type,
        codegen_context.context.i64_type().const_zero(),
        capacity,
        operation,
    )
}

pub(super) fn set_array_payload_length<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    length: IntValue<'context>,
    operation: &str,
) -> Result<(), CodegenError> {
    let set_len_fn = declare_or_get_opal_array_set_len(codegen_context);
    let payload_ptr = codegen_context.builder.build_pointer_cast(
        array_value,
        codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default()),
        &env.next_name(format!("{operation}.set_len.cast").as_str()),
    )?;
    let args: [BasicMetadataValueEnum<'context>; 2] = [payload_ptr.into(), length.into()];
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        set_len_fn,
        &args,
        &env.next_name(format!("{operation}.set_len").as_str()),
    )?;
    Ok(())
}

fn declare_or_get_opal_array_set_len<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_set_len") {
        return function;
    }

    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i64_type = codegen_context.context.i64_type();
    let function_type = codegen_context
        .context
        .void_type()
        .fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    module.add_function("opal_array_set_len", function_type, None)
}

pub(super) fn copy_existing_array_elements<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    source_ptr: PointerValue<'context>,
    destination_ptr: PointerValue<'context>,
    length: IntValue<'context>,
) -> Result<(), CodegenError> {
    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("append.copy.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("append.copy.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("append.copy.exit"));

    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("append.copy.index"),
    )?;
    codegen_context.builder.build_store(
        index_alloca,
        codegen_context.context.i64_type().const_zero(),
    )?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, &env.next_name("append.copy.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        length,
        &env.next_name("append.copy.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the copy loop guard ensures `index_value < length`, so the source index is in bounds.
    let source_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            source_ptr,
            &[index_value],
            &env.next_name("append.copy.src"),
        )?
    };
    // SAFETY: destination uses the same guarded index into a buffer sized for at least `length` elements.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            destination_ptr,
            &[index_value],
            &env.next_name("append.copy.dst"),
        )?
    };
    let copied_value = codegen_context
        .builder
        .build_load(source_slot, &env.next_name("append.copy.value"))?;
    retain_rc_element_if_needed(
        codegen_context,
        env,
        element_core_type,
        copied_value,
        "append.copy",
    )?;
    codegen_context
        .builder
        .build_store(destination_slot, copied_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("append.copy.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{rc_object_is_reuse_eligible, rc_object_is_unique};
    use crate::codegen::context::CodegenContext;
    use inkwell::AddressSpace;
    use inkwell::context::Context;

    fn create_test_function<'context>(
        codegen_context: &CodegenContext<'context>,
        name: &str,
    ) -> inkwell::values::FunctionValue<'context> {
        let function = codegen_context.module.add_function(
            name,
            codegen_context.context.void_type().fn_type(&[], false),
            None,
        );
        let entry = codegen_context
            .context
            .append_basic_block(function, "entry");
        codegen_context.builder.position_at_end(entry);
        function
    }

    #[test]
    fn array_helper_wrappers_emit_rc_uniqueness_predicates() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "array_helper_predicates_test");
        let _function = create_test_function(&codegen_context, "array_helper_predicates_fn");

        let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
        let pointer_alloca = codegen_context
            .builder
            .build_alloca(i8_ptr_type, "array_obj")
            .expect("array helper predicate test should allocate rc object storage");
        let pointer_value = codegen_context
            .builder
            .build_load(pointer_alloca, "array_obj.load")
            .expect("array helper predicate test should load rc object storage")
            .into_pointer_value();

        let unique_result = rc_object_is_unique(&codegen_context, pointer_value);
        assert!(
            unique_result.is_ok(),
            "array helper uniqueness wrapper should emit successfully"
        );
        let reuse_result = rc_object_is_reuse_eligible(&codegen_context, pointer_value);
        assert!(
            reuse_result.is_ok(),
            "array helper reuse wrapper should emit successfully"
        );

        let ir = codegen_context.module.print_to_string().to_string();
        assert!(
            ir.contains("call i32 @opal_rc_is_unique")
                && ir.contains("call i32 @opal_rc_is_reuse_eligible"),
            "array helper wrappers should lower to both runtime predicate calls: {ir}"
        );
    }
}
