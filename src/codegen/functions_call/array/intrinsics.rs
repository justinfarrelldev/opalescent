#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::super::ast_type_to_core_type_for_signature;
use super::super::functions_call_helpers::{current_function, llvm_basic_type_to_core_type};
use super::super::resolve_callee_function;
use super::helpers::{
    allocate_array_with_capacity, compute_next_array_capacity, copy_existing_array_elements,
    infer_array_callback_return_core_type, infer_map_callback_return_core_type,
    resolve_array_identifier_binding, retain_rc_element_if_needed, set_array_payload_length,
    store_array_binding, trap_on_invalid_array_state, validate_array_operation_metadata,
};
use crate::ast::{Expr, LiteralValue};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{codegen_expression, CodegenEnv};
use crate::codegen::expressions_array::infer_expression_core_type;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue};

pub(super) fn is_array_intrinsic_name(name: &str) -> bool {
    matches!(name, "append" | "array_filled" | "reserve" | "clear")
}

pub(super) fn codegen_array_intrinsic_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    intrinsic_name: &str,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match intrinsic_name {
        "append" => codegen_append_call(codegen_context, env, args),
        "array_filled" => codegen_array_filled_call(codegen_context, env, args),
        "reserve" => codegen_array_reserve_call(codegen_context, env, args),
        "clear" => codegen_array_clear_call(codegen_context, env, args),
        _ => Err(CodegenError::new(format!(
            "array intrinsic '{intrinsic_name}' is not implemented"
        ))),
    }
}

pub(super) fn codegen_array_member_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    member: &str,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match member {
        "push" => codegen_array_push_call(codegen_context, env, receiver, args),
        "pop" => codegen_array_pop_call(codegen_context, env, receiver, args),
        "map" => codegen_array_map_call(codegen_context, env, receiver, args),
        "filter" => codegen_array_filter_call(codegen_context, env, receiver, args),
        "reduce" => codegen_array_reduce_call(codegen_context, env, receiver, args),
        "zip" => crate::codegen::functions_call::array::zip::codegen_array_zip_call(
            codegen_context,
            env,
            receiver,
            args,
        ),
        _ => Err(CodegenError::new(format!(
            "array method '{member}' is not implemented yet"
        ))),
    }
}

pub(super) fn codegen_append_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 2 {
        return Err(CodegenError::new(format!(
            "append expects exactly 2 arguments but received {}",
            args.len()
        )));
    }

    let result_ptr = lower_array_append_operation(codegen_context, env, "append", &args[0], &args[1])?;
    Ok(result_ptr.as_basic_value_enum())
}

fn codegen_array_filled_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 2 {
        return Err(CodegenError::new(format!(
            "array_filled expects exactly 2 arguments but received {}",
            args.len()
        )));
    }

    let length_value = codegen_expression(codegen_context, env, &args[0], Some(&CoreType::Int64))?.into_int_value();
    let is_negative = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SLT,
        length_value,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("array_filled.length.negative"),
    )?;
    trap_on_invalid_array_state(
        codegen_context,
        env,
        is_negative,
        "array_filled length must be non-negative",
        "array_filled.length",
    )?;

    let element_core_type = infer_array_filled_element_core_type(env, &args[1]);
    let filled_value = codegen_expression(codegen_context, env, &args[1], Some(&element_core_type))?;

    let (result_array, result_ptr) = allocate_array_with_capacity(
        codegen_context,
        env,
        "array_filled",
        &element_core_type,
        length_value,
    )?;

    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("array_filled.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("array_filled.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("array_filled.exit"));
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("array_filled.index"),
    )?;
    codegen_context.builder.build_store(index_alloca, codegen_context.context.i64_type().const_zero())?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, &env.next_name("array_filled.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        length_value,
        &env.next_name("array_filled.cond"),
    )?;
    codegen_context.builder.build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: `index_value < length_value` from the loop condition, so the slot is in bounds.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            result_ptr,
            &[index_value],
            &env.next_name("array_filled.slot"),
        )?
    };
    retain_rc_element_if_needed(codegen_context, env, &element_core_type, filled_value, "array_filled.value")?;
    codegen_context.builder.build_store(destination_slot, filled_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("array_filled.next"),
    )?;
    codegen_context.builder.build_store(index_alloca, next_index)?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    set_array_payload_length(codegen_context, env, result_array, length_value, "array_filled")?;
    Ok(result_array.as_basic_value_enum())
}

