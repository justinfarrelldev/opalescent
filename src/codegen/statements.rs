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
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_statement, codegen_loop_expression_into_slots, codegen_loop_statement,
    codegen_return_statement,
};
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::build_error_aggregate;
use crate::codegen::expressions::{ArrayMetadata, CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::FunctionValue;

#[path = "statements/inference.rs"]
#[doc = "Expression and annotation type inference helpers for statement lowering."]
mod inference;
#[path = "statements/runtime_type_info.rs"]
#[doc = "Runtime return and guard-success type mapping helpers for statement lowering."]
mod runtime_type_info;
use self::inference::{ast_type_to_core_type_for_let, infer_core_type_from_expr};
use self::runtime_type_info::infer_guard_success_core_type;

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
            for statement in statements {
                codegen_statement(codegen_context, env, statement)?;
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
        Stmt::Continue { .. } => codegen_continue_statement(codegen_context, env),
        Stmt::Comment { .. } => Ok(()),
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
        let alloca = codegen_context
            .builder
            .build_alloca(initializer_value.get_type(), binding.name.as_str())?;
        let _store_instruction = codegen_context
            .builder
            .build_store(alloca, initializer_value)?;
        alloca
    } else {
        let alloca_type = core_type_to_llvm(codegen_context.context, &declared_type);
        codegen_context
            .builder
            .build_alloca(alloca_type, binding.name.as_str())?
    };

    let array_length = initializer.and_then(|initializer_expr| {
        if let Expr::Array { ref elements, .. } = *initializer_expr {
            u32::try_from(elements.len()).ok()
        } else {
            None
        }
    });
    let array_capacity = array_length;
    let pending_array_metadata =
        if array_length.is_none() && matches!(declared_type, CoreType::Array(_)) {
            env.take_pending_array_metadata()
        } else {
            env.set_pending_array_metadata(None);
            None
        };

    env.variables.insert(
        binding.name.clone(),
        VariableBinding {
            alloca,
            core_type: declared_type,
            length: array_length,
            capacity: array_capacity,
            is_mutable: binding.is_mutable,
        },
    );
    if let Some(ArrayMetadata { length, capacity }) = pending_array_metadata {
        let len_binding_name = format!("{}_len", binding.name);
        let len_alloca = codegen_context
            .builder
            .build_alloca(length.get_type(), len_binding_name.as_str())?;
        let _store_len = codegen_context.builder.build_store(len_alloca, length)?;
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

        let cap_binding_name = format!("{}_cap", binding.name);
        let cap_alloca = codegen_context
            .builder
            .build_alloca(capacity.get_type(), cap_binding_name.as_str())?;
        let _store_cap = codegen_context.builder.build_store(cap_alloca, capacity)?;
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
        let binding_type = binding
            .type_annotation
            .as_ref()
            .map(ast_type_to_core_type_for_let)
            .transpose()?
            .unwrap_or(CoreType::Int64);
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
                core_type: binding_type,
                length: None,
                capacity: None,
                is_mutable: binding.is_mutable,
            },
        );
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
) -> Result<(), CodegenError> {
    let loop_context = env
        .current_loop()
        .cloned()
        .ok_or_else(|| CodegenError::new(String::from("continue used outside of loop body")))?;
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

/// Lower a simple identifier assignment into a store.
fn codegen_assignment<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    value: &Expr,
) -> Result<(), CodegenError> {
    if let Expr::Identifier { ref name, .. } = *target {
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
        let binding_alloca = binding_snapshot.alloca;
        let binding_type = binding_snapshot.core_type.clone();

        let rhs_value = codegen_expression(codegen_context, env, value, Some(&binding_type))?;
        let pending_array_metadata = if matches!(binding_type, CoreType::Array(_)) {
            env.take_pending_array_metadata()
        } else {
            env.set_pending_array_metadata(None);
            None
        };
        let _store_instruction = codegen_context
            .builder
            .build_store(binding_alloca, rhs_value)?;
        if let Some(ArrayMetadata { length, capacity }) = pending_array_metadata {
            let len_binding_name = format!("{name}_len");
            if let Some(len_binding) = env.variables.get(len_binding_name.as_str()) {
                let _store_len = codegen_context
                    .builder
                    .build_store(len_binding.alloca, length)?;
            }
            let cap_binding_name = format!("{name}_cap");
            if let Some(cap_binding) = env.variables.get(cap_binding_name.as_str()) {
                let _store_cap = codegen_context
                    .builder
                    .build_store(cap_binding.alloca, capacity)?;
            }
            if let Some(binding) = env.variables.get_mut(name.as_str()) {
                binding.length = None;
                binding.capacity = None;
            }
        }
        return Ok(());
    }

    Err(CodegenError::new(String::from(
        "assignment target must be an identifier",
    )))
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
                let success_binding_name = success_binding.unwrap_or("");
                let success_alloca = if success_binding.is_some() {
                    let alloca = codegen_context
                        .builder
                        .build_alloca(success_value.get_type(), success_binding_name)?;
                    let _success_init = codegen_context
                        .builder
                        .build_store(alloca, success_value.get_type().const_zero())?;
                    Some(alloca)
                } else {
                    None
                };
                let len_binding_name = format!("{success_binding_name}_len");
                let cap_binding_name = format!("{success_binding_name}_cap");
                let metadata_allocas = if success_binding.is_some()
                    && matches!(success_core_type, CoreType::Array(_))
                    && field_count >= 3
                {
                    let length_alloca = codegen_context.builder.build_alloca(
                        codegen_context.context.i64_type(),
                        len_binding_name.as_str(),
                    )?;
                    let _length_init = codegen_context.builder.build_store(
                        length_alloca,
                        codegen_context.context.i64_type().const_zero(),
                    )?;
                    let capacity_alloca = codegen_context.builder.build_alloca(
                        codegen_context.context.i64_type(),
                        cap_binding_name.as_str(),
                    )?;
                    let _capacity_init = codegen_context.builder.build_store(
                        capacity_alloca,
                        codegen_context.context.i64_type().const_zero(),
                    )?;
                    Some((length_alloca, capacity_alloca))
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
                        .build_store(success_slot, success_value)?;
                }
                if let Some((length_alloca, capacity_alloca)) = metadata_allocas {
                    let length_value = codegen_context.builder.build_extract_value(
                        struct_value,
                        1,
                        env.next_name("guard.len").as_str(),
                    )?;
                    let _store_len = codegen_context
                        .builder
                        .build_store(length_alloca, length_value)?;
                    let capacity_value = if field_count >= 4 {
                        codegen_context.builder.build_extract_value(
                            struct_value,
                            2,
                            env.next_name("guard.cap").as_str(),
                        )?
                    } else {
                        length_value
                    };
                    let _store_cap = codegen_context
                        .builder
                        .build_store(capacity_alloca, capacity_value)?;
                }
                if let Some(block) = codegen_context.builder.get_insert_block() {
                    if block.get_terminator().is_none() {
                        let _jump_merge = codegen_context
                            .builder
                            .build_unconditional_branch(merge_block)?;
                    }
                }

                codegen_context.builder.position_at_end(else_block);
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
                env.push_active_guard_error_slot(error_alloca);

                codegen_statement(codegen_context, env, else_body)?;

                let popped_guard_error_slot = env.pop_active_guard_error_slot();
                debug_assert!(
                    popped_guard_error_slot == Some(error_alloca),
                    "guard error slot stack should unwind in LIFO order"
                );

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
                    if let Some((len_alloca, cap_alloca)) = metadata_allocas {
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
    let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
    Ok(())
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
