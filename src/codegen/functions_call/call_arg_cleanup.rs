extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, VariableBinding, codegen_expression};
use crate::codegen::scope_tracker::{
    cleanup_scopes_to_depth_with_malloc_string_release, expr_requires_malloc_string_cleanup,
    mark_binding_malloc_string_cleanup,
};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::BasicValueEnum;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CallArgCleanupDisposition {
    Borrowed,
    CleanupByCaller,
    Transferred,
}

#[derive(Debug)]
pub(super) struct CallArgCleanupRecord {
    pub(super) binding_name: String,
    pub(super) disposition: CallArgCleanupDisposition,
}

fn direct_call_arg_requires_malloc_string_cleanup<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    argument: &Expr,
) -> bool {
    match *argument {
        Expr::Identifier { .. } => false,
        Expr::StringInterpolation { .. } => true,
        Expr::Parenthesized { ref expr, .. } => {
            direct_call_arg_requires_malloc_string_cleanup(codegen_context, env, expr.as_ref())
        }
        Expr::Propagate { ref call, .. } => {
            direct_call_arg_requires_malloc_string_cleanup(codegen_context, env, call.as_ref())
        }
        _ => expr_requires_malloc_string_cleanup(codegen_context, env, argument, &BTreeMap::new()),
    }
}

fn call_argument_takes_owned_value(env: &CodegenEnv<'_>, callee: &Expr, arg_index: usize) -> bool {
    let &Expr::Identifier { ref name, .. } = callee else {
        return false;
    };
    let runtime_name = env
        .imported_functions
        .get(name.as_str())
        .map_or(name.as_str(), String::as_str);

    match (runtime_name, arg_index) {
        _ => false,
    }
}

fn call_arg_cleanup_disposition<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    callee: &Expr,
    argument: &Expr,
    arg_index: usize,
) -> CallArgCleanupDisposition {
    if !direct_call_arg_requires_malloc_string_cleanup(codegen_context, env, argument) {
        return CallArgCleanupDisposition::Borrowed;
    }
    if call_argument_takes_owned_value(env, callee, arg_index) {
        return CallArgCleanupDisposition::Transferred;
    }
    CallArgCleanupDisposition::CleanupByCaller
}

fn register_call_arg_cleanup_binding<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: BasicValueEnum<'context>,
) -> Result<String, CodegenError> {
    if !value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "call-argument cleanup requires pointer value",
        )));
    }

    let binding_name = env.next_name("call.arg.cleanup");
    let alloca = codegen_context
        .builder
        .build_alloca(value.get_type(), binding_name.as_str())?;
    codegen_context.builder.build_store(alloca, value)?;
    env.variables.insert(
        binding_name.clone(),
        VariableBinding {
            alloca,
            core_type: CoreType::String,
            length: None,
            capacity: None,
            is_mutable: false,
        },
    );
    env.register_scope_binding(binding_name.as_str());
    mark_binding_malloc_string_cleanup(env, binding_name.as_str());
    Ok(binding_name)
}

pub(super) fn lower_call_argument<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    arg_index: usize,
    argument: &Expr,
    cleanup_records: &mut Vec<CallArgCleanupRecord>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let lowered = codegen_expression(codegen_context, env, argument, None)?;
    let disposition = call_arg_cleanup_disposition(codegen_context, env, callee, argument, arg_index);
    if disposition != CallArgCleanupDisposition::Borrowed {
        let binding_name = register_call_arg_cleanup_binding(codegen_context, env, lowered)?;
        cleanup_records.push(CallArgCleanupRecord {
            binding_name,
            disposition,
        });
    }
    Ok(lowered)
}

pub(super) fn cleanup_call_argument_temporaries<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target_depth: usize,
    cleanup_records: &[CallArgCleanupRecord],
) -> Result<(), CodegenError> {
    let transferred_names = cleanup_records
        .iter()
        .filter(|record| record.disposition == CallArgCleanupDisposition::Transferred)
        .map(|record| record.binding_name.clone())
        .collect::<Vec<_>>();
    cleanup_scopes_to_depth_with_malloc_string_release(
        codegen_context,
        env,
        target_depth,
        transferred_names.as_slice(),
    )?;
    for binding_name in transferred_names {
        let _removed_binding = env.variables.remove(binding_name.as_str());
        let _removed_indices = env.variable_field_indices.remove(binding_name.as_str());
        let _removed_aliases = env.variable_field_aliases.remove(binding_name.as_str());
    }
    Ok(())
}
