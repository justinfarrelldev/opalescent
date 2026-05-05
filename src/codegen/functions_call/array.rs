#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use self::helpers::{
    allocate_array_buffer, compute_next_array_capacity, copy_existing_array_elements,
    infer_array_callback_return_core_type, infer_map_callback_return_core_type,
    resolve_array_identifier_binding, store_array_binding_with_metadata,
    trap_on_invalid_array_state, validate_array_operation_metadata,
};
use super::functions_call_helpers::{current_function, llvm_basic_type_to_core_type};
use super::resolve_callee_function;
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{ArrayMetadata, CodegenEnv, codegen_expression};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, PointerValue};

#[path = "array/helpers.rs"]
mod helpers;

pub(super) fn is_append_intrinsic_name(name: &str) -> bool {
    matches!(name, "append")
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

    let (result_ptr, metadata) =
        lower_array_append_operation(codegen_context, env, "append", &args[0], &args[1])?;
    env.set_pending_array_metadata(Some(metadata));

    Ok(result_ptr.as_basic_value_enum())
}

#[expect(
    clippy::pattern_type_mismatch,
    reason = "matching borrowed core types keeps lowering code readable"
)]
fn lower_array_append_operation<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operation: &str,
    array_expr: &Expr,
    element_expr: &Expr,
) -> Result<(PointerValue<'context>, ArrayMetadata<'context>), CodegenError> {
    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, operation, array_expr)?;
    let element_core_type = match &array_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "{operation} expects an array receiver, found '{}'",
                array_binding.core_type
            )));
        }
    };

    let appended_value =
        codegen_expression(codegen_context, env, element_expr, Some(&element_core_type))?;
    validate_array_operation_metadata(
        codegen_context,
        env,
        operation,
        base_ptr,
        length_value,
        capacity_value,
    )?;
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
    trap_on_invalid_array_state(
        codegen_context,
        env,
        next_length_overflowed,
        next_length_overflow_message.as_str(),
        "append.len.overflow",
    )?;
    let next_capacity =
        compute_next_array_capacity(codegen_context, env, operation, capacity_value, next_length)?;
    let result_ptr = allocate_array_buffer(
        codegen_context,
        env,
        operation,
        &element_core_type,
        next_capacity,
    )?;

    copy_existing_array_elements(codegen_context, env, base_ptr, result_ptr, array_length)?;

    // SAFETY: `result_ptr` points to a newly allocated buffer with capacity at least `next_length`,
    // so indexing the append slot at `array_length` is in bounds.
    let appended_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            result_ptr,
            &[array_length],
            &env.next_name("append.slot"),
        )?
    };
    codegen_context
        .builder
        .build_store(appended_slot, appended_value)?;

    Ok((
        result_ptr,
        ArrayMetadata {
            length: next_length,
            capacity: next_capacity,
        },
    ))
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

    let Expr::Identifier {
        name: ref receiver_name,
        ..
    } = *receiver
    else {
        return Err(CodegenError::new(String::from(
            "array member calls currently require identifier receivers",
        )));
    };

    let (result_ptr, metadata) =
        lower_array_append_operation(codegen_context, env, "push", receiver, &args[0])?;
    store_array_binding_with_metadata(
        codegen_context,
        env,
        receiver_name.as_str(),
        result_ptr,
        metadata,
        "push",
    )?;

    Ok(codegen_context
        .context
        .struct_type(&[], false)
        .const_zero()
        .as_basic_value_enum())
}

