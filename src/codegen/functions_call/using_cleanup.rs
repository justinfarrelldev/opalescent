//! Using-cleanup helpers for call-expression lowering.

#![allow(
    clippy::missing_docs_in_private_items,
    reason = "internal using-cleanup call lowering helpers"
)]

extern crate alloc;

use super::functions_call_helpers::emit_function_default_return;
use crate::ast::{BorrowKind, Expr};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::scope_tracker::cleanup_return_scopes_preserving_codegen_env_with_error;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::IntPredicate;
use inkwell::values::{FunctionValue, IntValue, PointerValue};

pub(super) fn consume_using_cleanup_obligation_after_success<'context>(
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
) {
    let Some(binding_name) = using_cleanup_close_binding(env, callee, args) else {
        return;
    };
    env.consume_using_cleanup_obligation(binding_name.as_str());
}

pub(super) fn prepare_using_cleanup_success_flag<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
) -> Result<Option<PointerValue<'context>>, CodegenError> {
    let Some(binding_name) = using_cleanup_close_binding(env, callee, args) else {
        return Ok(None);
    };
    let flag = codegen_context.builder.build_alloca(
        codegen_context.context.bool_type(),
        env.next_name("using.cleanup.consumed").as_str(),
    )?;
    codegen_context
        .builder
        .build_store(flag, codegen_context.context.bool_type().const_zero())?;
    env.set_using_cleanup_runtime_consumed_flag(binding_name.as_str(), flag);
    Ok(Some(flag))
}

pub(super) fn mark_using_cleanup_success<'context>(
    codegen_context: &CodegenContext<'context>,
    flag: PointerValue<'context>,
) -> Result<(), CodegenError> {
    codegen_context.builder.build_store(
        flag,
        codegen_context.context.bool_type().const_int(1, false),
    )?;
    Ok(())
}

pub(super) fn mark_using_cleanup_transfer<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    error_ptr: PointerValue<'context>,
    flag: PointerValue<'context>,
    variant: &str,
) -> Result<(), CodegenError> {
    let current_block = codegen_context
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new(String::from("using transfer missing insertion block")))?;
    let current_fn = current_block
        .get_parent()
        .ok_or_else(|| CodegenError::new(String::from("using transfer missing function")))?;
    let transfer_block = codegen_context
        .context
        .append_basic_block(current_fn, env.next_name("using.transfer.mark").as_str());
    let ordinary_block = codegen_context
        .context
        .append_basic_block(current_fn, env.next_name("using.transfer.keep").as_str());
    let continue_block = codegen_context
        .context
        .append_basic_block(current_fn, env.next_name("using.transfer.cont").as_str());
    let is_transfer = build_error_variant_match(codegen_context, env, error_ptr, variant)?;
    codegen_context.builder.build_conditional_branch(
        is_transfer,
        transfer_block,
        ordinary_block,
    )?;
    codegen_context.builder.position_at_end(transfer_block);
    mark_using_cleanup_success(codegen_context, flag)?;
    codegen_context
        .builder
        .build_unconditional_branch(continue_block)?;
    codegen_context.builder.position_at_end(ordinary_block);
    codegen_context
        .builder
        .build_unconditional_branch(continue_block)?;
    codegen_context.builder.position_at_end(continue_block);
    Ok(())
}

pub(super) fn using_cleanup_close_binding<'context>(
    env: &CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
) -> Option<String> {
    let Expr::Identifier { ref name, .. } = *callee else {
        return None;
    };
    let runtime_name = env
        .imported_functions
        .get(name.as_str())
        .map_or_else(|| name.as_str(), String::as_str);
    if runtime_name != "terminal_session_close_sync" {
        return None;
    }
    let Some(Expr::BorrowArgument {
        target,
        borrow_kind: BorrowKind::MutableRef,
        ..
    }) = args.first()
    else {
        return None;
    };
    let Expr::Identifier {
        name: ref binding_name,
        ..
    } = **target
    else {
        return None;
    };
    Some(binding_name.clone())
}

pub(super) fn using_cleanup_close_transfer<'context>(
    env: &CodegenEnv<'context>,
    callee: &Expr,
    args: &[Expr],
) -> Option<(String, String)> {
    using_cleanup_close_binding(env, callee, args).and_then(|binding_name| {
        using_cleanup_transfer_variant(env, binding_name.as_str())
            .map(|variant| (binding_name, variant))
    })
}

