#![allow(
    clippy::all,
    clippy::too_many_lines,
    clippy::needless_pass_by_ref_mut,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Expr, LabeledValue, LetBinding, Stmt, Type};
use crate::codegen::adts::product_field_indices_from_constructor;
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::{
    codegen_if_statement, codegen_loop_expression_into_slots, codegen_loop_statement,
    codegen_return_statement,
};
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::type_mapping::{AstTypeMappingError, ast_type_to_core_type};
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::types::BasicTypeEnum;
use inkwell::values::FunctionValue;

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
            success_binding.as_str(),
            error_binding.as_str(),
            else_body.as_ref(),
        ),
        Stmt::For { .. } | Stmt::While { .. } | Stmt::Loop { .. } => {
            codegen_loop_statement(codegen_context, env, stmt)
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
    let pending_array_length =
        if array_length.is_none() && matches!(declared_type, CoreType::Array(_)) {
            env.take_pending_array_length()
        } else {
            env.set_pending_array_length(None);
            None
        };

    env.variables.insert(
        binding.name.clone(),
        VariableBinding {
            alloca,
            core_type: declared_type,
            length: array_length,
            is_mutable: binding.is_mutable,
        },
    );
    if let Some(length_value) = pending_array_length {
        let len_binding_name = format!("{}_len", binding.name);
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
        let _store_instruction = codegen_context
            .builder
            .build_store(binding_alloca, rhs_value)?;
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
    success_binding: &str,
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
                let success_alloca = codegen_context
                    .builder
                    .build_alloca(success_value.get_type(), success_binding)?;
                let _success_init = codegen_context
                    .builder
                    .build_store(success_alloca, success_value.get_type().const_zero())?;
                let len_binding_name = format!("{success_binding}_len");
                let len_alloca =
                    if matches!(success_core_type, CoreType::Array(_)) && field_count >= 3 {
                        let length_alloca = codegen_context.builder.build_alloca(
                            codegen_context.context.i64_type(),
                            len_binding_name.as_str(),
                        )?;
                        let _length_init = codegen_context.builder.build_store(
                            length_alloca,
                            codegen_context.context.i64_type().const_zero(),
                        )?;
                        Some(length_alloca)
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
                let _store_ok = codegen_context
                    .builder
                    .build_store(success_alloca, success_value)?;
                if let Some(length_alloca) = len_alloca {
                    let length_value = codegen_context.builder.build_extract_value(
                        struct_value,
                        1,
                        env.next_name("guard.len").as_str(),
                    )?;
                    let _store_len = codegen_context
                        .builder
                        .build_store(length_alloca, length_value)?;
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
                        is_mutable: false,
                    },
                );

                codegen_statement(codegen_context, env, else_body)?;

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
                if let Some(len_alloca) = len_alloca {
                    env.variables.insert(
                        len_binding_name,
                        VariableBinding {
                            alloca: len_alloca,
                            core_type: CoreType::Int64,
                            length: None,
                            is_mutable: false,
                        },
                    );
                }
                env.variables.insert(
                    success_binding.to_owned(),
                    VariableBinding {
                        alloca: success_alloca,
                        core_type: success_core_type,
                        length: None,
                        is_mutable: false,
                    },
                );

                return Ok(());
            }
        }
    }

    let inferred_type = infer_core_type_from_expr(codegen_context, env, expression);
    let alloca = codegen_context
        .builder
        .build_alloca(value.get_type(), success_binding)?;
    let _store_instruction = codegen_context.builder.build_store(alloca, value)?;

    env.variables.insert(
        success_binding.to_owned(),
        VariableBinding {
            alloca,
            core_type: inferred_type,
            length: None,
            is_mutable: false,
        },
    );

    Ok(())
}

