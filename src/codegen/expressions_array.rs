#![allow(
    clippy::all,
    dead_code,
    clippy::let_underscore_untyped,
    clippy::missing_docs_in_private_items,
    clippy::needless_pass_by_value,
    clippy::pattern_type_mismatch,
    clippy::too_many_lines,
    clippy::undocumented_unsafe_blocks,
    unfulfilled_lint_expectations,
    reason = "internal codegen implementation module"
)]

extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression, current_function};
use crate::codegen::rc_emitter::RcEmitter;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use inkwell::module::Linkage;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use inkwell::types::BasicType;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::{AddressSpace, IntPredicate};

/// Lower array literals into RC-backed payload allocations.
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

/// Lower array indexing against RC-backed payload headers.
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
    let loaded = codegen_context
        .builder
        .build_load(element_ptr, &env.next_name("array.load"))?;

    let _ = expected_type;
    Ok(loaded)
}

pub fn codegen_identifier_indexed_array_assignment<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    object: &Expr,
    index: &Expr,
    value: &Expr,
) -> Result<(), CodegenError> {
    let Expr::Identifier { ref name, .. } = *object else {
        return Err(CodegenError::new(String::from(
            "indexed assignment currently requires identifier array receiver",
        )));
    };

    let Some(binding) = env.variables.get(name).cloned() else {
        return Err(CodegenError::new(format!(
            "assignment target '{name}' not found"
        )));
    };
    let binding_alloca = binding.alloca;
    if !binding.is_mutable {
        return Err(CodegenError::new(format!(
            "cannot assign to immutable variable: {name}"
        )));
    }

    let element_core_type = match binding.core_type.clone() {
        CoreType::Array(element_type) => *element_type,
        other => {
            return Err(CodegenError::new(format!(
                "index assignment expects array receiver, found '{other}'"
            )));
        }
    };

    let array_value = load_array_payload_ptr_from_binding(codegen_context, env, name, binding)?;
    let array_length =
        load_array_length_from_value(codegen_context, env, array_value, "index.assign")?;
    let array_capacity =
        load_array_capacity_from_value(codegen_context, env, array_value, "index.assign")?;
    let source_data_ptr = load_array_data_ptr_for_element_type(
        codegen_context,
        env,
        array_value,
        &element_core_type,
        "index.assign",
    )?;

    let index_value =
        codegen_expression(codegen_context, env, index, Some(&CoreType::Int64))?.into_int_value();
    emit_array_bounds_check(codegen_context, env, index_value, array_length)?;

    let replacement_value =
        codegen_expression(codegen_context, env, value, Some(&element_core_type))?;
    let (cloned_array_value, cloned_data_ptr) = allocate_array_payload(
        codegen_context,
        env,
        &element_core_type,
        array_length,
        array_capacity,
        "index.assign",
    )?;
    clone_array_elements_into_payload(
        codegen_context,
        env,
        &element_core_type,
        source_data_ptr,
        cloned_data_ptr,
        array_length,
        "index.assign",
    )?;

    let overwrite_slot = build_array_element_ptr(codegen_context, env, cloned_data_ptr, index_value)?;
    release_rc_value_if_needed(
        codegen_context,
        &element_core_type,
        codegen_context
            .builder
            .build_load(overwrite_slot, &env.next_name("index.assign.old.load"))?,
    )?;
    retain_rc_value_if_needed(codegen_context, &element_core_type, replacement_value)?;
    codegen_context
        .builder
        .build_store(overwrite_slot, replacement_value)?;
    codegen_context
        .builder
        .build_store(binding_alloca, cloned_array_value)?;
    Ok(())
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

fn codegen_nested_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    nested_element_core: &CoreType,
    count: u32,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let nested_array_core = CoreType::Array(Box::new(nested_element_core.clone()));
    codegen_flat_array_literal(codegen_context, env, elements, &nested_array_core, count)
}

