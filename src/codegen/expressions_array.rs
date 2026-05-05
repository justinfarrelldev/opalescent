#![allow(
    clippy::all,
    clippy::missing_const_for_fn,
    reason = "internal codegen implementation module"
)]

extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{
    ArrayMetadata, CodegenEnv, codegen_expression, current_function,
};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::IntPredicate;
use inkwell::types::BasicType;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, IntValue, PointerValue};

/// Lower array literals while preserving runtime metadata for nested arrays.
pub fn codegen_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let element_core = array_literal_element_core_type(expected_type);
    let count = u32::try_from(elements.len()).map_err(|conversion_error| {
        CodegenError::new(format!("array literal is too large: {conversion_error}"))
    })?;

    match *element_core {
        CoreType::Array(ref nested_element_core) => codegen_nested_array_literal(
            codegen_context,
            env,
            elements,
            nested_element_core.as_ref(),
            count,
        ),
        _ => codegen_flat_array_literal(codegen_context, env, elements, element_core, count),
    }
}

/// Lower array indexing and surface nested row metadata for chained access.
pub fn codegen_array_access<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    object: &Expr,
    index: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let object_core_type = infer_expression_core_type(env, object).ok_or_else(|| {
        CodegenError::new(String::from(
            "array access receiver type could not be inferred",
        ))
    })?;
    let element_core_type = match object_core_type {
        CoreType::Array(element_type) => element_type.as_ref().clone(),
        other => {
            return Err(CodegenError::new(format!(
                "index access expects array receiver, found '{other}'"
            )));
        }
    };
    let (base_ptr, array_length) =
        resolve_array_access_base_and_length(codegen_context, env, object, &element_core_type)?;

    let index_value =
        codegen_expression(codegen_context, env, index, Some(&CoreType::Int64))?.into_int_value();

    emit_array_bounds_check(codegen_context, env, index_value, array_length)?;

    let element_ptr = build_array_element_ptr(codegen_context, env, base_ptr, index_value)?;

    if matches!(element_core_type, CoreType::Array(_)) {
        return load_nested_row_value(codegen_context, env, element_ptr);
    }

    let loaded = codegen_context
        .builder
        .build_load(element_ptr, &env.next_name("array.load"))?;
    if matches!(expected_type, Some(&CoreType::Array(_))) {
        env.set_pending_array_metadata(Some(ArrayMetadata {
            length: array_length,
            capacity: array_length,
        }));
    }
    Ok(loaded)
}

