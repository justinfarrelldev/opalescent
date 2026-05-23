#![allow(
    clippy::all,
    clippy::too_many_lines,
    clippy::needless_pass_by_ref_mut,
    clippy::missing_docs_in_private_items,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Expr, LabeledValue, LetBinding, Stmt};
use crate::codegen::adts::product_field_indices_from_constructor;
use crate::codegen::binding_store::{
    StoreMode, binding_requires_rc_cleanup, initialize_binding_value,
    store_binding_overwrite_rc_safe, store_binding_overwrite_rc_safe_with_mode,
};
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_statement, codegen_loop_expression_into_slots, codegen_loop_statement,
    codegen_return_statement,
};
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::build_error_aggregate;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::expressions_array::{
    codegen_identifier_indexed_array_assignment, materialize_runtime_array_from_raw_elements,
};
use crate::codegen::scope_tracker::{
    cleanup_return_scopes_preserving_codegen_env,
    cleanup_scopes_to_depth_preserving_codegen_env,
    expr_requires_malloc_string_cleanup,
    infer_loop_break_binding_requires_malloc_string_cleanup,
    mark_binding_malloc_string_cleanup,
};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::{BasicValue, FunctionValue};

#[path = "statements/inference.rs"]
#[doc = "Expression and annotation type inference helpers for statement lowering."]
mod inference;
#[path = "statements/runtime_type_info.rs"]
#[doc = "Runtime return and guard-success type mapping helpers for statement lowering."]
mod runtime_type_info;
use self::inference::{ast_type_to_core_type_for_let, infer_core_type_from_expr};
use self::runtime_type_info::{infer_guard_success_core_type, known_runtime_return_type};

/// Lower one typed statement into LLVM IR side effects.
pub fn codegen_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    stmt: &Stmt,
) -> Result<(), CodegenError> {
    match *stmt {
        Stmt::Let {
            ref binding,
            ref initializer,
            ..
        } => codegen_let_statement(codegen_context, env, binding, initializer.as_ref()),
        Stmt::LetDestructure {
            ref bindings,
            ref initializer,
            ..
        } => codegen_let_destructure_statement(
            codegen_context,
            env,
            bindings.as_slice(),
            initializer,
        ),
        Stmt::Assignment {
            ref target,
            ref value,
            ..
        } => codegen_assignment(codegen_context, env, target, value),
        Stmt::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => codegen_if_statement(
            codegen_context,
            env,
            condition,
            then_branch.as_ref(),
            else_branch.as_deref(),
        ),
        Stmt::Guard {
            ref expression,
            ref success_binding,
            ref error_binding,
            ref else_body,
            ..
        } => codegen_guard_statement(
            codegen_context,
            env,
            expression.as_ref(),
            success_binding.as_deref(),
            error_binding.as_str(),
            else_body.as_ref(),
        ),
        Stmt::For { .. } | Stmt::While { .. } | Stmt::Loop { .. } => {
            codegen_loop_statement(codegen_context, env, stmt)
        }
        Stmt::PropagateGuardError {
            ref error_binding, ..
        } => {
            codegen_guard_error_propagation_statement(codegen_context, env, error_binding.as_str())
        }
        Stmt::Return { ref values, .. } => {
            codegen_return_statement(codegen_context, env, values.as_slice())
        }
        Stmt::Block { ref statements, .. } => {
            let _block_scope_depth = env.enter_scope();
            for statement in statements {
                codegen_statement(codegen_context, env, statement)?;
            }
            if let Some(current_block) = codegen_context.builder.get_insert_block() {
                if current_block.get_terminator().is_none() {
                    crate::codegen::scope_tracker::cleanup_scopes_to_depth_with_malloc_string_release(
                        codegen_context,
                        env,
                        env.current_scope_depth().saturating_sub(1),
                        &[],
                    )?;
                } else {
                    unwind_scope_without_cleanup(env);
                }
            } else {
                unwind_scope_without_cleanup(env);
            }
            Ok(())
        }
        Stmt::Expression { ref expr, .. } => {
            let _value = codegen_expression(codegen_context, env, expr, None)?;
            Ok(())
        }
        Stmt::Break { ref values, .. } => {
            codegen_break_statement(codegen_context, env, values.as_slice())
        }
        Stmt::Continue { ref values, .. } => {
            codegen_continue_statement(codegen_context, env, values.as_slice())
        }
        Stmt::Comment { .. } => Ok(()),
    }
}