fn codegen_array_reserve_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 2 {
        return Err(CodegenError::new(format!(
            "reserve expects exactly 2 arguments but received {}",
            args.len()
        )));
    }

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "reserve", &args[0])?;
    let requested_capacity = codegen_expression(codegen_context, env, &args[1], Some(&CoreType::Int64))?.into_int_value();
    let requested_negative = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SLT,
        requested_capacity,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("reserve.capacity.negative"),
    )?;
    trap_on_invalid_array_state(
        codegen_context,
        env,
        requested_negative,
        "reserve capacity must be non-negative",
        "reserve.capacity",
    )?;

    validate_array_operation_metadata(codegen_context, env, "reserve", base_ptr, length_value, capacity_value)?;

    let element_core_type = match array_binding.core_type {
        CoreType::Array(ref element_type) => element_type.as_ref().clone(),
        _ => return Err(CodegenError::new(String::from("reserve expects an array receiver"))),
    };

    let requested_greater = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::UGT,
        requested_capacity,
        capacity_value,
        &env.next_name("reserve.capacity.requested_greater"),
    )?;
    let result_capacity = codegen_context
        .builder
        .build_select(
            requested_greater,
            requested_capacity,
            capacity_value,
            &env.next_name("reserve.capacity.max"),
        )?
        .into_int_value();
    let (result_array, result_ptr) = allocate_array_with_capacity(codegen_context, env, "reserve", &element_core_type, result_capacity)?;
    copy_existing_array_elements(codegen_context, env, &element_core_type, base_ptr, result_ptr, length_value)?;
    set_array_payload_length(codegen_context, env, result_array, length_value, "reserve")?;
    Ok(result_array.as_basic_value_enum())
}

fn codegen_array_clear_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "clear expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "clear", &args[0])?;
    validate_array_operation_metadata(codegen_context, env, "clear", base_ptr, length_value, capacity_value)?;
    let element_core_type = match array_binding.core_type {
        CoreType::Array(ref element_type) => element_type.as_ref().clone(),
        _ => return Err(CodegenError::new(String::from("clear expects an array receiver"))),
    };

    let (result_array, _result_ptr) = allocate_array_with_capacity(codegen_context, env, "clear", &element_core_type, capacity_value)?;
    set_array_payload_length(codegen_context, env, result_array, codegen_context.context.i64_type().const_zero(), "clear")?;
    Ok(result_array.as_basic_value_enum())
}

fn infer_array_filled_element_core_type(env: &CodegenEnv<'_>, value_expr: &Expr) -> CoreType {
    if let Expr::Cast { ref target_type, .. } = *value_expr {
        if let Ok(core_type) = ast_type_to_core_type_for_signature(target_type) {
            return core_type;
        }
    }
    if let Expr::Literal { value: LiteralValue::Integer(_), .. } = *value_expr {
        return CoreType::Int64;
    }
    infer_expression_core_type(env, value_expr).unwrap_or(CoreType::Int64)
}

