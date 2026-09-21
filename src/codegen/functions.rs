#![allow(
    clippy::all,
    clippy::pattern_type_mismatch,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Decl, Expr, ImportItem, Visibility};
use crate::codegen::binding_store::{binding_requires_rc_cleanup, initialize_binding_value};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, ValueAccessorBinding, VariableBinding};
use crate::codegen::expressions_array::materialize_runtime_array_from_raw_elements;
use crate::codegen::rc_emitter::RcEmitter;
use crate::codegen::statements::{codegen_statement, unwind_scope_without_cleanup};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::DLLStorageClass;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::{BasicValue, FunctionValue};

use crate::codegen::functions_call::declare_external_imported_function;
pub use crate::codegen::functions_call::{
    ast_type_to_core_type_for_signature, build_function_type, codegen_call_expression,
    codegen_guard_expression, codegen_propagate_expression, emit_c_main_wrapper,
    emit_default_return,
};

#[must_use]
pub fn top_level_value_accessor_name(name: &str) -> String {
    format!("__opalescent_value_{name}")
}

#[doc = "Lower a function declaration and optionally emit a C main wrapper."]
#[expect(
    clippy::too_many_lines,
    reason = "Function declaration requires complex parameter binding"
)]
pub fn codegen_function_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    declaration: &Decl,
) -> Result<FunctionValue<'context>, CodegenError> {
    let &Decl::Function {
        ref name,
        ref parameters,
        ref return_types,
        ref error_types,
        ref body,
        is_entry,
        ref visibility,
        ..
    } = declaration
    else {
        return Err(CodegenError::new(String::from(
            "expected function declaration",
        )));
    };

    let parameter_core_types = parameters
        .iter()
        .map(|parameter| ast_type_to_core_type_for_signature(&parameter.param_type))
        .collect::<Result<Vec<_>, _>>()?;
    let returns = return_types.as_ref().map_or_else(
        || Ok(vec![CoreType::Unit]),
        |types| {
            types
                .iter()
                .map(ast_type_to_core_type_for_signature)
                .collect::<Result<Vec<_>, _>>()
        },
    )?;
    let error_core_types = error_types
        .iter()
        .map(|error_type| CoreType::Generic {
            name: error_type.clone(),
            type_args: Vec::new(),
        })
        .collect::<Vec<_>>();
    env.imported_signatures.insert(
        name.clone(),
        CoreType::Function {
            parameters: parameter_core_types.clone(),
            return_types: returns.clone(),
            error_types: error_core_types.clone(),
            generic_params: Vec::new(),
        },
    );
    let function_returns_owned_string =
        !is_entry && returns.len() == 1 && returns.first() == Some(&CoreType::String);
    let function_name = if is_entry {
        format!("__opalescent_entry_{name}")
    } else {
        name.clone()
    };

    let mut parameter_types = Vec::new();
    for core_type in &parameter_core_types {
        match core_type {
            CoreType::Array(element_type) if !is_entry => {
                parameter_types.push(
                    core_type_to_llvm(codegen_context.context, element_type.as_ref())
                        .ptr_type(AddressSpace::default())
                        .into(),
                );
                parameter_types.push(codegen_context.context.i64_type().into());
            }
            _ => parameter_types.push(core_type_to_llvm(codegen_context.context, core_type).into()),
        }
    }
    let function_type = build_function_type(
        codegen_context,
        &parameter_types,
        &returns,
        &error_core_types,
    )?;
    let function_linkage = if is_entry || matches!(*visibility, Visibility::Public) {
        Some(Linkage::External)
    } else {
        Some(Linkage::Internal)
    };
    let function = codegen_context.module.add_function(
        function_name.as_str(),
        function_type,
        function_linkage,
    );
    if function_returns_owned_string {
        env.owned_string_functions.insert(name.clone(), true);
    }

    if (is_entry || matches!(*visibility, Visibility::Public))
        && codegen_context.target.platform == crate::build_system::targets::Platform::Windows
    {
        function
            .as_global_value()
            .set_dll_storage_class(DLLStorageClass::Export);
    }

    let entry = codegen_context
        .context
        .append_basic_block(function, "entry");
    codegen_context.builder.position_at_end(entry);

    let _function_scope_depth = env.enter_scope();
    let mut llvm_param_index = 0_usize;
    for (index, parameter) in parameters.iter().enumerate() {
        match &parameter_core_types[index] {
            CoreType::Array(element_type) if !is_entry => {
                let Some(data_param) =
                    function.get_nth_param(u32::try_from(llvm_param_index).map_err(
                        |conversion_error| CodegenError::new(format!("{conversion_error}")),
                    )?)
                else {
                    return Err(CodegenError::new(String::from(
                        "missing array data parameter",
                    )));
                };
                let Some(length_param) = function.get_nth_param(
                    u32::try_from(llvm_param_index.saturating_add(1)).map_err(
                        |conversion_error| CodegenError::new(format!("{conversion_error}")),
                    )?,
                ) else {
                    return Err(CodegenError::new(String::from(
                        "missing array length parameter",
                    )));
                };
                let runtime_array = materialize_runtime_array_from_raw_elements(
                    codegen_context,
                    env,
                    data_param.into_pointer_value(),
                    length_param.into_int_value(),
                    element_type.as_ref(),
                    "function.param.array",
                )?;
                let alloca = codegen_context
                    .builder
                    .build_alloca(runtime_array.get_type(), parameter.name.as_str())?;
                env.variables.insert(
                    parameter.name.clone(),
                    VariableBinding {
                        alloca,
                        core_type: parameter_core_types[index].clone(),
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
                env.register_scope_binding(parameter.name.as_str());
                initialize_binding_value(
                    codegen_context,
                    env,
                    parameter.name.as_str(),
                    runtime_array.as_basic_value_enum(),
                    "function.param.init",
                    false,
                )?;
                llvm_param_index = llvm_param_index.saturating_add(2);
            }
            _ => {
                let Some(param_value) =
                    function.get_nth_param(u32::try_from(llvm_param_index).map_err(
                        |conversion_error| CodegenError::new(format!("{conversion_error}")),
                    )?)
                else {
                    return Err(CodegenError::new(String::from(
                        "missing function parameter",
                    )));
                };
                let alloca = codegen_context
                    .builder
                    .build_alloca(param_value.get_type(), parameter.name.as_str())?;
                env.variables.insert(
                    parameter.name.clone(),
                    VariableBinding {
                        alloca,
                        core_type: parameter_core_types[index].clone(),
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
                env.register_scope_binding(parameter.name.as_str());
                initialize_binding_value(
                    codegen_context,
                    env,
                    parameter.name.as_str(),
                    param_value,
                    "function.param.init",
                    false,
                )?;
                llvm_param_index = llvm_param_index.saturating_add(1);
            }
        }
    }

    codegen_statement(codegen_context, env, body)?;
    if let Some(block) = codegen_context.builder.get_insert_block() {
        if block.get_terminator().is_none() {
            emit_default_return(codegen_context, env, &returns)?;
        }
    }

    if is_entry {
        emit_c_main_wrapper(codegen_context, function)?;
    }

    unwind_scope_without_cleanup(env);
    Ok(function)
}

#[expect(
    clippy::too_many_lines,
    reason = "top-level declarations handle cache initialization and RC bookkeeping together"
)]
pub fn codegen_top_level_value_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
    initializer: &Expr,
    visibility: &Visibility,
    core_type: &CoreType,
) -> Result<(), CodegenError> {
    let accessor_name = top_level_value_accessor_name(binding_name);
    let value_type = core_type_to_llvm(codegen_context.context, core_type);
    let linkage = if matches!(*visibility, Visibility::Public) {
        Some(Linkage::External)
    } else {
        Some(Linkage::Internal)
    };
    let accessor_function = codegen_context
        .module
        .get_function(accessor_name.as_str())
        .unwrap_or_else(|| {
            codegen_context.module.add_function(
                accessor_name.as_str(),
                value_type.fn_type(&[], false),
                linkage,
            )
        });
    if matches!(*visibility, Visibility::Public)
        && codegen_context.target.platform == crate::build_system::targets::Platform::Windows
    {
        accessor_function
            .as_global_value()
            .set_dll_storage_class(DLLStorageClass::Export);
    }

    let cache_name = format!("{accessor_name}.cache");
    let cache_global = codegen_context
        .module
        .get_global(cache_name.as_str())
        .unwrap_or_else(|| {
            codegen_context
                .module
                .add_global(value_type, None, cache_name.as_str())
        });
    cache_global.set_linkage(Linkage::Internal);
    cache_global.set_initializer(&value_type.const_zero());

    let init_name = format!("{accessor_name}.initialized");
    let init_global = codegen_context
        .module
        .get_global(init_name.as_str())
        .unwrap_or_else(|| {
            codegen_context.module.add_global(
                codegen_context.context.bool_type(),
                None,
                init_name.as_str(),
            )
        });
    init_global.set_linkage(Linkage::Internal);
    init_global.set_initializer(&codegen_context.context.bool_type().const_zero());

    let entry_block = codegen_context
        .context
        .append_basic_block(accessor_function, "entry");
    let cached_block = codegen_context
        .context
        .append_basic_block(accessor_function, "cached");
    let init_block = codegen_context
        .context
        .append_basic_block(accessor_function, "init");
    codegen_context.builder.position_at_end(entry_block);
    let init_flag = codegen_context
        .builder
        .build_load(init_global.as_pointer_value(), "value.init.flag")?
        .into_int_value();
    let is_initialized = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        init_flag,
        codegen_context.context.bool_type().const_zero(),
        "value.init.ready",
    )?;
    codegen_context
        .builder
        .build_conditional_branch(is_initialized, cached_block, init_block)?;

    codegen_context.builder.position_at_end(cached_block);
    let cached_value = codegen_context
        .builder
        .build_load(cache_global.as_pointer_value(), "value.cached.load")?;
    if binding_requires_rc_cleanup(core_type) {
        let pointer_value = cached_value.into_pointer_value();
        let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
        emitter.emit_inc(pointer_value)?;
    }
    codegen_context.builder.build_return(Some(&cached_value))?;

    codegen_context.builder.position_at_end(init_block);
    let initialized_value = crate::codegen::expressions::codegen_expression(
        codegen_context,
        env,
        initializer,
        Some(core_type),
    )?;
    if binding_requires_rc_cleanup(core_type) {
        let pointer_value = initialized_value.into_pointer_value();
        let emitter = RcEmitter::new(&codegen_context.builder, &codegen_context.module);
        emitter.emit_inc(pointer_value)?;
    }
    codegen_context
        .builder
        .build_store(cache_global.as_pointer_value(), initialized_value)?;
    codegen_context.builder.build_store(
        init_global.as_pointer_value(),
        codegen_context.context.bool_type().const_int(1, false),
    )?;
    codegen_context
        .builder
        .build_return(Some(&initialized_value))?;

    env.value_accessors.insert(
        binding_name.to_owned(),
        ValueAccessorBinding {
            accessor_name,
            core_type: core_type.clone(),
        },
    );
    Ok(())
}

