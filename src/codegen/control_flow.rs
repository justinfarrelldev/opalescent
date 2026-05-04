#![allow(
    clippy::all,
    clippy::missing_docs_in_private_items,
    clippy::pattern_type_mismatch,
    clippy::panic,
    reason = "internal codegen implementation module"
)]
extern crate alloc;

use crate::ast::{Expr, LabeledValue, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::error_abi::{
    build_error_aggregate, build_success_aggregate, build_void_error_aggregate,
    build_void_success_aggregate, intern_variant_name,
};
use crate::codegen::expressions::{CodegenEnv, LoopContext, VariableBinding, codegen_expression};
use crate::codegen::statements::codegen_statement;
use crate::type_system::types::CoreType;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::IntPredicate;
use inkwell::types::StructType;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, PointerValue};

#[doc = "Lower if statement control-flow blocks."]
pub fn codegen_if_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    condition: &Expr,
    then_branch: &Stmt,
    else_branch: Option<&Stmt>,
) -> Result<(), CodegenError> {
    let condition_value = codegen_expression(codegen_context, env, condition, None)?;
    let condition_int = condition_value.into_int_value();
    let function = current_function(codegen_context)?;
    let then_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("if.then").as_str());
    let else_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("if.else").as_str());
    let merge_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("if.merge").as_str());
    let _cond_br =
        codegen_context
            .builder
            .build_conditional_branch(condition_int, then_block, else_block)?;

    codegen_context.builder.position_at_end(then_block);
    codegen_statement(codegen_context, env, then_branch)?;
    if let Some(current_block) = codegen_context.builder.get_insert_block() {
        if current_block.get_terminator().is_none() {
            let _jump_merge = codegen_context
                .builder
                .build_unconditional_branch(merge_block)?;
        }
    }

    codegen_context.builder.position_at_end(else_block);
    if let Some(stmt) = else_branch {
        codegen_statement(codegen_context, env, stmt)?;
    }
    if let Some(current_block) = codegen_context.builder.get_insert_block() {
        if current_block.get_terminator().is_none() {
            let _jump_merge = codegen_context
                .builder
                .build_unconditional_branch(merge_block)?;
        }
    }

    codegen_context.builder.position_at_end(merge_block);
    Ok(())
}

#[doc = "Lower if expression with phi merge value."]
pub fn codegen_if_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    condition: &Expr,
    then_branch: &Stmt,
    else_branch: Option<&Stmt>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let condition_value = codegen_expression(codegen_context, env, condition, None)?;
    let condition_int = condition_value.into_int_value();
    let function = current_function(codegen_context)?;
    let then_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("ifexpr.then").as_str());
    let else_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("ifexpr.else").as_str());
    let merge_block = codegen_context
        .context
        .append_basic_block(function, env.next_name("ifexpr.merge").as_str());

    let _cond_br =
        codegen_context
            .builder
            .build_conditional_branch(condition_int, then_block, else_block)?;

    codegen_context.builder.position_at_end(then_block);
    let then_value = statement_to_value(codegen_context, env, then_branch)?;
    let then_end = codegen_context
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new(String::from("if expression then block missing")))?;
    if then_end.get_terminator().is_none() {
        let _jump_merge = codegen_context
            .builder
            .build_unconditional_branch(merge_block)?;
    }

    codegen_context.builder.position_at_end(else_block);
    let else_value = if let Some(stmt) = else_branch {
        statement_to_value(codegen_context, env, stmt)?
    } else {
        codegen_context
            .context
            .struct_type(&[], false)
            .const_zero()
            .as_basic_value_enum()
    };
    let else_end = codegen_context
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new(String::from("if expression else block missing")))?;
    if else_end.get_terminator().is_none() {
        let _jump_merge = codegen_context
            .builder
            .build_unconditional_branch(merge_block)?;
    }

    codegen_context.builder.position_at_end(merge_block);
    if then_value.get_type() == else_value.get_type() {
        let phi = codegen_context
            .builder
            .build_phi(then_value.get_type(), env.next_name("ifexpr.phi").as_str())?;
        phi.add_incoming(&[(&then_value, then_end), (&else_value, else_end)]);
        Ok(phi.as_basic_value())
    } else {
        Ok(then_value)
    }
}