fn lower_array_append_operation<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    array_expr: &Expr,
    element_expr: &Expr,
) -> Result<PointerValue<'context>, CodegenError> {
    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, operation, array_expr)?;
    let element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| CodegenError::new(format!("{operation} expects an array receiver, found '{}'", array_binding.core_type)))?
        .clone();

    let appended_value = codegen_expression(codegen_context, env, element_expr, Some(&element_core_type))?;
    validate_array_operation_metadata(codegen_context, env, operation, base_ptr, length_value, capacity_value)?;
    let array_length = length_value;
    let next_length = codegen_context.builder.build_int_add(
        array_length,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("append.len"),
    )?;
    let next_length_overflowed = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULE,
        next_length,
        array_length,
        &env.next_name("append.len.overflow"),
    )?;
    let next_length_overflow_message = format!("{operation} array length overflow");
    trap_on_invalid_array_state(codegen_context, env, next_length_overflowed, next_length_overflow_message.as_str(), "append.len.overflow")?;
    let next_capacity = compute_next_array_capacity(codegen_context, env, operation, capacity_value, next_length)?;
    let (result_array, result_ptr) = allocate_array_with_capacity(codegen_context, env, operation, &element_core_type, next_capacity)?;
    copy_existing_array_elements(codegen_context, env, &element_core_type, base_ptr, result_ptr, array_length)?;

    // SAFETY: the copy step allocated `next_length` capacity and `array_length` is the append slot.
    let appended_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(result_ptr, &[array_length], &env.next_name("append.slot"))?
    };
    retain_rc_element_if_needed(codegen_context, env, &element_core_type, appended_value, "append.value")?;
    codegen_context.builder.build_store(appended_slot, appended_value)?;
    set_array_payload_length(codegen_context, env, result_array, next_length, operation)?;
    Ok(result_array)
}

fn codegen_array_push_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "array method 'push' expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let Expr::Identifier { name: ref receiver_name, .. } = *receiver else {
        return Err(CodegenError::new(String::from("array member calls currently require identifier receivers")));
    };

    let result_ptr = lower_array_append_operation(codegen_context, env, "push", receiver, &args[0])?;
    store_array_binding(codegen_context, env, receiver_name.as_str(), result_ptr, "push")?;
    Ok(codegen_context.context.struct_type(&[], false).const_zero().as_basic_value_enum())
}

fn codegen_array_pop_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if !args.is_empty() {
        return Err(CodegenError::new(format!(
            "array method 'pop' expects exactly 0 arguments but received {}",
            args.len()
        )));
    }

    let Expr::Identifier { name: ref receiver_name, .. } = *receiver else {
        return Err(CodegenError::new(String::from("array member calls currently require identifier receivers")));
    };

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "pop", receiver)?;
    validate_array_operation_metadata(codegen_context, env, "pop", base_ptr, length_value, capacity_value)?;
    let element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| CodegenError::new(format!("pop expects an array receiver, found '{}'", array_binding.core_type)))?
        .clone();

    let current_function = current_function(codegen_context)?;
    let pop_value_block = codegen_context.context.append_basic_block(current_function, &env.next_name("pop.value"));
    let pop_empty_block = codegen_context.context.append_basic_block(current_function, &env.next_name("pop.empty"));
    let is_empty = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        length_value,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("pop.is_empty"),
    )?;
    codegen_context.builder.build_conditional_branch(is_empty, pop_empty_block, pop_value_block)?;

    codegen_context.builder.position_at_end(pop_empty_block);
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(codegen_context, "opal_runtime_error")
        .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context.builder.build_global_string_ptr("pop on empty array", &env.next_name("pop.msg"))?.as_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(runtime_fn, &[msg.into()], &env.next_name("pop.trap"))?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(pop_value_block);
    let next_length = codegen_context.builder.build_int_sub(
        length_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("pop.len"),
    )?;
    // SAFETY: `next_length` is the last valid index after decrementing a non-empty array.
    let popped_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(base_ptr, &[next_length], &env.next_name("pop.slot"))?
    };
    let popped_value = codegen_context.builder.build_load(popped_slot, &env.next_name("pop.value.load"))?;
    let (result_array, result_ptr) = allocate_array_with_capacity(codegen_context, env, "pop", &element_core_type, capacity_value)?;
    copy_existing_array_elements(codegen_context, env, &element_core_type, base_ptr, result_ptr, next_length)?;
    set_array_payload_length(codegen_context, env, result_array, next_length, "pop")?;
    store_array_binding(codegen_context, env, receiver_name.as_str(), result_array, "pop")?;
    Ok(popped_value)
}

