extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::rc_emitter::RcEmitter;
use crate::type_system::types::CoreType;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::values::{BasicValue, BasicValueEnum};

/// Lower sum variant constructors into tagged-union struct values or pointer-backed nominals.
pub fn codegen_sum_variant_constructor<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    fields: &[crate::ast::ConstructorField],
    expected_type: Option<&CoreType>,
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
        codegen_context.context.i64_type().const_int(0, false),
    )?;

    if let Some(first_field) = fields.first() {
        let _payload_value = codegen_expression(codegen_context, env, &first_field.value, None)?;
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
