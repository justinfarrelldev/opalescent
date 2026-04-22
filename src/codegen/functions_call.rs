extern crate alloc;

use crate::ast::{Expr, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::monomorphization::ensure_monomorphized_function_declaration;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::type_mapping::{AstTypeMappingError, ast_type_to_core_type};
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};

#[path = "functions_call_helpers.rs"]
#[doc = "Helper utilities for call-expression lowering internals."]
mod functions_call_helpers;
use self::functions_call_helpers::{
    current_function, emit_function_default_return, infer_guard_binding_core_type,
};

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
        if struct_value.get_type().count_fields() >= 2 {
            let error_field = codegen_context.builder.build_extract_value(
                struct_value,
                1,
                env.next_name("propagate.err").as_str(),
            )?;
            let current_fn = current_function(codegen_context)?;
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
            emit_function_default_return(codegen_context, current_fn)?;
            codegen_context.builder.position_at_end(continue_block);
            return codegen_context
                .builder
                .build_extract_value(struct_value, 0, env.next_name("propagate.ok").as_str())
                .map_err(CodegenError::from);
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
        if struct_value.get_type().count_fields() >= 1 {
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
            env.variables.insert(
                binding_name.to_owned(),
                VariableBinding {
                    alloca,
                    core_type: binding_core_type,
                    length: None,
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
            let is_stdlib_name =
                crate::codegen::functions_stdlib::STDLIB_NAMES.contains(&name.as_str());
            let base_function = if let Some(imported_runtime_name) =
                env.imported_functions.get(name)
            {
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
            let function_type =
                build_function_type(codegen_context, &metadata_params, &return_core_types);
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
                        is_mutable: false,
                    },
                );
                shadowed_bindings.push((capture.clone(), previous_binding));
            }

            // Codegen lambda body
            let codegen_result: Result<(), CodegenError> = match body {
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
                        crate::codegen::statements::codegen_statement(codegen_context, env, stmt)?;
                    }
                    // If no explicit return, emit default
                    if codegen_context.builder.get_insert_block().is_some() {
                        emit_default_return(codegen_context, env, &return_core_types)?;
                    }
                    Ok(())
                }
            };

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

/// Declare (or return existing) extern LLVM function for an imported symbol signature.
///
/// # Errors
/// Returns `CodegenError` when the imported symbol is not a function type.
fn declare_external_imported_function<'context>(
    codegen_context: &CodegenContext<'context>,
    function_name: &str,
    signature: &CoreType,
) -> Result<FunctionValue<'context>, CodegenError> {
    if let Some(existing) = codegen_context.module.get_function(function_name) {
        return Ok(existing);
    }

    let CoreType::Function {
        ref parameters,
        ref return_types,
        ..
    } = *signature
    else {
        return Err(CodegenError::new(format!(
            "imported symbol '{function_name}' is not callable"
        )));
    };

    let mut lowered_parameter_core_types = Vec::new();
    for core_type in parameters {
        lowered_parameter_core_types.push(core_type.clone());
        if matches!(*core_type, CoreType::Array(_)) {
            lowered_parameter_core_types.push(CoreType::Int64);
        }
    }

    let parameter_types = lowered_parameter_core_types
        .iter()
        .map(|core_type| core_type_to_llvm(codegen_context.context, core_type).into())
        .collect::<Vec<BasicMetadataTypeEnum<'context>>>();
    let function_type = build_function_type(codegen_context, &parameter_types, return_types);

    Ok(codegen_context.module.add_function(
        function_name,
        function_type,
        Some(inkwell::module::Linkage::External),
    ))
}

#[doc = "Build LLVM function type from core parameter and return types."]
pub fn build_function_type<'context>(
    codegen_context: &CodegenContext<'context>,
    parameters: &[BasicMetadataTypeEnum<'context>],
    returns: &[CoreType],
) -> inkwell::types::FunctionType<'context> {
    if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
        return codegen_context
            .context
            .void_type()
            .fn_type(parameters, false);
    }
    if returns.len() == 1 {
        let return_type = core_type_to_llvm(codegen_context.context, &returns[0]);
        return return_type.fn_type(parameters, false);
    }
    let aggregate_fields = returns
        .iter()
        .map(|core_type| core_type_to_llvm(codegen_context.context, core_type))
        .collect::<Vec<_>>();
    let aggregate = codegen_context
        .context
        .struct_type(aggregate_fields.as_slice(), false);
    aggregate.fn_type(parameters, false)
}