fn codegen_array_map_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "array method 'map' expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "map", receiver)?;
    validate_array_operation_metadata(codegen_context, env, "map", base_ptr, length_value, capacity_value)?;
    let _input_element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| CodegenError::new(format!("map expects an array receiver, found '{}'", array_binding.core_type)))?
        .clone();

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[0], None)?;
    if let Some(block) = insertion_block { codegen_context.builder.position_at_end(block); }
    let output_element_core_type = infer_map_callback_return_core_type(env, &args[0])
        .or_else(|| callback_function.get_type().get_return_type().map(llvm_basic_type_to_core_type))
        .ok_or_else(|| CodegenError::new(String::from("array map callback must declare a concrete return type")))?;
    if matches!(output_element_core_type, CoreType::Variable(_)) {
        return Err(CodegenError::new(String::from("array map callback return type remained unresolved during code generation")));
    }

    let (result_array, result_data_ptr) = allocate_array_with_capacity(codegen_context, env, "map", &output_element_core_type, length_value)?;
    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context.context.append_basic_block(current_function, &env.next_name("map.loop"));
    let body_block = codegen_context.context.append_basic_block(current_function, &env.next_name("map.body"));
    let exit_block = codegen_context.context.append_basic_block(current_function, &env.next_name("map.exit"));
    let index_alloca = codegen_context.builder.build_alloca(codegen_context.context.i64_type(), &env.next_name("map.index"))?;
    codegen_context.builder.build_store(index_alloca, codegen_context.context.i64_type().const_zero())?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context.builder.build_load(index_alloca, &env.next_name("map.index.load"))?.into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(inkwell::IntPredicate::ULT, index_value, length_value, &env.next_name("map.cond"))?;
    codegen_context.builder.build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the loop guard ensures `index_value < length_value`.
    let source_slot = unsafe { codegen_context.builder.build_in_bounds_gep(base_ptr, &[index_value], &env.next_name("map.src"))? };
    let source_value = codegen_context.builder.build_load(source_slot, &env.next_name("map.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> = vec![source_value.into()];
    if let Expr::Lambda { ref captured_variables, .. } = args[0] {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!("map callback capture '{capture}' not found in scope")));
            };
            let captured_value = codegen_context.builder.build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let mapped_value = codegen_context.builder.build_call(callback_function, callback_args.as_slice(), &env.next_name("map.call"))?.try_as_basic_value().basic().ok_or_else(|| CodegenError::new(String::from("array map callback must return a value")))?;
    // SAFETY: destination buffer was allocated for `length_value` slots and the loop index is in bounds.
    let destination_slot = unsafe { codegen_context.builder.build_in_bounds_gep(result_data_ptr, &[index_value], &env.next_name("map.dst"))? };
    codegen_context.builder.build_store(destination_slot, mapped_value)?;
    let next_index = codegen_context.builder.build_int_add(index_value, codegen_context.context.i64_type().const_int(1, false), &env.next_name("map.next"))?;
    codegen_context.builder.build_store(index_alloca, next_index)?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    set_array_payload_length(codegen_context, env, result_array, length_value, "map")?;
    Ok(result_array.as_basic_value_enum())
}