#[doc = "Lower while/for/loop control flow."]
#[expect(
    clippy::too_many_lines,
    reason = "Loop lowering handles while/loop/for variants in one control-flow builder"
)]
pub fn codegen_loop_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    stmt: &Stmt,
) -> Result<(), CodegenError> {
    match *stmt {
        Stmt::While {
            ref condition,
            ref body,
            ..
        } => {
            let function = current_function(codegen_context)?;
            let header = codegen_context
                .context
                .append_basic_block(function, env.next_name("while.header").as_str());
            let loop_body = codegen_context
                .context
                .append_basic_block(function, env.next_name("while.body").as_str());
            let exit = codegen_context
                .context
                .append_basic_block(function, env.next_name("while.exit").as_str());
            let _jump_header = codegen_context.builder.build_unconditional_branch(header)?;
            codegen_context.builder.position_at_end(header);
            let condition_value = codegen_expression(codegen_context, env, condition, None)?;
            let _cond = codegen_context.builder.build_conditional_branch(
                condition_value.into_int_value(),
                loop_body,
                exit,
            )?;
            codegen_context.builder.position_at_end(loop_body);
            emit_loop_body_with_targets(
                codegen_context,
                env,
                body.as_ref(),
                header,
                exit,
                &[],
                &[],
            )?;
            if let Some(current_block) = codegen_context.builder.get_insert_block() {
                if current_block.get_terminator().is_none() {
                    let _back = codegen_context.builder.build_unconditional_branch(header)?;
                }
            }
            codegen_context.builder.position_at_end(exit);
            Ok(())
        }
        Stmt::Loop { ref body, .. } => {
            codegen_loop_expression_into_slots(codegen_context, env, body.as_ref(), &[], &[])
        }
        Stmt::For {
            ref variable,
            ref iterable,
            ref body,
            ..
        } => {
            let (iterable_ptr, iterable_length, element_core_type) = match *iterable {
                Expr::Identifier { ref name, .. } => {
                    let Some(binding) = env.variables.get(name).cloned() else {
                        return Err(CodegenError::new(format!(
                            "unknown array variable '{name}' in for loop"
                        )));
                    };

                    let &CoreType::Array(ref element_core_type) = &binding.core_type else {
                        return Err(CodegenError::new(format!(
                            "for loop iterable '{name}' is not an array"
                        )));
                    };

                    let loaded_iterable = codegen_context
                        .builder
                        .build_load(binding.alloca, env.next_name("for.iterable.ptr").as_str())?;
                    let array_ptr = if loaded_iterable.is_pointer_value() {
                        loaded_iterable.into_pointer_value()
                    } else {
                        // SAFETY: `binding.alloca` points to a stack-allocated array aggregate.
                        // The [0, 0] GEP computes the pointer to the first element.
                        unsafe {
                            codegen_context.builder.build_in_bounds_gep(
                                binding.alloca,
                                &[
                                    codegen_context.context.i32_type().const_zero(),
                                    codegen_context.context.i32_type().const_zero(),
                                ],
                                env.next_name("for.iterable.base").as_str(),
                            )?
                        }
                    };

                    let array_length = if let Some(length) = binding.length {
                        codegen_context
                            .context
                            .i64_type()
                            .const_int(u64::from(length), false)
                    } else {
                        let len_binding_name = format!("{name}_len");
                        let Some(length_binding) = env.variables.get(len_binding_name.as_str())
                        else {
                            return Err(CodegenError::new(format!(
                                "array length binding '{len_binding_name}' missing for for loop iterable '{name}'"
                            )));
                        };
                        codegen_context
                            .builder
                            .build_load(length_binding.alloca, len_binding_name.as_str())?
                            .into_int_value()
                    };

                    (array_ptr, array_length, element_core_type.as_ref().clone())
                }
                Expr::Array { ref elements, .. } => {
                    let element_core_type =
                        elements
                            .first()
                            .map_or(CoreType::Int64, |first| match *first {
                                Expr::Literal {
                                    value: crate::ast::LiteralValue::Float(_),
                                    ..
                                } => CoreType::Float64,
                                Expr::Literal {
                                    value: crate::ast::LiteralValue::String(_),
                                    ..
                                } => CoreType::String,
                                Expr::Literal {
                                    value: crate::ast::LiteralValue::Boolean(_),
                                    ..
                                } => CoreType::Boolean,
                                _ => CoreType::Int64,
                            });
                    let iterable_expected_type =
                        CoreType::Array(Box::new(element_core_type.clone()));
                    let iterable_value = codegen_expression(
                        codegen_context,
                        env,
                        iterable,
                        Some(&iterable_expected_type),
                    )?;
                    (
                        iterable_value.into_pointer_value(),
                        codegen_context.context.i64_type().const_int(
                            u64::try_from(elements.len()).map_err(|conversion_error| {
                                CodegenError::new(format!(
                                    "for loop iterable length conversion failed: {conversion_error}"
                                ))
                            })?,
                            false,
                        ),
                        element_core_type,
                    )
                }
                _ => {
                    return Err(CodegenError::new(String::from(
                        "for loop iterable must be an array variable or array literal",
                    )));
                }
            };

            let index_alloca = codegen_context.builder.build_alloca(
                codegen_context.context.i64_type(),
                env.next_name("for.index").as_str(),
            )?;
            let _index_init = codegen_context.builder.build_store(
                index_alloca,
                codegen_context.context.i64_type().const_zero(),
            )?;

            let function = current_function(codegen_context)?;
            let header = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.header").as_str());
            let loop_body = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.body").as_str());
            let increment = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.increment").as_str());
            let exit = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.exit").as_str());
            let _jump_header = codegen_context.builder.build_unconditional_branch(header)?;
            codegen_context.builder.position_at_end(header);

            let current_index = codegen_context
                .builder
                .build_load(index_alloca, env.next_name("for.index.load").as_str())?
                .into_int_value();
            let in_bounds = codegen_context.builder.build_int_compare(
                IntPredicate::ULT,
                current_index,
                iterable_length,
                env.next_name("for.bounds").as_str(),
            )?;
            let _branch = codegen_context
                .builder
                .build_conditional_branch(in_bounds, loop_body, exit)?;

            codegen_context.builder.position_at_end(loop_body);

            // SAFETY: iterable_ptr points to contiguous array elements and current_index is
            // guarded by `current_index < iterable_length` in loop header.
            let element_ptr = unsafe {
                codegen_context.builder.build_in_bounds_gep(
                    iterable_ptr,
                    &[current_index],
                    env.next_name("for.element.ptr").as_str(),
                )?
            };
            let element_value = codegen_context
                .builder
                .build_load(element_ptr, env.next_name("for.element").as_str())?;
            let iteration_alloca = codegen_context
                .builder
                .build_alloca(element_value.get_type(), variable.as_str())?;
            let _store_iteration_value = codegen_context
                .builder
                .build_store(iteration_alloca, element_value)?;

            let previous_binding = env.variables.insert(
                variable.clone(),
                VariableBinding {
                    alloca: iteration_alloca,
                    core_type: element_core_type,
                    length: None,
                    is_mutable: false,
                },
            );

            emit_loop_body_with_targets(
                codegen_context,
                env,
                body.as_ref(),
                increment,
                exit,
                &[],
                &[],
            )?;

            if let Some(previous) = previous_binding {
                env.variables.insert(variable.clone(), previous);
            } else {
                let _removed = env.variables.remove(variable);
            }

            if let Some(current_block) = codegen_context.builder.get_insert_block() {
                if current_block.get_terminator().is_none() {
                    let _to_increment = codegen_context
                        .builder
                        .build_unconditional_branch(increment)?;
                }
            }

            codegen_context.builder.position_at_end(increment);
            let index_before_increment = codegen_context
                .builder
                .build_load(index_alloca, env.next_name("for.index.reload").as_str())?
                .into_int_value();
            let index_after_increment = codegen_context.builder.build_int_add(
                index_before_increment,
                codegen_context.context.i64_type().const_int(1, false),
                env.next_name("for.index.next").as_str(),
            )?;
            let _store_next_index = codegen_context
                .builder
                .build_store(index_alloca, index_after_increment)?;
            let _back_to_header = codegen_context.builder.build_unconditional_branch(header)?;

            codegen_context.builder.position_at_end(exit);
            Ok(())
        }
        _ => Err(CodegenError::new(String::from("expected loop statement"))),
    }
}

