#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::pattern_type_mismatch,
    clippy::similar_names,
    clippy::too_many_lines,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Expr, Stmt, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::expressions_array::{
    codegen_array_at_call, codegen_string_at_call, infer_expression_core_type,
    load_array_data_ptr_for_element_type, load_array_length_from_value,
    materialize_runtime_array_from_raw_elements,
};
use crate::codegen::monomorphization::ensure_monomorphized_function_declaration;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};

#[path = "functions_call/array.rs"]
mod array;
#[path = "functions_call/call_arg_cleanup.rs"]
mod call_arg_cleanup;
#[path = "functions_call_helpers.rs"]
#[doc = "Helper utilities for call-expression lowering internals."]
mod functions_call_helpers;
#[path = "functions_call/string_array_calls.rs"]
mod string_array_calls;
#[path = "functions_call/tail.rs"]
mod tail;
#[path = "functions_call/using_cleanup.rs"]
mod using_cleanup;
use self::array::{
    codegen_array_intrinsic_call, codegen_array_member_call, is_array_intrinsic_name,
};
use self::call_arg_cleanup::{cleanup_call_argument_temporaries, lower_call_argument};
use self::functions_call_helpers::{
    caller_returns_error_aggregate, current_function, infer_guard_binding_core_type,
    llvm_metadata_type_to_core_type, uses_aggregate_result_dispatch,
};
use self::string_array_calls::{
    extract_error_abi_success_value, maybe_lower_specialized_string_array_call,
};
use self::tail::declare_external_imported_function;
use self::using_cleanup::{
    build_error_variant_match, consume_using_cleanup_obligation_after_success,
    emit_cleanup_aware_error_return, using_cleanup_close_binding, using_cleanup_transfer_variant,
};

pub fn build_function_type<'context>(
    codegen_context: &CodegenContext<'context>,
    parameters: &[BasicMetadataTypeEnum<'context>],
    returns: &[CoreType],
    error_types: &[CoreType],
) -> Result<inkwell::types::FunctionType<'context>, CodegenError> {
    tail::build_function_type(codegen_context, parameters, returns, error_types)
}

pub fn emit_default_return(
    codegen_context: &CodegenContext<'_>,
    env: &mut CodegenEnv<'_>,
    returns: &[CoreType],
) -> Result<(), CodegenError> {
    tail::emit_default_return(codegen_context, env, returns)
}

pub fn ast_type_to_core_type_for_signature(ast_type: &Type) -> Result<CoreType, CodegenError> {
    tail::ast_type_to_core_type_for_signature(ast_type)
}

pub fn emit_c_main_wrapper<'context>(
    codegen_context: &CodegenContext<'context>,
    entry_function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    tail::emit_c_main_wrapper(codegen_context, entry_function)
}

fn lower_array_argument<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    argument: &Expr,
    element_core_type: &CoreType,
) -> Result<(PointerValue<'context>, IntValue<'context>), CodegenError> {
    let argument_value = codegen_expression(
        codegen_context,
        env,
        argument,
        Some(&CoreType::Array(Box::new(element_core_type.clone()))),
    )?;
    let argument_value = extract_error_abi_success_value(codegen_context, env, argument_value)?;
    if !argument_value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "array argument should lower to pointer value",
        )));
    }

    let array_payload = argument_value.into_pointer_value();
    let length_value =
        load_array_length_from_value(codegen_context, env, array_payload, "call.arg")?;
    let data_ptr = load_array_data_ptr_for_element_type(
        codegen_context,
        env,
        array_payload,
        element_core_type,
        "call.arg",
    )?;
    Ok((data_ptr, length_value))
}

fn expected_argument_core_type<'context>(
    env: &CodegenEnv<'context>,
    callee: &Expr,
    function: FunctionValue<'context>,
    arg_index: usize,
) -> Option<CoreType> {
    if let Expr::Identifier { ref name, .. } = *callee {
        if name == "print" {
            return None;
        }
        if let Some(&CoreType::Function { ref parameters, .. }) = env.imported_signatures.get(name)
        {
            if let Some(parameter) = parameters.get(arg_index) {
                return Some(parameter.clone());
            }
        }
    }

    let uses_sret =
        uses_aggregate_result_dispatch(function) && function.get_type().get_return_type().is_none();
    let llvm_arg_index = if uses_sret {
        arg_index.saturating_add(1)
    } else {
        arg_index
    };
    function
        .get_type()
        .get_param_types()
        .get(llvm_arg_index)
        .copied()
        .map(llvm_metadata_type_to_core_type)
}