pub(crate) fn unwind_scope_without_cleanup<'context>(env: &mut CodegenEnv<'context>) {
    let Some(scope_bindings) = env.scope_stack.pop() else {
        return;
    };

    for binding_name in scope_bindings.into_iter().rev() {
        let _removed_binding = env.variables.remove(binding_name.as_str());
        let _removed_indices = env.variable_field_indices.remove(binding_name.as_str());
        let _removed_aliases = env.variable_field_aliases.remove(binding_name.as_str());
    }
}

/// Lower a `let` statement by allocating storage and binding initializer values.
fn codegen_let_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding: &LetBinding,
    initializer: Option<&Expr>,
) -> Result<(), CodegenError> {
    if let Some(initializer_expr) = initializer {
        if let &Expr::Loop { .. } = initializer_expr {
            let loop_binding = [binding.clone()];
            return codegen_let_destructure_statement(
                codegen_context,
                env,
                loop_binding.as_slice(),
                initializer_expr,
            );
        }
    }

    let (declared_type, lowered_initializer) = if let Some(ref annotation) = binding.type_annotation
    {
        let declared_type = ast_type_to_core_type_for_let(annotation)?;
        let lowered = if let Some(init_expr) = initializer {
            Some(codegen_expression(
                codegen_context,
                env,
                init_expr,
                Some(&declared_type),
            )?)
        } else {
            None
        };
        (declared_type, lowered)
    } else if let Some(init_expr) = initializer {
        let inferred_type = infer_core_type_from_expr(codegen_context, env, init_expr);
        let lowered = codegen_expression(codegen_context, env, init_expr, Some(&inferred_type))?;
        (inferred_type, Some(lowered))
    } else {
        (CoreType::Unit, None)
    };

    let alloca = if let Some(initializer_value) = lowered_initializer {
        codegen_context
            .builder
            .build_alloca(initializer_value.get_type(), binding.name.as_str())?
    } else {
        let alloca_type = core_type_to_llvm(codegen_context.context, &declared_type);
        codegen_context
            .builder
            .build_alloca(alloca_type, binding.name.as_str())?
    };

    env.variables.insert(
        binding.name.clone(),
        VariableBinding {
            alloca,
            core_type: declared_type.clone(),
            length: None,
            capacity: None,
            is_mutable: binding.is_mutable,
        },
    );
    env.register_scope_binding(binding.name.as_str());
    if declared_type == CoreType::String
        && initializer.is_some_and(|initializer_expr| {
            expr_requires_malloc_string_cleanup(
                codegen_context,
                env,
                initializer_expr,
                &BTreeMap::new(),
            )
        })
    {
        mark_binding_malloc_string_cleanup(env, binding.name.as_str());
    }
    if let (Some(initializer_expr), Some(initializer_value)) = (initializer, lowered_initializer) {
        let retain_new_value = matches!(*initializer_expr, Expr::Identifier { .. });
        initialize_binding_value(
            codegen_context,
            env,
            binding.name.as_str(),
            initializer_value,
            "let.init",
            retain_new_value,
        )?;
    }
    if let Some(&Expr::Constructor { .. }) = initializer {
        if let Some(field_indices) = initializer.and_then(product_field_indices_from_constructor) {
            env.variable_field_indices
                .insert(binding.name.clone(), field_indices);
            if let Some(&Expr::Constructor { ref fields, .. }) = initializer {
                let mut field_aliases = alloc::collections::BTreeMap::new();
                for field in fields {
                    if let Expr::Identifier { ref name, .. } = field.value {
                        field_aliases.insert(field.name.clone(), name.clone());
                    }
                }
                if !field_aliases.is_empty() {
                    env.variable_field_aliases
                        .insert(binding.name.clone(), field_aliases);
                }
            }
        }
    }

    Ok(())
}

