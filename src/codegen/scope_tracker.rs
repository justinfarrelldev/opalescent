#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::needless_pass_by_ref_mut,
    reason = "internal codegen scope tracking and cleanup module"
)]

extern crate alloc;

use crate::ast::{Expr, LetBinding, Stmt};
use crate::codegen::binding_store::{binding_requires_rc_cleanup, release_binding_value_if_needed};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

pub(crate) const MALLOC_STRING_CLEANUP_KEY: &str = "__opal_cleanup_kind__";
pub(crate) const MALLOC_STRING_CLEANUP_VALUE: &str = "malloc_string";

impl<'context> CodegenEnv<'context> {
    #[must_use]
    pub fn enter_scope(&mut self) -> usize {
        self.scope_stack.push(Vec::new());
        self.scope_stack.len()
    }

    #[must_use]
    pub fn current_scope_depth(&self) -> usize {
        self.scope_stack.len()
    }

    pub fn register_scope_binding(&mut self, name: &str) {
        let Some(scope_bindings) = self.scope_stack.last_mut() else {
            return;
        };
        if !scope_bindings.iter().any(|binding_name| binding_name == name) {
            scope_bindings.push(name.to_owned());
        }
    }

    pub fn release_scope_binding_value(
        &mut self,
        codegen_context: &CodegenContext<'context>,
        name: &str,
        transferred_names: &[String],
    ) -> Result<(), CodegenError> {
        if transferred_names.iter().any(|transferred_name| transferred_name == name) {
            return Ok(());
        }

        let Some(binding) = self.variables.remove(name) else {
            return Ok(());
        };
        let _removed_indices = self.variable_field_indices.remove(name);
        let _removed_aliases = self.variable_field_aliases.remove(name);

        if matches!(binding.core_type, CoreType::String) {
            return Ok(());
        }
        if !binding_requires_rc_cleanup(&binding.core_type) {
            return Ok(());
        }

        let loaded_value = codegen_context.builder.build_load(
            binding.alloca,
            self.next_name("scope.release.load").as_str(),
        )?;
        release_binding_value_if_needed(
            codegen_context,
            &binding.core_type,
            loaded_value,
            "scope cleanup",
        )
    }

    pub fn exit_scope_cleanup(
        &mut self,
        codegen_context: &CodegenContext<'context>,
        transferred_names: &[String],
    ) -> Result<(), CodegenError> {
        let Some(scope_bindings) = self.scope_stack.pop() else {
            return Ok(());
        };

        for binding_name in scope_bindings.into_iter().rev() {
            self.release_scope_binding_value(codegen_context, binding_name.as_str(), transferred_names)?;
        }
        Ok(())
    }

    pub fn cleanup_all_scopes_for_return(
        &mut self,
        codegen_context: &CodegenContext<'context>,
        transferred_names: &[String],
    ) -> Result<(), CodegenError> {
        self.cleanup_scopes_to_depth(codegen_context, 0, transferred_names)
    }

    pub fn cleanup_scopes_to_depth(
        &mut self,
        codegen_context: &CodegenContext<'context>,
        target_depth: usize,
        transferred_names: &[String],
    ) -> Result<(), CodegenError> {
        while self.scope_stack.len() > target_depth {
            self.exit_scope_cleanup(codegen_context, transferred_names)?;
        }
        Ok(())
    }
}

pub(crate) fn cleanup_return_scopes_preserving_codegen_env<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    transferred_names: &[String],
) -> Result<(), CodegenError> {
    let original_variables = env.variables.clone();
    let original_field_indices = env.variable_field_indices.clone();
    let original_field_aliases = env.variable_field_aliases.clone();
    let original_scope_stack = env.scope_stack.clone();

    let args_binding_is_unregistered = env.variables.contains_key("args")
        && !env
            .scope_stack
            .iter()
            .any(|scope| scope.iter().any(|binding_name| binding_name == "args"));
    if args_binding_is_unregistered {
        env.scope_stack.push(vec![String::from("args")]);
    }

    let cleanup_result = cleanup_scopes_to_depth_with_malloc_string_release(
        codegen_context,
        env,
        0,
        transferred_names,
    );
    env.variables = original_variables;
    env.variable_field_indices = original_field_indices;
    env.variable_field_aliases = original_field_aliases;
    env.scope_stack = original_scope_stack;
    cleanup_result
}

pub(crate) fn expr_requires_malloc_string_cleanup<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expr: &Expr,
    local_malloc_string_bindings: &BTreeMap<String, bool>,
) -> bool {
    match expr {
        &Expr::Identifier { ref name, .. } => local_malloc_string_bindings.get(name).copied().unwrap_or_else(|| {
            env.variable_field_aliases
                .get(name)
                .and_then(|metadata| metadata.get(MALLOC_STRING_CLEANUP_KEY))
                .is_some_and(|value| value == MALLOC_STRING_CLEANUP_VALUE)
        }),
        &Expr::Call { ref callee, .. } => call_returns_owned_string(codegen_context, env, callee.as_ref()),
        &Expr::Propagate { ref call, .. } => {
            expr_requires_malloc_string_cleanup(codegen_context, env, call.as_ref(), local_malloc_string_bindings)
        }
        &Expr::Parenthesized { ref expr, .. } => {
            expr_requires_malloc_string_cleanup(codegen_context, env, expr.as_ref(), local_malloc_string_bindings)
        }
        _ => false,
    }
}

