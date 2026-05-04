//! Error-bearing ABI for Opalescent functions.
//!
//! This module implements the canonical ABI for functions that can return errors.
//! The ABI is designed for efficient error checking and propagation, matching the
//! definitions in `runtime/opal_runtime.h:125-141`.
//!
//! ### Supported ABI Shapes
//!
//! Functions that return a value `T` and can error use the shape `{T, i8*}`.
//! Functions that return `void` and can error use the shape `{i8*, i8*}`.
//!
//! ### Encoding Semantics
//!
//! - **Success**: The error field (index 1) contains a `null` pointer.
//! - **Error**: The error field (index 1) contains a non-null pointer to a
//!   globally interned string representing the error variant name.
//!
//! ### Error Field Index Rule
//!
//! The error field is typically at index 1 for the 2-field user ABI.
//! The general rule implemented is `(field_count >= 3) ? 2 : 1`.
//!
//! ### Unsupported Cases
//!
//! - **Aggregate-T returns**: Only scalar returns are currently supported in the error-bearing ABI.
//! - **Payload-bearing errors**: Only simple error variants (without data) are supported.
//!
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
use inkwell::AddressSpace;
use inkwell::context::Context;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue, StructValue};

fn i8_ptr_type<'context>(ctx: &'context Context) -> inkwell::types::PointerType<'context> {
    ctx.i8_type().ptr_type(AddressSpace::default())
}

fn build_aggregate_type<'context>(
    ctx: &'context Context,
    success_type: BasicTypeEnum<'context>,
) -> StructType<'context> {
    ctx.struct_type(&[success_type, i8_ptr_type(ctx).into()], false)
}

fn insert_aggregate_fields<'context>(
    codegen_context: &CodegenContext<'context>,
    aggregate_type: StructType<'context>,
    success_value: BasicValueEnum<'context>,
    error_value: BasicValueEnum<'context>,
    name: &str,
) -> Result<StructValue<'context>, CodegenError> {
    let mut aggregate = aggregate_type.get_undef();
    aggregate = codegen_context
        .builder
        .build_insert_value(aggregate, success_value, 0, name)?
        .into_struct_value();
    aggregate = codegen_context
        .builder
        .build_insert_value(aggregate, error_value, 1, name)?
        .into_struct_value();
    Ok(aggregate)
}

fn zero_value<'context>(success_type: BasicTypeEnum<'context>) -> BasicValueEnum<'context> {
    success_type.const_zero().as_basic_value_enum()
}

#[doc = "Build the canonical non-void error return type `{T, i8*}`."]
pub fn build_error_return_type<'context>(
    ctx: &'context Context,
    success_llvm_type: Option<BasicTypeEnum<'context>>,
) -> StructType<'context> {
    build_aggregate_type(
        ctx,
        success_llvm_type.unwrap_or_else(|| i8_ptr_type(ctx).into()),
    )
}

#[doc = "Return the error field index for a canonical result aggregate."]
pub fn error_field_index(field_count: u32) -> u32 {
    if field_count >= 3 { 2 } else { 1 }
}

#[doc = "Build a successful `{T, i8*}` result with a null error pointer."]
pub fn build_success_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
    success_value: BasicValueEnum<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type =
        build_error_return_type(codegen_context.context, Some(success_value.get_type()));
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        success_value,
        i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum(),
        "error_abi.success",
    )
}

#[doc = "Build an error `{T, i8*}` result with a default success payload."]
pub fn build_error_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
    success_type: BasicTypeEnum<'context>,
    error_value: PointerValue<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type = build_error_return_type(codegen_context.context, Some(success_type));
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        zero_value(success_type),
        error_value.as_basic_value_enum(),
        "error_abi.error",
    )
}

#[doc = "Build the canonical void success aggregate `{i8*, i8*}`."]
pub fn build_void_success_aggregate<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<StructValue<'context>, CodegenError> {
    let aggregate_type = build_error_return_type(codegen_context.context, None);
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum(),
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
    insert_aggregate_fields(
        codegen_context,
        aggregate_type,
        i8_ptr_type(codegen_context.context)
            .const_null()
            .as_basic_value_enum(),
        error_value.as_basic_value_enum(),
        "error_abi.void.error",
    )
}

#[doc = "Canonicalize a variant name for LLVM symbol interning."]
pub fn intern_variant_name<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    variant_name: &str,
) -> PointerValue<'context> {
    codegen_context
        .builder
        .build_global_string_ptr(variant_name, &env.next_name("variant.name"))
        .expect("global string pointer creation should succeed")
        .as_pointer_value()
}