#[doc = "Lower a function call expression."]
#[expect(
    clippy::too_many_lines,
    reason = "Function call requires complex argument binding"
)]
pub fn codegen_call_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    generic_args: Option<&[Type]>,
    args: &[Expr],
    _expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if let Expr::Identifier { ref name, .. } = *callee {
        let imported_name = env.imported_functions.get(name).cloned();
        let direct_intrinsic = is_array_intrinsic_name(name.as_str());
        let imported_intrinsic = imported_name
            .as_deref()
            .is_some_and(is_array_intrinsic_name);
        if direct_intrinsic || imported_intrinsic {
            let intrinsic_name = if direct_intrinsic {
                name.clone()
            } else {
                imported_name.ok_or_else(|| {
                    CodegenError::new(format!("missing imported intrinsic mapping for '{name}'"))
                })?
            };
            return codegen_array_intrinsic_call(
                codegen_context,
                env,
                intrinsic_name.as_str(),
                args,
            );
        }
    }
    if let Expr::Member {
        ref object,
        ref member,
        ..
    } = *callee
    {
        if member == "at" && args.len() == 1 {
            match infer_expression_core_type(env, object.as_ref()) {
                Some(CoreType::String) => {
                    return codegen_string_at_call(codegen_context, env, object.as_ref(), &args[0]);
                }
                Some(CoreType::Array(_)) => {
                    return codegen_array_at_call(codegen_context, env, object.as_ref(), &args[0]);
                }
                _ => {}
            }
        }

        if let Expr::Identifier { ref name, .. } = *object.as_ref() {
            if env
                .variables
                .get(name.as_str())
                .is_some_and(|binding| matches!(binding.core_type, CoreType::Array(_)))
            {
                return codegen_array_member_call(
                    codegen_context,
                    env,
                    object.as_ref(),
                    member.as_str(),
                    args,
                );
            }
        }
    }

    let function = resolve_callee_function(codegen_context, env, callee, generic_args)?;
    let call_temp_scope_depth = env.current_scope_depth();
    let _call_temp_scope = env.enter_scope();
    let mut cleanup_records = Vec::new();

    let call_result = (|| -> Result<BasicValueEnum<'context>, CodegenError> {
        let mut lowered_args: Vec<BasicMetadataValueEnum<'context>> = Vec::new();
        let mut first_lowered_arg: Option<BasicValueEnum<'context>> = None;
        let specialized = maybe_lower_specialized_string_array_call(
            codegen_context,
            env,
            callee,
            args,
            &mut lowered_args,
            &mut cleanup_records,
        )?;
        if !specialized {
            for (index, arg) in args.iter().enumerate() {
                let expected_arg_type = expected_argument_core_type(env, callee, function, index);
                if let Some(&CoreType::Array(ref element_core_type)) = expected_arg_type.as_ref() {
                    let (array_ptr, array_length) = lower_array_argument(
                        codegen_context,
                        env,
                        arg,
                        element_core_type.as_ref(),
                    )?;
                    if index == 0 {
                        first_lowered_arg = Some(array_ptr.as_basic_value_enum());
                    }
                    lowered_args.push(array_ptr.into());
                    lowered_args.push(array_length.into());
                    continue;
                }

                let lowered = lower_call_argument(
                    codegen_context,
                    env,
                    callee,
                    index,
                    arg,
                    expected_arg_type.as_ref(),
                    &mut cleanup_records,
                )?;
                if index == 0 {
                    first_lowered_arg = Some(lowered);
                }
                lowered_args.push(lowered.into());
            }
        }

        if let Expr::Lambda {
            ref captured_variables,
            ..
        } = *callee
        {
            for capture in captured_variables {
                if let Some(binding) = env.variables.get(capture.as_str()) {
                    let loaded = codegen_context
                        .builder
                        .build_load(binding.alloca, capture.as_str())?;
                    lowered_args.push(loaded.into());
                } else {
                    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
                        codegen_context,
                        "opal_runtime_error",
                    )
                    .ok_or_else(|| {
                        CodegenError::new(String::from("opal_runtime_error declaration missing"))
                    })?;
                    let error_message = format!("captured variable '{capture}' not found in scope");
                    let msg = codegen_context
                        .builder
                        .build_global_string_ptr(error_message.as_str(), &env.next_name("cap.msg"))?
                        .as_pointer_value();
                    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
                        runtime_fn,
                        &[msg.into()],
                        &env.next_name("cap.call"),
                    )?;
                    let _: inkwell::values::InstructionValue =
                        codegen_context.builder.build_unreachable()?;
                    let continuation = codegen_context
                        .context
                        .append_basic_block(current_function(codegen_context)?, "capture.cont");
                    codegen_context.builder.position_at_end(continuation);
                    lowered_args.push(codegen_context.context.i64_type().get_undef().into());
                }
            }
        }

        if let Expr::Identifier { ref name, .. } = *callee {
            if name == "print" {
                if let Some(print_value) = first_lowered_arg {
                    let void_value = codegen_context
                        .context
                        .struct_type(&[], false)
                        .const_zero()
                        .as_basic_value_enum();
                    if print_value.is_int_value() {
                        let int_value = print_value.into_int_value();
                        let bit_width = int_value.get_type().get_bit_width();
                        if bit_width == 1_u32 {
                            let bool_to_string_fn =
                                crate::codegen::functions_stdlib::declare_stdlib_function(
                                    codegen_context,
                                    "bool_to_string",
                                )
                                .ok_or_else(|| {
                                    CodegenError::new(String::from(
                                        "bool_to_string declaration missing",
                                    ))
                                })?;
                            let puts_fn =
                                codegen_context.module.get_function("puts").ok_or_else(|| {
                                    CodegenError::new(String::from("puts declaration missing"))
                                })?;
                            let bool_as_i8 = codegen_context.builder.build_int_z_extend(
                                int_value,
                                codegen_context.context.i8_type(),
                                &env.next_name("print.bool.i8"),
                            )?;
                            let bool_string_ptr = codegen_context
                                .builder
                                .build_call(
                                    bool_to_string_fn,
                                    &[bool_as_i8.as_basic_value_enum().into()],
                                    &env.next_name("print.bool.str"),
                                )?
                                .try_as_basic_value()
                                .basic()
                                .ok_or_else(|| {
                                    CodegenError::new(String::from(
                                        "bool_to_string should return pointer value",
                                    ))
                                })?
                                .into_pointer_value();
                            let _: inkwell::values::CallSiteValue =
                                codegen_context.builder.build_call(
                                    puts_fn,
                                    &[bool_string_ptr.into()],
                                    &env.next_name("print.bool.puts"),
                                )?;
                            let i8_ptr = codegen_context
                                .context
                                .i8_type()
                                .ptr_type(AddressSpace::default());
                            let free_fn_type = codegen_context
                                .context
                                .void_type()
                                .fn_type(&[i8_ptr.into()], false);
                            let free_fn = codegen_context
                                .module
                                .get_function("free")
                                .unwrap_or_else(|| {
                                    codegen_context
                                        .module
                                        .add_function("free", free_fn_type, None)
                                });
                            let _: inkwell::values::CallSiteValue =
                                codegen_context.builder.build_call(
                                    free_fn,
                                    &[bool_string_ptr.into()],
                                    &env.next_name("print.bool.free"),
                                )?;
                            return Ok(void_value);
                        }

                        let print_fn_name = match bit_width {
                            8 => "print_int8",
                            16 => "print_int16",
                            32 => "print_int32",
                            _ => "print_int64",
                        };
                        let print_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
                            codegen_context,
                            print_fn_name,
                        )
                        .ok_or_else(|| {
                            CodegenError::new(format!("{print_fn_name} declaration missing"))
                        })?;
                        let _: inkwell::values::CallSiteValue =
                            codegen_context.builder.build_call(
                                print_fn,
                                &[int_value.into()],
                                &env.next_name("print.int"),
                            )?;
                        return Ok(void_value);
                    }

                    if print_value.is_float_value() {
                        let float_value = print_value.into_float_value();
                        let bit_width = float_value.get_type().get_bit_width();
                        let print_fn_name = match bit_width {
                            32 => "print_float32",
                            _ => "print_float64",
                        };
                        let print_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
                            codegen_context,
                            print_fn_name,
                        )
                        .ok_or_else(|| {
                            CodegenError::new(format!("{print_fn_name} declaration missing"))
                        })?;
                        let _: inkwell::values::CallSiteValue =
                            codegen_context.builder.build_call(
                                print_fn,
                                &[float_value.into()],
                                &env.next_name("print.float"),
                            )?;
                        return Ok(void_value);
                    }
                }
            }
        }

        if uses_aggregate_result_dispatch(function) {
            if let Some(return_type) = function.get_type().get_return_type() {
                if return_type.is_struct_type() {
                    let result_struct_type = return_type.into_struct_type();
                    if result_struct_type.count_fields() >= 2 {
                        let result_alloca = codegen_context.builder.build_alloca(
                            result_struct_type,
                            env.next_name("call.result").as_str(),
                        )?;
                        let call = codegen_context.builder.build_call(
                            function,
                            lowered_args.as_slice(),
                            env.next_name("call").as_str(),
                        )?;
                        if let Some(result_value) = call.try_as_basic_value().basic() {
                            codegen_context
                                .builder
                                .build_store(result_alloca, result_value)?;
                        } else {
                            return Err(CodegenError::new(String::from(
                                "aggregate error-abi call should return struct result",
                            )));
                        }
                        return codegen_context
                            .builder
                            .build_load(result_alloca, env.next_name("call.result.load").as_str())
                            .map_err(CodegenError::from);
                    }
                }
            }

            let result_param = function
                .get_type()
                .get_param_types()
                .first()
                .copied()
                .ok_or_else(|| {
                    CodegenError::new(String::from(
                        "aggregate runtime call missing result storage parameter",
                    ))
                })?;
            let result_struct_type = result_param
                .into_pointer_type()
                .get_element_type()
                .into_struct_type();
            let result_alloca = codegen_context
                .builder
                .build_alloca(result_struct_type, env.next_name("call.result").as_str())?;
            let mut call_args = Vec::with_capacity(lowered_args.len().saturating_add(1));
            call_args.push(result_alloca.into());
            call_args.extend(lowered_args);
            let _call = codegen_context.builder.build_call(
                function,
                call_args.as_slice(),
                env.next_name("call").as_str(),
            )?;
            return codegen_context
                .builder
                .build_load(result_alloca, env.next_name("call.result.load").as_str())
                .map_err(CodegenError::from);
        }

        let call_args = lowered_args;

        let call = codegen_context.builder.build_call(
            function,
            call_args.as_slice(),
            env.next_name("call").as_str(),
        )?;
        let call_result = call.try_as_basic_value().basic().map_or_else(
            || {
                codegen_context
                    .context
                    .struct_type(&[], false)
                    .const_zero()
                    .as_basic_value_enum()
            },
            |value| value,
        );

        if let Expr::Identifier { ref name, .. } = *callee {
            let runtime_name = env
                .imported_functions
                .get(name.as_str())
                .map_or_else(|| name.as_str(), String::as_str);
            let direct_runtime_boolean = matches!(
                runtime_name,
                "terminal_supports_ansi" | "environment_variable_exists" | "string_is_blank"
            );
            if direct_runtime_boolean && call_result.is_int_value() {
                let int_value = call_result.into_int_value();
                if int_value.get_type().get_bit_width() == 8_u32 {
                    let coerced = codegen_context.builder.build_int_compare(
                        IntPredicate::NE,
                        int_value,
                        codegen_context.context.i8_type().const_zero(),
                        env.next_name("call.bool.i1").as_str(),
                    )?;
                    return Ok(coerced.as_basic_value_enum());
                }
            }
        }

        Ok(call_result)
    })();

    let cleanup_result = cleanup_call_argument_temporaries(
        codegen_context,
        env,
        call_temp_scope_depth,
        cleanup_records.as_slice(),
    );
    match call_result {
        Ok(value) => {
            cleanup_result?;
            Ok(value)
        }
        Err(error) => Err(error),
    }
}