/// Lower a destructuring `let` from a loop expression into preallocated slots.
fn codegen_let_destructure_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    bindings: &[LetBinding],
    initializer: &Expr,
) -> Result<(), CodegenError> {
    let Expr::Loop { ref body, .. } = *initializer else {
        return Err(CodegenError::new(String::from(
            "destructuring let currently requires loop expression initializer",
        )));
    };

    let mut slots = Vec::new();
    let mut labels = Vec::new();
    for binding in bindings {
        let binding_type = if let Some(annotation) = binding.type_annotation.as_ref() {
            ast_type_to_core_type_for_let(annotation)?
        } else {
            infer_loop_break_binding_type(
                codegen_context,
                env,
                body.as_ref(),
                bindings,
                binding.name.as_str(),
            )
        };
        let slot_type = core_type_to_llvm(codegen_context.context, &binding_type);
        let alloca = codegen_context
            .builder
            .build_alloca(slot_type, binding.name.as_str())?;
        slots.push(alloca);
        labels.push(binding.name.clone());
        env.variables.insert(
            binding.name.clone(),
            VariableBinding {
                alloca,
                core_type: binding_type.clone(),
                length: None,
                capacity: None,
                is_mutable: binding.is_mutable,
            },
        );
        env.register_scope_binding(binding.name.as_str());
        if binding_type == CoreType::String
            && infer_loop_break_binding_requires_malloc_string_cleanup(
                codegen_context,
                env,
                body.as_ref(),
                bindings,
                binding.name.as_str(),
            )
        {
            mark_binding_malloc_string_cleanup(env, binding.name.as_str());
        }
    }

    codegen_loop_expression_into_slots(
        codegen_context,
        env,
        body.as_ref(),
        slots.as_slice(),
        labels.as_slice(),
    )
}

/// Lower a `break` statement using the active loop frame.
fn codegen_break_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
) -> Result<(), CodegenError> {
    let loop_context = env
        .current_loop()
        .cloned()
        .ok_or_else(|| CodegenError::new(String::from("break used outside of loop body")))?;
    store_break_values_into_slots(
        codegen_context,
        env,
        values,
        loop_context.break_slots.as_slice(),
        loop_context.break_labels.as_slice(),
    )?;
    let transferred_names = collect_transferred_identifier_names(values);
    cleanup_scopes_to_depth_preserving_codegen_env(
        codegen_context,
        env,
        loop_context.scope_depth,
        transferred_names.as_slice(),
    )?;
    let _branch = codegen_context
        .builder
        .build_unconditional_branch(loop_context.break_target)?;
    let continuation = codegen_context
        .context
        .append_basic_block(current_function(codegen_context)?, "break.after");
    codegen_context.builder.position_at_end(continuation);
    Ok(())
}

/// Lower a `continue` statement using the active loop frame.
fn codegen_continue_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
) -> Result<(), CodegenError> {
    if !values.is_empty() {
        return Err(CodegenError::new(String::from(
            "continue payloads are not supported during code generation",
        )));
    }

    let loop_context = env
        .current_loop()
        .cloned()
        .ok_or_else(|| CodegenError::new(String::from("continue used outside of loop body")))?;
    cleanup_scopes_to_depth_preserving_codegen_env(
        codegen_context,
        env,
        loop_context.scope_depth,
        &[],
    )?;
    let _branch = codegen_context
        .builder
        .build_unconditional_branch(loop_context.continue_target)?;
    let continuation = codegen_context
        .context
        .append_basic_block(current_function(codegen_context)?, "continue.after");
    codegen_context.builder.position_at_end(continuation);
    Ok(())
}

