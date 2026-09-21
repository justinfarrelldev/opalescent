#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::super::functions_call_helpers::current_function;
use super::helpers::{
    allocate_array_with_capacity, resolve_array_identifier_binding, retain_rc_element_if_needed,
    set_array_payload_length, validate_array_operation_metadata,
};
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::{
    build_error_aggregate_for_return_type, build_error_return_type, build_success_aggregate,
    intern_variant_name,
};
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};

#[expect(
    clippy::too_many_lines,
    reason = "insert lowering builds success/error aggregates with array copy loops"
)]
pub(super) fn codegen_array_insert_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 2 {
        return Err(CodegenError::new(format!(
            "array method 'insert' expects exactly 2 arguments but received {}",
            args.len()
        )));
    }

    let operation = "insert";
    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, operation, receiver)?;
    let element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| {
            CodegenError::new(format!(
                "insert expects an array receiver, found '{}'",
                array_binding.core_type
            ))
        })?
        .clone();
    let index_value = codegen_expression(codegen_context, env, &args[0], Some(&CoreType::Int64))?
        .into_int_value();
    let inserted_value =
        codegen_expression(codegen_context, env, &args[1], Some(&element_core_type))?;

    validate_array_operation_metadata(
        codegen_context,
        env,
        operation,
        base_ptr,
        length_value,
        capacity_value,
    )?;

    let result_type = build_error_return_type(
        codegen_context.context,
        Some(core_type_to_llvm(
            codegen_context.context,
            &CoreType::Array(alloc::boxed::Box::new(element_core_type.clone())),
        )),
    );
    let result_alloca = codegen_context
        .builder
        .build_alloca(result_type, &env.next_name("array.insert.result"))?;
    let zero = codegen_context.context.i64_type().const_zero();
    let is_non_negative = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SGE,
        index_value,
        zero,
        &env.next_name("array.insert.non_negative"),
    )?;
    let is_at_most_length = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SLE,
        index_value,
        length_value,
        &env.next_name("array.insert.in_bounds"),
    )?;
    let is_valid = codegen_context.builder.build_and(
        is_non_negative,
        is_at_most_length,
        &env.next_name("array.insert.valid"),
    )?;

    let current_fn = current_function(codegen_context)?;
    let success_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.insert.success"));
    let error_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.insert.error"));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.insert.cont"));
    codegen_context
        .builder
        .build_conditional_branch(is_valid, success_block, error_block)?;

    codegen_context.builder.position_at_end(success_block);
    let next_length = codegen_context.builder.build_int_add(
        length_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("array.insert.len"),
    )?;
    let (result_array, result_ptr) = allocate_array_with_capacity(
        codegen_context,
        env,
        operation,
        &element_core_type,
        next_length,
    )?;
    copy_array_range_with_offsets(
        codegen_context,
        env,
        &element_core_type,
        base_ptr,
        result_ptr,
        zero,
        zero,
        index_value,
        "array.insert.prefix",
    )?;
    // SAFETY: index validity was checked above and the destination array was allocated with next_length capacity.
    let insert_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            result_ptr,
            &[index_value],
            &env.next_name("array.insert.slot"),
        )?
    };
    retain_rc_element_if_needed(
        codegen_context,
        env,
        &element_core_type,
        inserted_value,
        "array.insert.value",
    )?;
    codegen_context
        .builder
        .build_store(insert_slot, inserted_value)?;
    let dest_after_insert = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("array.insert.dest_tail"),
    )?;
    let tail_count = codegen_context.builder.build_int_sub(
        length_value,
        index_value,
        &env.next_name("array.insert.tail_count"),
    )?;
    copy_array_range_with_offsets(
        codegen_context,
        env,
        &element_core_type,
        base_ptr,
        result_ptr,
        index_value,
        dest_after_insert,
        tail_count,
        "array.insert.tail",
    )?;
    set_array_payload_length(codegen_context, env, result_array, next_length, operation)?;
    let success_result =
        build_success_aggregate(codegen_context, result_array.as_basic_value_enum())?;
    codegen_context
        .builder
        .build_store(result_alloca, success_result)?;
    codegen_context
        .builder
        .build_unconditional_branch(cont_block)?;

    codegen_context.builder.position_at_end(error_block);
    let error_ptr = intern_variant_name(codegen_context, env, "IndexOutOfBoundsError")?;
    let error_result =
        build_error_aggregate_for_return_type(codegen_context, result_type, error_ptr)?;
    codegen_context
        .builder
        .build_store(result_alloca, error_result)?;
    codegen_context
        .builder
        .build_unconditional_branch(cont_block)?;

    codegen_context.builder.position_at_end(cont_block);
    codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("array.insert.result.load"))
        .map_err(Into::into)
}

