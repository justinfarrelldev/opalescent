#![allow(
    clippy::all,
    clippy::pattern_type_mismatch,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Decl, ImportItem, Visibility};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding};
use crate::codegen::statements::codegen_statement;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::DLLStorageClass;
use inkwell::module::Linkage;
use inkwell::types::{BasicMetadataTypeEnum, BasicType};
use inkwell::values::FunctionValue;

pub use crate::codegen::functions_call::{
    ast_type_to_core_type_for_signature, build_function_type, codegen_call_expression,
    codegen_guard_expression, codegen_propagate_expression, emit_c_main_wrapper,
    emit_default_return,
};

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
    let function_name = if is_entry {
        format!("__opalescent_entry_{name}")
    } else {
        name.clone()
    };

    let mut lowered_parameter_core_types = Vec::new();
    for core_type in &parameter_core_types {
        lowered_parameter_core_types.push(core_type.clone());
        if matches!(*core_type, CoreType::Array(_)) {
            lowered_parameter_core_types.push(CoreType::Int64);
        }
    }
    let parameter_types = lowered_parameter_core_types
        .iter()
        .map(|core_type| match core_type {
            CoreType::Array(element_type) => {
                core_type_to_llvm(codegen_context.context, element_type)
                    .ptr_type(AddressSpace::default())
                    .into()
            }
            _ => core_type_to_llvm(codegen_context.context, core_type).into(),
        })
        .collect::<Vec<BasicMetadataTypeEnum<'context>>>();
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

    let mut lowered_parameter_index = 0_usize;
    for (index, parameter) in parameters.iter().enumerate() {
        let Some(param_value) =
            function
                .get_nth_param(u32::try_from(lowered_parameter_index).map_err(
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
        let _store = codegen_context.builder.build_store(alloca, param_value)?;
        let length = None;

        if matches!(parameter_core_types[index], CoreType::Array(_)) {
            let len_binding_name = format!("{}_len", parameter.name);
            let len_parameter_index = lowered_parameter_index.saturating_add(1);
            let Some(len_param_value) = function.get_nth_param(
                u32::try_from(len_parameter_index)
                    .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?,
            ) else {
                return Err(CodegenError::new(format!(
                    "missing function array length parameter for '{}'",
                    parameter.name
                )));
            };
            let len_alloca = codegen_context
                .builder
                .build_alloca(len_param_value.get_type(), len_binding_name.as_str())?;
            let _store_len = codegen_context
                .builder
                .build_store(len_alloca, len_param_value)?;
            env.variables.insert(
                len_binding_name,
                VariableBinding {
                    alloca: len_alloca,
                    core_type: CoreType::Int64,
                    length: None,
                    is_mutable: false,
                },
            );
            lowered_parameter_index = lowered_parameter_index.saturating_add(1);
        }

        env.variables.insert(
            parameter.name.clone(),
            VariableBinding {
                alloca,
                core_type: parameter_core_types[index].clone(),
                length,
                is_mutable: false,
            },
        );

        lowered_parameter_index = lowered_parameter_index.saturating_add(1);
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

    Ok(function)
}

#[doc = "Lower import declarations by declaring known stdlib externs and alias mappings."]
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
                    // Not a function (e.g. a type alias) — no runtime declaration needed.
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