/// Store loop-break payload values into pre-allocated binding slots.
fn store_break_values_into_slots<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
    break_slots: &[inkwell::values::PointerValue<'context>],
    break_labels: &[String],
) -> Result<(), CodegenError> {
    if break_slots.is_empty() {
        return Ok(());
    }

    for (index, slot) in break_slots.iter().copied().enumerate() {
        let matching_value = break_labels
            .get(index)
            .and_then(|label| values.iter().find(|value| value.label == *label))
            .or_else(|| values.get(index));
        let Some(value) = matching_value else {
            continue;
        };
        let lowered_value = codegen_expression(codegen_context, env, &value.value, None)?;
        let _store = codegen_context.builder.build_store(slot, lowered_value)?;
    }

    Ok(())
}

pub(crate) fn collect_transferred_identifier_names(values: &[LabeledValue]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| match &value.value {
            &Expr::Identifier { ref name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn infer_loop_break_binding_type<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    bindings: &[LetBinding],
    binding_name: &str,
) -> CoreType {
    let mut local_bindings = BTreeMap::new();
    infer_loop_break_binding_type_with_locals(
        codegen_context,
        env,
        stmt,
        bindings,
        binding_name,
        &mut local_bindings,
    )
}

fn infer_loop_break_binding_type_with_locals<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    bindings: &[LetBinding],
    binding_name: &str,
    local_bindings: &mut BTreeMap<String, CoreType>,
) -> CoreType {
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
            selected_value.map_or(CoreType::Int64, |value| {
                infer_loop_break_value_type(codegen_context, env, &value.value, local_bindings)
            })
        }
        &Stmt::Block { ref statements, .. } => {
            let mut scoped_bindings = local_bindings.clone();
            for statement in statements {
                let inferred = infer_loop_break_binding_type_with_locals(
                    codegen_context,
                    env,
                    statement,
                    bindings,
                    binding_name,
                    &mut scoped_bindings,
                );
                if inferred != CoreType::Int64 {
                    return inferred;
                }
                register_inferred_local_binding(codegen_context, env, statement, &mut scoped_bindings);
            }
            CoreType::Int64
        }
        &Stmt::If {
            ref then_branch,
            ref else_branch,
            ..
        } => {
            let mut then_bindings = local_bindings.clone();
            let then_type = infer_loop_break_binding_type_with_locals(
                codegen_context,
                env,
                then_branch.as_ref(),
                bindings,
                binding_name,
                &mut then_bindings,
            );
            if then_type != CoreType::Int64 {
                return then_type;
            }
            else_branch.as_deref().map_or(CoreType::Int64, |else_stmt| {
                let mut else_bindings = local_bindings.clone();
                infer_loop_break_binding_type_with_locals(
                    codegen_context,
                    env,
                    else_stmt,
                    bindings,
                    binding_name,
                    &mut else_bindings,
                )
            })
        }
        &Stmt::Guard { ref else_body, .. } | &Stmt::Loop { body: ref else_body, .. } => {
            let mut nested_bindings = local_bindings.clone();
            infer_loop_break_binding_type_with_locals(
                codegen_context,
                env,
                else_body.as_ref(),
                bindings,
                binding_name,
                &mut nested_bindings,
            )
        }
        &Stmt::While { ref body, .. } | &Stmt::For { ref body, .. } => {
            let mut nested_bindings = local_bindings.clone();
            infer_loop_break_binding_type_with_locals(
                codegen_context,
                env,
                body.as_ref(),
                bindings,
                binding_name,
                &mut nested_bindings,
            )
        }
        _ => CoreType::Int64,
    }
}