fn codegen_flat_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    element_core: &CoreType,
    count: u32,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let count_value = codegen_context
        .context
        .i64_type()
        .const_int(u64::from(count), false);
    let (array_value, data_ptr) = allocate_array_payload(
        codegen_context,
        env,
        element_core,
        count_value,
        count_value,
        "array.literal",
    )?;

    for (index, element_expr) in elements.iter().enumerate() {
        let idx = u64::try_from(index).map_err(|conversion_error| {
            CodegenError::new(format!("array index conversion failed: {conversion_error}"))
        })?;
        let index_value = codegen_context.context.i64_type().const_int(idx, false);
        let ptr = build_array_element_ptr(codegen_context, env, data_ptr, index_value)?;
        let value = codegen_expression(codegen_context, env, element_expr, Some(element_core))?;
        retain_rc_value_if_needed(codegen_context, element_core, value)?;
        let _store_instruction = codegen_context.builder.build_store(ptr, value)?;
    }

    Ok(array_value.as_basic_value_enum())
}

pub(crate) fn allocate_array_payload<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    length: IntValue<'context>,
    capacity: IntValue<'context>,
    name_prefix: &str,
) -> Result<(PointerValue<'context>, PointerValue<'context>), CodegenError> {
    let (element_size, element_align) = array_element_layout(codegen_context, element_core_type)?;
    let alloc_fn = declare_or_get_opal_array_alloc(codegen_context);
    let drop_children_fn = array_drop_children_fn_ptr(codegen_context, element_core_type)?;
    let call = codegen_context.builder.build_call(
        alloc_fn,
        &[
            element_size.into(),
            element_align.into(),
            size_t_value(codegen_context, env, length, name_prefix)?.into(),
            size_t_value(codegen_context, env, capacity, name_prefix)?.into(),
            drop_children_fn.into(),
        ],
        &env.next_name(format!("{name_prefix}.alloc").as_str()),
    )?;
    let array_value = call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_array_alloc returned no value")))?
        .into_pointer_value();
    trap_on_null_array_allocation(codegen_context, env, array_value, name_prefix)?;
    let data_ptr = load_array_data_ptr_for_element_type(
        codegen_context,
        env,
        array_value,
        element_core_type,
        name_prefix,
    )?;
    Ok((array_value, data_ptr))
}

pub(crate) fn load_array_payload_ptr_from_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    binding: crate::codegen::expressions::VariableBinding<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let loaded = codegen_context
        .builder
        .build_load(binding.alloca, &env.next_name(format!("{binding_name}.array.load").as_str()))?;
    if !loaded.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "array binding '{binding_name}' did not lower to a pointer value"
        )));
    }
    cast_array_payload_to_i8_ptr(codegen_context, env, loaded.into_pointer_value(), binding_name)
}

pub(crate) fn load_array_length_from_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    name_prefix: &str,
) -> Result<IntValue<'context>, CodegenError> {
    let len_fn = declare_or_get_opal_array_len(codegen_context);
    let array_payload = cast_array_payload_to_i8_ptr(codegen_context, env, array_value, name_prefix)?;
    let call = codegen_context.builder.build_call(
        len_fn,
        &[array_payload.into()],
        &env.next_name(format!("{name_prefix}.len").as_str()),
    )?;
    Ok(call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_array_len returned no value")))?
        .into_int_value())
}

pub(crate) fn load_array_capacity_from_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    name_prefix: &str,
) -> Result<IntValue<'context>, CodegenError> {
    let cap_fn = declare_or_get_opal_array_cap(codegen_context);
    let array_payload = cast_array_payload_to_i8_ptr(codegen_context, env, array_value, name_prefix)?;
    let call = codegen_context.builder.build_call(
        cap_fn,
        &[array_payload.into()],
        &env.next_name(format!("{name_prefix}.cap").as_str()),
    )?;
    Ok(call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_array_cap returned no value")))?
        .into_int_value())
}