fn codegen_array_filter_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "array method 'filter' expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "filter", receiver)?;
    validate_array_operation_metadata(codegen_context, env, "filter", base_ptr, length_value, capacity_value)?;
    let input_element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| CodegenError::new(format!("filter expects an array receiver, found '{}'", array_binding.core_type)))?
        .clone();

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[0], None)?;
    if let Some(block) = insertion_block { codegen_context.builder.position_at_end(block); }
    let predicate_return_core_type = infer_array_callback_return_core_type(env, &args[0])
        .or_else(|| callback_function.get_type().get_return_type().map(llvm_basic_type_to_core_type))
        .ok_or_else(|| CodegenError::new(String::from("array filter predicate must declare a concrete return type")))?;
    if predicate_return_core_type != CoreType::Boolean {
        return Err(CodegenError::new(String::from("array filter predicate must return boolean")));
    }

    let index_alloca = codegen_context.builder.build_alloca(codegen_context.context.i64_type(), &env.next_name("filter.index"))?;
    let write_index_alloca = codegen_context.builder.build_alloca(codegen_context.context.i64_type(), &env.next_name("filter.write_index"))?;
    codegen_context.builder.build_store(index_alloca, codegen_context.context.i64_type().const_zero())?;
    codegen_context.builder.build_store(write_index_alloca, codegen_context.context.i64_type().const_zero())?;

    let current_function = current_function(codegen_context)?;
    let (result_array, result_data_ptr) = allocate_array_with_capacity(codegen_context, env, "filter", &input_element_core_type, length_value)?;
    let loop_block = codegen_context.context.append_basic_block(current_function, &env.next_name("filter.loop"));
    let body_block = codegen_context.context.append_basic_block(current_function, &env.next_name("filter.body"));
    let keep_block = codegen_context.context.append_basic_block(current_function, &env.next_name("filter.keep"));
    let skip_block = codegen_context.context.append_basic_block(current_function, &env.next_name("filter.skip"));
    let exit_block = codegen_context.context.append_basic_block(current_function, &env.next_name("filter.exit"));
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context.builder.build_load(index_alloca, &env.next_name("filter.index.load"))?.into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(inkwell::IntPredicate::ULT, index_value, length_value, &env.next_name("filter.cond"))?;
    codegen_context.builder.build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the loop guard ensures `index_value < length_value`.
    let source_slot = unsafe { codegen_context.builder.build_in_bounds_gep(base_ptr, &[index_value], &env.next_name("filter.src"))? };
    let source_value = codegen_context.builder.build_load(source_slot, &env.next_name("filter.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> = vec![source_value.into()];
    if let Expr::Lambda { ref captured_variables, .. } = args[0] {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!("filter callback capture '{capture}' not found in scope")));
            };
            let captured_value = codegen_context.builder.build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let predicate_value = codegen_context.builder.build_call(callback_function, callback_args.as_slice(), &env.next_name("filter.call"))?.try_as_basic_value().basic().ok_or_else(|| CodegenError::new(String::from("array filter predicate must return a value")))?;
    if !predicate_value.is_int_value() {
        return Err(CodegenError::new(String::from("array filter predicate must lower to a boolean value")));
    }
    codegen_context.builder.build_conditional_branch(predicate_value.into_int_value(), keep_block, skip_block)?;

    codegen_context.builder.position_at_end(keep_block);
    let write_index_value = codegen_context.builder.build_load(write_index_alloca, &env.next_name("filter.write_index.load"))?.into_int_value();
    // SAFETY: `write_index_value` counts kept elements and stays within the preallocated buffer.
    let destination_slot = unsafe { codegen_context.builder.build_in_bounds_gep(result_data_ptr, &[write_index_value], &env.next_name("filter.dst"))? };
    codegen_context.builder.build_store(destination_slot, source_value)?;
    let next_write_index = codegen_context.builder.build_int_add(write_index_value, codegen_context.context.i64_type().const_int(1, false), &env.next_name("filter.write_index.next"))?;
    codegen_context.builder.build_store(write_index_alloca, next_write_index)?;
    codegen_context.builder.build_unconditional_branch(skip_block)?;

    codegen_context.builder.position_at_end(skip_block);
    let next_index = codegen_context.builder.build_int_add(index_value, codegen_context.context.i64_type().const_int(1, false), &env.next_name("filter.next"))?;
    codegen_context.builder.build_store(index_alloca, next_index)?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    let final_length = codegen_context.builder.build_load(write_index_alloca, &env.next_name("filter.length.final"))?.into_int_value();
    set_array_payload_length(codegen_context, env, result_array, final_length, "filter")?;
    Ok(result_array.as_basic_value_enum())
}

