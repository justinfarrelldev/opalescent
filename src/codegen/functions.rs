extern crate alloc;

use crate::ast::{Decl, Expr, ImportItem, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{codegen_expression, CodegenEnv, CodegenError, VariableBinding};
use crate::codegen::monomorphization::ensure_monomorphized_function_declaration;
use crate::codegen::statements::codegen_statement;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};
use inkwell::AddressSpace;

#[doc = "Lower a function declaration and optionally emit a C main wrapper."]
pub fn codegen_function_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    declaration: &Decl,
) -> Result<FunctionValue<'context>, CodegenError> {
    let &Decl::Function {
        ref name,
        ref parameters,
        ref return_types,
        ref body,
        is_entry,
        ..
    } = declaration
    else {
        return Err(CodegenError::new(String::from(
            "expected function declaration",
        )));
    };

    let parameter_core_types = parameters
        .iter()
        .map(|parameter| ast_type_to_core_type(&parameter.param_type))
        .collect::<Result<Vec<_>, _>>()?;
    let returns = return_types.as_ref().map_or_else(
        || Ok(vec![CoreType::Unit]),
        |types| {
            types
                .iter()
                .map(ast_type_to_core_type)
                .collect::<Result<Vec<_>, _>>()
        },
    )?;
    let function_name = if is_entry {
        format!("__opalescent_entry_{name}")
    } else {
        name.clone()
    };

    let parameter_types = parameter_core_types
        .iter()
        .map(|core_type| core_type_to_llvm(codegen_context.context, core_type).into())
        .collect::<Vec<BasicMetadataTypeEnum<'context>>>();
    let function_type = build_function_type(codegen_context, &parameter_types, &returns);
    let function = codegen_context
        .module
        .add_function(function_name.as_str(), function_type, None);
    let entry = codegen_context
        .context
        .append_basic_block(function, "entry");
    codegen_context.builder.position_at_end(entry);

    for (index, parameter) in parameters.iter().enumerate() {
        let Some(param_value) =
            function
                .get_nth_param(u32::try_from(index).map_err(|conversion_error| {
                    CodegenError::new(format!("{conversion_error}"))
                })?)
        else {
            return Err(CodegenError::new(String::from(
                "missing function parameter",
            )));
        };
        let alloca = codegen_context
            .builder
            .build_alloca(param_value.get_type(), parameter.name.as_str())?;
        let _store = codegen_context.builder.build_store(alloca, param_value)?;
        env.variables.insert(
            parameter.name.clone(),
            VariableBinding {
                alloca,
                core_type: parameter_core_types[index].clone(),
            },
        );
    }

    codegen_statement(codegen_context, env, body)?;
    if let Some(block) = codegen_context.builder.get_insert_block() {
        if block.get_terminator().is_none() {
            emit_default_return(codegen_context, &returns)?;
        }
    }

    if is_entry {
        emit_c_main_wrapper(codegen_context, function)?;
    }

    Ok(function)
}

#[doc = "Lower import declarations by declaring known stdlib externs and alias mappings."]
pub fn codegen_import_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    declaration: &Decl,
) -> Result<(), CodegenError> {
    let &Decl::Import {
        ref items,
        ref source,
        ..
    } = declaration
    else {
        return Err(CodegenError::new(String::from(
            "expected import declaration",
        )));
    };

    for item in items {
        match *item {
            ImportItem::Named {
                ref name,
                ref alias,
                ..
            } => {
                let runtime_name = resolve_imported_runtime_name(source.as_str(), name.as_str())?;
                let stdlib_function =
                    declare_stdlib_function(codegen_context, runtime_name.as_str()).ok_or_else(
                        || {
                            CodegenError::new(format!(
                                "unsupported stdlib import '{name}' from module '{source}'"
                            ))
                        },
                    )?;
                let local_name = alias.as_ref().unwrap_or(name).clone();
                env.imported_functions.insert(
                    local_name,
                    stdlib_function
                        .get_name()
                        .to_str()
                        .map_or_else(|_| runtime_name.clone(), alloc::borrow::ToOwned::to_owned),
                );
            }
            ImportItem::Type { .. } => {}
            ImportItem::Glob { .. } => {
                return Err(CodegenError::new(format!(
                    "glob imports are not supported in codegen for module '{source}'"
                )));
            }
        }
    }

    Ok(())
}