pub(crate) fn load_array_data_ptr_for_element_type<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    element_core_type: &CoreType,
    name_prefix: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    let data_fn = declare_or_get_opal_array_data(codegen_context);
    let (_, element_align) = array_element_layout(codegen_context, element_core_type)?;
    let array_payload = cast_array_payload_to_i8_ptr(codegen_context, env, array_value, name_prefix)?;
    let call = codegen_context.builder.build_call(
        data_fn,
        &[array_payload.into(), element_align.into()],
        &env.next_name(format!("{name_prefix}.data").as_str()),
    )?;
    let raw_data_ptr = call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_array_data returned no value")))?
        .into_pointer_value();
    Ok(codegen_context.builder.build_pointer_cast(
        raw_data_ptr,
        core_type_to_llvm(codegen_context.context, element_core_type)
            .ptr_type(AddressSpace::default()),
        &env.next_name(format!("{name_prefix}.typed.data").as_str()),
    )?)
}

fn declare_or_get_opal_array_alloc<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_alloc") {
        return function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(
        &[
            i64_type.into(),
            i64_type.into(),
            i64_type.into(),
            i64_type.into(),
            i8_ptr_type.into(),
        ],
        false,
    );
    module.add_function("opal_array_alloc", function_type, None)
}

fn declare_or_get_opal_array_len<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_len") {
        return function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_array_len", function_type, None)
}

fn declare_or_get_opal_array_cap<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_cap") {
        return function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_array_cap", function_type, None)
}

fn declare_or_get_opal_array_data<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_data") {
        return function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    module.add_function("opal_array_data", function_type, None)
}

fn array_drop_children_fn_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    element_core_type: &CoreType,
) -> Result<PointerValue<'context>, CodegenError> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    if !is_rc_bearing_element_type(element_core_type) {
        return Ok(i8_ptr_type.const_null());
    }

    let callback = declare_or_get_array_drop_children_fn(codegen_context)?;
    Ok(callback.as_global_value().as_pointer_value().const_cast(i8_ptr_type))
}

fn declare_or_get_array_drop_children_fn<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_array_drop_children") {
        return Ok(function);
    }

    let context = codegen_context.context;
    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
    let i8_ptr_ptr_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let i8_ptr_ptr_ptr_type = i8_ptr_ptr_type.ptr_type(AddressSpace::default());
    let size_t_ptr_type = context.i64_type().ptr_type(AddressSpace::default());
    let function_type = context.void_type().fn_type(
        &[
            i8_ptr_type.into(),
            i8_ptr_ptr_ptr_type.into(),
            size_t_ptr_type.into(),
            size_t_ptr_type.into(),
        ],
        false,
    );
    let function = module.add_function(
        "opal_array_drop_children",
        function_type,
        Some(Linkage::Internal),
    );
    let entry = context.append_basic_block(function, "entry");
    let current_block = codegen_context.builder.get_insert_block();
    codegen_context.builder.position_at_end(entry);

    let array_payload = function
        .get_nth_param(0)
        .expect("opal_array_drop_children should receive payload")
        .into_pointer_value();
    let stack = function
        .get_nth_param(1)
        .expect("opal_array_drop_children should receive stack")
        .into_pointer_value();
    let stack_top = function
        .get_nth_param(2)
        .expect("opal_array_drop_children should receive stack_top")
        .into_pointer_value();
    let stack_cap = function
        .get_nth_param(3)
        .expect("opal_array_drop_children should receive stack_cap")
        .into_pointer_value();

    let len_fn = declare_or_get_opal_array_len(codegen_context);
    let data_fn = declare_or_get_opal_array_data(codegen_context);
    let drop_child_fn = declare_or_get_opal_rc_drop_child(codegen_context);

    let length_value = codegen_context
        .builder
        .build_call(len_fn, &[array_payload.into()], "array.drop.len")?
        .try_as_basic_value()
        .basic()
        .expect("opal_array_len should return value")
        .into_int_value();
    let data_ptr = codegen_context
        .builder
        .build_call(
            data_fn,
            &[
                array_payload.into(),
                context.i64_type().const_int(8, false).into(),
            ],
            "array.drop.data",
        )?
        .try_as_basic_value()
        .basic()
        .expect("opal_array_data should return value")
        .into_pointer_value();
    let typed_data_ptr = codegen_context.builder.build_pointer_cast(
        data_ptr,
        i8_ptr_type.ptr_type(AddressSpace::default()),
        "array.drop.typed.data",
    )?;

    let index_alloca = codegen_context
        .builder
        .build_alloca(context.i64_type(), "array.drop.index")?;
    codegen_context
        .builder
        .build_store(index_alloca, context.i64_type().const_zero())?;

    let loop_block = context.append_basic_block(function, "loop");
    let body_block = context.append_basic_block(function, "body");
    let exit_block = context.append_basic_block(function, "exit");
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, "array.drop.index.load")?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        IntPredicate::ULT,
        index_value,
        length_value,
        "array.drop.cond",
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    let element_slot = unsafe {
        codegen_context
            .builder
            .build_in_bounds_gep(typed_data_ptr, &[index_value], "array.drop.slot")?
    };
    let child_value = codegen_context
        .builder
        .build_load(element_slot, "array.drop.child")?
        .into_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        drop_child_fn,
        &[child_value.into(), stack.into(), stack_top.into(), stack_cap.into()],
        "array.drop.child.call",
    )?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        context.i64_type().const_int(1, false),
        "array.drop.next",
    )?;
    codegen_context.builder.build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;

    codegen_context.builder.position_at_end(exit_block);
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_return(None)?;

    if let Some(block) = current_block {
        codegen_context.builder.position_at_end(block);
    }

    Ok(function)
}