#[doc = "Lower import declarations by declaring known stdlib externs and alias mappings."]
#[expect(
    clippy::too_many_lines,
    reason = "import declaration lowering handles local, standard, and terminal proposal imports"
)]
pub fn codegen_import_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    declaration: &Decl,
) -> Result<(), CodegenError> {
    let Decl::Import {
        ref items,
        ref source,
        ..
    } = *declaration
    else {
        return Err(CodegenError::new(String::from(
            "expected import declaration",
        )));
    };

    // Local imports (./path or ../path): generate extern declarations from imported_signatures.
    if source.starts_with("./") || source.starts_with("../") {
        return codegen_local_import_declaration(codegen_context, env, items, source);
    }

    for item in items {
        match *item {
            ImportItem::Named {
                ref name,
                ref alias,
                ..
            } => {
                let local_name = alias.as_ref().unwrap_or(name).clone();
                if source == "standard"
                    && matches!(
                        name.as_str(),
                        "append" | "array_filled" | "reserve" | "clear"
                    )
                {
                    env.imported_functions.insert(local_name, name.clone());
                    continue;
                }
                if source == "standard.terminal"
                    && !crate::type_system::is_terminal_proposal_codegen_gated_import(
                        source.as_str(),
                        name.as_str(),
                    )
                {
                    let Some(signature) = env
                        .imported_signatures
                        .get(name.as_str())
                        .cloned()
                        .or_else(|| {
                            crate::type_system::terminal_proposal_function_signature(
                                source.as_str(),
                                name.as_str(),
                            )
                        })
                    else {
                        return Err(CodegenError::new(format!(
                            "missing imported signature for terminal proposal symbol '{name}'"
                        )));
                    };
                    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
                        codegen_context,
                        name.as_str(),
                    )
                    .map_or_else(
                        || {
                            declare_external_imported_function(
                                codegen_context,
                                name.as_str(),
                                &signature,
                            )
                        },
                        Ok,
                    )?;
                    env.imported_functions.insert(
                        local_name,
                        runtime_fn
                            .get_name()
                            .to_str()
                            .map_or_else(|_| name.clone(), alloc::borrow::ToOwned::to_owned),
                    );
                    continue;
                }
                let runtime_name = crate::codegen::functions_stdlib::resolve_imported_runtime_name(
                    source.as_str(),
                    name.as_str(),
                )?;
                let stdlib_function = crate::codegen::functions_stdlib::declare_stdlib_function(
                    codegen_context,
                    runtime_name.as_str(),
                )
                .ok_or_else(|| {
                    CodegenError::new(format!(
                        "unsupported stdlib import '{name}' from module '{source}'"
                    ))
                })?;
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

/// Declare extern functions for local (file-based) imports using resolved type signatures.
fn codegen_local_import_declaration<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    items: &[ImportItem],
    source: &str,
) -> Result<(), CodegenError> {
    for item in items {
        match *item {
            ImportItem::Named {
                ref name,
                ref alias,
                ..
            } => {
                let local_name = alias.as_ref().unwrap_or(name).clone();
                // Look up the resolved type signature from the imported_signatures map.
                let Some(core_type) = env.imported_signatures.get(name).cloned() else {
                    // Symbol not found in signatures — may be a type-only import; skip.
                    continue;
                };
                let CoreType::Function {
                    ref parameters,
                    ref return_types,
                    ref error_types,
                    ..
                } = core_type
                else {
                    let accessor_name = top_level_value_accessor_name(name.as_str());
                    let accessor_type =
                        core_type_to_llvm(codegen_context.context, &core_type).fn_type(&[], false);
                    let _extern_accessor = codegen_context
                        .module
                        .get_function(accessor_name.as_str())
                        .unwrap_or_else(|| {
                            codegen_context.module.add_function(
                                accessor_name.as_str(),
                                accessor_type,
                                Some(Linkage::External),
                            )
                        });
                    env.value_accessors.insert(
                        local_name,
                        ValueAccessorBinding {
                            accessor_name,
                            core_type,
                        },
                    );
                    continue;
                };
                // Build lowered parameter types (arrays get an extra length i64 param).
                let mut lowered_params: Vec<BasicMetadataTypeEnum<'context>> = Vec::new();
                for param_type in parameters {
                    lowered_params.push(match param_type {
                        CoreType::Array(element_type) => {
                            core_type_to_llvm(codegen_context.context, element_type)
                                .ptr_type(AddressSpace::default())
                                .into()
                        }
                        _ => core_type_to_llvm(codegen_context.context, param_type).into(),
                    });
                    if matches!(*param_type, CoreType::Array(_)) {
                        lowered_params.push(codegen_context.context.i64_type().into());
                    }
                }
                let fn_type = build_function_type(
                    codegen_context,
                    &lowered_params,
                    return_types,
                    error_types,
                )?;
                // Declare the function as external (defined in another object file).
                let extern_fn = codegen_context.module.add_function(
                    name.as_str(),
                    fn_type,
                    Some(Linkage::External),
                );
                env.imported_functions.insert(
                    local_name,
                    extern_fn
                        .get_name()
                        .to_str()
                        .map_or_else(|_| name.clone(), alloc::borrow::ToOwned::to_owned),
                );
            }
            ImportItem::Type { .. } => {
                // Type imports have no runtime representation — skip.
            }
            ImportItem::Glob { .. } => {
                return Err(CodegenError::new(format!(
                    "glob imports are not supported in codegen for local module '{source}'"
                )));
            }
        }
    }
    Ok(())
}
