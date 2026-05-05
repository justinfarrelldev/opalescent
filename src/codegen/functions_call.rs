#![allow(
    clippy::all,
    clippy::similar_names,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Expr, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{ArrayMetadata, CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::monomorphization::ensure_monomorphized_function_declaration;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};

#[path = "functions_call/array.rs"]
mod array;
#[path = "functions_call_helpers.rs"]
#[doc = "Helper utilities for call-expression lowering internals."]
mod functions_call_helpers;
#[path = "functions_call/tail.rs"]
mod tail;
use self::array::{codegen_append_call, codegen_array_member_call, is_append_intrinsic_name};
use self::functions_call_helpers::{
    caller_returns_error_aggregate, current_function, emit_function_default_return,
    infer_guard_binding_core_type, uses_aggregate_result_dispatch,
};
use self::tail::declare_external_imported_function;

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

#[doc = "Lower a function call expression."]
#[expect(
    clippy::too_many_lines,
    reason = "Function call requires complex argument binding and array length handling"
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
        let imported_name = env.imported_functions.get(name).map(String::as_str);
        if is_append_intrinsic_name(name.as_str())
            || imported_name.is_some_and(is_append_intrinsic_name)
        {
            return codegen_append_call(codegen_context, env, args);
        }
    }
    if let Expr::Member {
        ref object,
        ref member,
        ..
    } = *callee
    {
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
    let mut lowered_args: Vec<BasicMetadataValueEnum<'context>> = Vec::new();
    let mut first_lowered_arg: Option<BasicValueEnum<'context>> = None;
    for (index, arg) in args.iter().enumerate() {
        let lowered = codegen_expression(codegen_context, env, arg, None)?;
        if index == 0 {
            first_lowered_arg = Some(lowered);
        }
        lowered_args.push(lowered.into());

        let maybe_length = match *arg {
            Expr::Identifier { ref name, .. } => env.variables.get(name).and_then(|binding| {
                if !matches!(binding.core_type, CoreType::Array(_)) {
                    return None;
                }

                if let Some(length) = binding.length {
                    return Some(
                        codegen_context
                            .context
                            .i64_type()
                            .const_int(u64::from(length), false)
                            .as_basic_value_enum()
                            .into(),
                    );
                }

                let len_binding_name = format!("{name}_len");
                env.variables
                    .get(len_binding_name.as_str())
                    .and_then(|len_binding| {
                        codegen_context
                            .builder
                            .build_load(len_binding.alloca, len_binding_name.as_str())
                            .ok()
                            .map(Into::into)
                    })
            }),
            Expr::Array { ref elements, .. } => u64::try_from(elements.len()).ok().map(|length| {
                codegen_context
                    .context
                    .i64_type()
                    .const_int(length, false)
                    .as_basic_value_enum()
                    .into()
            }),
            _ => None,
        };

        if let Some(length) = maybe_length {
            lowered_args.push(length);
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
                        let free_fn =
                            codegen_context
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
                    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
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
                    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
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
                    let result_alloca = codegen_context
                        .builder
                        .build_alloca(result_struct_type, env.next_name("call.result").as_str())?;
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

    Ok(call_result)
}

#[doc = "Lower propagate expression control flow."]
pub fn codegen_propagate_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    call_expr: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    env.set_pending_array_metadata(None);
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
            None,
        )?
    } else {
        codegen_expression(codegen_context, env, call_expr, None)?
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
            emit_function_default_return(codegen_context, current_fn, forward_error)?;
            codegen_context.builder.position_at_end(continue_block);
            let success_value = codegen_context
                .builder
                .build_extract_value(struct_value, 0, env.next_name("propagate.ok").as_str())
                .map_err(CodegenError::from)?;
            if field_count >= 3 {
                let length_value = codegen_context.builder.build_extract_value(
                    struct_value,
                    1,
                    env.next_name("propagate.len").as_str(),
                )?;
                let capacity_value = if field_count >= 4 {
                    codegen_context.builder.build_extract_value(
                        struct_value,
                        2,
                        env.next_name("propagate.cap").as_str(),
                    )?
                } else {
                    length_value
                };
                env.set_pending_array_metadata(Some(ArrayMetadata {
                    length: length_value.into_int_value(),
                    capacity: capacity_value.into_int_value(),
                }));
            }
            return Ok(success_value);
        }
    }
    env.set_pending_array_metadata(None);
    Ok(value)
}