fn call_returns_owned_string<'context>(
    _codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    callee: &Expr,
) -> bool {
    let &Expr::Identifier { ref name, .. } = callee else {
        return false;
    };

    runtime_returns_owned_string(name)
        || env
            .imported_functions
            .get(name)
            .is_some_and(|runtime_name| runtime_returns_owned_string(runtime_name.as_str()))
        || env
            .owned_string_functions
            .get(name)
            .copied()
            .unwrap_or(false)
}

fn runtime_returns_owned_string(name: &str) -> bool {
    matches!(
        name,
        "string_join"
            | "string_builder_finish"
            | "take_input"
            | "bytes_to_hex"
            | "path_file_name"
            | "path_file_extension"
            | "read_text_sync"
            | "read_first_line_sync"
            | "int8_to_string"
            | "int16_to_string"
            | "int32_to_string"
            | "int64_to_string"
            | "uint8_to_string"
            | "uint16_to_string"
            | "uint32_to_string"
            | "uint64_to_string"
            | "float32_to_string"
            | "float64_to_string"
            | "bool_to_string"
    )
}

pub(crate) fn infer_loop_break_binding_requires_malloc_string_cleanup<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    bindings: &[LetBinding],
    binding_name: &str,
) -> bool {
    let mut local_malloc_string_bindings = BTreeMap::new();
    infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
        codegen_context,
        env,
        stmt,
        bindings,
        binding_name,
        &mut local_malloc_string_bindings,
    )
}

fn infer_loop_break_binding_requires_malloc_string_cleanup_with_locals<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    bindings: &[LetBinding],
    binding_name: &str,
    local_malloc_string_bindings: &mut BTreeMap<String, bool>,
) -> bool {
    match stmt {
        &Stmt::Break { ref values, .. } => {
            let selected_value = if bindings.len() == 1 {
                values.first()
            } else {
                values
                    .iter()
                    .find(|value| value.label == binding_name)
                    .or_else(|| {
                        bindings
                            .iter()
                            .position(|binding| binding.name == binding_name)
                            .and_then(|index| values.get(index))
                    })
            };
            selected_value.is_some_and(|value| {
                expr_requires_malloc_string_cleanup(
                    codegen_context,
                    env,
                    &value.value,
                    local_malloc_string_bindings,
                )
            })
        }
        &Stmt::Block { ref statements, .. } => {
            let mut scoped = local_malloc_string_bindings.clone();
            for statement in statements {
                if infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
                    codegen_context,
                    env,
                    statement,
                    bindings,
                    binding_name,
                    &mut scoped,
                ) {
                    return true;
                }
                register_local_malloc_string_binding(codegen_context, env, statement, &mut scoped);
            }
            false
        }
        &Stmt::If {
            ref then_branch,
            ref else_branch,
            ..
        } => {
            let mut then_scoped = local_malloc_string_bindings.clone();
            if infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
                codegen_context,
                env,
                then_branch.as_ref(),
                bindings,
                binding_name,
                &mut then_scoped,
            ) {
                return true;
            }
            else_branch.as_deref().is_some_and(|else_stmt| {
                let mut else_scoped = local_malloc_string_bindings.clone();
                infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
                    codegen_context,
                    env,
                    else_stmt,
                    bindings,
                    binding_name,
                    &mut else_scoped,
                )
            })
        }
        &Stmt::Guard { ref else_body, .. } | &Stmt::Loop { body: ref else_body, .. } => {
            let mut nested = local_malloc_string_bindings.clone();
            infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
                codegen_context,
                env,
                else_body.as_ref(),
                bindings,
                binding_name,
                &mut nested,
            )
        }
        &Stmt::While { ref body, .. } | &Stmt::For { ref body, .. } => {
            let mut nested = local_malloc_string_bindings.clone();
            infer_loop_break_binding_requires_malloc_string_cleanup_with_locals(
                codegen_context,
                env,
                body.as_ref(),
                bindings,
                binding_name,
                &mut nested,
            )
        }
        _ => false,
    }
}

fn register_local_malloc_string_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    local_malloc_string_bindings: &mut BTreeMap<String, bool>,
) {
    if let &Stmt::Let {
        ref binding,
        initializer: Some(ref initializer),
        ..
    } = stmt
    {
        let requires_cleanup = binding.type_annotation.as_ref().is_some_and(|annotation| {
            ast_type_to_core_type(annotation).ok() == Some(CoreType::String)
        }) || expr_requires_malloc_string_cleanup(
            codegen_context,
            env,
            initializer,
            local_malloc_string_bindings,
        );
        if requires_cleanup {
            local_malloc_string_bindings.insert(binding.name.clone(), true);
        }
    }
}

