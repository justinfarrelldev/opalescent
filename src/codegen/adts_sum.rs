extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
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

    if let Some(layout_name) = variant_layout_name {
        if let Some(field_layout) = env.adt_field_layouts.get(layout_name).cloned() {
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
        return store_sum_variant_as_nominal_payload(codegen_context, env, tagged_type, alloca);
    }

    codegen_context
        .builder
        .build_load(alloca, &env.next_name("sum.value"))
        .map_err(CodegenError::from)
}

/// Copy one tagged-union stack value into a pointer-backed nominal payload.
fn store_sum_variant_as_nominal_payload<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    tagged_type: inkwell::types::StructType<'context>,
    alloca: inkwell::values::PointerValue<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let payload_size = tagged_type.size_of().ok_or_else(|| {
        CodegenError::new(String::from(
            "could not compute payload size for sum constructor nominal payload",
        ))
    })?;
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    let payload_ptr = emitter.emit_alloc(payload_size, None)?;
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