fn infer_loop_break_value_type<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expr: &Expr,
    local_bindings: &BTreeMap<String, CoreType>,
) -> CoreType {
    if let &Expr::Identifier { ref name, .. } = expr {
        if let Some(local_type) = local_bindings.get(name) {
            return local_type.clone();
        }
    }

    if let &Expr::Call { ref callee, .. } = expr {
        if let &Expr::Identifier { ref name, .. } = callee.as_ref() {
            if let Some(runtime_name) = env.imported_functions.get(name) {
                if runtime_name == "string_join" {
                    return CoreType::String;
                }
                if let Some(runtime_type) = known_runtime_return_type(runtime_name.as_str()) {
                    return runtime_type;
                }
            }
            if name == "string_join" {
                return CoreType::String;
            }
            if let Some(runtime_type) = known_runtime_return_type(name.as_str()) {
                return runtime_type;
            }
        }
    }

    infer_core_type_from_expr(codegen_context, env, expr)
}

fn register_inferred_local_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    stmt: &Stmt,
    local_bindings: &mut BTreeMap<String, CoreType>,
) {
    if let &Stmt::Let {
        ref binding,
        initializer: Some(ref initializer),
        ..
    } = stmt
    {
        let binding_type = binding.type_annotation.as_ref().map_or_else(
            || infer_loop_break_value_type(codegen_context, env, initializer, local_bindings),
            |annotation| ast_type_to_core_type_for_let(annotation).unwrap_or(CoreType::Int64),
        );
        local_bindings.insert(binding.name.clone(), binding_type);
    }
}


/// Lower a simple identifier assignment into a store.
fn codegen_assignment<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    value: &Expr,
) -> Result<(), CodegenError> {
    match *target {
        Expr::Identifier { ref name, .. } => {
            let Some(binding_snapshot) = env.variables.get(name) else {
                return Err(CodegenError::new(format!(
                    "assignment target '{name}' not found"
                )));
            };
            if !binding_snapshot.is_mutable {
                return Err(CodegenError::new(format!(
                    "cannot assign to immutable variable: {name}"
                )));
            }
            let binding_type = binding_snapshot.core_type.clone();

            let rhs_value = codegen_expression(codegen_context, env, value, Some(&binding_type))?;
            match assignment_store_mode(value, &binding_type) {
                StoreMode::Retain => store_binding_overwrite_rc_safe(
                    codegen_context,
                    env,
                    name.as_str(),
                    rhs_value,
                    "assign",
                ),
                StoreMode::TakeOwned => store_binding_overwrite_rc_safe_with_mode(
                    codegen_context,
                    env,
                    name.as_str(),
                    rhs_value,
                    "assign",
                    StoreMode::TakeOwned,
                ),
            }
        }
        Expr::Index {
            ref object,
            ref index,
            ..
        } => codegen_identifier_indexed_array_assignment(
            codegen_context,
            env,
            object.as_ref(),
            index.as_ref(),
            value,
        ),
        _ => Err(CodegenError::new(String::from(
            "assignment target must be an identifier or identifier-backed index expression",
        ))),
    }
}