fn codegen_array_reduce_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 2 {
        return Err(CodegenError::new(format!(
            "array method 'reduce' expects exactly 2 arguments but received {}",
            args.len()
        )));
    }

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "reduce", receiver)?;
    validate_array_operation_metadata(codegen_context, env, "reduce", base_ptr, length_value, capacity_value)?;
    let _input_element_core_type = array_binding
        .core_type
        .array_element_type()
        .ok_or_else(|| CodegenError::new(format!("reduce expects an array receiver, found '{}'", array_binding.core_type)))?
        .clone();

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[1], None)?;
    if let Some(block) = insertion_block { codegen_context.builder.position_at_end(block); }
    let accumulator_core_type = infer_array_callback_return_core_type(env, &args[1])
        .or_else(|| callback_function.get_type().get_return_type().map(llvm_basic_type_to_core_type))
        .ok_or_else(|| CodegenError::new(String::from("array reduce callback must declare a concrete return type")))?;
    if matches!(accumulator_core_type, CoreType::Variable(_)) {
        return Err(CodegenError::new(String::from("array reduce callback return type remained unresolved during code generation")));
    }

    let initial_value = codegen_expression(codegen_context, env, &args[0], Some(&accumulator_core_type))?;
    let accumulator_alloca = codegen_context.builder.build_alloca(initial_value.get_type(), &env.next_name("reduce.acc"))?;
    codegen_context.builder.build_store(accumulator_alloca, initial_value)?;
    let index_alloca = codegen_context.builder.build_alloca(codegen_context.context.i64_type(), &env.next_name("reduce.index"))?;
    codegen_context.builder.build_store(index_alloca, codegen_context.context.i64_type().const_zero())?;

    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context.context.append_basic_block(current_function, &env.next_name("reduce.loop"));
    let body_block = codegen_context.context.append_basic_block(current_function, &env.next_name("reduce.body"));
    let exit_block = codegen_context.context.append_basic_block(current_function, &env.next_name("reduce.exit"));
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context.builder.build_load(index_alloca, &env.next_name("reduce.index.load"))?.into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(inkwell::IntPredicate::ULT, index_value, length_value, &env.next_name("reduce.cond"))?;
    codegen_context.builder.build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    let accumulator_value = codegen_context.builder.build_load(accumulator_alloca, &env.next_name("reduce.acc.load"))?;
    // SAFETY: the loop guard ensures `index_value < length_value`.
    let source_slot = unsafe { codegen_context.builder.build_in_bounds_gep(base_ptr, &[index_value], &env.next_name("reduce.src"))? };
    let source_value = codegen_context.builder.build_load(source_slot, &env.next_name("reduce.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> = vec![accumulator_value.into(), source_value.into()];
    if let Expr::Lambda { ref captured_variables, .. } = args[1] {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!("reduce callback capture '{capture}' not found in scope")));
            };
            let captured_value = codegen_context.builder.build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let next_accumulator = codegen_context.builder.build_call(callback_function, callback_args.as_slice(), &env.next_name("reduce.call"))?.try_as_basic_value().basic().ok_or_else(|| CodegenError::new(String::from("array reduce callback must return a value")))?;
    codegen_context.builder.build_store(accumulator_alloca, next_accumulator)?;
    let next_index = codegen_context.builder.build_int_add(index_value, codegen_context.context.i64_type().const_int(1, false), &env.next_name("reduce.next"))?;
    codegen_context.builder.build_store(index_alloca, next_index)?;
    codegen_context.builder.build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    codegen_context.builder.build_load(accumulator_alloca, &env.next_name("reduce.result")).map_err(Into::into)
}
