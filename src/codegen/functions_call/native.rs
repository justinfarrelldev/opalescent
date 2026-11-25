//! Unwrap internal test outcomes before ordinary application result handling.
extern crate alloc;
use super::call_arg_cleanup::{CallArgCleanupDisposition, CallArgCleanupRecord};
use crate::ast::{Expr, Type};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::native_abort::emit_native_abort;
use crate::type_system::types::CoreType;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue};

/// Resolve edge interception and concrete generic calls before ordinary call lowering.
pub(super) fn redirect<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    callee: &Expr,
    generic_args: Option<&[Type]>,
    args: &[Expr],
    expected: Option<&CoreType>,
) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
    let Expr::Identifier { ref name, span, id } = *callee else {
        return Ok(None);
    };
    let (alias, remaining) = if let Some(target) = env.native_double_calls.get(name).cloned() {
        // Compiler-private names contain a dot, preventing source shadowing.
        let alias = env.next_name("__native_double_call");
        if let Some(signature) = env.imported_signatures.get(name).cloned() {
            env.imported_signatures.insert(alias.clone(), signature);
        }
        env.imported_functions.insert(alias.clone(), target);
        (alias, generic_args)
    } else if let Some(alias) =
        crate::codegen::native_generics::call_alias(ctx, env, name, generic_args, args, expected)?
    {
        (alias, None)
    } else {
        return Ok(None);
    };
    super::codegen_call_expression(
        ctx,
        env,
        &Expr::Identifier {
            name: alias,
            span,
            id,
        },
        remaining,
        args,
        expected,
    )
    .map(Some)
}

/// Native reference arguments preserve lvalue addresses; ordinary owned values
/// use the existing expression/temporary ownership lowering.
pub(super) fn lower_native_argument<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    callee: &Expr,
    index: usize,
    argument: &Expr,
    expected: Option<&CoreType>,
    cleanup_records: &mut Vec<CallArgCleanupRecord>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if let Expr::BorrowArgument { ref target, .. } = *argument {
        if let Expr::Identifier { ref name, .. } = **target {
            return env
                .variables
                .get(name)
                .map(|binding| binding.alloca.as_basic_value_enum())
                .ok_or_else(|| {
                    CodegenError::new("native borrow target has no original value storage")
                });
        }
        return crate::codegen::adts::codegen_product_field_address(codegen, env, target)
            .map(|address| address.as_basic_value_enum());
    }
    super::call_arg_cleanup::lower_call_argument(
        codegen,
        env,
        callee,
        index,
        argument,
        expected,
        cleanup_records,
    )
}

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
