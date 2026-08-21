//! `using` statement lowering through existing lexical scope cleanup.

extern crate alloc;

use crate::ast::{Expr, LetBinding, Stmt};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::format;

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
    let Some((cleanup_operation, cleanup_errors)) = cleanup_registration_for_type(name.as_str())
    else {
        return Err(CodegenError::new(format!(
            "using binding '{}' has no compiler-visible cleanup registration for {name}",
            binding.name
        )));
    };
    env.register_using_cleanup_obligation(binding.name.as_str(), cleanup_operation, cleanup_errors);
    Ok(())
}

fn cleanup_registration_for_type(
    resource_type: &str,
) -> Option<(&'static str, &'static [&'static str])> {
    match resource_type {
        "SystemWaitSet" => Some(("system_wait_set_drop", &[])),
        "SystemOwnedWaitRegistration" => Some(("system_owned_wait_registration_drop", &[])),
        "ProcessControlSource" => Some(("process_control_source_drop", &[])),
        "MonotonicTimer" => Some(("monotonic_timer_drop", &[])),
        "CancellationSource" => Some(("cancellation_source_drop", &[])),
        "TerminalSession" => Some((
            "terminal_session_close_sync",
            &["TerminalSessionRestoreError"],
        )),
        "TerminalChordRouter" => Some(("terminal_chord_router_drop", &[])),
        "TerminalTestScenario" => Some(("terminal_test_scenario_drop", &[])),
        "TerminalTestBackendActivation" => Some(("terminal_test_backend_activation_drop", &[])),
        _ => None,
    }
}
