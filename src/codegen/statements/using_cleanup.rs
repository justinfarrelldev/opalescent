//! `using` statement lowering through existing lexical scope cleanup.

extern crate alloc;

use crate::ast::{BorrowKind, Expr, LetBinding, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::{format, string::String};

/// Lower `using` as a lexical scope-bound acquisition.
pub(super) fn codegen_using_statement<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    binding: &LetBinding,
    acquisition: &Expr,
    body: &Stmt,
) -> Result<(), CodegenError> {
    let _using_scope_depth = env.enter_scope();
    super::codegen_let_statement(codegen_context, env, binding, Some(acquisition))?;
    register_using_cleanup_obligation(env, binding)?;
    super::codegen_statement(codegen_context, env, body)?;
    if let Some(current_block) = codegen_context.builder.get_insert_block() {
        if current_block.get_terminator().is_none() {
            crate::codegen::scope_tracker::cleanup_scopes_to_depth_with_malloc_string_release(
                codegen_context,
                env,
                env.current_scope_depth().saturating_sub(1),
                &[],
            )?;
        } else {
            super::unwind_scope_without_cleanup(env);
        }
    } else {
        super::unwind_scope_without_cleanup(env);
    }
    Ok(())
}

pub(super) fn prepare_using_cleanup_success_flag<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
) -> Result<Option<inkwell::values::PointerValue<'context>>, CodegenError> {
    let Some(binding_name) = using_cleanup_close_binding(env, expression) else {
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
    flag: inkwell::values::PointerValue<'context>,
) -> Result<(), CodegenError> {
    codegen_context.builder.build_store(
        flag,
        codegen_context.context.bool_type().const_int(1, false),
    )?;
    Ok(())
}

fn using_cleanup_close_binding<'context>(
    env: &CodegenEnv<'context>,
    expression: &Expr,
) -> Option<String> {
    let Expr::Call {
        ref callee,
        ref args,
        ..
    } = *expression
    else {
        return None;
    };
    let Expr::Identifier { ref name, .. } = **callee else {
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

fn register_using_cleanup_obligation<'context>(
    env: &mut CodegenEnv<'context>,
    binding: &LetBinding,
) -> Result<(), CodegenError> {
    let Some(binding_info) = env.variables.get(binding.name.as_str()) else {
        return Err(CodegenError::new(format!(
            "using binding '{}' was not registered before cleanup obligation setup",
            binding.name
        )));
    };
    let CoreType::Generic { name, type_args } = &binding_info.core_type else {
        return Err(CodegenError::new(format!(
            "using binding '{}' must lower to a direct cleanup-registered resource type",
            binding.name
        )));
    };
    if !type_args.is_empty() {
        return Err(CodegenError::new(format!(
            "using binding '{}' must lower to a direct cleanup-registered resource type",
            binding.name
        )));
    }
    let Some((cleanup_operation, cleanup_errors, transfer)) =
        cleanup_registration_for_type(name.as_str())
    else {
        return Err(CodegenError::new(format!(
            "using binding '{}' has no compiler-visible cleanup registration for {name}",
            binding.name
        )));
    };
    env.register_using_cleanup_obligation(
        binding.name.as_str(),
        cleanup_operation,
        cleanup_errors,
        transfer,
    );
    Ok(())
}

fn cleanup_registration_for_type(
    resource_type: &str,
) -> Option<(
    &'static str,
    &'static [&'static str],
    Option<(&'static str, &'static str, &'static str)>,
)> {
    match resource_type {
        "SystemWaitSet" => Some(("system_wait_set_drop", &[], None)),
        "SystemOwnedWaitRegistration" => Some(("system_owned_wait_registration_drop", &[], None)),
        "ProcessControlSource" => Some(("process_control_source_drop", &[], None)),
        "MonotonicTimer" => Some(("monotonic_timer_drop", &[], None)),
        "CancellationSource" => Some(("cancellation_source_drop", &[], None)),
        "TerminalSession" => Some((
            "terminal_session_close_sync",
            &["TerminalSessionRestoreError"],
            Some((
                "terminal_session_close_sync",
                "TerminalSessionRestoreError",
                "CloseRestorePending",
            )),
        )),
        "TerminalChordRouter" => Some(("terminal_chord_router_drop", &[], None)),
        "TerminalTestScenario" => Some(("terminal_test_scenario_drop", &[], None)),
        "TerminalTestBackendActivation" => {
            Some(("terminal_test_backend_activation_drop", &[], None))
        }
        _ => None,
    }
}