pub(super) fn using_cleanup_transfer_variant<'context>(
    env: &CodegenEnv<'context>,
    binding_name: &str,
) -> Option<String> {
    env.using_cleanup_obligations
        .iter()
        .rev()
        .find(|obligation| obligation.binding_name == binding_name)
        .and_then(|obligation| obligation.transfer.as_ref())
        .filter(|transfer| {
            transfer.operation == "terminal_session_close_sync"
                && transfer.error_family == "TerminalSessionRestoreError"
        })
        .map(|transfer| transfer.variant.clone())
}

fn ensure_strcmp_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    codegen_context
        .module
        .get_function("strcmp")
        .unwrap_or_else(|| {
            let i8_ptr = codegen_context
                .context
                .i8_type()
                .ptr_type(AddressSpace::default());
            let fn_type = codegen_context
                .context
                .i32_type()
                .fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
            codegen_context.module.add_function("strcmp", fn_type, None)
        })
}

pub(super) fn build_error_variant_match<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    error_ptr: PointerValue<'context>,
    variant: &str,
) -> Result<IntValue<'context>, CodegenError> {
    let variant_ptr = codegen_context
        .builder
        .build_global_string_ptr(variant, env.next_name("using.transfer.variant").as_str())?
        .as_pointer_value();
    let strcmp_call = codegen_context.builder.build_call(
        ensure_strcmp_function(codegen_context),
        &[error_ptr.into(), variant_ptr.into()],
        env.next_name("using.transfer.strcmp").as_str(),
    )?;
    let strcmp_result = strcmp_call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("strcmp returned void")))?
        .into_int_value();
    Ok(codegen_context.builder.build_int_compare(
        IntPredicate::EQ,
        strcmp_result,
        codegen_context.context.i32_type().const_zero(),
        env.next_name("using.transfer.matches").as_str(),
    )?)
}

pub fn emit_cleanup_aware_error_return<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    current_fn: FunctionValue<'context>,
    body_error: Option<PointerValue<'context>>,
    transferred_names: &[String],
) -> Result<(), CodegenError> {
    let cleanup_error = cleanup_return_scopes_preserving_codegen_env_with_error(
        codegen_context,
        env,
        transferred_names,
    )?;
    if let Some(body_error_ptr) = body_error {
        let current_block = codegen_context
            .builder
            .get_insert_block()
            .ok_or_else(|| CodegenError::new(String::from("missing cleanup return block")))?;
        let cleanup_primary = codegen_context
            .context
            .append_basic_block(current_fn, env.next_name("cleanup.primary").as_str());
        let body_primary = codegen_context
            .context
            .append_basic_block(current_fn, env.next_name("cleanup.body").as_str());
        codegen_context.builder.position_at_end(current_block);
        let cleanup_failed = codegen_context
            .builder
            .build_is_not_null(cleanup_error, env.next_name("cleanup.failed").as_str())?;
        let _branch = codegen_context.builder.build_conditional_branch(
            cleanup_failed,
            cleanup_primary,
            body_primary,
        )?;
        codegen_context.builder.position_at_end(cleanup_primary);
        emit_function_default_return(codegen_context, current_fn, Some(cleanup_error))?;
        codegen_context.builder.position_at_end(body_primary);
        emit_function_default_return(codegen_context, current_fn, Some(body_error_ptr))?;
        return Ok(());
    }
    let cleanup_failed = codegen_context
        .builder
        .build_is_not_null(cleanup_error, env.next_name("cleanup.failed").as_str())?;
    let cleanup_primary = codegen_context
        .context
        .append_basic_block(current_fn, env.next_name("cleanup.primary").as_str());
    let success_cont = codegen_context
        .context
        .append_basic_block(current_fn, env.next_name("cleanup.success").as_str());
    let _branch = codegen_context.builder.build_conditional_branch(
        cleanup_failed,
        cleanup_primary,
        success_cont,
    )?;
    codegen_context.builder.position_at_end(cleanup_primary);
    emit_function_default_return(codegen_context, current_fn, Some(cleanup_error))?;
    codegen_context.builder.position_at_end(success_cont);
    Ok(())
}