#[doc = "Emit a default return for current function based on return shape."]
pub fn emit_default_return(
    codegen_context: &CodegenContext<'_>,
    env: &mut CodegenEnv<'_>,
    returns: &[CoreType],
) -> Result<(), CodegenError> {
    if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }

    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let msg = codegen_context
        .builder
        .build_global_string_ptr("missing return statement", &env.next_name("ret.msg"))?
        .as_pointer_value();
    let _: inkwell::values::CallSiteValue = codegen_context.builder.build_call(
        runtime_fn,
        &[msg.into()],
        &env.next_name("ret.call"),
    )?;
    let _: inkwell::values::InstructionValue = codegen_context.builder.build_unreachable()?;
    Ok(())
}

#[doc = "Map AST types to core types needed for codegen signatures."]
pub fn ast_type_to_core_type_for_signature(ast_type: &Type) -> Result<CoreType, CodegenError> {
    if matches!(*ast_type, Type::Function { .. }) {
        return Err(CodegenError::new(String::from(
            "unsupported function type annotation",
        )));
    }

    ast_type_to_core_type(ast_type).map_err(|error| match error {
        AstTypeMappingError::TypeNotFound { type_name, .. } => {
            CodegenError::new(format!("unsupported type '{type_name}'"))
        }
    })
}

#[doc = "Emit C ABI main wrapper that dispatches to Opalescent entry."]
#[doc = ""]
#[doc = "# Panics"]
#[doc = ""]
#[doc = "Panics if the main function does not have an argv parameter."]
pub fn emit_c_main_wrapper<'context>(
    codegen_context: &CodegenContext<'context>,
    entry_function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    if codegen_context.module.get_function("main").is_some() {
        return Ok(());
    }
    let i32_type = codegen_context.context.i32_type();
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let argv_type = i8_ptr_type.ptr_type(AddressSpace::default());
    let c_main_type = i32_type.fn_type(&[i32_type.into(), argv_type.into()], false);
    let c_main = codegen_context
        .module
        .add_function("main", c_main_type, None);
    let block = codegen_context.context.append_basic_block(c_main, "entry");
    codegen_context.builder.position_at_end(block);

    let _argv = c_main
        .get_nth_param(1)
        .expect("main must have argv param")
        .into_pointer_value();

    let parameter_types = entry_function.get_type().get_param_types();
    let mut args: Vec<BasicMetadataValueEnum<'context>> = Vec::with_capacity(parameter_types.len());
    for parameter_type in parameter_types {
        let argument = match parameter_type {
            BasicMetadataTypeEnum::ArrayType(array_type) => array_type.const_zero().into(),
            BasicMetadataTypeEnum::FloatType(float_type) => float_type.const_zero().into(),
            BasicMetadataTypeEnum::IntType(int_type) => int_type.const_zero().into(),
            BasicMetadataTypeEnum::PointerType(pointer_type) => pointer_type.const_null().into(),
            BasicMetadataTypeEnum::StructType(struct_type) => struct_type.const_zero().into(),
            BasicMetadataTypeEnum::VectorType(vector_type) => vector_type.const_zero().into(),
            BasicMetadataTypeEnum::ScalableVectorType(vector_type) => {
                vector_type.get_undef().into()
            }
            BasicMetadataTypeEnum::MetadataType(_) => {
                return Err(CodegenError::new(String::from(
                    "entry function cannot use metadata parameters",
                )));
            }
        };
        args.push(argument);
    }

    let _call =
        codegen_context
            .builder
            .build_call(entry_function, args.as_slice(), "entry.call")?;
    let _ret = codegen_context.builder.build_return(Some(
        &codegen_context.context.i32_type().const_int(0, false),
    ))?;
    Ok(())
}

#[cfg(test)]
mod functions_call_tests {
    use super::resolve_callee_function;
    use crate::ast::{Expr, NodeId};
    use crate::codegen::context::CodegenContext;
    use crate::codegen::expressions::CodegenEnv;
    use crate::token::{Position, Span};
    use inkwell::context::Context;

    #[doc = "Create a deterministic test span for function-resolution unit tests."]
    fn test_span() -> Span {
        Span::single(Position::new(1, 1, 0))
    }

    #[doc = "Create an identifier expression for callee-resolution tests."]
    fn identifier(name: &str) -> Expr {
        Expr::Identifier {
            name: name.to_owned(),
            span: test_span(),
            id: NodeId(1),
        }
    }

