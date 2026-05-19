#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use super::super::current_function;
use super::helpers::{
    allocate_array_with_capacity, resolve_array_identifier_binding, set_array_payload_length,
    validate_array_operation_metadata,
};
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use inkwell::AddressSpace;
use inkwell::types::BasicType;
use inkwell::values::BasicValueEnum;

#[expect(
    clippy::too_many_lines,
    clippy::pattern_type_mismatch,
    reason = "zip lowering builds a dedicated binary loop that constructs Pair elements"
)]
pub(super) fn codegen_array_zip_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    receiver: &Expr,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if args.len() != 1 {
        return Err(CodegenError::new(format!(
            "array method 'zip' expects exactly 1 argument but received {}",
            args.len()
        )));
    }

    let (_left_name, left_binding, left_base_ptr, left_length_value, left_capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "zip", receiver)?;
    validate_array_operation_metadata(
        codegen_context,
        env,
        "zip",
        left_base_ptr,
        left_length_value,
        left_capacity_value,
    )?;
    let left_element_core_type = match &left_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "zip expects an array receiver, found '{}'",
                left_binding.core_type
            )));
        }
    };

    let (_right_name, right_binding, right_base_ptr, right_length_value, right_capacity_value) =
        resolve_array_identifier_binding(codegen_context, env, "zip", &args[0])?;
    validate_array_operation_metadata(
        codegen_context,
        env,
        "zip",
        right_base_ptr,
        right_length_value,
        right_capacity_value,
    )?;
    let right_element_core_type = match &right_binding.core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        _ => {
            return Err(CodegenError::new(format!(
                "zip expects an array argument, found '{}'",
                right_binding.core_type
            )));
        }
    };

    let zipped_length = codegen_context
        .builder
        .build_select(
            codegen_context.builder.build_int_compare(
                inkwell::IntPredicate::ULT,
                left_length_value,
                right_length_value,
                &env.next_name("zip.min.cond"),
            )?,
            left_length_value,
            right_length_value,
            &env.next_name("zip.min"),
        )?
        .into_int_value();
    let pair_core_type = CoreType::Generic {
        name: "Pair".to_owned(),
        type_args: vec![left_element_core_type, right_element_core_type],
    };
    let result_pointer_type = core_type_to_llvm(codegen_context.context, &pair_core_type)
        .ptr_type(AddressSpace::default());
    let result_alloca = codegen_context
        .builder
        .build_alloca(result_pointer_type, &env.next_name("zip.result.ptr"))?;

    let current_function = current_function(codegen_context)?;
    let loop_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("zip.loop"));
    let body_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("zip.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_function, &env.next_name("zip.exit"));
    let (result_array, result_data_ptr) = allocate_array_with_capacity(
        codegen_context,
        env,
        "zip",
        &pair_core_type,
        zipped_length,
    )?;
    codegen_context
        .builder
        .build_store(result_alloca, result_array)?;
    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name("zip.index"),
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
        .build_load(index_alloca, &env.next_name("zip.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::ULT,
        index_value,
        zipped_length,
        &env.next_name("zip.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    // SAFETY: the loop guard ensures `index_value < zipped_length <= left_length_value`, so this left-array access is in bounds.
    let left_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            left_base_ptr,
            &[index_value],
            &env.next_name("zip.left.src"),
        )?
    };
    // SAFETY: the loop guard ensures `index_value < zipped_length <= right_length_value`, so this right-array access is in bounds.
    let right_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            right_base_ptr,
            &[index_value],
            &env.next_name("zip.right.src"),
        )?
    };
    let left_value = codegen_context
        .builder
        .build_load(left_slot, &env.next_name("zip.left.value"))?;
    let right_value = codegen_context
        .builder
        .build_load(right_slot, &env.next_name("zip.right.value"))?;
    let pair_struct_type =
        core_type_to_llvm(codegen_context.context, &pair_core_type).into_struct_type();
    let mut pair_value = pair_struct_type.get_undef();
    pair_value = codegen_context
        .builder
        .build_insert_value(pair_value, left_value, 0, &env.next_name("zip.pair.first"))?
        .into_struct_value();
    pair_value = codegen_context
        .builder
        .build_insert_value(
            pair_value,
            right_value,
            1,
            &env.next_name("zip.pair.second"),
        )?
        .into_struct_value();
    // SAFETY: the destination buffer is allocated with `zipped_length` slots, and the loop guard keeps the write index in bounds.
    let destination_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            result_data_ptr,
            &[index_value],
            &env.next_name("zip.dst"),
        )?
    };
    codegen_context
        .builder
        .build_store(destination_slot, pair_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name("zip.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    set_array_payload_length(codegen_context, env, result_array, zipped_length, "zip")?;
    let final_result_ptr = codegen_context
        .builder
        .build_load(result_alloca, &env.next_name("zip.result.final"))?;
    Ok(final_result_ptr)
}
