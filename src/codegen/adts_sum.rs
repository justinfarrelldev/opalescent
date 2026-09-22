extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::expressions_array::{
    declare_or_get_opal_rc_drop_child, requires_rc_runtime_hooks,
};
use crate::codegen::rc_emitter::RcEmitter;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::types::StructType;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

/// Cast a pointer-backed nominal for field access, routing sum variants through their payload.
pub fn pointer_for_pointer_backed_field_access<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    type_name: &str,
    object_ptr: PointerValue<'context>,
    struct_type: StructType<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    if type_name.contains('.') {
        let tagged_type = codegen_context.context.struct_type(
            &[
                codegen_context.context.i64_type().into(),
                codegen_context.context.i8_type().array_type(64).into(),
            ],
            false,
        );
        let tagged_ptr = codegen_context.builder.build_pointer_cast(
            object_ptr,
            tagged_type.ptr_type(AddressSpace::default()),
            &env.next_name("variant.field.tagged.cast"),
        )?;
        // SAFETY: GEP selects the payload byte-array field inside the tagged union.
        let payload_array_ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                tagged_ptr,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_int(1, false),
                ],
                &env.next_name("variant.field.payload.ptr"),
            )?
        };
        return codegen_context
            .builder
            .build_pointer_cast(
                payload_array_ptr,
                struct_type.ptr_type(AddressSpace::default()),
                &env.next_name("variant.field.payload.cast"),
            )
            .map_err(CodegenError::from);
    }
    codegen_context
        .builder
        .build_pointer_cast(
            object_ptr,
            struct_type.ptr_type(AddressSpace::default()),
            &env.next_name("field.ptr.cast"),
        )
        .map_err(CodegenError::from)
}

/// Lower sum variant constructors into tagged-union struct values or pointer-backed nominals.
#[expect(
    clippy::too_many_lines,
    reason = "sum constructor lowering keeps tag, payload, and copy ABI together"
)]
pub fn codegen_sum_variant_constructor<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    fields: &[crate::ast::ConstructorField],
    expected_type: Option<&CoreType>,
    variant_tag: i64,
    variant_layout_name: Option<&str>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let tagged_type = codegen_context.context.struct_type(
        &[
            codegen_context.context.i64_type().into(),
            codegen_context.context.i8_type().array_type(64).into(),
        ],
        false,
    );
    let alloca = codegen_context
        .builder
        .build_alloca(tagged_type, &env.next_name("sum.alloca"))?;

    // SAFETY: GEP targets the tag field on the same stack-allocated tagged union value.
    let tag_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            alloca,
            &[
                codegen_context.context.i32_type().const_zero(),
                codegen_context.context.i32_type().const_zero(),
            ],
            &env.next_name("sum.tag.ptr"),
        )?
    };
    let _store_tag = codegen_context.builder.build_store(
        tag_ptr,
        codegen_context
            .context
            .i64_type()
            .const_int(u64::try_from(variant_tag).unwrap_or_default(), true),
    )?;

    let mut variant_field_layout: Option<alloc::vec::Vec<(String, CoreType)>> = None;
    if let Some(layout_name) = variant_layout_name {
        if let Some(field_layout) = env.adt_field_layout(layout_name).cloned() {
            variant_field_layout = Some(field_layout.clone());
            let payload_struct_type = codegen_context.context.struct_type(
                field_layout
                    .iter()
                    .map(|&(_, ref field_type)| {
                        core_type_to_llvm(codegen_context.context, field_type)
                    })
                    .collect::<alloc::vec::Vec<_>>()
                    .as_slice(),
                false,
            );
            // SAFETY: GEP selects the payload byte-array field inside the tagged union alloca.
            let payload_array_ptr = unsafe {
                codegen_context.builder.build_in_bounds_gep(
                    alloca,
                    &[
                        codegen_context.context.i32_type().const_zero(),
                        codegen_context.context.i32_type().const_int(1, false),
                    ],
                    &env.next_name("sum.payload.ptr"),
                )?
            };
            let payload_struct_ptr = codegen_context.builder.build_pointer_cast(
                payload_array_ptr,
                payload_struct_type.ptr_type(AddressSpace::default()),
                &env.next_name("sum.payload.struct.cast"),
            )?;
            for (field_index, &(ref field_name, ref field_type)) in field_layout.iter().enumerate()
            {
                let Some(constructor_field) = fields.iter().find(|field| field.name == *field_name)
                else {
                    continue;
                };
                let lowered_value = codegen_expression(
                    codegen_context,
                    env,
                    &constructor_field.value,
                    Some(field_type),
                )?;
                let Ok(converted_index) = u64::try_from(field_index) else {
                    continue;
                };
                // SAFETY: GEP indexes a field within the payload struct using a known layout index.
                let field_ptr = unsafe {
                    codegen_context.builder.build_in_bounds_gep(
                        payload_struct_ptr,
                        &[
                            codegen_context.context.i32_type().const_zero(),
                            codegen_context
                                .context
                                .i32_type()
                                .const_int(converted_index, false),
                        ],
                        &env.next_name("sum.payload.field.ptr"),
                    )?
                };
                let _store_field = codegen_context
                    .builder
                    .build_store(field_ptr, lowered_value)?;
            }
        } else {
            for field in fields {
                let _payload_value = codegen_expression(codegen_context, env, &field.value, None)?;
            }
        }
    } else {
        for field in fields {
            let _payload_value = codegen_expression(codegen_context, env, &field.value, None)?;
        }
    }

    if let Some(&CoreType::Generic { .. }) = expected_type {
        let drop_children_fn = match (variant_layout_name, variant_field_layout.as_deref()) {
            (Some(layout_name), Some(field_layout))
                if field_layout
                    .iter()
                    .any(|&(_, ref field_type)| requires_rc_runtime_hooks(field_type)) =>
            {
                Some(declare_or_get_sum_variant_drop_children_fn(
                    codegen_context,
                    env,
                    layout_name,
                    field_layout,
                )?)
            }
            _ => None,
        };
        return store_sum_variant_as_nominal_payload(
            codegen_context,
            env,
            tagged_type,
            alloca,
            drop_children_fn,
        );
    }

    codegen_context
        .builder
        .build_load(alloca, &env.next_name("sum.value"))
        .map_err(CodegenError::from)
}