/// Infer semantic success type for values bound by `guard ... into`.
fn infer_guard_success_core_type<'context>(
    env: &CodegenEnv<'context>,
    expression: &Expr,
    success_value_type: inkwell::types::BasicTypeEnum<'context>,
) -> CoreType {
    let Expr::Call { ref callee, .. } = *expression else {
        return llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64);
    };
    let Expr::Identifier { ref name, .. } = *callee.as_ref() else {
        return llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64);
    };

    if let Some(runtime_name) = env.imported_functions.get(name) {
        if let Some(core_type) = known_guard_success_type(runtime_name.as_str()) {
            return core_type;
        }
    }
    if let Some(core_type) = known_guard_success_type(name) {
        return core_type;
    }

    llvm_return_type_to_core_type(Some(success_value_type)).unwrap_or(CoreType::Int64)
}

/// Convert parsed AST type annotations into backend core types.
fn ast_type_to_core_type_for_let(ast_type: &Type) -> Result<CoreType, CodegenError> {
    if !is_supported_let_type(ast_type) {
        return Err(CodegenError::new(String::from(
            "unsupported type annotation in let binding",
        )));
    }

    match ast_type_to_core_type(ast_type) {
        Ok(core_type) => Ok(core_type),
        Err(AstTypeMappingError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
            name: type_name,
            type_args: Vec::new(),
        }),
    }
}

/// Return whether an AST type is currently supported for let annotations.
fn is_supported_let_type(ast_type: &Type) -> bool {
    match *ast_type {
        Type::Basic { .. } => true,
        Type::Array {
            ref element_type, ..
        } => is_supported_let_type(element_type),
        Type::Function { .. } | Type::Generic { .. } => false,
    }
}

/// Infer a fallback core type for let initializers without explicit annotations.
fn infer_core_type_from_expr<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expr: &Expr,
) -> CoreType {
    match *expr {
        Expr::Literal { ref value, .. } => match *value {
            crate::ast::LiteralValue::Integer(_) => CoreType::Int64,
            crate::ast::LiteralValue::Float(_) => CoreType::Float64,
            crate::ast::LiteralValue::String(_) => CoreType::String,
            crate::ast::LiteralValue::Boolean(_) => CoreType::Boolean,
            crate::ast::LiteralValue::Void => CoreType::Unit,
        },
        Expr::Array { ref elements, .. } => elements.first().map_or_else(
            || CoreType::Array(alloc::boxed::Box::new(CoreType::Int64)),
            |first| {
                let element_core = infer_core_type_from_expr(codegen_context, env, first);
                CoreType::Array(alloc::boxed::Box::new(element_core))
            },
        ),
        Expr::Call { ref callee, .. } => {
            infer_call_return_type(codegen_context, env, callee).unwrap_or(CoreType::Int64)
        }
        Expr::Propagate { ref call, .. } => infer_core_type_from_expr(codegen_context, env, call),
        Expr::Identifier { ref name, .. } => env
            .variables
            .get(name)
            .map_or(CoreType::Int64, |binding| binding.core_type.clone()),
        Expr::Member {
            ref object,
            ref member,
            ..
        } => {
            let object_type = infer_core_type_from_expr(codegen_context, env, object);
            match object_type {
                CoreType::String | CoreType::Array(_) if member == "length" => CoreType::Int64,
                CoreType::Generic {
                    ref name,
                    ref type_args,
                } if name == "Bytes" && type_args.is_empty() && member == "length" => {
                    CoreType::Int32
                }
                _ => CoreType::Int64,
            }
        }
        _ => CoreType::Int64,
    }
}

/// Infer return core type for call expressions when callee metadata is available.
fn infer_call_return_type<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    callee: &Expr,
) -> Option<CoreType> {
    let Expr::Identifier { ref name, .. } = *callee else {
        return None;
    };

    if let Some(runtime_name) = env.imported_functions.get(name) {
        if let Some(runtime_return_type) = known_runtime_return_type(runtime_name.as_str()) {
            return Some(runtime_return_type);
        }
    }
    if let Some(runtime_return_type) = known_runtime_return_type(name) {
        return Some(runtime_return_type);
    }

    if let Some(function) = codegen_context.module.get_function(name) {
        return llvm_return_type_to_core_type(function.get_type().get_return_type());
    }

    env.imported_functions.get(name).and_then(|runtime_name| {
        codegen_context
            .module
            .get_function(runtime_name.as_str())
            .and_then(|function| {
                llvm_return_type_to_core_type(function.get_type().get_return_type())
            })
    })
}