#[doc = "Lower a function call expression."]
pub fn codegen_call_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    generic_args: Option<&[Type]>,
    args: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let function = resolve_callee_function(codegen_context, env, callee, generic_args)?;
    let mut lowered_args = args
        .iter()
        .map(|arg| codegen_expression(codegen_context, env, arg, None).map(Into::into))
        .collect::<Result<Vec<BasicMetadataValueEnum<'context>>, CodegenError>>()?;

    if let &Expr::Lambda {
        ref captured_variables,
        ..
    } = callee
    {
        for capture in captured_variables {
            if let Some(binding) = env.variables.get(capture) {
                let loaded = codegen_context
                    .builder
                    .build_load(binding.alloca, capture.as_str())?;
                lowered_args.push(loaded.into());
            } else {
                lowered_args.push(
                    codegen_context
                        .context
                        .i64_type()
                        .const_zero()
                        .as_basic_value_enum()
                        .into(),
                );
            }
        }
    }

    let call = codegen_context.builder.build_call(
        function,
        lowered_args.as_slice(),
        env.next_name("call").as_str(),
    )?;
    call.try_as_basic_value().basic().map_or_else(
        || {
            Ok(codegen_context
                .context
                .struct_type(&[], false)
                .const_zero()
                .as_basic_value_enum())
        },
        Ok,
    )
}

#[doc = "Lower propagate expression control flow."]
pub fn codegen_propagate_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    call_expr: &Expr,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let value = if let &Expr::Call {
        ref callee,
        ref args,
        ..
    } = call_expr
    {
        codegen_call_expression(codegen_context, env, callee.as_ref(), None, args.as_slice())?
    } else {
        codegen_expression(codegen_context, env, call_expr, None)?
    };
    if value.is_struct_value() {
        let struct_value = value.into_struct_value();
        if struct_value.get_type().count_fields() >= 2 {
            let flag = codegen_context
                .builder
                .build_extract_value(
                    struct_value,
                    1,
                    env.next_name("propagate.err.flag").as_str(),
                )?
                .into_int_value();
            let current_fn = current_function(codegen_context)?;
            let early_return = codegen_context
                .context
                .append_basic_block(current_fn, env.next_name("propagate.ret").as_str());
            let continue_block = codegen_context
                .context
                .append_basic_block(current_fn, env.next_name("propagate.cont").as_str());
            let _branch = codegen_context.builder.build_conditional_branch(
                flag,
                early_return,
                continue_block,
            )?;
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
    let value = if let &Expr::Call {
        ref callee,
        ref args,
        ..
    } = guarded_expr
    {
        codegen_call_expression(codegen_context, env, callee.as_ref(), None, args.as_slice())?
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
            env.variables.insert(
                binding_name.to_owned(),
                VariableBinding {
                    alloca,
                    core_type: llvm_basic_type_to_core_type(success_value.get_type()),
                },
            );
            return Ok(success_value);
        }
    }
    Ok(value)
}

#[doc = "Resolve the called function value for identifier or lambda callees."]
fn resolve_callee_function<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    generic_args: Option<&[Type]>,
) -> Result<FunctionValue<'context>, CodegenError> {
    match *callee {
        Expr::Identifier { ref name, .. } => {
            let is_stdlib_name = matches!(
                name.as_str(),
                "print"
                    | "printf"
                    | "take_input"
                    | "opal_take_input"
                    | "random_int32"
                    | "opal_random_int32"
                    | "string_to_int32"
                    | "opal_string_to_int32"
                    | "print_int"
                    | "opal_print_int"
            );
            let base_function = if let Some(imported_runtime_name) =
                env.imported_functions.get(name)
            {
                codegen_context
                    .module
                    .get_function(imported_runtime_name.as_str())
                    .or_else(|| {
                        declare_stdlib_function(codegen_context, imported_runtime_name.as_str())
                    })
                    .ok_or_else(|| {
                        CodegenError::new(format!(
                            "missing runtime function for imported symbol '{name}'"
                        ))
                    })?
            } else if let Some(existing) = codegen_context.module.get_function(name.as_str()) {
                existing
            } else if let Some(stdlib_function) = declare_stdlib_function(codegen_context, name) {
                stdlib_function
            } else {
                return Err(CodegenError::new(format!("unknown function: {name}")));
            };
            if let Some(explicit_generic_args) = generic_args {
                let concrete_types = explicit_generic_args
                    .iter()
                    .map(ast_type_to_core_type)
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
            ..
        } => {
            let mut parameter_types = params
                .iter()
                .map(|param| ast_type_to_core_type(&param.param_type))
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
                .map(ast_type_to_core_type)
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
            emit_default_return(codegen_context, &return_core_types)?;
            Ok(function)
        }
        _ => Err(CodegenError::new(String::from(
            "unsupported call callee expression",
        ))),
    }
}