#[doc = "Lower `loop =>` expression body and optionally store break payloads into slots."]
pub fn codegen_loop_expression_into_slots<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    body: &Stmt,
    break_slots: &[PointerValue<'context>],
    break_labels: &[alloc::string::String],
) -> Result<(), CodegenError> {
    let function = current_function(codegen_context)?;
    let loop_header = codegen_context
        .context
        .append_basic_block(function, env.next_name("loop.header").as_str());
    let loop_body = codegen_context
        .context
        .append_basic_block(function, env.next_name("loop.body").as_str());
    let exit = codegen_context
        .context
        .append_basic_block(function, env.next_name("loop.exit").as_str());

    let _jump_header = codegen_context
        .builder
        .build_unconditional_branch(loop_header)?;
    codegen_context.builder.position_at_end(loop_header);
    let _jump_body = codegen_context
        .builder
        .build_unconditional_branch(loop_body)?;

    codegen_context.builder.position_at_end(loop_body);
    emit_loop_body_with_targets(
        codegen_context,
        env,
        body,
        loop_header,
        exit,
        break_slots,
        break_labels,
    )?;
    if let Some(current_block) = codegen_context.builder.get_insert_block() {
        if current_block.get_terminator().is_none() {
            let _back = codegen_context
                .builder
                .build_unconditional_branch(loop_header)?;
        }
    }

    codegen_context.builder.position_at_end(exit);
    Ok(())
}