#[doc = "Lower propagate expression control flow."]
pub fn codegen_propagate_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    call_expr: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let explicit_close_transfer = if let Expr::Call {
        ref callee,
        ref args,
        ..
    } = *call_expr
    {
        using_cleanup_close_binding(env, callee.as_ref(), args.as_slice()).and_then(
            |binding_name| {
                using_cleanup_transfer_variant(env, binding_name.as_str())
                    .map(|variant| (binding_name, variant))
            },
        )
    } else {
        None
    };
    let value = if let Expr::Call {
        ref callee,
        ref args,
        ..
    } = *call_expr
    {
        codegen_call_expression(
            codegen_context,
            env,
            callee.as_ref(),
            None,
            args.as_slice(),
            expected_type,
        )?
    } else {
        codegen_expression(codegen_context, env, call_expr, expected_type)?
    };
    if value.is_struct_value() {
        let struct_value = value.into_struct_value();
        let field_count = struct_value.get_type().count_fields();
        if field_count >= 2 {
            let error_field_index = crate::codegen::error_abi::error_field_index(field_count);
            let error_field = codegen_context.builder.build_extract_value(
                struct_value,
                error_field_index,
                env.next_name("propagate.err").as_str(),
            )?;
            let current_fn = current_function(codegen_context)?;
            let forward_error = error_field
                .is_pointer_value()
                .then(|| error_field.into_pointer_value())
                .filter(|_| caller_returns_error_aggregate(current_fn));
            let early_return = codegen_context
                .context
                .append_basic_block(current_fn, env.next_name("propagate.ret").as_str());
            let continue_block = codegen_context
                .context
                .append_basic_block(current_fn, env.next_name("propagate.cont").as_str());
            if error_field.is_pointer_value() {
                let is_error = codegen_context.builder.build_is_not_null(
                    error_field.into_pointer_value(),
                    env.next_name("propagate.is_err").as_str(),
                )?;
                let _branch = codegen_context.builder.build_conditional_branch(
                    is_error,
                    early_return,
                    continue_block,
                )?;
            } else {
                let flag = error_field.into_int_value();
                let _branch = codegen_context.builder.build_conditional_branch(
                    flag,
                    early_return,
                    continue_block,
                )?;
            }
            codegen_context.builder.position_at_end(early_return);
            if let (Some(body_error), Some((binding_name, transfer_variant))) =
                (forward_error, explicit_close_transfer)
            {
                let transfer_cleanup = codegen_context.context.append_basic_block(
                    current_fn,
                    env.next_name("using.transfer.cleanup").as_str(),
                );
                let ordinary_cleanup = codegen_context.context.append_basic_block(
                    current_fn,
                    env.next_name("using.ordinary.cleanup").as_str(),
                );
                let is_transfer = build_error_variant_match(
                    codegen_context,
                    env,
                    body_error,
                    transfer_variant.as_str(),
                )?;
                let _branch = codegen_context.builder.build_conditional_branch(
                    is_transfer,
                    transfer_cleanup,
                    ordinary_cleanup,
                )?;
                codegen_context.builder.position_at_end(transfer_cleanup);
                emit_cleanup_aware_error_return(
                    codegen_context,
                    env,
                    current_fn,
                    Some(body_error),
                    &[binding_name],
                )?;
                codegen_context.builder.position_at_end(ordinary_cleanup);
                emit_cleanup_aware_error_return(
                    codegen_context,
                    env,
                    current_fn,
                    Some(body_error),
                    &[],
                )?;
            } else {
                emit_cleanup_aware_error_return(
                    codegen_context,
                    env,
                    current_fn,
                    forward_error,
                    &[],
                )?;
            }
            codegen_context.builder.position_at_end(continue_block);
            if let Expr::Call {
                ref callee,
                ref args,
                ..
            } = *call_expr
            {
                consume_using_cleanup_obligation_after_success(
                    env,
                    callee.as_ref(),
                    args.as_slice(),
                );
            }
            let success_field_count = crate::codegen::error_abi::error_field_index(field_count);
            if success_field_count == 0 {
                return Ok(value);
            }
            let first_success_value = codegen_context
                .builder
                .build_extract_value(struct_value, 0, env.next_name("propagate.ok").as_str())
                .map_err(CodegenError::from)?;
            if let Some(&CoreType::Array(ref element_core_type)) = expected_type {
                if success_field_count == 2 && first_success_value.is_pointer_value() {
                    let length_value = codegen_context
                        .builder
                        .build_extract_value(
                            struct_value,
                            1,
                            env.next_name("propagate.len").as_str(),
                        )
                        .map_err(CodegenError::from)?
                        .into_int_value();
                    let runtime_array = materialize_runtime_array_from_raw_elements(
                        codegen_context,
                        env,
                        first_success_value.into_pointer_value(),
                        length_value,
                        element_core_type.as_ref(),
                        "propagate.array",
                    )?;
                    return Ok(runtime_array.as_basic_value_enum());
                }
            }
            if success_field_count == 1 {
                return Ok(first_success_value);
            }
            let mut success_fields = Vec::new();
            success_fields.push(first_success_value);
            for index in 1..success_field_count {
                success_fields.push(
                    codegen_context
                        .builder
                        .build_extract_value(
                            struct_value,
                            index,
                            env.next_name("propagate.ok.more").as_str(),
                        )
                        .map_err(CodegenError::from)?
                        .as_basic_value_enum(),
                );
            }
            let success_aggregate_type = codegen_context.context.struct_type(
                success_fields
                    .iter()
                    .map(BasicValueEnum::get_type)
                    .collect::<Vec<_>>()
                    .as_slice(),
                false,
            );
            let mut success_aggregate = success_aggregate_type.get_undef();
            for (index, field) in success_fields.iter().enumerate() {
                success_aggregate = codegen_context
                    .builder
                    .build_insert_value(
                        success_aggregate,
                        *field,
                        u32::try_from(index).map_err(|conversion_error| {
                            CodegenError::new(format!("{conversion_error}"))
                        })?,
                        env.next_name("propagate.ok.insert").as_str(),
                    )?
                    .into_struct_value();
            }
            return Ok(success_aggregate.as_basic_value_enum());
        }
    }
    Ok(value)
}