/// Lower a guard statement by evaluating and binding the success value.
#[expect(
    clippy::too_many_lines,
    reason = "guard statement codegen requires handling multiple cases"
)]
fn codegen_guard_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
    success_binding: Option<&str>,
    error_binding: &str,
    else_body: &Stmt,
) -> Result<(), CodegenError> {
    let value = codegen_expression(codegen_context, env, expression, None)?;
    if value.is_struct_value() {
        let struct_value = value.into_struct_value();
        let field_count = struct_value.get_type().count_fields();
        if field_count >= 2 {
            let error_field_index = if field_count >= 3 { 2 } else { 1 };
            let error_value = codegen_context.builder.build_extract_value(
                struct_value,
                error_field_index,
                env.next_name("guard.err").as_str(),
            )?;

            if error_value.is_pointer_value() {
                let success_value = codegen_context.builder.build_extract_value(
                    struct_value,
                    0,
                    env.next_name("guard.ok").as_str(),
                )?;
                let success_core_type =
                    infer_guard_success_core_type(env, expression, success_value.get_type());
                let normalized_success_value = if field_count >= 3 {
                    if let CoreType::Array(ref element_core_type) = success_core_type {
                        if success_value.is_pointer_value() {
                            let length_value = codegen_context.builder.build_extract_value(
                                struct_value,
                                1,
                                env.next_name("guard.len").as_str(),
                            )?;
                            let runtime_array = materialize_runtime_array_from_raw_elements(
                                codegen_context,
                                env,
                                success_value.into_pointer_value(),
                                length_value.into_int_value(),
                                element_core_type.as_ref(),
                                "guard.array",
                            )?;
                            runtime_array.as_basic_value_enum()
                        } else {
                            success_value
                        }
                    } else {
                        success_value
                    }
                } else {
                    success_value
                };
                let success_binding_name = success_binding.unwrap_or("");
                let success_alloca = if success_binding.is_some() {
                    let alloca = codegen_context
                        .builder
                        .build_alloca(normalized_success_value.get_type(), success_binding_name)?;
                    let _success_init = codegen_context
                        .builder
                        .build_store(alloca, normalized_success_value.get_type().const_zero())?;
                    Some(alloca)
                } else {
                    None
                };

                let current_fn = current_function(codegen_context)?;
                let success_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.success").as_str());
                let else_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.else").as_str());
                let merge_block = codegen_context
                    .context
                    .append_basic_block(current_fn, env.next_name("guard.merge").as_str());

                let error_ptr = error_value.into_pointer_value();
                let is_success = codegen_context
                    .builder
                    .build_is_null(error_ptr, env.next_name("guard.is_success").as_str())?;
                let _branch = codegen_context.builder.build_conditional_branch(
                    is_success,
                    success_block,
                    else_block,
                )?;

                codegen_context.builder.position_at_end(success_block);
                if let Some(success_slot) = success_alloca {
                    let _store_ok = codegen_context
                        .builder
                        .build_store(success_slot, normalized_success_value)?;
                }
                if let Some(block) = codegen_context.builder.get_insert_block() {
                    if block.get_terminator().is_none() {
                        let _jump_merge = codegen_context
                            .builder
                            .build_unconditional_branch(merge_block)?;
                    }
                }

                codegen_context.builder.position_at_end(else_block);
                let _else_scope_depth = env.enter_scope();
                let error_alloca = codegen_context
                    .builder
                    .build_alloca(error_value.get_type(), error_binding)?;
                let _store_err = codegen_context
                    .builder
                    .build_store(error_alloca, error_value)?;
                let previous_error_binding = env.variables.insert(
                    error_binding.to_owned(),
                    VariableBinding {
                        alloca: error_alloca,
                        core_type: CoreType::String,
                        length: None,
                        capacity: None,
                        is_mutable: false,
                    },
                );
                env.register_scope_binding(error_binding);
                env.push_active_guard_error_slot(error_alloca);

                codegen_statement(codegen_context, env, else_body)?;

                let popped_guard_error_slot = env.pop_active_guard_error_slot();
                debug_assert!(
                    popped_guard_error_slot == Some(error_alloca),
                    "guard error slot stack should unwind in LIFO order"
                );

                if let Some(block) = codegen_context.builder.get_insert_block() {
                    if block.get_terminator().is_none() {
                        crate::codegen::scope_tracker::cleanup_scopes_to_depth_with_malloc_string_release(
                            codegen_context,
                            env,
                            env.current_scope_depth().saturating_sub(1),
                            &[],
                        )?;
                    } else {
                        unwind_scope_without_cleanup(env);
                    }
                } else {
                    unwind_scope_without_cleanup(env);
                }

                if let Some(previous) = previous_error_binding {
                    env.variables.insert(error_binding.to_owned(), previous);
                } else {
                    let _: Option<_> = env.variables.remove(error_binding);
                }

                if let Some(block) = codegen_context.builder.get_insert_block() {
                    if block.get_terminator().is_none() {
                        let _jump_merge = codegen_context
                            .builder
                            .build_unconditional_branch(merge_block)?;
                    }
                }

                codegen_context.builder.position_at_end(merge_block);
                if success_binding.is_some() {
                    let Some(success_slot) = success_alloca else {
                        return Err(CodegenError::new(String::from(
                            "guard success binding slot missing in bound guard path",
                        )));
                    };
                    env.variables.insert(
                        success_binding_name.to_owned(),
                        VariableBinding {
                            alloca: success_slot,
                            core_type: success_core_type,
                            length: None,
                            capacity: None,
                            is_mutable: false,
                        },
                    );
                    env.register_scope_binding(success_binding_name);
                }

                return Ok(());
            }
        }
    }

    if let Some(success_name) = success_binding {
        let inferred_type = infer_core_type_from_expr(codegen_context, env, expression);
        let alloca = codegen_context
            .builder
            .build_alloca(value.get_type(), success_name)?;
        let _store_instruction = codegen_context.builder.build_store(alloca, value)?;

        env.variables.insert(
            success_name.to_owned(),
            VariableBinding {
                alloca,
                core_type: inferred_type,
                length: None,
                capacity: None,
                is_mutable: false,
            },
        );
        env.register_scope_binding(success_name);
    }

    Ok(())
}