pub fn infer_expression_core_type(env: &CodegenEnv<'_>, expr: &Expr) -> Option<CoreType> {
    match *expr {
        Expr::Identifier { ref name, .. } => env
            .variables
            .get(name.as_str())
            .map(|binding| binding.core_type.clone()),
        Expr::Array { ref elements, .. } => elements.first().map_or_else(
            || Some(CoreType::Array(Box::new(CoreType::Int64))),
            |first| {
                infer_expression_core_type(env, first)
                    .map(|element| CoreType::Array(Box::new(element)))
            },
        ),
        Expr::Index { ref object, .. } => match infer_expression_core_type(env, object.as_ref()) {
            Some(CoreType::Array(element_type)) => Some(element_type.as_ref().clone()),
            _ => None,
        },
        Expr::Member {
            ref object,
            ref member,
            ..
        } => match infer_expression_core_type(env, object.as_ref()) {
            Some(CoreType::Array(_)) if member == "length" => Some(CoreType::Int64),
            Some(CoreType::Array(element_type)) => Some(element_type.as_ref().clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Derive the literal element type from the optional expected array type.
fn array_literal_element_core_type(expected_type: Option<&CoreType>) -> &CoreType {
    expected_type.map_or(&CoreType::Int64, |core_type| match *core_type {
        CoreType::Array(ref element) => element.as_ref(),
        _ => &CoreType::Int64,
    })
}

/// Lower nested array literals into row-value structs carrying ptr/len/cap.
fn codegen_nested_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    nested_element_core: &CoreType,
    count: u32,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let row_struct_type = array_value_struct_type(codegen_context, nested_element_core);
    let array_type = row_struct_type.array_type(count);
    let array_alloca = codegen_context
        .builder
        .build_alloca(array_type, &env.next_name("array.alloca"))?;

    for (index, element_expr) in elements.iter().enumerate() {
        let idx = u64::try_from(index).map_err(|conversion_error| {
            CodegenError::new(format!("array index conversion failed: {conversion_error}"))
        })?;
        let value = codegen_expression(
            codegen_context,
            env,
            element_expr,
            Some(&CoreType::Array(Box::new(nested_element_core.clone()))),
        )?;
        let metadata = env.take_pending_array_metadata().ok_or_else(|| {
            CodegenError::new(String::from(
                "nested array literal element did not publish array metadata",
            ))
        })?;
        let row_value = build_array_value_struct(
            codegen_context,
            env,
            row_struct_type,
            value.into_pointer_value(),
            metadata.length,
            metadata.capacity,
        )?;
        let ptr = build_array_store_ptr(codegen_context, env, array_alloca, idx)?;
        let _store_instruction = codegen_context.builder.build_store(ptr, row_value)?;
    }

    let base_ptr = build_array_base_ptr(codegen_context, env, array_alloca)?;
    publish_array_metadata(codegen_context, env, count);
    Ok(base_ptr.as_basic_value_enum())
}

/// Lower flat array literals and publish their static length metadata.
fn codegen_flat_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    element_core: &CoreType,
    count: u32,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let element_type = core_type_to_llvm(codegen_context.context, element_core);
    let array_type = element_type.array_type(count);
    let array_alloca = codegen_context
        .builder
        .build_alloca(array_type, &env.next_name("array.alloca"))?;

    for (index, element_expr) in elements.iter().enumerate() {
        let idx = u64::try_from(index).map_err(|conversion_error| {
            CodegenError::new(format!("array index conversion failed: {conversion_error}"))
        })?;
        let ptr = build_array_store_ptr(codegen_context, env, array_alloca, idx)?;
        let value = codegen_expression(codegen_context, env, element_expr, Some(element_core))?;
        let _store_instruction = codegen_context.builder.build_store(ptr, value)?;
    }

    let base_ptr = build_array_base_ptr(codegen_context, env, array_alloca)?;
    publish_array_metadata(codegen_context, env, count);
    Ok(base_ptr.as_basic_value_enum())
}

/// Record array metadata for the most recently lowered array-producing expression.
fn publish_array_metadata<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    count: u32,
) {
    let length = codegen_context
        .context
        .i64_type()
        .const_int(u64::from(count), false);
    env.set_pending_array_metadata(Some(ArrayMetadata {
        length,
        capacity: length,
    }));
}

/// Build a pointer to the indexed slot inside a stack-allocated LLVM array.
fn build_array_store_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_alloca: PointerValue<'context>,
    idx: u64,
) -> Result<PointerValue<'context>, CodegenError> {
    // SAFETY: The pointer comes from an alloca of the exact array type and the indices address an
    // element within that LLVM aggregate.
    unsafe {
        codegen_context
            .builder
            .build_in_bounds_gep(
                array_alloca,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_int(idx, false),
                ],
                &env.next_name("array.store.ptr"),
            )
            .map_err(Into::into)
    }
}

/// Build the base element pointer for a stack-allocated LLVM array.
fn build_array_base_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_alloca: PointerValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    // SAFETY: The alloca stores an LLVM array and `[0, 0]` points at its first element.
    unsafe {
        codegen_context
            .builder
            .build_in_bounds_gep(
                array_alloca,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_zero(),
                ],
                &env.next_name("array.base.ptr"),
            )
            .map_err(Into::into)
    }
}

/// Build a pointer to a dynamically indexed element from an array base pointer.
fn build_array_element_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    base_ptr: PointerValue<'context>,
    index_value: IntValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    // SAFETY: Bounds are checked immediately before this GEP and the base pointer targets the
    // first element of the backing array storage.
    unsafe {
        codegen_context
            .builder
            .build_in_bounds_gep(base_ptr, &[index_value], &env.next_name("array.load.ptr"))
            .map_err(Into::into)
    }
}

/// Load a nested row value and publish its row-specific metadata for chained access.
fn load_nested_row_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_ptr: PointerValue<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let row_value = codegen_context
        .builder
        .build_load(element_ptr, &env.next_name("array.row.load"))?
        .into_struct_value();
    let row_ptr = codegen_context
        .builder
        .build_extract_value(row_value, 0, &env.next_name("array.row.ptr"))?
        .into_pointer_value();
    let row_length = codegen_context
        .builder
        .build_extract_value(row_value, 1, &env.next_name("array.row.len"))?
        .into_int_value();
    let row_capacity = codegen_context
        .builder
        .build_extract_value(row_value, 2, &env.next_name("array.row.cap"))?
        .into_int_value();
    env.set_pending_array_metadata(Some(ArrayMetadata {
        length: row_length,
        capacity: row_capacity,
    }));
    Ok(row_ptr.as_basic_value_enum())
}

/// Construct the row-value struct type used for nested array elements.
fn array_value_struct_type<'context>(
    codegen_context: &CodegenContext<'context>,
    element_core_type: &CoreType,
) -> inkwell::types::StructType<'context> {
    let element_ptr_type = core_type_to_llvm(codegen_context.context, element_core_type)
        .ptr_type(inkwell::AddressSpace::default());
    codegen_context.context.struct_type(
        &[
            element_ptr_type.into(),
            codegen_context.context.i64_type().into(),
            codegen_context.context.i64_type().into(),
        ],
        false,
    )
}