fn declare_or_get_opal_rc_drop_child<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_rc_drop_child") {
        return function;
    }

    let context = codegen_context.context;
    let i8_ptr_type = context.i8_type().ptr_type(AddressSpace::default());
    let i8_ptr_ptr_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let i8_ptr_ptr_ptr_type = i8_ptr_ptr_type.ptr_type(AddressSpace::default());
    let size_t_ptr_type = context.i64_type().ptr_type(AddressSpace::default());
    let function_type = context.void_type().fn_type(
        &[
            i8_ptr_type.into(),
            i8_ptr_ptr_ptr_type.into(),
            size_t_ptr_type.into(),
            size_t_ptr_type.into(),
        ],
        false,
    );
    module.add_function("opal_rc_drop_child", function_type, None)
}

fn array_element_layout<'context>(
    codegen_context: &CodegenContext<'context>,
    element_core_type: &CoreType,
) -> Result<(IntValue<'context>, IntValue<'context>), CodegenError> {
    let element_type = core_type_to_llvm(codegen_context.context, element_core_type);
    let element_size = element_type
        .size_of()
        .ok_or_else(|| CodegenError::new(String::from("array element type has no size")))?;
    let element_size_i64 = if element_size.get_type().get_bit_width() == 64 {
        element_size
    } else {
        codegen_context.builder.build_int_z_extend(
            element_size,
            codegen_context.context.i64_type(),
            "array.elem_size.i64",
        )?
    };
    Ok((element_size_i64, element_size_i64))
}

fn cast_array_payload_to_i8_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    name_prefix: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    Ok(codegen_context.builder.build_pointer_cast(
        array_value,
        codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default()),
        &env.next_name(format!("{name_prefix}.payload.cast").as_str()),
    )?)
}

fn size_t_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: IntValue<'context>,
    name_prefix: &str,
) -> Result<IntValue<'context>, CodegenError> {
    if value.get_type().get_bit_width() == 64 {
        return Ok(value);
    }
    Ok(codegen_context.builder.build_int_z_extend(
        value,
        codegen_context.context.i64_type(),
        &env.next_name(format!("{name_prefix}.size_t").as_str()),
    )?)
}

pub(crate) fn build_array_element_ptr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    data_ptr: PointerValue<'context>,
    index_value: IntValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    unsafe {
        codegen_context
            .builder
            .build_in_bounds_gep(data_ptr, &[index_value], &env.next_name("array.load.ptr"))
            .map_err(Into::into)
    }
}

