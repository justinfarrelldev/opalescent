//! Unwrap internal test outcomes before ordinary application result handling.
extern crate alloc;
use super::call_arg_cleanup::{CallArgCleanupDisposition, CallArgCleanupRecord};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::native_abort::emit_native_abort;
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
    if codegen.native_function_outcome(current).is_none() {
        return Err(CodegenError::new(
            "native test abort cannot propagate through an uninstrumented source call boundary",
        ));
    }
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
    emit_native_abort(codegen, env, branch.failure_identity, &transferred)?;
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