#[expect(
    clippy::too_many_lines,
    clippy::pattern_type_mismatch,
    reason = "pop lowering needs explicit runtime trap and metadata updates"
)]
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

    let Expr::Identifier {
        name: ref receiver_name,
        ..
    } = *receiver
    else {
        return Err(CodegenError::new(String::from(
            "array member calls currently require identifier receivers",
        )));
    };

    let (_array_name, array_binding, base_ptr, length_value, capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "pop", receiver)?;
    validate_array_operation_metadata(
        codegen_context,
        env,
        "pop",
        base_ptr,
        length_value,
        capacity_value,
    )?;
    let element_core_type = match &array_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "pop expects an array receiver, found '{}'",
                array_binding.core_type
            )));
        }
    };

    let current_function = current_function(codegen_context)?;
    let pop_value_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("pop.value"));
    let pop_empty_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("pop.empty"));
    let is_empty = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        length_value,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("pop.is_empty"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(is_empty, pop_empty_block, pop_value_block)?;

    codegen_context.builder.position_at_end(pop_empty_block);
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context
        .builder
        .build_global_string_ptr("pop on empty array", &env.next_name("pop.msg"))?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        runtime_fn,
        &[msg.into()],
        &env.next_name("pop.trap"),
    )?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(pop_value_block);
    let next_length = codegen_context.builder.build_int_sub(
        length_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("pop.len"),
    )?;
    // SAFETY: `next_length` is computed from a non-empty array, so it is a valid last-element index.
    let popped_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[next_length],
            &env.next_name("pop.slot"),
        )?
    };
    let popped_value = codegen_context
        .builder
        .build_load(popped_slot, &env.next_name("pop.value.load"))?;
    let result_ptr = allocate_array_buffer(
        codegen_context,
        env,
        "pop",
        &element_core_type,
        capacity_value,
    )?;
    copy_existing_array_elements(codegen_context, env, base_ptr, result_ptr, next_length)?;
    store_array_binding_with_metadata(
        codegen_context,
        env,
        receiver_name.as_str(),
        result_ptr,
        ArrayMetadata {
            length: next_length,
            capacity: capacity_value,
        },
        "pop",
    )?;

    Ok(popped_value)
}

