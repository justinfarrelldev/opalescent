extern crate alloc;

use crate::ast::{Expr, LabeledValue, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{codegen_expression, CodegenEnv, CodegenError};
use crate::codegen::statements::codegen_statement;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
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
            ref iterable,
            ref body,
            ..
        } => {
            let _iterable_value = codegen_expression(codegen_context, env, iterable, None)?;
            let function = current_function(codegen_context)?;
            let header = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.header").as_str());
            let loop_body = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.body").as_str());
            let exit = codegen_context
                .context
                .append_basic_block(function, env.next_name("for.exit").as_str());
            let _jump_header = codegen_context.builder.build_unconditional_branch(header)?;
            codegen_context.builder.position_at_end(header);
            let _jump_body = codegen_context
                .builder
                .build_unconditional_branch(loop_body)?;
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
    match *stmt {
        Stmt::Block { ref statements, .. } => {
            for statement in statements {
                match *statement {
                    Stmt::Break { .. } => {
                        if let Stmt::Break { ref values, .. } = *statement {
                            store_break_values_into_slots(
                                codegen_context,
                                env,
                                values.as_slice(),
                                break_slots,
                                break_labels,
                            )?;
                        }
                        let _br = codegen_context
                            .builder
                            .build_unconditional_branch(break_target)?;
                    }
                    Stmt::Continue { .. } => {
                        let _br = codegen_context
                            .builder
                            .build_unconditional_branch(continue_target)?;
                    }
                    _ => codegen_statement(codegen_context, env, statement)?,
                }
            }
            Ok(())
        }
        Stmt::Break { .. } => {
            if let Stmt::Break { ref values, .. } = *stmt {
                store_break_values_into_slots(
                    codegen_context,
                    env,
                    values.as_slice(),
                    break_slots,
                    break_labels,
                )?;
            }
            let _br = codegen_context
                .builder
                .build_unconditional_branch(break_target)?;
            Ok(())
        }
        Stmt::Continue { .. } => {
            let _br = codegen_context
                .builder
                .build_unconditional_branch(continue_target)?;
            Ok(())
        }
        _ => codegen_statement(codegen_context, env, stmt),
    }
}

#[doc = "Store loop-break payload values into pre-allocated binding slots."]
fn store_break_values_into_slots<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    values: &[LabeledValue],
    break_slots: &[PointerValue<'context>],
    break_labels: &[alloc::string::String],
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
