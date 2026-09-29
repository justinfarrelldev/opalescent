//! String-aware array ownership helpers used by array expression lowering.
//!
//! Arrays store raw element payloads, so string arrays need dedicated drop hooks
//! that release each managed string value before the array payload is reclaimed.

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::string_ownership::declare_or_get_opal_string_release;
use inkwell::basic_block::BasicBlock;
use inkwell::module::Linkage;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

/// Free a raw `char**` container after its elements have been adopted.
pub(crate) fn free_owned_raw_string_array<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    raw_elements_ptr: PointerValue<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    let free_fn = declare_or_get_opal_string_array_free(codegen_context);
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let raw_string_array_ptr = codegen_context.builder.build_pointer_cast(
        raw_elements_ptr,
        i8_ptr_type.ptr_type(AddressSpace::default()),
        &env.next_name(format!("{name_prefix}.raw.free.cast").as_str()),
    )?;
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        free_fn,
        &[raw_string_array_ptr.into()],
        &env.next_name(format!("{name_prefix}.raw.free").as_str()),
    )?;
    Ok(())
}

/// Declare or return the callback that releases all string elements in an array.
pub(crate) fn declare_or_get_string_array_drop_children_fn<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_array_drop_strings") {
        return Ok(existing_function);
    }

    let drop_function = declare_drop_children_function(codegen_context, "opal_array_drop_strings");
    let current_block = codegen_context.builder.get_insert_block();
    let entry_block = codegen_context
        .context
        .append_basic_block(drop_function, "entry");
    codegen_context.builder.position_at_end(entry_block);

    let array_payload = get_pointer_param(drop_function, 0, "opal_array_drop_strings payload")?;
    let (length_value, typed_data_ptr) =
        load_drop_array_state(codegen_context, array_payload, "array.drop.strings")?;
    let release_fn = declare_or_get_opal_string_release(codegen_context);
    let (index_alloca, loop_block, body_block, exit_block) =
        create_drop_loop_blocks(codegen_context, drop_function, "array.drop.strings")?;
    let index_value = emit_drop_loop_condition(
        codegen_context,
        index_alloca,
        length_value,
        loop_block,
        body_block,
        exit_block,
        "array.drop.strings",
    )?;

    codegen_context.builder.position_at_end(body_block);
    emit_string_drop_body(codegen_context, typed_data_ptr, index_value, release_fn)?;
    emit_drop_loop_backedge(
        codegen_context,
        index_alloca,
        index_value,
        loop_block,
        "array.drop.strings",
    )?;
    emit_drop_loop_exit(codegen_context, exit_block, current_block)?;
    Ok(drop_function)
}

/// Declare or return the callback that decrements all RC child elements in an array.
pub(crate) fn declare_or_get_array_drop_children_fn<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_array_drop_children") {
        return Ok(existing_function);
    }

    let drop_function = declare_drop_children_function(codegen_context, "opal_array_drop_children");
    let current_block = codegen_context.builder.get_insert_block();
    let entry_block = codegen_context
        .context
        .append_basic_block(drop_function, "entry");
    codegen_context.builder.position_at_end(entry_block);

    let array_payload = get_pointer_param(drop_function, 0, "opal_array_drop_children payload")?;
    let stack = get_pointer_param(drop_function, 1, "opal_array_drop_children stack")?;
    let stack_top = get_pointer_param(drop_function, 2, "opal_array_drop_children stack_top")?;
    let stack_cap = get_pointer_param(drop_function, 3, "opal_array_drop_children stack_cap")?;
    let (length_value, typed_data_ptr) =
        load_drop_array_state(codegen_context, array_payload, "array.drop")?;
    let drop_child_fn = declare_or_get_opal_rc_drop_child(codegen_context);
    let (index_alloca, loop_block, body_block, exit_block) =
        create_drop_loop_blocks(codegen_context, drop_function, "array.drop")?;
    let index_value = emit_drop_loop_condition(
        codegen_context,
        index_alloca,
        length_value,
        loop_block,
        body_block,
        exit_block,
        "array.drop",
    )?;

    codegen_context.builder.position_at_end(body_block);
    emit_rc_child_drop_body(
        codegen_context,
        typed_data_ptr,
        index_value,
        drop_child_fn,
        stack,
        stack_top,
        stack_cap,
    )?;
    emit_drop_loop_backedge(
        codegen_context,
        index_alloca,
        index_value,
        loop_block,
        "array.drop",
    )?;
    emit_drop_loop_exit(codegen_context, exit_block, current_block)?;
    Ok(drop_function)
}

