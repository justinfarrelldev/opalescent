extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::expressions_array::{
    load_array_data_ptr_for_element_type, load_array_length_from_value,
    materialize_runtime_array_from_raw_elements,
};
use crate::codegen::functions_call::call_arg_cleanup::{
    CallArgCleanupRecord, lower_call_argument,
};
use crate::type_system::types::CoreType;
use alloc::{boxed::Box, string::String, vec::Vec};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, IntValue, PointerValue};

pub(super) fn extract_error_abi_success_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: BasicValueEnum<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if !value.is_struct_value() {
        return Ok(value);
    }
    let struct_value = value.into_struct_value();
    if struct_value.get_type().count_fields() < 2 {
        return Ok(value);
    }
    codegen_context
        .builder
        .build_extract_value(struct_value, 0, env.next_name("call.success").as_str())
        .map_err(CodegenError::from)
}

fn lower_string_array_argument<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    argument: BasicValueEnum<'context>,
) -> Result<(PointerValue<'context>, IntValue<'context>), CodegenError> {
    if argument.is_struct_value() {
        let struct_value = argument.into_struct_value();
        if struct_value.get_type().count_fields() >= 3 {
            let raw_data = codegen_context
                .builder
                .build_extract_value(struct_value, 0, env.next_name("call.arg.raw").as_str())?
                .into_pointer_value();
            let length_value = codegen_context
                .builder
                .build_extract_value(struct_value, 1, env.next_name("call.arg.len").as_str())?
                .into_int_value();
            let array_payload = materialize_runtime_array_from_raw_elements(
                codegen_context,
                env,
                raw_data,
                length_value,
                &CoreType::String,
                "call.arg.materialized",
            )?;
            return Ok((
                load_array_data_ptr_for_element_type(
                    codegen_context,
                    env,
                    array_payload,
                    &CoreType::String,
                    "call.arg.materialized",
                )?,
                length_value,
            ));
        }
    }

    let argument_value = extract_error_abi_success_value(codegen_context, env, argument)?;
    if !argument_value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "string[] argument should lower to pointer value",
        )));
    }
    let array_payload = argument_value.into_pointer_value();
    let length_value = load_array_length_from_value(codegen_context, env, array_payload, "call.arg")?;
    Ok((
        load_array_data_ptr_for_element_type(
            codegen_context,
            env,
            array_payload,
            &CoreType::String,
            "call.arg",
        )?,
        length_value,
    ))
}

pub(super) fn maybe_lower_specialized_string_array_call<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
    lowered_args: &mut Vec<BasicMetadataValueEnum<'context>>,
    cleanup_records: &mut Vec<CallArgCleanupRecord>,
) -> Result<bool, CodegenError> {
    let Expr::Identifier { name, .. } = callee else {
        return Ok(false);
    };
    let runtime_name = env
        .imported_functions
        .get(name.as_str())
        .cloned()
        .unwrap_or_else(|| name.clone());

    match runtime_name.as_str() {
        "terminal_draw_rows_sync" if args.len() >= 2 => {
            let rows_argument = codegen_expression(
                codegen_context,
                env,
                &args[1],
                Some(&CoreType::Array(Box::new(CoreType::String))),
            )?;
            let (rows_ptr, rows_count) = lower_string_array_argument(codegen_context, env, rows_argument)?;
            lowered_args.clear();
            lowered_args.push(codegen_expression(codegen_context, env, &args[0], None)?.into());
            lowered_args.push(rows_ptr.into());
            lowered_args.push(rows_count.into());
            Ok(true)
        }
        "join_path_components" if args.len() >= 2 => {
            let base_argument = lower_call_argument(
                codegen_context,
                env,
                callee,
                0,
                &args[0],
                None,
                cleanup_records,
            )?;
            let components_argument = codegen_expression(
                codegen_context,
                env,
                &args[1],
                Some(&CoreType::Array(Box::new(CoreType::String))),
            )?;
            let (components_ptr, components_count) =
                lower_string_array_argument(codegen_context, env, components_argument)?;
            lowered_args.clear();
            lowered_args.push(base_argument.into());
            lowered_args.push(components_ptr.into());
            lowered_args.push(components_count.into());
            Ok(true)
        }
        "string_join" if args.len() >= 2 => {
            let array_argument = codegen_expression(
                codegen_context,
                env,
                &args[0],
                Some(&CoreType::Array(Box::new(CoreType::String))),
            )?;
            let separator_argument = lower_call_argument(
                codegen_context,
                env,
                callee,
                1,
                &args[1],
                None,
                cleanup_records,
            )?;
            let (array_ptr, length_value) = lower_string_array_argument(codegen_context, env, array_argument)?;
            lowered_args.clear();
            lowered_args.push(array_ptr.into());
            lowered_args.push(length_value.into());
            lowered_args.push(separator_argument.into());
            Ok(true)
        }
        _ => Ok(false),
    }
}