#[doc = "Lower guard expression binding logic."]
pub fn codegen_guard_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    guarded_expr: &Expr,
    binding_name: &str,
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
            None,
        )?
    } else {
        codegen_expression(codegen_context, env, guarded_expr, None)?
    };
    if value.is_struct_value() {
        let struct_value = value.into_struct_value();
        let field_count = struct_value.get_type().count_fields();
        if field_count >= 1 {
            let success_value = codegen_context.builder.build_extract_value(
                struct_value,
                0,
                env.next_name("guard.ok").as_str(),
            )?;
            let alloca = codegen_context.builder.build_alloca(
                success_value.get_type(),
                env.next_name("guard.bind").as_str(),
            )?;
            let _store = codegen_context.builder.build_store(alloca, success_value)?;
            let binding_core_type =
                infer_guard_binding_core_type(env, guarded_expr, success_value.get_type());
            if matches!(binding_core_type, CoreType::Array(_)) && field_count >= 3 {
                let length_value = codegen_context.builder.build_extract_value(
                    struct_value,
                    1,
                    env.next_name("guard.bind.len").as_str(),
                )?;
                let len_binding_name = format!("{binding_name}_len");
                let len_alloca = codegen_context
                    .builder
                    .build_alloca(length_value.get_type(), len_binding_name.as_str())?;
                let _store_len = codegen_context
                    .builder
                    .build_store(len_alloca, length_value)?;
                env.variables.insert(
                    len_binding_name,
                    VariableBinding {
                        alloca: len_alloca,
                        core_type: CoreType::Int64,
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );

                let capacity_value = if field_count >= 4 {
                    codegen_context.builder.build_extract_value(
                        struct_value,
                        2,
                        env.next_name("guard.bind.cap").as_str(),
                    )?
                } else {
                    length_value
                };
                let cap_binding_name = format!("{binding_name}_cap");
                let cap_alloca = codegen_context
                    .builder
                    .build_alloca(capacity_value.get_type(), cap_binding_name.as_str())?;
                let _store_cap = codegen_context
                    .builder
                    .build_store(cap_alloca, capacity_value)?;
                env.variables.insert(
                    cap_binding_name,
                    VariableBinding {
                        alloca: cap_alloca,
                        core_type: CoreType::Int64,
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
            }
            env.variables.insert(
                binding_name.to_owned(),
                VariableBinding {
                    alloca,
                    core_type: binding_core_type,
                    length: None,
                    capacity: None,
                    is_mutable: false,
                },
            );
            return Ok(success_value);
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
            let is_stdlib_name = crate::codegen::functions_stdlib::STDLIB_NAMES
                .contains(&name.as_str())
                || is_append_intrinsic_name(name.as_str());
            let base_function = if let Some(imported_runtime_name) =
                env.imported_functions.get(name)
            {
                if is_append_intrinsic_name(imported_runtime_name.as_str()) {
                    return Err(CodegenError::new(String::from(
                        "append is compiler-lowered and does not resolve to a standalone runtime symbol",
                    )));
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
                .flat_map(|core_type| {
                    let mut lowered = Vec::with_capacity(2);
                    match core_type {
                        CoreType::Array(element_type) => {
                            lowered.push(
                                core_type_to_llvm(codegen_context.context, element_type)
                                    .ptr_type(AddressSpace::default())
                                    .into(),
                            );
                            lowered.push(codegen_context.context.i64_type().into());
                        }
                        _ => lowered
                            .push(core_type_to_llvm(codegen_context.context, core_type).into()),
                    }
                    lowered
                })
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
            let mut lowered_index = 0_usize;
            for (i, param) in params.iter().enumerate() {
                let param_value = args[lowered_index];
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
                if matches!(parameter_types[i], CoreType::Array(_)) {
                    let len_param_value = args[lowered_index + 1];
                    let len_binding_name = format!("{}_len", param.name);
                    let len_alloca = codegen_context
                        .builder
                        .build_alloca(len_param_value.get_type(), len_binding_name.as_str())?;
                    codegen_context
                        .builder
                        .build_store(len_alloca, len_param_value)?;
                    let previous_length_binding = env.variables.insert(
                        len_binding_name.clone(),
                        VariableBinding {
                            alloca: len_alloca,
                            core_type: CoreType::Int64,
                            length: None,
                            capacity: None,
                            is_mutable: false,
                        },
                    );
                    shadowed_bindings.push((len_binding_name, previous_length_binding));
                    lowered_index += 1;
                }
                lowered_index += 1;
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