    #[test]
    fn resolve_print_to_puts() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "resolve_print_to_puts");
        let mut env = CodegenEnv::new(true);

        let result =
            resolve_callee_function(&codegen_context, &mut env, &identifier("print"), None);
        assert!(
            result.is_ok(),
            "print should resolve successfully to stdlib puts"
        );

        let Ok(function) = result else {
            return;
        };
        assert_eq!(
            function.get_name().to_str(),
            Ok("puts"),
            "print should resolve to module function named puts"
        );

        let function_type = function.get_type();
        assert!(
            !function_type.is_var_arg(),
            "puts prototype should not be variadic"
        );

        let return_type_text = function_type
            .get_return_type()
            .map_or_else(String::new, |return_type| {
                return_type.print_to_string().to_string()
            });
        assert_eq!(return_type_text, "i32", "puts return type should be i32");

        let parameter_types = function_type.get_param_types();
        assert_eq!(
            parameter_types.len(),
            1,
            "puts should accept exactly one parameter"
        );
        let parameter_type_text = parameter_types[0].print_to_string().to_string();
        assert_eq!(
            parameter_type_text, "i8*",
            "puts first parameter should be i8 pointer"
        );
    }

    #[test]
    fn resolve_printf_to_variadic_printf() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "resolve_printf_to_variadic_printf");
        let mut env = CodegenEnv::new(true);

        let result =
            resolve_callee_function(&codegen_context, &mut env, &identifier("printf"), None);
        assert!(
            result.is_ok(),
            "printf should resolve successfully to libc printf"
        );

        let Ok(function) = result else {
            return;
        };
        assert_eq!(
            function.get_name().to_str(),
            Ok("printf"),
            "printf should resolve to module function named printf"
        );

        let function_type = function.get_type();
        assert!(
            function_type.is_var_arg(),
            "printf prototype should be variadic"
        );

        let return_type_text = function_type
            .get_return_type()
            .map_or_else(String::new, |return_type| {
                return_type.print_to_string().to_string()
            });
        assert_eq!(return_type_text, "i32", "printf return type should be i32");

        let parameter_types = function_type.get_param_types();
        assert_eq!(
            parameter_types.len(),
            1,
            "printf should declare one fixed parameter"
        );
        let parameter_type_text = parameter_types[0].print_to_string().to_string();
        assert_eq!(
            parameter_type_text, "i8*",
            "printf first fixed parameter should be i8 pointer"
        );
    }

    #[test]
    fn unknown_function_produces_error() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "unknown_function_produces_error");
        let mut env = CodegenEnv::new(true);

        let result = resolve_callee_function(
            &codegen_context,
            &mut env,
            &identifier("definitely_not_registered"),
            None,
        );
        assert!(
            result.is_err(),
            "unknown function names should return CodegenError"
        );

        let error_text = result
            .err()
            .map_or_else(String::new, |error| error.to_string());
        assert!(
            error_text.contains("unknown function: definitely_not_registered"),
            "error should include unknown function name, got: {error_text}"
        );
    }

    #[test]
    fn resolve_print_int64_declaration() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "resolve_print_int");
        let mut env = CodegenEnv::new(true);

        let result =
            resolve_callee_function(&codegen_context, &mut env, &identifier("print_int64"), None);
        assert!(result.is_ok(), "print_int64 should resolve successfully");

        let Ok(function) = result else {
            return;
        };
        assert_eq!(
            function.get_name().to_str(),
            Ok("print_int64"),
            "print_int64 should resolve to module function named print_int64"
        );

        let function_type = function.get_type();
        assert!(
            function_type.get_return_type().is_none(),
            "print_int64 should return void in LLVM"
        );

        let parameter_types = function_type.get_param_types();
        assert_eq!(
            parameter_types.len(),
            1,
            "print_int64 should accept exactly one parameter"
        );
        let parameter_type_text = parameter_types[0].print_to_string().to_string();
        assert_eq!(
            parameter_type_text, "i64",
            "print_int64 first parameter should be i64"
        );
    }

    #[test]
    fn opal_runtime_error_is_stdlib_name() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "opal_runtime_error_is_stdlib_name");
        let mut env = CodegenEnv::new(true);

        let result = resolve_callee_function(
            &codegen_context,
            &mut env,
            &identifier("opal_runtime_error"),
            None,
        );
        assert!(
            result.is_ok(),
            "opal_runtime_error should resolve successfully to stdlib function"
        );

        let Ok(function) = result else {
            return;
        };
        assert_eq!(
            function.get_name().to_str(),
            Ok("opal_runtime_error"),
            "opal_runtime_error should resolve to module function named opal_runtime_error"
        );

        let function_type = function.get_type();
        assert!(
            function_type.get_return_type().is_none(),
            "opal_runtime_error should return void in LLVM"
        );

        let parameter_types = function_type.get_param_types();
        assert_eq!(
            parameter_types.len(),
            1,
            "opal_runtime_error should accept exactly one parameter"
        );
        let parameter_type_text = parameter_types[0].print_to_string().to_string();
        assert_eq!(
            parameter_type_text, "i8*",
            "opal_runtime_error first parameter should be i8 pointer (string message)"
        );
    }
}