/// Fetch current LLVM function from builder insertion block.
fn codegen_guard_error_propagation_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    error_binding: &str,
) -> Result<(), CodegenError> {
    let Some(error_slot) = env.current_guard_error_slot() else {
        return Err(CodegenError::new(format!(
            "guard error binding '{error_binding}' is not active during propagation"
        )));
    };
    let loaded_error = codegen_context
        .builder
        .build_load(error_slot, env.next_name("guard.err.propagate").as_str())?;
    if !loaded_error.is_pointer_value() {
        return Err(CodegenError::new(format!(
            "guard error binding '{error_binding}' does not lower to a pointer error payload"
        )));
    }

    let current_fn = current_function(codegen_context)?;
    let Some(return_type) = current_fn.get_type().get_return_type() else {
        return Err(CodegenError::new(String::from(
            "guard error propagation requires caller to return an error aggregate",
        )));
    };
    if !return_type.is_struct_type() {
        return Err(CodegenError::new(String::from(
            "guard error propagation requires caller error aggregate return type",
        )));
    }

    let return_struct_type = return_type.into_struct_type();
    if return_struct_type.count_fields() != 2 {
        return Err(CodegenError::new(String::from(
            "guard error propagation requires canonical two-field error aggregate",
        )));
    }

    let success_type = return_struct_type
        .get_field_type_at_index(0)
        .ok_or_else(|| CodegenError::new(String::from("error aggregate missing success field")))?;
    let aggregate = build_error_aggregate(
        codegen_context,
        success_type,
        loaded_error.into_pointer_value(),
    )?;
    cleanup_return_scopes_preserving_codegen_env(codegen_context, env, &[])?;
    let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
    Ok(())
}

fn assignment_store_mode(value: &Expr, binding_type: &CoreType) -> StoreMode {
    match *value {
        // Array literals are allocated in this lowering flow, so assignment can transfer that
        // fresh RC owner directly into the binding without an extra retain.
        Expr::Array { .. } => StoreMode::TakeOwned,
        // `reserve` noop paths return an owned alias, so reassignment should consume that owner
        // instead of retaining the same pointer again.
        Expr::Call { ref callee, .. } if reserve_call_returns_owned_alias(callee.as_ref()) => {
            StoreMode::TakeOwned
        }
        // Align with reassignment leak semantics: call results transfer ownership when the
        // destination binding participates in RC cleanup.
        Expr::Call { .. } if binding_requires_take_owned_call_assignment(binding_type) => {
            StoreMode::TakeOwned
        }
        _ => StoreMode::Retain,
    }
}

fn binding_requires_take_owned_call_assignment(binding_type: &CoreType) -> bool {
    binding_requires_rc_cleanup(binding_type)
}

fn reserve_call_returns_owned_alias(callee: &Expr) -> bool {
    match *callee {
        Expr::Identifier { ref name, .. } => name == "reserve",
        _ => false,
    }
}

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