/// Map known runtime functions to language-level return `CoreType`.
#[expect(
    clippy::too_many_lines,
    reason = "Runtime return mapping is intentionally explicit and grouped by API surface"
)]
fn known_runtime_return_type(name: &str) -> Option<CoreType> {
    match name {
        "take_input"
        | "bytes_to_hex"
        | "path_file_name"
        | "path_file_extension"
        | "read_text_sync"
        | "read_first_line_sync" => Some(CoreType::String),
        "random_int8" => Some(CoreType::Int8),
        "random_int16" => Some(CoreType::Int16),
        "random_int32" | "bytes_length" => Some(CoreType::Int32),
        "random_int64" => Some(CoreType::Int64),
        "random_uint8" => Some(CoreType::UInt8),
        "random_uint16" => Some(CoreType::UInt16),
        "random_uint32" => Some(CoreType::UInt32),
        "random_uint64" => Some(CoreType::UInt64),
        "string_to_int8" => Some(CoreType::Generic {
            name: String::from("ParseResultI8"),
            type_args: Vec::new(),
        }),
        "string_to_int16" => Some(CoreType::Generic {
            name: String::from("ParseResultI16"),
            type_args: Vec::new(),
        }),
        "string_to_int32" => Some(CoreType::Generic {
            name: String::from("ParseResultI32"),
            type_args: Vec::new(),
        }),
        "string_to_int64" => Some(CoreType::Generic {
            name: String::from("ParseResultI64"),
            type_args: Vec::new(),
        }),
        "string_to_uint8" => Some(CoreType::Generic {
            name: String::from("ParseResultU8"),
            type_args: Vec::new(),
        }),
        "string_to_uint16" => Some(CoreType::Generic {
            name: String::from("ParseResultU16"),
            type_args: Vec::new(),
        }),
        "string_to_uint32" => Some(CoreType::Generic {
            name: String::from("ParseResultU32"),
            type_args: Vec::new(),
        }),
        "string_to_uint64" => Some(CoreType::Generic {
            name: String::from("ParseResultU64"),
            type_args: Vec::new(),
        }),
        "string_to_float32" => Some(CoreType::Generic {
            name: String::from("ParseResultF32"),
            type_args: Vec::new(),
        }),
        "string_to_float64" => Some(CoreType::Generic {
            name: String::from("ParseResultF64"),
            type_args: Vec::new(),
        }),
        "bytes_new" | "bytes_concatenate" | "bytes_from_hex" | "bytes_slice" => {
            Some(CoreType::Generic {
                name: String::from("Bytes"),
                type_args: Vec::new(),
            })
        }
        // Path manipulation — infallible path returns
        "path_from"
        | "join_path_components"
        | "path_parent_directory"
        | "normalize_path"
        | "absolute_path_sync" => Some(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }),
        // File reading — bytes returns
        "read_contents_sync" | "read_bytes_at_offset_sync" => Some(CoreType::Generic {
            name: String::from("Bytes"),
            type_args: Vec::new(),
        }),
        // File reading — string array return
        "read_lines_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::String))),
        // Unit-returning filesystem mutators
        "write_contents_sync"
        | "write_text_sync"
        | "write_contents_atomic_sync"
        | "write_text_atomic_sync"
        | "append_contents_sync"
        | "append_text_sync"
        | "write_bytes_at_offset_sync"
        | "create_file_sync"
        | "delete_file_sync"
        | "copy_file_sync"
        | "move_path_sync"
        | "create_directory_sync"
        | "create_directory_recursive_sync"
        | "delete_directory_sync"
        | "delete_directory_recursive_sync" => Some(CoreType::Unit),
        // Boolean-returning path and permission checks
        "path_exists_sync"
        | "is_file_sync"
        | "is_file_nofollow_sync"
        | "is_directory_sync"
        | "is_directory_nofollow_sync" => Some(CoreType::Boolean),
        // File management — FileMetadata returns
        "read_metadata_sync" | "read_metadata_nofollow_sync" => Some(CoreType::Generic {
            name: String::from("FileMetadata"),
            type_args: Vec::new(),
        }),
        // Directory operations — FilesystemPath[] return
        "list_directory_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }))),
        _ => None,
    }
}