#[doc = "Lower guard expression binding logic."]
pub fn codegen_guard_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    guarded_expr: &Expr,
    binding_name: &str,
    else_branch: &Stmt,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let value = if let Expr::Call {
        ref callee,
        ref args,
        ..
    } = *guarded_expr
    {
        codegen_call_expression(
            codegen_context,
            env,
            callee.as_ref(),
            None,
            args.as_slice(),
            expected_type,
        )?
    } else {
        codegen_expression(codegen_context, env, guarded_expr, expected_type)?
    };
    if value.is_struct_value() {
        let struct_value = value.into_struct_value();
        let field_count = struct_value.get_type().count_fields();
        if field_count >= 2 {
            let success_value = codegen_context.builder.build_extract_value(
                struct_value,
                0,
                env.next_name("guard.ok").as_str(),
            )?;
            let error_field_index = crate::codegen::error_abi::error_field_index(field_count);
            let error_value = codegen_context.builder.build_extract_value(
                struct_value,
                error_field_index,
                env.next_name("guard.err").as_str(),
            )?;
            if error_value.is_pointer_value() {
                let Stmt::Expression { ref expr, .. } = *else_branch else {
                    return Err(CodegenError::new(String::from(
                        "guard expression else branch must yield a value",
                    )));
                };

                let binding_alloca = codegen_context.builder.build_alloca(
                    success_value.get_type(),
                    env.next_name("guard.bind").as_str(),
                )?;
                let binding_core_type =
                    infer_guard_binding_core_type(env, guarded_expr, success_value.get_type());
                env.variables.insert(
                    binding_name.to_owned(),
                    VariableBinding {
                        alloca: binding_alloca,
                        core_type: binding_core_type,
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );

                let current_fn = current_function(codegen_context)?;
                let success_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.expr.success").as_str());
                let else_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.expr.else").as_str());
                let merge_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.expr.merge").as_str());
                let error_ptr = error_value.into_pointer_value();
                let is_success = codegen_context
                    .builder
                    .build_is_null(error_ptr, env.next_name("guard.expr.is_success").as_str())?;
                codegen_context.builder.build_conditional_branch(
                    is_success,
                    success_block,
                    else_block,
                )?;

                codegen_context.builder.position_at_end(success_block);
                codegen_context
                    .builder
                    .build_store(binding_alloca, success_value)?;
                let success_end = codegen_context.builder.get_insert_block().ok_or_else(|| {
                    CodegenError::new(String::from("guard expression success block missing"))
                })?;
                codegen_context
                    .builder
                    .build_unconditional_branch(merge_block)?;

                codegen_context.builder.position_at_end(else_block);
                let else_value = codegen_expression(codegen_context, env, expr, expected_type)?;
                let else_end = codegen_context.builder.get_insert_block().ok_or_else(|| {
                    CodegenError::new(String::from("guard expression else block missing"))
                })?;
                codegen_context
                    .builder
                    .build_unconditional_branch(merge_block)?;

                codegen_context.builder.position_at_end(merge_block);
                let phi = codegen_context.builder.build_phi(
                    success_value.get_type(),
                    env.next_name("guard.expr.phi").as_str(),
                )?;
                phi.add_incoming(&[(&success_value, success_end), (&else_value, else_end)]);
                return Ok(phi.as_basic_value());
            }
        }
    }
    Ok(value)
}