pub(crate) fn mark_binding_malloc_string_cleanup(env: &mut CodegenEnv<'_>, binding_name: &str) {
    let metadata = env
        .variable_field_aliases
        .entry(binding_name.to_owned())
        .or_default();
    metadata.insert(
        String::from(MALLOC_STRING_CLEANUP_KEY),
        String::from(MALLOC_STRING_CLEANUP_VALUE),
    );
}


fn collect_malloc_string_cleanup_bindings(
    env: &CodegenEnv<'_>,
    target_depth: usize,
    transferred_names: &[String],
) -> Vec<String> {
    let mut bindings = Vec::new();
    for scope_bindings in env.scope_stack.iter().skip(target_depth) {
        for binding_name in scope_bindings {
            if transferred_names.iter().any(|name| name == binding_name) {
                continue;
            }
            if env
                .variable_field_aliases
                .get(binding_name)
                .and_then(|metadata| metadata.get(MALLOC_STRING_CLEANUP_KEY))
                .is_some_and(|value| value == MALLOC_STRING_CLEANUP_VALUE)
            {
                bindings.push(binding_name.clone());
            }
        }
    }
    bindings
}

fn release_malloc_string_binding_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding_name: &str,
) -> Result<(), CodegenError> {
    let Some(binding) = env.variables.get(binding_name).cloned() else {
        return Ok(());
    };
    let loaded_value = codegen_context.builder.build_load(
        binding.alloca,
        env.next_name("scope.release.malloc_string").as_str(),
    )?;
    if !loaded_value.is_pointer_value() {
        return Ok(());
    }

    let i32_type = codegen_context.context.i32_type();
    let note_free_fn_type = codegen_context.context.void_type().fn_type(&[i32_type.into()], false);
    let note_free_fn = codegen_context
        .module
        .get_function("opal_rc_debug_note_free")
        .unwrap_or_else(|| {
            codegen_context
                .module
                .add_function("opal_rc_debug_note_free", note_free_fn_type, None)
        });
    let _note_free = codegen_context.builder.build_call(
        note_free_fn,
        &[i32_type.const_int(1, false).into()],
        env.next_name("scope.release.note_free").as_str(),
    )?;

    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(inkwell::AddressSpace::default());
    let free_fn_type = codegen_context.context.void_type().fn_type(&[i8_ptr.into()], false);
    let free_fn = codegen_context.module.get_function("free").unwrap_or_else(|| {
        codegen_context.module.add_function("free", free_fn_type, None)
    });
    let _free = codegen_context.builder.build_call(
        free_fn,
        &[loaded_value.into_pointer_value().into()],
        env.next_name("scope.release.free").as_str(),
    )?;
    Ok(())
}

fn clear_binding_cleanup_metadata(env: &mut CodegenEnv<'_>, binding_name: &str) {
    let remove_entry = env
        .variable_field_aliases
        .get_mut(binding_name)
        .map_or(false, |metadata| {
            metadata.remove(MALLOC_STRING_CLEANUP_KEY);
            metadata.is_empty()
        });
    if remove_entry {
        let _removed_aliases = env.variable_field_aliases.remove(binding_name);
    }
}

pub(crate) fn cleanup_scopes_to_depth_with_malloc_string_release<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target_depth: usize,
    transferred_names: &[String],
) -> Result<(), CodegenError> {
    let malloc_string_bindings = collect_malloc_string_cleanup_bindings(env, target_depth, transferred_names);
    for binding_name in &malloc_string_bindings {
        release_malloc_string_binding_value(codegen_context, env, binding_name.as_str())?;
    }

    let mut cleanup_skips = transferred_names.to_vec();
    cleanup_skips.extend(malloc_string_bindings.iter().cloned());
    let cleanup_result = env.cleanup_scopes_to_depth(codegen_context, target_depth, cleanup_skips.as_slice());
    if cleanup_result.is_ok() {
        for binding_name in malloc_string_bindings {
            clear_binding_cleanup_metadata(env, binding_name.as_str());
            let _removed_binding = env.variables.remove(binding_name.as_str());
            let _removed_indices = env.variable_field_indices.remove(binding_name.as_str());
        }
    }
    cleanup_result
}

pub(crate) fn cleanup_scopes_to_depth_preserving_codegen_env<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target_depth: usize,
    transferred_names: &[String],
) -> Result<(), CodegenError> {
    let original_variables = env.variables.clone();
    let original_field_indices = env.variable_field_indices.clone();
    let original_field_aliases = env.variable_field_aliases.clone();
    let original_scope_stack = env.scope_stack.clone();

    let cleanup_result = cleanup_scopes_to_depth_with_malloc_string_release(
        codegen_context,
        env,
        target_depth,
        transferred_names,
    );
    env.variables = original_variables;
    env.variable_field_indices = original_field_indices;
    env.variable_field_aliases = original_field_aliases;
    env.scope_stack = original_scope_stack;
    cleanup_result
}