#[expect(
    clippy::too_many_lines,
    reason = "remove_at lowering builds success/error aggregates with array copy loops"
)]
pub(super) fn codegen_array_remove_at_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "array method 'remove_at' expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let operation = "remove_at";
    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, operation, receiver)?;
    let element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| {
            CodegenError::new(format!(
                "remove_at expects an array receiver, found '{}'",
                array_binding.core_type
            ))
        })?
        .clone();
    let index_value = codegen_expression(codegen_context, env, &args[0], Some(&CoreType::Int64))?
        .into_int_value();

    validate_array_operation_metadata(
        codegen_context,
        env,
        operation,
        base_ptr,
        length_value,
        capacity_value,
    )?;

    let array_llvm_type = core_type_to_llvm(
        codegen_context.context,
        &CoreType::Array(alloc::boxed::Box::new(element_core_type.clone())),
    );
    let element_llvm_type = core_type_to_llvm(codegen_context.context, &element_core_type);
    let success_type = codegen_context
        .context
        .struct_type(&[array_llvm_type.into(), element_llvm_type.into()], false);
    let result_type = build_error_return_type(codegen_context.context, Some(success_type.into()));
    let result_alloca = codegen_context
        .builder
        .build_alloca(result_type, &env.next_name("array.remove_at.result"))?;

    let zero = codegen_context.context.i64_type().const_zero();
    let is_non_negative = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SGE,
        index_value,
        zero,
        &env.next_name("array.remove_at.non_negative"),
    )?;
    let is_below_length = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SLT,
        index_value,
        length_value,
        &env.next_name("array.remove_at.in_bounds"),
    )?;
    let is_valid = codegen_context.builder.build_and(
        is_non_negative,
        is_below_length,
        &env.next_name("array.remove_at.valid"),
    )?;

    let current_fn = current_function(codegen_context)?;
    let success_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.remove_at.success"));
    let error_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.remove_at.error"));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.remove_at.cont"));
    codegen_context
        .builder
        .build_conditional_branch(is_valid, success_block, error_block)?;

    codegen_context.builder.position_at_end(success_block);
    // SAFETY: index validity was checked above, so the removed element lies inside the source payload.
    let removed_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[index_value],
            &env.next_name("array.remove_at.removed.slot"),
        )?
    };
    let removed_value = codegen_context
        .builder
        .build_load(removed_slot, &env.next_name("array.remove_at.removed"))?;
    let returned_removed_value = duplicate_removed_string_if_needed(
        codegen_context,
        env,
        &element_core_type,
        removed_value,
    )?;
    retain_rc_element_if_needed(
        codegen_context,
        env,
        &element_core_type,
        returned_removed_value,
        "array.remove_at.removed",
    )?;
    let next_length = codegen_context.builder.build_int_sub(
        length_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("array.remove_at.len"),
    )?;
    let (result_array, result_ptr) = allocate_array_with_capacity(
        codegen_context,
        env,
        operation,
        &element_core_type,
        next_length,
    )?;
    copy_array_range_with_offsets(
        codegen_context,
        env,
        &element_core_type,
        base_ptr,
        result_ptr,
        zero,
        zero,
        index_value,
        "array.remove_at.prefix",
    )?;
    let source_after_removed = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("array.remove_at.source_tail"),
    )?;
    let tail_count = codegen_context.builder.build_int_sub(
        length_value,
        source_after_removed,
        &env.next_name("array.remove_at.tail_count"),
    )?;
    copy_array_range_with_offsets(
        codegen_context,
        env,
        &element_core_type,
        base_ptr,
        result_ptr,
        source_after_removed,
        index_value,
        tail_count,
        "array.remove_at.tail",
    )?;
    set_array_payload_length(codegen_context, env, result_array, next_length, operation)?;
    let mut success_value = success_type.get_undef();
    success_value = codegen_context
        .builder
        .build_insert_value(
            success_value,
            result_array,
            0,
            &env.next_name("array.remove_at.success.array"),
        )?
        .into_struct_value();
    success_value = codegen_context
        .builder
        .build_insert_value(
            success_value,
            returned_removed_value,
            1,
            &env.next_name("array.remove_at.success.value"),
        )?
        .into_struct_value();
    let success_result =
        build_success_aggregate(codegen_context, success_value.as_basic_value_enum())?;
    codegen_context
        .builder
        .build_store(result_alloca, success_result)?;
    codegen_context
        .builder
        .build_unconditional_branch(cont_block)?;

    codegen_context.builder.position_at_end(error_block);
    let error_ptr = intern_variant_name(codegen_context, env, "IndexOutOfBoundsError")?;
    let error_result =
        build_error_aggregate_for_return_type(codegen_context, result_type, error_ptr)?;
    codegen_context
        .builder
        .build_store(result_alloca, error_result)?;
    codegen_context
        .builder
        .build_unconditional_branch(cont_block)?;

    codegen_context.builder.position_at_end(cont_block);
    codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("array.remove_at.result.load"))
        .map_err(Into::into)
}