/// Declare the shared array drop callback signature with runtime stack parameters.
fn declare_drop_children_function<'context>(
    codegen_context: &CodegenContext<'context>,
    symbol: &str,
) -> FunctionValue<'context> {
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
    codegen_context
        .module
        .add_function(symbol, function_type, Some(Linkage::Internal))
}

/// Return a pointer parameter from a generated drop callback.
fn get_pointer_param<'context>(
    drop_function: FunctionValue<'context>,
    index: u32,
    description: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    drop_function
        .get_nth_param(index)
        .map(inkwell::values::BasicValueEnum::into_pointer_value)
        .ok_or_else(|| CodegenError::new(format!("{description} parameter missing")))
}

/// Load array length and data pointer for an array drop callback.
fn load_drop_array_state<'context>(
    codegen_context: &CodegenContext<'context>,
    array_payload: PointerValue<'context>,
    name_prefix: &str,
) -> Result<(IntValue<'context>, PointerValue<'context>), CodegenError> {
    let context = codegen_context.context;
    let len_fn = declare_or_get_opal_array_len(codegen_context);
    let data_fn = declare_or_get_opal_array_data(codegen_context);
    let length_value = codegen_context
        .builder
        .build_call(
            len_fn,
            &[array_payload.into()],
            &format!("{name_prefix}.len"),
        )?
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
            &format!("{name_prefix}.data"),
        )?
        .try_as_basic_value()
        .basic()
        .expect("opal_array_data should return value")
        .into_pointer_value();
    let typed_data_ptr = codegen_context.builder.build_pointer_cast(
        data_ptr,
        context
            .i8_type()
            .ptr_type(AddressSpace::default())
            .ptr_type(AddressSpace::default()),
        &format!("{name_prefix}.typed.data"),
    )?;
    Ok((length_value, typed_data_ptr))
}

/// Create the alloca and blocks that make up an array child-drop loop.
fn create_drop_loop_blocks<'context>(
    codegen_context: &CodegenContext<'context>,
    drop_function: FunctionValue<'context>,
    name_prefix: &str,
) -> Result<
    (
        PointerValue<'context>,
        BasicBlock<'context>,
        BasicBlock<'context>,
        BasicBlock<'context>,
    ),
    CodegenError,
> {
    let context = codegen_context.context;
    let index_alloca = codegen_context
        .builder
        .build_alloca(context.i64_type(), &format!("{name_prefix}.index"))?;
    codegen_context
        .builder
        .build_store(index_alloca, context.i64_type().const_zero())?;

    let loop_block = context.append_basic_block(drop_function, "loop");
    let body_block = context.append_basic_block(drop_function, "body");
    let exit_block = context.append_basic_block(drop_function, "exit");
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;
    Ok((index_alloca, loop_block, body_block, exit_block))
}

/// Emit the loop condition for an array child-drop loop and return the index value.
fn emit_drop_loop_condition<'context>(
    codegen_context: &CodegenContext<'context>,
    index_alloca: PointerValue<'context>,
    length_value: IntValue<'context>,
    loop_block: BasicBlock<'context>,
    body_block: BasicBlock<'context>,
    exit_block: BasicBlock<'context>,
    name_prefix: &str,
) -> Result<IntValue<'context>, CodegenError> {
    codegen_context.builder.position_at_end(loop_block);
    let index_value = codegen_context
        .builder
        .build_load(index_alloca, &format!("{name_prefix}.index.load"))?
        .into_int_value();
    let should_continue = codegen_context.builder.build_int_compare(
        IntPredicate::ULT,
        index_value,
        length_value,
        &format!("{name_prefix}.cond"),
    )?;
    codegen_context
        .builder
        .build_conditional_branch(should_continue, body_block, exit_block)?;
    Ok(index_value)
}

