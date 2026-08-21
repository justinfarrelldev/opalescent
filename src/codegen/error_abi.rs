//! Error-bearing ABI for Opalescent functions.
//!
//! This module implements the canonical ABI for functions that can return errors.
//! The ABI is designed for efficient error checking and propagation, matching the
//! definitions in `runtime/opal_runtime.h:125-141`.
//!
//! ### Supported ABI Shapes
//!
//! Functions with success values use `ordered_success_fields..., i8*`.
//! Functions that return `void` and can error use the shape `{i8*, i8*}`.
//!
//! ### Encoding Semantics
//!
//! - **Success**: The trailing error field contains a `null` pointer.
//! - **Error**: The trailing error field contains a non-null pointer to an
//!   immutable runtime error string representing the primary error identity.
//!
//! ### Error Field Index Rule
//!
//! The error field is always the trailing field in the aggregate.
//!
//! ### Unsupported Cases
//!
//! - **Payload-bearing errors**: Only simple error variants (without data) are supported.
#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::missing_const_for_fn,
    clippy::missing_panics_doc,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue, StructValue};

fn i8_ptr_type<'context>(ctx: &'context Context) -> inkwell::types::PointerType<'context> {
    ctx.i8_type().ptr_type(AddressSpace::default())
}

fn aggregate_fields_from_success_type<'context>(
    ctx: &'context Context,
    success_type: Option<BasicTypeEnum<'context>>,
) -> alloc::vec::Vec<BasicTypeEnum<'context>> {
    let mut fields = match success_type {
        Some(BasicTypeEnum::StructType(struct_type)) => struct_type.get_field_types(),
        Some(other) => alloc::vec![other],
        None => alloc::vec![i8_ptr_type(ctx).into()],
    };
    fields.push(i8_ptr_type(ctx).into());
    fields
}

fn build_aggregate_type<'context>(
    ctx: &'context Context,
    success_type: Option<BasicTypeEnum<'context>>,
) -> StructType<'context> {
    ctx.struct_type(
        aggregate_fields_from_success_type(ctx, success_type).as_slice(),
        false,
    )
}

fn flatten_success_value<'context>(
    codegen_context: &CodegenContext<'context>,
    success_value: BasicValueEnum<'context>,
    name: &str,
) -> Result<alloc::vec::Vec<BasicValueEnum<'context>>, CodegenError> {
    if success_value.is_struct_value() {
        let struct_value = success_value.into_struct_value();
        let mut flattened = alloc::vec::Vec::new();
        for index in 0..struct_value.get_type().count_fields() {
            flattened.push(
                codegen_context
                    .builder
                    .build_extract_value(struct_value, index, name)?
                    .as_basic_value_enum(),
            );
        }
        Ok(flattened)
    } else {
        Ok(alloc::vec![success_value])
    }
}

fn insert_aggregate_fields<'context>(
    codegen_context: &CodegenContext<'context>,
    aggregate_type: StructType<'context>,
    success_values: &[BasicValueEnum<'context>],
    error_value: BasicValueEnum<'context>,
    name: &str,
) -> Result<StructValue<'context>, CodegenError> {
    let mut aggregate = aggregate_type.get_undef();
    for (index, success_value) in success_values.iter().enumerate() {
        aggregate = codegen_context
            .builder
            .build_insert_value(
                aggregate,
                *success_value,
                u32::try_from(index)
                    .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?,
                name,
            )?
            .into_struct_value();
    }
    aggregate = codegen_context
        .builder
        .build_insert_value(
            aggregate,
            error_value,
            error_field_index(aggregate_type.count_fields()),
            name,
        )?
        .into_struct_value();
    Ok(aggregate)
}

fn zero_success_values_for_return_type<'context>(
    aggregate_type: StructType<'context>,
) -> alloc::vec::Vec<BasicValueEnum<'context>> {
    let error_index = error_field_index(aggregate_type.count_fields());
    aggregate_type
        .get_field_types()
        .into_iter()
        .enumerate()
        .filter_map(|(index, field_type)| {
            (u32::try_from(index).ok() != Some(error_index))
                .then_some(field_type.const_zero().as_basic_value_enum())
        })
        .collect()
}