fn duplicate_removed_string_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    removed_value: BasicValueEnum<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if !matches!(element_core_type, CoreType::String) {
        return Ok(removed_value);
    }
    let duplicate_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "string_insert_at",
    )
    .ok_or_else(|| CodegenError::new(String::from("string_insert_at declaration missing")))?;
    let empty = codegen_context
        .builder
        .build_global_string_ptr("", &env.next_name("array.remove_at.empty"))?
        .as_pointer_value();
    let zero = codegen_context.context.i64_type().const_zero();
    let call = codegen_context.builder.build_call(
        duplicate_fn,
        &[empty.into(), zero.into(), removed_value.into()],
        &env.next_name("array.remove_at.duplicate"),
    )?;
    let result = call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("string_insert_at returned void")))?
        .into_struct_value();
    codegen_context
        .builder
        .build_extract_value(result, 0, &env.next_name("array.remove_at.duplicate.value"))
        .map(|value| value.as_basic_value_enum())
        .map_err(Into::into)
}

fn copy_array_range_with_offsets<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    source_ptr: PointerValue<'context>,
    destination_ptr: PointerValue<'context>,
    source_start: IntValue<'context>,
    destination_start: IntValue<'context>,
    count: IntValue<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context.context.append_basic_block(
        current_function,
        &env.next_name(format!("{name_prefix}.loop").as_str()),
    );
    let body_block = codegen_context.context.append_basic_block(
        current_function,
        &env.next_name(format!("{name_prefix}.body").as_str()),
    );
    let exit_block = codegen_context.context.append_basic_block(
        current_function,
        &env.next_name(format!("{name_prefix}.exit").as_str()),
    );
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name(format!("{name_prefix}.index").as_str()),
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
        .build_load(
            index_alloca,
            &env.next_name(format!("{name_prefix}.index.load").as_str()),
        )?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        count,
        &env.next_name(format!("{name_prefix}.cond").as_str()),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    let source_index = codegen_context.builder.build_int_add(
        source_start,
        index_value,
        &env.next_name(format!("{name_prefix}.source.index").as_str()),
    )?;
    let destination_index = codegen_context.builder.build_int_add(
        destination_start,
        index_value,
        &env.next_name(format!("{name_prefix}.destination.index").as_str()),
    )?;
    // SAFETY: the loop count is derived from validated source ranges, keeping source_index in bounds.
    let source_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            source_ptr,
            &[source_index],
            &env.next_name(format!("{name_prefix}.source").as_str()),
        )?
    };
    // SAFETY: the destination range mirrors the validated copy count into an allocated destination payload.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            destination_ptr,
            &[destination_index],
            &env.next_name(format!("{name_prefix}.destination").as_str()),
        )?
    };
    let copied_value = codegen_context.builder.build_load(
        source_slot,
        &env.next_name(format!("{name_prefix}.value").as_str()),
    )?;
    retain_rc_element_if_needed(
        codegen_context,
        env,
        element_core_type,
        copied_value,
        name_prefix,
    )?;
    codegen_context
        .builder
        .build_store(destination_slot, copied_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name(format!("{name_prefix}.next").as_str()),
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