#[doc = "Resolve the called function value for identifier or lambda callees."]
#[expect(
    clippy::too_many_lines,
    clippy::arithmetic_side_effects,
    clippy::uninlined_format_args,
    clippy::pattern_type_mismatch,
    reason = "Lambda body codegen requires complex parameter/capture binding and body generation"
)]
fn resolve_callee_function<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    generic_args: Option<&[Type]>,
) -> Result<FunctionValue<'context>, CodegenError> {
    match *callee {
        Expr::Identifier { ref name, .. } => {
            let is_stdlib_name =
                crate::codegen::functions_stdlib::is_stdlib_runtime_name(name.as_str())
                    || is_array_intrinsic_name(name.as_str());
            let base_function = if let Some(imported_runtime_name) =
                env.imported_functions.get(name)
            {
                if is_array_intrinsic_name(imported_runtime_name.as_str()) {
                    return Err(CodegenError::new(format!(
                        "{imported_runtime_name} is compiler-lowered and does not resolve to a standalone runtime symbol",
                    )));
                }
                if let Some(error) =
                    crate::codegen::functions_stdlib::terminal_proposal_runtime_gate_error(
                        imported_runtime_name.as_str(),
                    )
                {
                    return Err(error);
                }
                codegen_context
                    .module
                    .get_function(imported_runtime_name.as_str())
                    .or_else(|| {
                        crate::codegen::functions_stdlib::declare_stdlib_function(
                            codegen_context,
                            imported_runtime_name.as_str(),
                        )
                    })
                    .ok_or_else(|| {
                        CodegenError::new(format!(
                            "missing runtime function for imported symbol '{name}'"
                        ))
                    })?
            } else if let Some(existing) = codegen_context.module.get_function(name.as_str()) {
                existing
            } else if let Some(imported_signature) = env.imported_signatures.get(name).cloned() {
                declare_external_imported_function(
                    codegen_context,
                    name.as_str(),
                    &imported_signature,
                )?
            } else if let Some(stdlib_function) =
                crate::codegen::functions_stdlib::declare_stdlib_function(codegen_context, name)
            {
                stdlib_function
            } else {
                return Err(CodegenError::new(format!("unknown function: {name}")));
            };
            if let Some(explicit_generic_args) = generic_args {
                let concrete_types = explicit_generic_args
                    .iter()
                    .map(ast_type_to_core_type_for_signature)
                    .collect::<Result<Vec<_>, _>>()?;
                if !concrete_types.is_empty() && !is_stdlib_name {
                    return Ok(ensure_monomorphized_function_declaration(
                        codegen_context,
                        env,
                        base_function,
                        name,
                        concrete_types.as_slice(),
                    ));
                }
            }
            Ok(base_function)
        }
        Expr::Lambda {
            ref params,
            ref return_types,
            ref error_types,
            ref captured_variables,
            ref body,
            ..
        } => {
            let mut parameter_types = params
                .iter()
                .map(|param| ast_type_to_core_type_for_signature(&param.param_type))
                .collect::<Result<Vec<_>, _>>()?;
            for capture in captured_variables {
                if let Some(binding) = env.variables.get(capture) {
                    parameter_types.push(binding.core_type.clone());
                } else {
                    parameter_types.push(CoreType::Int64);
                }
            }
            let return_core_types = return_types
                .iter()
                .map(ast_type_to_core_type_for_signature)
                .collect::<Result<Vec<_>, _>>()?;
            let metadata_params = parameter_types
                .iter()
                .map(|core_type| core_type_to_llvm(codegen_context.context, core_type).into())
                .collect::<Vec<BasicMetadataTypeEnum<'context>>>();
            let error_core_types = error_types
                .iter()
                .map(|error_type| CoreType::Generic {
                    name: error_type.clone(),
                    type_args: Vec::new(),
                })
                .collect::<Vec<_>>();
            let function_type = build_function_type(
                codegen_context,
                &metadata_params,
                &return_core_types,
                &error_core_types,
            )?;
            let lambda_name = env.next_name("lambda");
            let function =
                codegen_context
                    .module
                    .add_function(lambda_name.as_str(), function_type, None);
            let entry = codegen_context
                .context
                .append_basic_block(function, "entry");
            codegen_context.builder.position_at_end(entry);

            // Bind parameters to allocas
            let args: Vec<_> = function.get_params().into_iter().collect();
            let mut shadowed_bindings: Vec<(String, Option<VariableBinding<'context>>)> =
                Vec::new();
            for (i, param) in params.iter().enumerate() {
                let param_value = args[i];
                let alloca = codegen_context
                    .builder
                    .build_alloca(param_value.get_type(), &param.name)?;
                codegen_context.builder.build_store(alloca, param_value)?;
                let previous_binding = env.variables.insert(
                    param.name.clone(),
                    VariableBinding {
                        alloca,
                        core_type: parameter_types[i].clone(),
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
                shadowed_bindings.push((param.name.clone(), previous_binding));
            }

            // Bind captured variables to allocas
            for (i, capture) in captured_variables.iter().enumerate() {
                let capture_value = args[params.len() + i];
                let alloca = codegen_context
                    .builder
                    .build_alloca(capture_value.get_type(), &format!("capture_{}", capture))?;
                codegen_context.builder.build_store(alloca, capture_value)?;
                let previous_binding = env.variables.insert(
                    capture.clone(),
                    VariableBinding {
                        alloca,
                        core_type: parameter_types[params.len() + i].clone(),
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
                shadowed_bindings.push((capture.clone(), previous_binding));
            }

            // Codegen lambda body with isolated loop stack so nested lambdas never inherit outer loop targets.
            let codegen_result: Result<(), CodegenError> =
                env.with_loop_isolated(|env| match body {
                    crate::ast::LambdaBody::Expression(expr) => {
                        let result = crate::codegen::expressions::codegen_expression(
                            codegen_context,
                            env,
                            expr,
                            None,
                        )?;
                        codegen_context.builder.build_return(Some(&result))?;
                        Ok(())
                    }
                    crate::ast::LambdaBody::Block(stmts) => {
                        for stmt in stmts {
                            crate::codegen::statements::codegen_statement(
                                codegen_context,
                                env,
                                stmt,
                            )?;
                        }
                        // If no explicit return, emit default
                        if codegen_context.builder.get_insert_block().is_some() {
                            emit_default_return(codegen_context, env, &return_core_types)?;
                        }
                        Ok(())
                    }
                });

            for (binding_name, previous_binding) in shadowed_bindings.into_iter().rev() {
                if let Some(binding) = previous_binding {
                    env.variables.insert(binding_name, binding);
                } else {
                    env.variables.remove(binding_name.as_str());
                }
            }

            codegen_result?;

            Ok(function)
        }
        _ => Err(CodegenError::new(String::from(
            "unsupported call callee expression",
        ))),
    }
}