/// Build a `{ptr, len, cap}` runtime value for a nested array row.
fn build_array_value_struct<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    struct_type: inkwell::types::StructType<'context>,
    pointer: PointerValue<'context>,
    length: IntValue<'context>,
    capacity: IntValue<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let with_pointer = codegen_context
        .builder
        .build_insert_value(
            struct_type.get_undef(),
            pointer,
            0,
            &env.next_name("array.value.ptr"),
        )?
        .into_struct_value();
    let with_length = codegen_context
        .builder
        .build_insert_value(with_pointer, length, 1, &env.next_name("array.value.len"))?
        .into_struct_value();
    Ok(codegen_context
        .builder
        .build_insert_value(with_length, capacity, 2, &env.next_name("array.value.cap"))?
        .as_basic_value_enum())
}

/// Resolve an array receiver to a base pointer and runtime length for index lowering.
fn resolve_array_access_base_and_length<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    object: &Expr,
    element_core_type: &CoreType,
) -> Result<(PointerValue<'context>, IntValue<'context>), CodegenError> {
    if let Expr::Identifier { ref name, .. } = *object {
        let Some(binding) = env.variables.get(name).cloned() else {
            return Err(CodegenError::new(format!(
                "unknown array variable '{name}'"
            )));
        };
        let resolved_length =
            if let Some(length) = binding.length {
                codegen_context
                    .context
                    .i64_type()
                    .const_int(u64::from(length), false)
            } else {
                let len_binding_name = format!("{name}_len");
                let len_binding = env.variables.get(len_binding_name.as_str()).ok_or_else(|| {
                CodegenError::new(format!(
                    "array metadata binding '{len_binding_name}' is missing for index access"
                ))
            })?;
                codegen_context
                    .builder
                    .build_load(len_binding.alloca, len_binding_name.as_str())?
                    .into_int_value()
            };
        let loaded_ptr = codegen_context
            .builder
            .build_load(binding.alloca, &env.next_name("array.ptr.load"))?;
        let array_ptr = if loaded_ptr.is_pointer_value() {
            loaded_ptr.into_pointer_value()
        } else {
            // SAFETY: The variable binding alloca stores the backing LLVM array aggregate, so
            // `[0, 0]` addresses the first element when the value was not lowered as a pointer.
            unsafe {
                codegen_context.builder.build_in_bounds_gep(
                    binding.alloca,
                    &[
                        codegen_context.context.i32_type().const_zero(),
                        codegen_context.context.i32_type().const_zero(),
                    ],
                    &env.next_name("array.base.ptr"),
                )?
            }
        };
        return Ok((array_ptr, resolved_length));
    }

    let object_value = codegen_expression(
        codegen_context,
        env,
        object,
        Some(&CoreType::Array(Box::new(element_core_type.clone()))),
    )?;
    let metadata = env.take_pending_array_metadata().ok_or_else(|| {
        CodegenError::new(String::from(
            "array expression did not publish metadata for nested index access",
        ))
    })?;
    Ok((object_value.into_pointer_value(), metadata.length))
}

/// Emit the runtime trap for array bounds failures using the row-specific length.
fn emit_array_bounds_check<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    index_value: IntValue<'context>,
    length_value: IntValue<'context>,
) -> Result<(), CodegenError> {
    let is_out_of_bounds = codegen_context.builder.build_int_compare(
        IntPredicate::UGE,
        index_value,
        length_value,
        &env.next_name("array.bounds.check"),
    )?;
    let current_fn = current_function(codegen_context)?;
    let trap_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.trap"));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("array.cont"));
    let _branch = codegen_context.builder.build_conditional_branch(
        is_out_of_bounds,
        trap_block,
        cont_block,
    )?;

    codegen_context.builder.position_at_end(trap_block);
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_array_bounds_error",
    )
    .ok_or_else(|| {
        CodegenError::new(String::from("opal_array_bounds_error declaration missing"))
    })?;
    let index_arg = if index_value.get_type().get_bit_width() == 64 {
        index_value
    } else {
        codegen_context.builder.build_int_z_extend(
            index_value,
            codegen_context.context.i64_type(),
            &env.next_name("array.bounds.index.i64"),
        )?
    };
    let length_arg = if length_value.get_type().get_bit_width() == 64 {
        length_value
    } else {
        codegen_context.builder.build_int_z_extend(
            length_value,
            codegen_context.context.i64_type(),
            &env.next_name("array.bounds.length.i64"),
        )?
    };
    let trap_args: [BasicMetadataValueEnum<'context>; 2] = [index_arg.into(), length_arg.into()];
    let _call = codegen_context.builder.build_call(
        runtime_fn,
        &trap_args,
        &env.next_name("array.trap.call"),
    )?;
    let _unreachable = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(cont_block);
    Ok(())
}