#[expect(
    clippy::too_many_lines,
    clippy::pattern_type_mismatch,
    reason = "map lowering builds dedicated loop/control-flow blocks for each element"
)]
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
    validate_array_operation_metadata(
        codegen_context,
        env,
        "map",
        base_ptr,
        length_value,
        capacity_value,
    )?;
    let _input_element_core_type = match &array_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "map expects an array receiver, found '{}'",
                array_binding.core_type
            )));
        }
    };

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[0], None)?;
    if let Some(block) = insertion_block {
        codegen_context.builder.position_at_end(block);
    }
    let output_element_core_type = infer_map_callback_return_core_type(env, &args[0])
        .or_else(|| {
            callback_function
                .get_type()
                .get_return_type()
                .map(llvm_basic_type_to_core_type)
        })
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "array map callback must declare a concrete return type",
            ))
        })?;
    if matches!(output_element_core_type, CoreType::Variable(_)) {
        return Err(CodegenError::new(String::from(
            "array map callback return type remained unresolved during code generation",
        )));
    }

    let result_pointer_type = core_type_to_llvm(codegen_context.context, &output_element_core_type)
        .ptr_type(AddressSpace::default());
    let result_alloca = codegen_context
        .builder
        .build_alloca(result_pointer_type, &env.next_name("map.result.ptr"))?;
    let current_function = current_function(codegen_context)?;
    let empty_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("map.empty"));
    let non_empty_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("map.non_empty"));
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("map.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("map.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("map.exit"));
    let is_empty = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        length_value,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("map.is_empty"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(is_empty, empty_block, non_empty_block)?;

    codegen_context.builder.position_at_end(empty_block);
    codegen_context
        .builder
        .build_store(result_alloca, result_pointer_type.const_null())?;
    codegen_context
        .builder
        .build_unconditional_branch(exit_block)?;

    codegen_context.builder.position_at_end(non_empty_block);
    let result_ptr = allocate_array_buffer(
        codegen_context,
        env,
        "map",
        &output_element_core_type,
        length_value,
    )?;
    codegen_context
        .builder
        .build_store(result_alloca, result_ptr)?;
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("map.index"),
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
        .build_load(index_alloca, &env.next_name("map.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        length_value,
        &env.next_name("map.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the loop guard ensures `index_value < length_value`, so this source element access is in bounds.
    let source_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[index_value],
            &env.next_name("map.src"),
        )?
    };
    let source_value = codegen_context
        .builder
        .build_load(source_slot, &env.next_name("map.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> = vec![source_value.into()];
    if let Expr::Lambda {
        ref captured_variables,
        ..
    } = args[0]
    {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!(
                    "map callback capture '{capture}' not found in scope"
                )));
            };
            let captured_value = codegen_context
                .builder
                .build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let mapped_value = codegen_context
        .builder
        .build_call(
            callback_function,
            callback_args.as_slice(),
            &env.next_name("map.call"),
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("array map callback must return a value")))?;
    let destination_ptr = codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("map.result.load"))?
        .into_pointer_value();
    // SAFETY: the destination buffer is allocated with `length_value` slots, and the loop guard keeps the index in bounds.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            destination_ptr,
            &[index_value],
            &env.next_name("map.dst"),
        )?
    };
    codegen_context
        .builder
        .build_store(destination_slot, mapped_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("map.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    let final_result_ptr = codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("map.result.final"))?;
    env.set_pending_array_metadata(Some(ArrayMetadata {
        length: length_value,
        capacity: length_value,
    }));
    Ok(final_result_ptr)
}

#[expect(
    clippy::too_many_lines,
    clippy::pattern_type_mismatch,
    reason = "filter lowering builds a dedicated loop with packed writes for matching elements"
)]
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
    validate_array_operation_metadata(
        codegen_context,
        env,
        "filter",
        base_ptr,
        length_value,
        capacity_value,
    )?;
    let input_element_core_type = match &array_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "filter expects an array receiver, found '{}'",
                array_binding.core_type
            )));
        }
    };

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[0], None)?;
    if let Some(block) = insertion_block {
        codegen_context.builder.position_at_end(block);
    }
    let predicate_return_core_type = infer_array_callback_return_core_type(env, &args[0])
        .or_else(|| {
            callback_function
                .get_type()
                .get_return_type()
                .map(llvm_basic_type_to_core_type)
        })
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "array filter predicate must declare a concrete return type",
            ))
        })?;
    if predicate_return_core_type != CoreType::Boolean {
        return Err(CodegenError::new(String::from(
            "array filter predicate must return boolean",
        )));
    }

    let result_pointer_type = core_type_to_llvm(codegen_context.context, &input_element_core_type)
        .ptr_type(AddressSpace::default());
    let result_alloca = codegen_context
        .builder
        .build_alloca(result_pointer_type, &env.next_name("filter.result.ptr"))?;
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("filter.index"),
    )?;
    let write_index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("filter.write_index"),
    )?;
    codegen_context.builder.build_store(
        index_alloca,
        codegen_context.context.i64_type().const_zero(),
    )?;
    codegen_context.builder.build_store(
        write_index_alloca,
        codegen_context.context.i64_type().const_zero(),
    )?;

    let current_function = current_function(codegen_context)?;
    let empty_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.empty"));
    let non_empty_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.non_empty"));
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.body"));
    let keep_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.keep"));
    let skip_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.skip"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("filter.exit"));
    let is_empty = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::EQ,
        length_value,
        codegen_context.context.i64_type().const_zero(),
        &env.next_name("filter.is_empty"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(is_empty, empty_block, non_empty_block)?;

    codegen_context.builder.position_at_end(empty_block);
    codegen_context
        .builder
        .build_store(result_alloca, result_pointer_type.const_null())?;
    codegen_context
        .builder
        .build_unconditional_branch(exit_block)?;

    codegen_context.builder.position_at_end(non_empty_block);
    let result_ptr = allocate_array_buffer(
        codegen_context,
        env,
        "filter",
        &input_element_core_type,
        length_value,
    )?;
    codegen_context
        .builder
        .build_store(result_alloca, result_ptr)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, &env.next_name("filter.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        length_value,
        &env.next_name("filter.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the loop guard ensures `index_value < length_value`, so this source element access is in bounds.
    let source_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[index_value],
            &env.next_name("filter.src"),
        )?
    };
    let source_value = codegen_context
        .builder
        .build_load(source_slot, &env.next_name("filter.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> = vec![source_value.into()];
    if let Expr::Lambda {
        ref captured_variables,
        ..
    } = args[0]
    {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!(
                    "filter callback capture '{capture}' not found in scope"
                )));
            };
            let captured_value = codegen_context
                .builder
                .build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let predicate_value = codegen_context
        .builder
        .build_call(
            callback_function,
            callback_args.as_slice(),
            &env.next_name("filter.call"),
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| {
            CodegenError::new(String::from("array filter predicate must return a value"))
        })?;
    if !predicate_value.is_int_value() {
        return Err(CodegenError::new(String::from(
            "array filter predicate must lower to a boolean value",
        )));
    }
    codegen_context.builder.build_conditional_branch(
        predicate_value.into_int_value(),
        keep_block,
        skip_block,
    )?;

    codegen_context.builder.position_at_end(keep_block);
    let destination_ptr = codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("filter.result.load"))?
        .into_pointer_value();
    let write_index_value = codegen_context
        .builder
        .build_load(
            write_index_alloca,
            &env.next_name("filter.write_index.load"),
        )?
        .into_int_value();
    // SAFETY: `write_index_value` counts only kept elements and never exceeds the source length used to allocate the result buffer.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            destination_ptr,
            &[write_index_value],
            &env.next_name("filter.dst"),
        )?
    };
    codegen_context
        .builder
        .build_store(destination_slot, source_value)?;
    let next_write_index = codegen_context.builder.build_int_add(
        write_index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("filter.write_index.next"),
    )?;
    codegen_context
        .builder
        .build_store(write_index_alloca, next_write_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(skip_block)?;

    codegen_context.builder.position_at_end(skip_block);
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("filter.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    let final_result_ptr = codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("filter.result.final"))?;
    let final_length = codegen_context
        .builder
        .build_load(write_index_alloca, &env.next_name("filter.length.final"))?
        .into_int_value();
    env.set_pending_array_metadata(Some(ArrayMetadata {
        length: final_length,
        capacity: length_value,
    }));
    Ok(final_result_ptr)
}

#[expect(
    clippy::too_many_lines,
    clippy::pattern_type_mismatch,
    reason = "reduce lowering builds a dedicated loop that threads the seeded accumulator"
)]
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
    validate_array_operation_metadata(
        codegen_context,
        env,
        "reduce",
        base_ptr,
        length_value,
        capacity_value,
    )?;
    let _input_element_core_type = match &array_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "reduce expects an array receiver, found '{}'",
                array_binding.core_type
            )));
        }
    };

    let insertion_block = codegen_context.builder.get_insert_block();
    let callback_function = resolve_callee_function(codegen_context, env, &args[1], None)?;
    if let Some(block) = insertion_block {
        codegen_context.builder.position_at_end(block);
    }
    let accumulator_core_type = infer_array_callback_return_core_type(env, &args[1])
        .or_else(|| {
            callback_function
                .get_type()
                .get_return_type()
                .map(llvm_basic_type_to_core_type)
        })
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "array reduce callback must declare a concrete return type",
            ))
        })?;
    if matches!(accumulator_core_type, CoreType::Variable(_)) {
        return Err(CodegenError::new(String::from(
            "array reduce callback return type remained unresolved during code generation",
        )));
    }

    let initial_value =
        codegen_expression(codegen_context, env, &args[0], Some(&accumulator_core_type))?;
    let accumulator_alloca = codegen_context
        .builder
        .build_alloca(initial_value.get_type(), &env.next_name("reduce.acc"))?;
    codegen_context
        .builder
        .build_store(accumulator_alloca, initial_value)?;
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("reduce.index"),
    )?;
    codegen_context.builder.build_store(
        index_alloca,
        codegen_context.context.i64_type().const_zero(),
    )?;

    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("reduce.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("reduce.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("reduce.exit"));
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, &env.next_name("reduce.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        length_value,
        &env.next_name("reduce.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    let accumulator_value = codegen_context
        .builder
        .build_load(accumulator_alloca, &env.next_name("reduce.acc.load"))?;
    // SAFETY: the loop guard ensures `index_value < length_value`, so this source element access is in bounds.
    let source_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[index_value],
            &env.next_name("reduce.src"),
        )?
    };
    let source_value = codegen_context
        .builder
        .build_load(source_slot, &env.next_name("reduce.value"))?;
    let mut callback_args: Vec<BasicMetadataValueEnum<'context>> =
        vec![accumulator_value.into(), source_value.into()];
    if let Expr::Lambda {
        ref captured_variables,
        ..
    } = args[1]
    {
        for capture in captured_variables {
            let Some(binding) = env.variables.get(capture.as_str()) else {
                return Err(CodegenError::new(format!(
                    "reduce callback capture '{capture}' not found in scope"
                )));
            };
            let captured_value = codegen_context
                .builder
                .build_load(binding.alloca, capture.as_str())?;
            callback_args.push(captured_value.into());
        }
    }
    let next_accumulator = codegen_context
        .builder
        .build_call(
            callback_function,
            callback_args.as_slice(),
            &env.next_name("reduce.call"),
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| {
            CodegenError::new(String::from("array reduce callback must return a value"))
        })?;
    codegen_context
        .builder
        .build_store(accumulator_alloca, next_accumulator)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("reduce.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    codegen_context
        .builder
        .build_load(accumulator_alloca, &env.next_name("reduce.result"))
        .map_err(Into::into)
}