#[doc = "Lower return statement including aggregate multi-return values."]
pub fn codegen_return_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
) -> Result<(), CodegenError> {
    if let Some(error_return_type) = current_error_return_type(codegen_context)? {
        return codegen_error_aware_return_statement(
            codegen_context,
            env,
            values,
            error_return_type,
        );
    }

    if values.is_empty() {
        let _ret = codegen_context.builder.build_return(None)?;
        return Ok(());
    }
    if values.len() == 1 {
        let value = codegen_expression(codegen_context, env, &values[0].value, None)?;
        if value.is_struct_value() && value.into_struct_value().get_type().count_fields() == 0 {
            let _ret = codegen_context.builder.build_return(None)?;
            return Ok(());
        }
        let _ret = codegen_context.builder.build_return(Some(&value))?;
        return Ok(());
    }
    let lowered = values
        .iter()
        .map(|value| codegen_expression(codegen_context, env, &value.value, None))
        .collect::<Result<Vec<_>, _>>()?;
    let aggregate_type = codegen_context.context.struct_type(
        lowered
            .iter()
            .map(BasicValueEnum::get_type)
            .collect::<Vec<_>>()
            .as_slice(),
        false,
    );
    let mut aggregate = aggregate_type.get_undef();
    for (index, value) in lowered.iter().enumerate() {
        aggregate = codegen_context
            .builder
            .build_insert_value(
                aggregate,
                *value,
                u32::try_from(index)
                    .map_err(|conversion_error| CodegenError::new(format!("{conversion_error}")))?,
                env.next_name("ret.insert").as_str(),
            )?
            .into_struct_value();
    }
    let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
    Ok(())
}