/// Emit one string-element release for a string array drop callback.
fn emit_string_drop_body<'context>(
    codegen_context: &CodegenContext<'context>,
    typed_data_ptr: PointerValue<'context>,
    index_value: IntValue<'context>,
    release_fn: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    // SAFETY: `typed_data_ptr` is the start of the array payload's element
    // storage and `index_value` is guarded by the loop's length comparison.
    let element_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            typed_data_ptr,
            &[index_value],
            "array.drop.strings.slot",
        )?
    };
    let string_value = codegen_context
        .builder
        .build_load(element_slot, "array.drop.strings.value")?
        .into_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        release_fn,
        &[string_value.into()],
        "array.drop.strings.release",
    )?;
    Ok(())
}

/// Emit one RC-child decrement for an array drop callback.
fn emit_rc_child_drop_body<'context>(
    codegen_context: &CodegenContext<'context>,
    typed_data_ptr: PointerValue<'context>,
    index_value: IntValue<'context>,
    drop_child_fn: FunctionValue<'context>,
    stack: PointerValue<'context>,
    stack_top: PointerValue<'context>,
    stack_cap: PointerValue<'context>,
) -> Result<(), CodegenError> {
    // SAFETY: `typed_data_ptr` is the start of the array payload's element
    // storage and `index_value` is guarded by the loop's length comparison.
    let element_slot = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            typed_data_ptr,
            &[index_value],
            "array.drop.slot",
        )?
    };
    let child_value = codegen_context
        .builder
        .build_load(element_slot, "array.drop.child")?
        .into_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        drop_child_fn,
        &[
            child_value.into(),
            stack.into(),
            stack_top.into(),
            stack_cap.into(),
        ],
        "array.drop.child.call",
    )?;
    Ok(())
}

/// Advance an array child-drop loop and jump back to its condition.
fn emit_drop_loop_backedge<'context>(
    codegen_context: &CodegenContext<'context>,
    index_alloca: PointerValue<'context>,
    index_value: IntValue<'context>,
    loop_block: BasicBlock<'context>,
    name_prefix: &str,
) -> Result<(), CodegenError> {
    let next_index = codegen_context.builder.build_int_add(
        index_value,
        codegen_context.context.i64_type().const_int(1, false),
        &format!("{name_prefix}.next"),
    )?;
    codegen_context
        .builder
        .build_store(index_alloca, next_index)?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_block)?;
    Ok(())
}

/// Terminate a generated drop callback and restore the previous insertion point.
fn emit_drop_loop_exit<'context>(
    codegen_context: &CodegenContext<'context>,
    exit_block: BasicBlock<'context>,
    previous_block: Option<BasicBlock<'context>>,
) -> Result<(), CodegenError> {
    codegen_context.builder.position_at_end(exit_block);
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_return(None)?;
    if let Some(restore_block) = previous_block {
        codegen_context.builder.position_at_end(restore_block);
    }
    Ok(())
}

/// Declare or return the runtime helper that frees raw string pointer arrays.
fn declare_or_get_opal_string_array_free<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_string_array_free") {
        return existing_function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i8_ptr_ptr_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let function_type = codegen_context
        .context
        .void_type()
        .fn_type(&[i8_ptr_ptr_type.into()], false);
    module.add_function("opal_string_array_free", function_type, None)
}

/// Declare or return the runtime helper that drops one RC child element.
fn declare_or_get_opal_rc_drop_child<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_rc_drop_child") {
        return existing_function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i8_ptr_ptr_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let size_t_ptr_type = codegen_context
        .context
        .i64_type()
        .ptr_type(AddressSpace::default());
    let function_type = codegen_context.context.void_type().fn_type(
        &[
            i8_ptr_type.into(),
            i8_ptr_ptr_type.ptr_type(AddressSpace::default()).into(),
            size_t_ptr_type.into(),
            size_t_ptr_type.into(),
        ],
        false,
    );
    module.add_function("opal_rc_drop_child", function_type, None)
}

/// Declare or return the runtime helper that reads an array payload length.
fn declare_or_get_opal_array_len<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_array_len") {
        return existing_function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_array_len", function_type, None)
}

/// Declare or return the runtime helper that reads an array payload data pointer.
fn declare_or_get_opal_array_data<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(existing_function) = module.get_function("opal_array_data") {
        return existing_function;
    }
    let i64_type = codegen_context.context.i64_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(&[i8_ptr_type.into(), i64_type.into()], false);
    module.add_function("opal_array_data", function_type, None)
}
