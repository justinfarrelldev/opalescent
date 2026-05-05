#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::current_function;
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{ArrayMetadata, CodegenEnv, VariableBinding};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, FunctionValue, IntValue, PointerValue};

pub(super) fn store_array_binding_with_metadata<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    array_value: PointerValue<'context>,
    metadata: ArrayMetadata<'context>,
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

    codegen_context
        .builder
        .build_store(binding_snapshot.alloca, array_value)?;

    let len_binding_name = format!("{binding_name}_len");
    if let Some(len_binding) = env.variables.get(len_binding_name.as_str()).cloned() {
        codegen_context
            .builder
            .build_store(len_binding.alloca, metadata.length)?;
    } else {
        let len_alloca = codegen_context
            .builder
            .build_alloca(metadata.length.get_type(), len_binding_name.as_str())?;
        codegen_context
            .builder
            .build_store(len_alloca, metadata.length)?;
        env.variables.insert(
            len_binding_name,
            VariableBinding {
                alloca: len_alloca,
                core_type: CoreType::Int64,
                length: None,
                capacity: None,
                is_mutable: false,
            },
        );
    }

    let cap_binding_name = format!("{binding_name}_cap");
    if let Some(cap_binding) = env.variables.get(cap_binding_name.as_str()).cloned() {
        codegen_context
            .builder
            .build_store(cap_binding.alloca, metadata.capacity)?;
    } else {
        let cap_alloca = codegen_context
            .builder
            .build_alloca(metadata.capacity.get_type(), cap_binding_name.as_str())?;
        codegen_context
            .builder
            .build_store(cap_alloca, metadata.capacity)?;
        env.variables.insert(
            cap_binding_name,
            VariableBinding {
                alloca: cap_alloca,
                core_type: CoreType::Int64,
                length: None,
                capacity: None,
                is_mutable: false,
            },
        );
    }

    if let Some(binding) = env.variables.get_mut(binding_name) {
        binding.length = None;
        binding.capacity = None;
    }
    env.set_pending_array_metadata(None);
    Ok(())
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

    let loaded_ptr = codegen_context
        .builder
        .build_load(binding.alloca, &env.next_name("append.array.load"))?;
    let base_ptr = if loaded_ptr.is_pointer_value() {
        loaded_ptr.into_pointer_value()
    } else {
        // SAFETY: this fallback addresses the first field of the stack slot that stores the array value,
        // which is the array pointer when LLVM lowers the binding as an aggregate.
        unsafe {
            codegen_context.builder.build_in_bounds_gep(
                binding.alloca,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_zero(),
                ],
                &env.next_name("append.array.base"),
            )?
        }
    };

    let length_value =
        resolve_array_metadata_value(codegen_context, env, name, &binding, true, operation)?;
    let capacity_value =
        resolve_array_metadata_value(codegen_context, env, name, &binding, false, operation)?;
    Ok((
        name.clone(),
        binding,
        base_ptr,
        length_value,
        capacity_value,
    ))
}

fn resolve_array_metadata_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    name: &str,
    binding: &VariableBinding<'context>,
    resolve_length: bool,
    operation: &str,
) -> Result<IntValue<'context>, CodegenError> {
    let known_value = if resolve_length {
        binding.length
    } else {
        binding.capacity
    };
    if let Some(value) = known_value {
        return Ok(codegen_context
            .context
            .i64_type()
            .const_int(u64::from(value), false));
    }

    let suffix = if resolve_length { "len" } else { "cap" };
    let binding_name = format!("{name}_{suffix}");
    if let Some(metadata_binding) = env.variables.get(binding_name.as_str()) {
        return Ok(codegen_context
            .builder
            .build_load(metadata_binding.alloca, binding_name.as_str())?
            .into_int_value());
    }

    if !resolve_length {
        return resolve_array_metadata_value(codegen_context, env, name, binding, true, operation);
    }

    Err(CodegenError::new(format!(
        "array metadata binding '{binding_name}' is missing for {operation} lowering"
    )))
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

pub(super) fn allocate_array_buffer<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    element_core_type: &CoreType,
    capacity: IntValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let zero_capacity = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        capacity,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("append.cap.zero"),
    )?;
    let zero_capacity_message =
        format!("{operation} requires positive array capacity for allocation");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        zero_capacity,
        zero_capacity_message.as_str(),
        "append.cap.zero",
    )?;

    let element_type = core_type_to_llvm(codegen_context.context, element_core_type);
    let element_size = element_type
        .size_of()
        .ok_or_else(|| CodegenError::new(String::from("append element type has no size")))?;
    let element_size_i64 = if element_size.get_type().get_bit_width() == 64 {
        element_size
    } else {
        codegen_context.builder.build_int_z_extend(
            element_size,
            codegen_context.context.i64_type(),
            &env.next_name("append.elem_size.i64"),
        )?
    };
    let total_bytes = codegen_context.builder.build_int_mul(
        capacity,
        element_size_i64,
        &env.next_name("append.bytes"),
    )?;
    let reconstructed_element_size = codegen_context.builder.build_int_unsigned_div(
        total_bytes,
        capacity,
        &env.next_name("append.bytes.reconstructed_size"),
    )?;
    let allocation_overflow = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        reconstructed_element_size,
        element_size_i64,
        &env.next_name("append.bytes.overflow"),
    )?;
    let allocation_overflow_message = format!("{operation} array allocation size overflow");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        allocation_overflow,
        allocation_overflow_message.as_str(),
        "append.bytes.overflow",
    )?;

    let malloc_fn = ensure_malloc_function(codegen_context);
    let allocation = codegen_context.builder.build_call(
        malloc_fn,
        &[total_bytes.as_basic_value_enum().into()],
        &env.next_name("append.malloc"),
    )?;
    let raw_ptr = allocation
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("append malloc returned void")))?
        .into_pointer_value();
    let non_zero_bytes = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        total_bytes,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("append.bytes.non_zero"),
    )?;
    let allocation_is_null = codegen_context
        .builder
        .build_is_null(raw_ptr, &env.next_name("append.malloc.is_null"))?;
    let allocation_failed = codegen_context.builder.build_and(
        non_zero_bytes,
        allocation_is_null,
        &env.next_name("append.malloc.failed"),
    )?;
    let allocation_failed_message = format!("{operation} array allocation failed");
    trap_on_invalid_array_state(
        codegen_context,
        env,
        allocation_failed,
        allocation_failed_message.as_str(),
        "append.malloc.failed",
    )?;
    Ok(codegen_context.builder.build_pointer_cast(
        raw_ptr,
        element_type.ptr_type(AddressSpace::default()),
        &env.next_name("append.ptr"),
    )?)
}

pub(super) fn copy_existing_array_elements<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
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

fn ensure_malloc_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i64_type = codegen_context.context.i64_type();
    codegen_context.module.get_function("malloc").map_or_else(
        || {
            let malloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
            codegen_context
                .module
                .add_function("malloc", malloc_type, None)
        },
        |existing| existing,
    )
}