fn resolve_array_access_base_and_length<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    object: &Expr,
    element_core_type: &CoreType,
) -> Result<(PointerValue<'context>, IntValue<'context>), CodegenError> {
    let array_value = if let Expr::Identifier { ref name, .. } = *object {
        let Some(binding) = env.variables.get(name).cloned() else {
            return Err(CodegenError::new(format!(
                "unknown array variable '{name}'"
            )));
        };
        load_array_payload_ptr_from_binding(codegen_context, env, name, binding)?
    } else {
        let object_value = codegen_expression(
            codegen_context,
            env,
            object,
            Some(&CoreType::Array(Box::new(element_core_type.clone()))),
        )?;
        if !object_value.is_pointer_value() {
            return Err(CodegenError::new(String::from(
                "array expression did not lower to a payload pointer",
            )));
        }
        cast_array_payload_to_i8_ptr(
            codegen_context,
            env,
            object_value.into_pointer_value(),
            "array.access.expr",
        )?
    };
    let array_length =
        load_array_length_from_value(codegen_context, env, array_value, "array.access")?;
    let base_ptr = load_array_data_ptr_for_element_type(
        codegen_context,
        env,
        array_value,
        element_core_type,
        "array.access",
    )?;
    Ok((base_ptr, array_length))
}

fn trap_on_null_array_allocation<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    array_value: PointerValue<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    let is_null = codegen_context
        .builder
        .build_is_null(array_value, &env.next_name(format!("{name_prefix}.alloc.is_null").as_str()))?;
    let current_fn = current_function(codegen_context)?;
    let trap_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name(format!("{name_prefix}.alloc.trap").as_str()));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name(format!("{name_prefix}.alloc.cont").as_str()));
    codegen_context
        .builder
        .build_conditional_branch(is_null, trap_block, cont_block)?;

    codegen_context.builder.position_at_end(trap_block);
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context
        .builder
        .build_global_string_ptr(
            format!("{name_prefix} array allocation failed").as_str(),
            &env.next_name(format!("{name_prefix}.alloc.msg").as_str()),
        )?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        runtime_fn,
        &[msg.into()],
        &env.next_name(format!("{name_prefix}.alloc.call").as_str()),
    )?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(cont_block);
    Ok(())
}

fn clone_array_elements_into_payload<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element_core_type: &CoreType,
    source_ptr: PointerValue<'context>,
    destination_ptr: PointerValue<'context>,
    length: IntValue<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    let current_fn = current_function(codegen_context)?;
    let loop_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name(format!("{name_prefix}.copy.loop").as_str()));
    let body_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name(format!("{name_prefix}.copy.body").as_str()));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name(format!("{name_prefix}.copy.exit").as_str()));

    let index_alloca = codegen_context.builder.build_alloca(
        codegen_context.context.i64_type(),
        &env.next_name(format!("{name_prefix}.copy.index").as_str()),
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
            &env.next_name(format!("{name_prefix}.copy.index.load").as_str()),
        )?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        IntPredicate::ULT,
        index_value,
        length,
        &env.next_name(format!("{name_prefix}.copy.cond").as_str()),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;

    codegen_context.builder.position_at_end(body_block);
    let source_slot = build_array_element_ptr(codegen_context, env, source_ptr, index_value)?;
    let destination_slot =
        build_array_element_ptr(codegen_context, env, destination_ptr, index_value)?;
    let copied_value = codegen_context.builder.build_load(
        source_slot,
        &env.next_name(format!("{name_prefix}.copy.value").as_str()),
    )?;
    retain_rc_value_if_needed(codegen_context, element_core_type, copied_value)?;
    codegen_context
        .builder
        .build_store(destination_slot, copied_value)?;
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &env.next_name(format!("{name_prefix}.copy.next").as_str()),
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

fn retain_rc_value_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    element_core_type: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    if !is_rc_bearing_element_type(element_core_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "RC-bearing array element type '{element_core_type}' expected pointer value during indexed assignment"
        )));
    }
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_inc(value.into_pointer_value())
}

fn release_rc_value_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    element_core_type: &CoreType,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    if !is_rc_bearing_element_type(element_core_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "RC-bearing array element type '{element_core_type}' expected pointer value during indexed assignment"
        )));
    }
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_dec(value.into_pointer_value())
}

pub(crate) const fn is_rc_bearing_element_type(element_core_type: &CoreType) -> bool {
    matches!(element_core_type, CoreType::Array(_) | CoreType::Generic { .. })
}

pub(crate) fn emit_array_bounds_check<'context>(
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