/// Build or fetch a child-drop callback specialized to one immutable sum variant payload.
#[expect(
    clippy::too_many_lines,
    reason = "variant payload child-drop callback mirrors nominal product callback lowering"
)]
fn declare_or_get_sum_variant_drop_children_fn<'context>(
    codegen_context: &CodegenContext<'context>,
    _env: &mut CodegenEnv<'context>,
    layout_name: &str,
    field_layout: &[(String, CoreType)],
) -> Result<PointerValue<'context>, CodegenError> {
    let sanitized_layout_name = layout_name
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>();
    let function_name = format!("__opalescent_drop_children_sum_{sanitized_layout_name}");
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    if let Some(function) = codegen_context.module.get_function(function_name.as_str()) {
        return Ok(function
            .as_global_value()
            .as_pointer_value()
            .const_cast(i8_ptr_type));
    }

    let context = codegen_context.context;
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
    let function = codegen_context.module.add_function(
        function_name.as_str(),
        function_type,
        Some(inkwell::module::Linkage::Internal),
    );
    let entry = context.append_basic_block(function, "entry");
    let current_block = codegen_context.builder.get_insert_block();
    codegen_context.builder.position_at_end(entry);

    let payload = function
        .get_nth_param(0)
        .expect("sum child-drop callback should receive payload")
        .into_pointer_value();
    let stack = function
        .get_nth_param(1)
        .expect("sum child-drop callback should receive stack")
        .into_pointer_value();
    let stack_top = function
        .get_nth_param(2)
        .expect("sum child-drop callback should receive stack_top")
        .into_pointer_value();
    let stack_cap = function
        .get_nth_param(3)
        .expect("sum child-drop callback should receive stack_cap")
        .into_pointer_value();

    let tagged_type = context.struct_type(
        &[
            context.i64_type().into(),
            context.i8_type().array_type(64).into(),
        ],
        false,
    );
    let tagged_ptr = codegen_context.builder.build_pointer_cast(
        payload,
        tagged_type.ptr_type(AddressSpace::default()),
        "sum.drop.tagged.cast",
    )?;
    // SAFETY: GEP selects the payload byte-array field within the tagged union payload.
    let payload_array_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            tagged_ptr,
            &[
                context.i32_type().const_zero(),
                context.i32_type().const_int(1, false),
            ],
            "sum.drop.payload.ptr",
        )?
    };
    let payload_struct_type = context.struct_type(
        field_layout
            .iter()
            .map(|&(_, ref field_type)| core_type_to_llvm(context, field_type))
            .collect::<alloc::vec::Vec<_>>()
            .as_slice(),
        false,
    );
    let payload_struct_ptr = codegen_context.builder.build_pointer_cast(
        payload_array_ptr,
        payload_struct_type.ptr_type(AddressSpace::default()),
        "sum.drop.payload.cast",
    )?;
    let drop_child_fn = declare_or_get_opal_rc_drop_child(codegen_context);
    for (index, &(_, ref field_type)) in field_layout.iter().enumerate() {
        if !requires_rc_runtime_hooks(field_type) {
            continue;
        }
        let converted_index = u64::try_from(index)
            .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?;
        // SAFETY: GEP indexes a payload field using the manifest-provided variant layout.
        let field_ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                payload_struct_ptr,
                &[
                    context.i32_type().const_zero(),
                    context.i32_type().const_int(converted_index, false),
                ],
                "sum.drop.field.ptr",
            )?
        };
        let field_value = codegen_context
            .builder
            .build_load(field_ptr, "sum.drop.field.load")?
            .into_pointer_value();
        let child_ptr = codegen_context.builder.build_pointer_cast(
            field_value,
            i8_ptr_type,
            "sum.drop.child.cast",
        )?;
        let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
            drop_child_fn,
            &[
                child_ptr.into(),
                stack.into(),
                stack_top.into(),
                stack_cap.into(),
            ],
            "sum.drop.child.call",
        )?;
    }
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_return(None)?;
    if let Some(block) = current_block {
        codegen_context.builder.position_at_end(block);
    }
    Ok(function
        .as_global_value()
        .as_pointer_value()
        .const_cast(i8_ptr_type))
}

/// Copy one tagged-union stack value into a pointer-backed nominal payload.
fn store_sum_variant_as_nominal_payload<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    tagged_type: inkwell::types::StructType<'context>,
    alloca: inkwell::values::PointerValue<'context>,
    drop_children_fn: Option<PointerValue<'context>>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let payload_size = tagged_type.size_of().ok_or_else(|| {
        CodegenError::new(String::from(
            "could not compute payload size for sum constructor nominal payload",
        ))
    })?;
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    let payload_ptr = emitter.emit_alloc(payload_size, drop_children_fn)?;
    let typed_ptr = codegen_context.builder.build_pointer_cast(
        payload_ptr,
        tagged_type.ptr_type(AddressSpace::default()),
        &env.next_name("sum.payload.cast"),
    )?;
    let loaded = codegen_context
        .builder
        .build_load(alloca, &env.next_name("sum.copy.value"))?;
    let _store_payload = codegen_context.builder.build_store(typed_ptr, loaded)?;
    Ok(payload_ptr.as_basic_value_enum())
}