#[doc = "Declare known stdlib functions with precise LLVM prototypes."]
fn declare_stdlib_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i32_type = codegen_context.context.i32_type();
    let i64_type = codegen_context.context.i64_type();
    match name {
        "print" => codegen_context.module.get_function("puts").or_else(|| {
            let puts_type = i32_type.fn_type(&[i8_ptr_type.into()], false);
            Some(codegen_context.module.add_function("puts", puts_type, None))
        }),
        "printf" => codegen_context.module.get_function("printf").or_else(|| {
            let printf_type = i32_type.fn_type(&[i8_ptr_type.into()], true);
            Some(
                codegen_context
                    .module
                    .add_function("printf", printf_type, None),
            )
        }),
        "opal_take_input" | "take_input" => codegen_context
            .module
            .get_function("opal_take_input")
            .or_else(|| {
                let function_type = i8_ptr_type.fn_type(&[], false);
                Some(
                    codegen_context
                        .module
                        .add_function("opal_take_input", function_type, None),
                )
            }),
        "opal_string_to_int32" | "string_to_int32" => codegen_context
            .module
            .get_function("opal_string_to_int32")
            .or_else(|| {
                let function_type = i64_type.fn_type(&[i8_ptr_type.into()], false);
                Some(codegen_context.module.add_function(
                    "opal_string_to_int32",
                    function_type,
                    None,
                ))
            }),
        "opal_random_int32" | "random_int32" => codegen_context
            .module
            .get_function("opal_random_int32")
            .or_else(|| {
                let function_type = i64_type.fn_type(&[i64_type.into(), i64_type.into()], false);
                Some(
                    codegen_context
                        .module
                        .add_function("opal_random_int32", function_type, None),
                )
            }),
        "opal_print_int" | "print_int" => codegen_context
            .module
            .get_function("opal_print_int")
            .or_else(|| {
                let function_type = codegen_context
                    .context
                    .void_type()
                    .fn_type(&[i64_type.into()], false);
                Some(
                    codegen_context
                        .module
                        .add_function("opal_print_int", function_type, None),
                )
            }),
        _ => None,
    }
}

#[doc = "Resolve imported stdlib symbol to concrete runtime function name."]
fn resolve_imported_runtime_name(
    module_name: &str,
    symbol_name: &str,
) -> Result<String, CodegenError> {
    match (module_name, symbol_name) {
        ("standard", "take_input") => Ok(String::from("opal_take_input")),
        ("standard", "string_to_int32") => Ok(String::from("opal_string_to_int32")),
        ("standard", "print_int") => Ok(String::from("opal_print_int")),
        ("math", "random_int32") => Ok(String::from("opal_random_int32")),
        ("standard", "print") => Ok(String::from("print")),
        _ => Err(CodegenError::new(format!(
            "unknown import symbol '{symbol_name}' in module '{module_name}'"
        ))),
    }
}

#[doc = "Build LLVM function type from core parameter and return types."]
fn build_function_type<'context>(
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
fn emit_default_return(
    codegen_context: &CodegenContext<'_>,
    returns: &[CoreType],
) -> Result<(), CodegenError> {
    if returns.is_empty() || (returns.len() == 1 && matches!(returns[0], CoreType::Unit)) {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }
    if returns.len() == 1 {
        let default_value = core_type_to_llvm(codegen_context.context, &returns[0]).const_zero();
        let _ret = codegen_context.builder.build_return(Some(&default_value))?;
        return Ok(());
    }
    let fields = returns
        .iter()
        .map(|core_type| core_type_to_llvm(codegen_context.context, core_type))
        .collect::<Vec<_>>();
    let aggregate_type = codegen_context
        .context
        .struct_type(fields.as_slice(), false);
    let mut aggregate = aggregate_type.get_undef();
    for (index, field_type) in fields.iter().enumerate() {
        aggregate = codegen_context
            .builder
            .build_insert_value(
                aggregate,
                field_type.const_zero(),
                u32::try_from(index)
                    .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?,
                "ret.agg.insert",
            )?
            .into_struct_value();
    }
    let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
    Ok(())
}

#[doc = "Emit early-return default for propagate error path."]
fn emit_function_default_return<'context>(
    codegen_context: &CodegenContext<'context>,
    function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    let return_type = function.get_type().get_return_type();
    if return_type.is_none() {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }
    let Some(return_basic_type) = return_type else {
        return Err(CodegenError::new(String::from(
            "invalid function return type",
        )));
    };
    let zero = return_basic_type.const_zero();
    let _ret = codegen_context.builder.build_return(Some(&zero))?;
    Ok(())
}