fn codegen_error_aware_return_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
    error_return_type: StructType<'context>,
) -> Result<(), CodegenError> {
    if values.is_empty() {
        let aggregate = build_void_success_aggregate(codegen_context)?;
        let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
        return Ok(());
    }

    if values.len() != 1 {
        return Err(CodegenError::new(String::from(
            "errors-bearing functions returning multiple values are not yet supported",
        )));
    }

    let labeled_value = &values[0];
    if labeled_value.label == "err" {
        let variant_name = extract_error_variant_name(&labeled_value.value)?;
        let error_ptr = intern_variant_name(codegen_context, env, variant_name.as_str());
        let success_field_type = error_return_type
            .get_field_types()
            .first()
            .copied()
            .ok_or_else(|| {
                CodegenError::new(String::from("error ABI return type missing success field"))
            })?;
        let aggregate = if success_field_type.is_pointer_type() {
            build_void_error_aggregate(codegen_context, error_ptr)?
        } else {
            build_error_aggregate(codegen_context, success_field_type, error_ptr)?
        };
        let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
        return Ok(());
    }

    let value = codegen_expression(codegen_context, env, &labeled_value.value, None)?;
    if value.is_struct_value() && value.into_struct_value().get_type().count_fields() == 0 {
        let aggregate = build_void_success_aggregate(codegen_context)?;
        let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
        return Ok(());
    }

    let aggregate = build_success_aggregate(codegen_context, value)?;
    let _ret = codegen_context.builder.build_return(Some(&aggregate))?;
    Ok(())
}

fn current_error_return_type<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<Option<StructType<'context>>, CodegenError> {
    let function = current_function(codegen_context)?;
    let Some(return_type) = function.get_type().get_return_type() else {
        return Ok(None);
    };
    if !return_type.is_struct_type() {
        return Ok(None);
    }

    let struct_type = return_type.into_struct_type();
    if is_error_abi_return_type(struct_type) {
        Ok(Some(struct_type))
    } else {
        Ok(None)
    }
}

fn is_error_abi_return_type<'context>(struct_type: StructType<'context>) -> bool {
    if struct_type.count_fields() != 2 {
        return false;
    }

    let field_types = struct_type.get_field_types();
    if !field_types[1].is_pointer_type() {
        return false;
    }

    let error_pointee = field_types[1].into_pointer_type().get_element_type();
    error_pointee.is_int_type() && error_pointee.into_int_type().get_bit_width() == 8
}

fn extract_error_variant_name(expr: &Expr) -> Result<String, CodegenError> {
    match expr {
        Expr::Identifier { name, .. } => Ok(name.clone()),
        Expr::Member { member, .. } => Ok(member.clone()),
        Expr::Constructor { fields, .. } if !fields.is_empty() => {
            Err(CodegenError::new(String::from(
                "payload-bearing error variants not yet supported in user-defined functions",
            )))
        }
        Expr::Constructor { callee, .. } => extract_error_variant_name(callee.as_ref()),
        _ => Err(CodegenError::new(String::from(
            "error returns must use `return err: VariantName`",
        ))),
    }
}

#[doc = "Extract value form from statement for if-expression lowering."]
fn statement_to_value<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    stmt: &Stmt,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match *stmt {
        Stmt::Expression { ref expr, .. } => codegen_expression(codegen_context, env, expr, None),
        Stmt::Return { ref values, .. } => {
            if values.len() == 1 {
                codegen_expression(codegen_context, env, &values[0].value, None)
            } else {
                Ok(codegen_context
                    .context
                    .struct_type(&[], false)
                    .const_zero()
                    .as_basic_value_enum())
            }
        }
        _ => {
            codegen_statement(codegen_context, env, stmt)?;
            Ok(codegen_context
                .context
                .struct_type(&[], false)
                .const_zero()
                .as_basic_value_enum())
        }
    }
}