/// Map known runtime result wrappers to the success type produced by `guard`.
fn known_guard_success_type(name: &str) -> Option<CoreType> {
    match name {
        "string_to_int8" => Some(CoreType::Int8),
        "string_to_int16" => Some(CoreType::Int16),
        "string_to_int32" => Some(CoreType::Int32),
        "string_to_int64" => Some(CoreType::Int64),
        "string_to_uint8" => Some(CoreType::UInt8),
        "string_to_uint16" => Some(CoreType::UInt16),
        "string_to_uint32" => Some(CoreType::UInt32),
        "string_to_uint64" => Some(CoreType::UInt64),
        "string_to_float32" => Some(CoreType::Float32),
        "string_to_float64" => Some(CoreType::Float64),
        "bytes_from_hex" | "bytes_slice" | "read_contents_sync" | "read_bytes_at_offset_sync" => {
            Some(CoreType::Generic {
                name: String::from("Bytes"),
                type_args: Vec::new(),
            })
        }
        // Filesystem — path returns
        "absolute_path_sync" => Some(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }),
        // Filesystem — string return
        "read_text_sync" | "read_first_line_sync" => Some(CoreType::String),
        // Filesystem — string array return
        "read_lines_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::String))),
        // Filesystem — void returns (writing, file mgmt, dir ops, permissions)
        "write_contents_sync"
        | "write_text_sync"
        | "write_contents_atomic_sync"
        | "write_text_atomic_sync"
        | "append_contents_sync"
        | "append_text_sync"
        | "write_bytes_at_offset_sync"
        | "create_file_sync"
        | "delete_file_sync"
        | "copy_file_sync"
        | "move_path_sync"
        | "create_directory_sync"
        | "create_directory_recursive_sync"
        | "delete_directory_sync"
        | "delete_directory_recursive_sync" => Some(CoreType::Unit),
        // Filesystem — boolean returns
        "path_exists_sync"
        | "is_file_sync"
        | "is_file_nofollow_sync"
        | "is_directory_sync"
        | "is_directory_nofollow_sync" => Some(CoreType::Boolean),
        // Filesystem — FileMetadata returns
        "read_metadata_sync" | "read_metadata_nofollow_sync" => Some(CoreType::Generic {
            name: String::from("FileMetadata"),
            type_args: Vec::new(),
        }),
        // Filesystem — FilesystemPath[] return
        "list_directory_sync" => Some(CoreType::Array(alloc::boxed::Box::new(CoreType::Generic {
            name: String::from("FilesystemPath"),
            type_args: Vec::new(),
        }))),
        _ => None,
    }
}

/// Convert LLVM return type metadata to fallback `CoreType` when possible.
fn llvm_return_type_to_core_type(return_type: Option<BasicTypeEnum<'_>>) -> Option<CoreType> {
    return_type.map(|llvm_type| match llvm_type {
        BasicTypeEnum::IntType(int_type) => match int_type.get_bit_width() {
            1 => CoreType::Boolean,
            8 => CoreType::Int8,
            16 => CoreType::Int16,
            32 => CoreType::Int32,
            _ => CoreType::Int64,
        },
        BasicTypeEnum::FloatType(float_type) => {
            if float_type.get_bit_width() == 32 {
                CoreType::Float32
            } else {
                CoreType::Float64
            }
        }
        BasicTypeEnum::PointerType(_) => CoreType::String,
        BasicTypeEnum::ArrayType(_) => CoreType::Array(alloc::boxed::Box::new(CoreType::Int64)),
        BasicTypeEnum::StructType(_)
        | BasicTypeEnum::VectorType(_)
        | BasicTypeEnum::ScalableVectorType(_) => CoreType::Unit,
    })
}

/// Fetch current LLVM function from builder insertion block.
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
