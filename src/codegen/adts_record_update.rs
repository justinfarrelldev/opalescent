#![allow(
    clippy::missing_docs_in_private_items,
    reason = "internal record update lowering helpers are implementation details"
)]

extern crate alloc;

use super::{adts_sum, declare_or_get_nominal_drop_children_fn, infer_product_core_type};
use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::expressions_array::requires_rc_runtime_hooks;
use crate::codegen::rc_emitter::RcEmitter;
use crate::codegen::string_ownership::{
    emit_string_retain, string_expr_needs_retain_at_ownership_boundary,
};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::values::{BasicValue, BasicValueEnum};

#[doc = "Lower block `with:` record updates by copying a product receiver and replacing fields."]
#[expect(
    clippy::too_many_lines,
    reason = "record update lowering mirrors nominal product construction with copied fields"
)]
#[expect(
    clippy::undocumented_unsafe_blocks,
    reason = "field GEP indices come from checked product layout metadata"
)]
pub fn codegen_record_update_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let Expr::RecordUpdate {
        ref receiver,
        ref fields,
        ..
    } = *expr
    else {
        return Err(CodegenError::new(String::from(
            "expected record update expression",
        )));
    };

    let receiver_core_type = infer_product_core_type(env, receiver.as_ref())
        .or_else(|| expected_type.cloned())
        .ok_or_else(|| {
            CodegenError::new(String::from(
                "record update receiver type could not be inferred",
            ))
        })?;
    let CoreType::Generic {
        ref name,
        ref type_args,
    } = receiver_core_type
    else {
        return Err(CodegenError::new(format!(
            "record update receiver must be a product type, found '{receiver_core_type}'"
        )));
    };
    if !type_args.is_empty() {
        return Err(CodegenError::new(format!(
            "record update codegen for generic product type '{name}' is not implemented yet"
        )));
    }

    let field_layout = env
        .adt_field_layout(name.as_str())
        .cloned()
        .ok_or_else(|| CodegenError::new(format!("missing field layout for product '{name}'")))?;
    let field_types = field_layout
        .iter()
        .map(|&(_, ref field_type)| core_type_to_llvm(codegen_context.context, field_type))
        .collect::<Vec<_>>();
    let struct_type = codegen_context
        .context
        .struct_type(field_types.as_slice(), false);
    let receiver_value = codegen_expression(
        codegen_context,
        env,
        receiver.as_ref(),
        Some(&receiver_core_type),
    )?;

    let update_map = fields
        .iter()
        .map(|field| (field.name.as_str(), field))
        .collect::<BTreeMap<_, _>>();
    for field in fields {
        if !field_layout
            .iter()
            .any(|&(ref field_name, _)| field_name == &field.name)
        {
            return Err(CodegenError::new(format!(
                "unknown field '{}' in record update for '{name}'",
                field.name
            )));
        }
    }

    let mut lowered_fields = Vec::with_capacity(field_layout.len());
    for (index, &(ref field_name, ref field_type)) in field_layout.iter().enumerate() {
        let lowered = if let Some(update_field) = update_map.get(field_name.as_str()).copied() {
            let value =
                codegen_expression(codegen_context, env, &update_field.value, Some(field_type))?;
            retain_record_update_field_if_needed(
                codegen_context,
                env,
                field_type,
                Some(&update_field.value),
                false,
                value,
            )?;
            value
        } else {
            let field_index = u64::try_from(index)
                .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?;
            let value = if receiver_value.is_pointer_value() {
                let typed_ptr = adts_sum::pointer_for_pointer_backed_field_access(
                    codegen_context,
                    env,
                    name.as_str(),
                    receiver_value.into_pointer_value(),
                    struct_type,
                )?;
                let field_ptr = unsafe {
                    codegen_context.builder.build_in_bounds_gep(
                        typed_ptr,
                        &[
                            codegen_context.context.i32_type().const_zero(),
                            codegen_context
                                .context
                                .i32_type()
                                .const_int(field_index, false),
                        ],
                        &env.next_name("record.update.field.ptr"),
                    )?
                };
                codegen_context
                    .builder
                    .build_load(field_ptr, &env.next_name("record.update.field.load"))?
            } else {
                codegen_context.builder.build_extract_value(
                    receiver_value.into_struct_value(),
                    u32::try_from(index).map_err(|conversion_error| {
                        CodegenError::new(format!("{conversion_error}"))
                    })?,
                    &env.next_name("record.update.field.extract"),
                )?
            };
            retain_record_update_field_if_needed(
                codegen_context,
                env,
                field_type,
                None,
                true,
                value,
            )?;
            value
        };
        lowered_fields.push(lowered);
    }

    let payload_size = struct_type.size_of().ok_or_else(|| {
        CodegenError::new(format!(
            "could not compute payload size for record update '{name}'"
        ))
    })?;
    let drop_children_fn = if field_layout.iter().any(|&(_, ref field_type)| {
        requires_rc_runtime_hooks(field_type) || field_type == &CoreType::String
    }) {
        let callback = declare_or_get_nominal_drop_children_fn(
            codegen_context,
            &receiver_core_type,
            field_layout.as_slice(),
        )?;
        let i8_ptr_type = codegen_context
            .context
            .i8_type()
            .ptr_type(AddressSpace::default());
        Some(
            callback
                .as_global_value()
                .as_pointer_value()
                .const_cast(i8_ptr_type),
        )
    } else {
        None
    };
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    let payload_ptr = emitter.emit_alloc(payload_size, drop_children_fn)?;
    let typed_ptr = codegen_context.builder.build_pointer_cast(
        payload_ptr,
        struct_type.ptr_type(AddressSpace::default()),
        &env.next_name("record.update.payload.cast"),
    )?;
    for (index, value) in lowered_fields.iter().enumerate() {
        let converted_index = u64::try_from(index)
            .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?;
        let field_ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                typed_ptr,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context
                        .context
                        .i32_type()
                        .const_int(converted_index, false),
                ],
                &env.next_name("record.update.store.ptr"),
            )?
        };
        let _store = codegen_context.builder.build_store(field_ptr, *value)?;
    }
    Ok(payload_ptr.as_basic_value_enum())
}

fn retain_record_update_field_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    field_type: &CoreType,
    source_expr: Option<&Expr>,
    copied_from_receiver: bool,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    let should_retain = copied_from_receiver
        || source_expr.is_some_and(string_expr_needs_retain_at_ownership_boundary);
    if !should_retain {
        return Ok(());
    }
    if field_type == &CoreType::String {
        return emit_string_retain(
            codegen_context,
            value,
            env.next_name("record.update.string.retain").as_str(),
        );
    }
    if !requires_rc_runtime_hooks(field_type) {
        return Ok(());
    }
    if !value.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "record update expected pointer value for RC field type '{field_type}'"
        )));
    }
    let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
    emitter.emit_inc(value.into_pointer_value())
}