#[doc = "Emit loop body with explicit break/continue targets."]
fn emit_loop_body_with_targets<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    stmt: &Stmt,
    continue_target: inkwell::basic_block::BasicBlock<'context>,
    break_target: inkwell::basic_block::BasicBlock<'context>,
    break_slots: &[PointerValue<'context>],
    break_labels: &[alloc::string::String],
) -> Result<(), CodegenError> {
    env.push_loop(LoopContext {
        continue_target,
        break_target,
        break_slots: break_slots.to_vec(),
        break_labels: break_labels.to_vec(),
    });

    let result = codegen_statement(codegen_context, env, stmt);
    let popped_loop = env.pop_loop();
    debug_assert!(
        popped_loop.is_some(),
        "emit_loop_body_with_targets should pop the loop context it pushed"
    );
    if popped_loop.is_none() {
        return Err(CodegenError::new(String::from(
            "loop context stack underflow in emit_loop_body_with_targets",
        )));
    }

    result
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

#[cfg(test)]
mod tests {
    use super::codegen_return_statement;
    use crate::ast::{ConstructorField, Expr, LabeledValue, LiteralValue};
    use crate::codegen::context::CodegenContext;
    use crate::codegen::error::CodegenError;
    use crate::codegen::error_abi::build_error_return_type;
    use crate::codegen::expressions::CodegenEnv;
    use crate::token::{Position, Span};
    use inkwell::context::Context;

    fn test_span() -> Span {
        Span::single(Position::new(1, 1, 0))
    }

    fn labeled_value(label: &str, value: Expr) -> LabeledValue {
        LabeledValue {
            label: label.to_owned(),
            value,
            span: test_span(),
            id: crate::ast::NodeId(1),
        }
    }

    fn ident(name: &str) -> Expr {
        Expr::Identifier {
            name: name.to_owned(),
            span: test_span(),
            id: crate::ast::NodeId(2),
        }
    }

    fn string_lit(value: &str) -> Expr {
        Expr::Literal {
            value: LiteralValue::String(value.to_owned()),
            span: test_span(),
            id: crate::ast::NodeId(3),
        }
    }

    fn create_error_return_function<'context>(
        codegen_context: &CodegenContext<'context>,
        function_name: &str,
    ) {
        let function_type =
            build_error_return_type(codegen_context.context, None).fn_type(&[], false);
        let function = codegen_context
            .module
            .add_function(function_name, function_type, None);
        let entry = codegen_context
            .context
            .append_basic_block(function, "entry");
        codegen_context.builder.position_at_end(entry);
    }

    #[test]
    fn return_statement_builds_void_error_abi_success_and_error_returns() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "control_flow_return_error_abi");
        let mut env = CodegenEnv::new(true);

        create_error_return_function(&codegen_context, "void_success");
        let success_result = codegen_return_statement(&codegen_context, &mut env, &[]);
        assert!(
            success_result.is_ok(),
            "void errors return without expr should lower to success aggregate"
        );

        create_error_return_function(&codegen_context, "void_error");
        let error_result = codegen_return_statement(
            &codegen_context,
            &mut env,
            &[labeled_value("err", ident("NotFound"))],
        );
        assert!(
            error_result.is_ok(),
            "return err: NotFound should lower to error aggregate"
        );

        let ir = codegen_context.module.print_to_string().to_string();
        assert!(
            ir.contains("ret { i8*, i8* }") || ir.contains("ret {i8*, i8*}"),
            "error-aware return lowering should emit aggregate returns: {ir}"
        );
        assert!(
            ir.contains("NotFound\\00") || ir.contains("c\"NotFound\\00\""),
            "error-aware return lowering should intern the variant name: {ir}"
        );
    }

    #[test]
    fn return_statement_rejects_payload_bearing_error_variants() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "control_flow_return_payload_error");
        let mut env = CodegenEnv::new(true);
        create_error_return_function(&codegen_context, "payload_error");

        let payload_error = codegen_return_statement(
            &codegen_context,
            &mut env,
            &[labeled_value(
                "err",
                Expr::Constructor {
                    callee: Box::new(Expr::Member {
                        object: Box::new(ident("AppError")),
                        member: String::from("NotFound"),
                        span: test_span(),
                        id: crate::ast::NodeId(4),
                    }),
                    fields: vec![ConstructorField {
                        name: String::from("reason"),
                        value: string_lit("x"),
                        span: test_span(),
                    }],
                    span: test_span(),
                    id: crate::ast::NodeId(5),
                },
            )],
        );

        match payload_error {
            Err(CodegenError { message, .. }) => assert!(
                message.contains(
                    "payload-bearing error variants not yet supported in user-defined functions"
                ),
                "unexpected payload-bearing error message: {message}"
            ),
            Ok(()) => panic!("payload-bearing error return should fail"),
        }
    }
}