#[doc = "Emit C ABI main wrapper that dispatches to Opalescent entry."]
fn emit_c_main_wrapper<'context>(
    codegen_context: &CodegenContext<'context>,
    entry_function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    if codegen_context.module.get_function("main").is_some() {
        return Ok(());
    }
    let c_main_type = codegen_context.context.i32_type().fn_type(&[], false);
    let c_main = codegen_context
        .module
        .add_function("main", c_main_type, None);
    let block = codegen_context.context.append_basic_block(c_main, "entry");
    codegen_context.builder.position_at_end(block);
    let args: [BasicMetadataValueEnum<'context>; 0] = [];
    let _call = codegen_context
        .builder
        .build_call(entry_function, &args, "entry.call")?;
    let _ret = codegen_context.builder.build_return(Some(
        &codegen_context.context.i32_type().const_int(0, false),
    ))?;
    Ok(())
}

#[doc = "Map AST types to core types needed for codegen signatures."]
fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, CodegenError> {
    match *ast_type {
        Type::Basic { ref name, .. } => match name.as_str() {
            "int8" => Ok(CoreType::Int8),
            "int16" => Ok(CoreType::Int16),
            "int32" => Ok(CoreType::Int32),
            "int64" => Ok(CoreType::Int64),
            "uint8" => Ok(CoreType::UInt8),
            "uint16" => Ok(CoreType::UInt16),
            "uint32" => Ok(CoreType::UInt32),
            "uint64" => Ok(CoreType::UInt64),
            "float32" => Ok(CoreType::Float32),
            "float64" => Ok(CoreType::Float64),
            "string" => Ok(CoreType::String),
            "boolean" => Ok(CoreType::Boolean),
            "void" | "unit" => Ok(CoreType::Unit),
            _ => Err(CodegenError::new(format!("unsupported type '{name}'"))),
        },
        Type::Array {
            ref element_type, ..
        } => Ok(CoreType::Array(alloc::boxed::Box::new(
            ast_type_to_core_type(element_type.as_ref())?,
        ))),
        Type::Generic {
            ref name,
            ref type_args,
            ..
        } => {
            let resolved_args = type_args
                .iter()
                .map(ast_type_to_core_type)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(CoreType::Generic {
                name: name.clone(),
                type_args: resolved_args,
            })
        }
        Type::Function { .. } => Err(CodegenError::new(String::from(
            "unsupported function type annotation",
        ))),
    }
}

#[doc = "Fetch current LLVM function from builder insertion block."]
fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let Some(block) = codegen_context.builder.get_insert_block() else {
        return Err(CodegenError::new(String::from(
            "builder is not positioned in a block",
        )));
    };
    block
        .get_parent()
        .ok_or_else(|| CodegenError::new(String::from("insert block does not have parent")))
}

#[doc = "Approximate core type mapping from LLVM basic value type."]
fn llvm_basic_type_to_core_type(llvm_type: inkwell::types::BasicTypeEnum<'_>) -> CoreType {
    if llvm_type.is_int_type() {
        let int_type = llvm_type.into_int_type();
        return match int_type.get_bit_width() {
            1 => CoreType::Boolean,
            8 => CoreType::Int8,
            16 => CoreType::Int16,
            32 => CoreType::Int32,
            _ => CoreType::Int64,
        };
    }
    if llvm_type.is_float_type() {
        return CoreType::Float64;
    }
    if llvm_type.is_pointer_type() {
        return CoreType::String;
    }
    if llvm_type.is_array_type() {
        return CoreType::Array(alloc::boxed::Box::new(CoreType::Int64));
    }
    CoreType::Unit
}

#[cfg(test)]
mod tests {
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
    fn resolve_print_int_to_opal_print_int_declaration() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "resolve_print_int");
        let mut env = CodegenEnv::new(true);

        let result =
            resolve_callee_function(&codegen_context, &mut env, &identifier("print_int"), None);
        assert!(
            result.is_ok(),
            "print_int should resolve successfully to opal_print_int"
        );

        let Ok(function) = result else {
            return;
        };
        assert_eq!(
            function.get_name().to_str(),
            Ok("opal_print_int"),
            "print_int should resolve to module function named opal_print_int"
        );

        let function_type = function.get_type();
        assert!(
            function_type.get_return_type().is_none(),
            "opal_print_int should return void in LLVM"
        );

        let parameter_types = function_type.get_param_types();
        assert_eq!(
            parameter_types.len(),
            1,
            "opal_print_int should accept exactly one parameter"
        );
        let parameter_type_text = parameter_types[0].print_to_string().to_string();
        assert_eq!(
            parameter_type_text, "i64",
            "opal_print_int first parameter should be i64"
        );
    }
}