#[doc = "Build the canonical error return aggregate from a success type shape."]
pub fn build_error_return_type<'context>(
    ctx: &'context Context,
    success_llvm_type: Option<BasicTypeEnum<'context>>,
) -> StructType<'context> {
    build_aggregate_type(ctx, success_llvm_type)
}

#[doc = "Return the trailing error field index for a canonical result aggregate."]
pub fn error_field_index(field_count: u32) -> u32 {
    field_count.saturating_sub(1)
}

#[doc = "True when a struct type matches the canonical error ABI shape."]
pub fn is_error_abi_struct_type<'context>(struct_type: StructType<'context>) -> bool {
    if struct_type.count_fields() < 2 {
        return false;
    }

    let field_types = struct_type.get_field_types();
    let Ok(error_index) = usize::try_from(error_field_index(struct_type.count_fields())) else {
        return false;
    };
    let Some(error_type) = field_types.get(error_index) else {
        return false;
    };
    if !error_type.is_pointer_type() {
        return false;
    }
    let error_pointee = error_type.into_pointer_type().get_element_type();
    error_pointee.is_int_type() && error_pointee.into_int_type().get_bit_width() == 8
}

#[doc = "Build a successful error-ABI aggregate with a null error pointer."]
pub fn build_success_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
    success_value: BasicValueEnum<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type =
        build_error_return_type(codegen_context.context, Some(success_value.get_type()));
    let success_values =
        flatten_success_value(codegen_context, success_value, "error_abi.success.flatten")?;
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        success_values.as_slice(),
        i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum(),
        "error_abi.success",
    )
}

#[doc = "Build an error aggregate matching the full return ABI shape."]
pub fn build_error_aggregate_for_return_type<'context>(
    codegen_context: &CodegenContext<'context>,
    aggregate_type: StructType<'context>,
    error_value: PointerValue<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let success_values = zero_success_values_for_return_type(aggregate_type);
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        success_values.as_slice(),
        error_value.as_basic_value_enum(),
        "error_abi.error",
    )
}

#[doc = "Build an error aggregate using only the success type shape."]
pub fn build_error_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
    success_type: BasicTypeEnum<'context>,
    error_value: PointerValue<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type = build_error_return_type(codegen_context.context, Some(success_type));
    build_error_aggregate_for_return_type(codegen_context, aggregate_type, error_value)
}

#[doc = "Build the canonical void success aggregate `{i8*, i8*}`."]
pub fn build_void_success_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type = build_error_return_type(codegen_context.context, None);
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        &[i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum()],
        i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum(),
        "error_abi.void.success",
    )
}

#[doc = "Build the canonical void error aggregate `{i8*, i8*}`."]
pub fn build_void_error_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
    error_value: PointerValue<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type = build_error_return_type(codegen_context.context, None);
    build_error_aggregate_for_return_type(codegen_context, aggregate_type, error_value)
}

/// Attach an already-evaluated cause to an immutable runtime error value.
pub fn attach_error_cause<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    primary: PointerValue<'context>,
    cause: PointerValue<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let attach = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_error_attach_cause",
    )
    .ok_or_else(|| {
        CodegenError::new(String::from("opal_error_attach_cause declaration missing"))
    })?;
    let call = codegen_context.builder.build_call(
        attach,
        &[primary.into(), cause.into()],
        env.next_name("error.attach").as_str(),
    )?;
    call.try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_error_attach_cause returned void")))
        .map(|value| value.into_pointer_value())
}

#[doc = "Canonicalize a variant name into a fresh immutable runtime error value."]
pub fn intern_variant_name<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    variant_name: &str,
) -> Result<PointerValue<'context>, CodegenError> {
    let variant_ptr = codegen_context
        .builder
        .build_global_string_ptr(variant_name, &env.next_name("variant.name"))?
        .as_pointer_value();
    let constructor = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_error_new",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_error_new declaration missing")))?;
    let call = codegen_context.builder.build_call(
        constructor,
        &[variant_ptr.into()],
        env.next_name("error.new").as_str(),
    )?;
    call.try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("opal_error_new returned void")))
        .map(|value| value.into_pointer_value())
}
