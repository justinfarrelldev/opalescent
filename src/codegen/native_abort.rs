//! Shared abort exits for assertions and instrumented source calls.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::CodegenEnv;
use super::scope_tracker::cleanup_return_scopes_preserving_codegen_env_with_error;
use inkwell::values::{IntValue, PointerValue};

/// Exit only existing lexical scopes, then preserve the originating test abort.
/// No external-resource registry, discovery or automatic World cleanup is added.
///
/// # Errors
/// Rejects an uninstrumented source boundary or invalid LLVM scope emission.
pub fn emit_native_abort<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    failure: IntValue<'context>,
    transferred: &[String],
) -> Result<(), CodegenError> {
    let current = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("native abort has no enclosing function"))?;
    let outcome = codegen.native_function_outcome(current).ok_or_else(|| {
        CodegenError::new(
            "native test abort cannot propagate through an uninstrumented source call boundary",
        )
    })?;
    let has_cleanup_errors = env
        .using_cleanup_obligations
        .iter()
        .any(|obligation| !obligation.consumed);
    let cleanup_error =
        cleanup_return_scopes_preserving_codegen_env_with_error(codegen, env, transferred)?;
    if has_cleanup_errors {
        record_secondary_cleanup_error(codegen, failure, cleanup_error)?;
    }
    let aborted = outcome.build_aborted(codegen, failure)?;
    codegen.builder.build_return(Some(&aborted))?;
    Ok(())
}

/// Keep a declared lexical cleanup error secondary to the original test failure.
fn record_secondary_cleanup_error<'context>(
    codegen: &CodegenContext<'context>,
    failure: IntValue<'context>,
    error: PointerValue<'context>,
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
