//! Unwrap internal test outcomes before ordinary application result handling.
extern crate alloc;
use super::call_arg_cleanup::{CallArgCleanupDisposition, CallArgCleanupRecord};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::scope_tracker::cleanup_return_scopes_preserving_codegen_env_with_error;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};

/// Existing source ownership scopes exit on the abort edge. This is not an
/// external-resource registry: only already-declared lexical cleanups execute.
pub(super) fn lower_native_call<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    function: FunctionValue<'context>,
    arguments: &[BasicMetadataValueEnum<'context>],
    cleanup_records: &[CallArgCleanupRecord],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let callee = codegen
        .native_function_outcome(function)
        .ok_or_else(|| CodegenError::new("native call has no declared outcome ABI"))?;
    let current = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("native call has no caller"))?;
    let caller = codegen.native_function_outcome(current).ok_or_else(|| {
        CodegenError::new(
            "native test abort cannot propagate through an uninstrumented source call boundary",
        )
    })?;
    let result = codegen
        .builder
        .build_call(function, arguments, &env.next_name("native.call"))?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("native call did not produce its declared outcome"))?
        .into_struct_value();
    let branch = callee.branch_after_call(codegen, result)?;
    codegen.builder.position_at_end(branch.aborted);
    let transferred: Vec<_> = cleanup_records
        .iter()
        .filter(|record| record.disposition == CallArgCleanupDisposition::Transferred)
        .map(|record| record.binding_name.clone())
        .collect();
    let has_cleanup_errors = env
        .using_cleanup_obligations
        .iter()
        .any(|obligation| !obligation.consumed);
    let cleanup_error =
        cleanup_return_scopes_preserving_codegen_env_with_error(codegen, env, &transferred)?;
    if has_cleanup_errors {
        record_secondary_cleanup_error(codegen, branch.failure_identity, cleanup_error)?;
    }
    let aborted = caller.build_aborted(codegen, branch.failure_identity)?;
    codegen.builder.build_return(Some(&aborted))?;
    codegen.builder.position_at_end(branch.completed);
    Ok(callee
        .completed_application_value(codegen, result)?
        .unwrap_or_else(|| {
            codegen
                .context
                .struct_type(&[], false)
                .get_undef()
                .as_basic_value_enum()
        }))
}

/// Keep a declared lexical cleanup error secondary to the original test failure.
fn record_secondary_cleanup_error<'context>(
    codegen: &CodegenContext<'context>,
    failure: inkwell::values::IntValue<'context>,
    error: inkwell::values::PointerValue<'context>,
) -> Result<(), CodegenError> {
    let current = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("cleanup error has no caller"))?;
    let failed = codegen
        .context
        .append_basic_block(current, "test.cleanup.failed");
    let continuation = codegen
        .context
        .append_basic_block(current, "test.cleanup.continue");
    let has_error = codegen
        .builder
        .build_is_not_null(error, "test.cleanup.has_error")?;
    codegen
        .builder
        .build_conditional_branch(has_error, failed, continuation)?;
    codegen.builder.position_at_end(failed);
    let signature = codegen.context.void_type().fn_type(
        &[codegen.context.i64_type().into(), error.get_type().into()],
        false,
    );
    let name = "opal_test_record_secondary_application_error";
    let record = codegen
        .module
        .get_function(name)
        .unwrap_or_else(|| codegen.module.add_function(name, signature, None));
    codegen
        .builder
        .build_call(record, &[failure.into(), error.into()], "")?;
    codegen.builder.build_unconditional_branch(continuation)?;
    codegen.builder.position_at_end(continuation);
    Ok(())
}
