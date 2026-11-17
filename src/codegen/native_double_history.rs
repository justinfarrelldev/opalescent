//! Typed owned argument-history assertions, independent of call-count assertions.
use super::native_assertion_storage::entry_slot;
use super::native_callback_boundary::c_function;
use super::{
    context::CodegenContext,
    error::CodegenError,
    expressions::{CodegenEnv, codegen_expression},
};
use crate::ast::testing::{Expectation, Matcher};
use inkwell::{AddressSpace, IntPredicate};

/// Expected arguments are evaluated once; C history contains copied values only.
pub fn lower<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    let Matcher::CalledWith { ref arguments } = expectation.matcher else {
        return Err(CodegenError::new("expected typed history matcher"));
    };
    let (key, signature) = super::native_double_configuration::signature(env, &expectation.actual)?;
    let helpers = super::native_double_rules::callbacks(ctx, env, &key, &signature)?;
    let layout = super::native_double_rules::layout(ctx, &signature)?;
    let handle = codegen_expression(ctx, env, &expectation.actual, None)?;
    let values =
        super::native_double_configuration::evaluate_arguments(ctx, env, &signature, arguments)?;
    let storage = entry_slot(ctx, layout.logical.into(), "double.history.expected")?;
    for (index, value) in values.into_iter().enumerate() {
        super::native_double_configuration::store(ctx, storage, index, value)?;
    }
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let expected = ctx
        .builder
        .build_pointer_cast(storage, pointer, "double.history.values")?;
    let found = entry_slot(
        ctx,
        ctx.context.i32_type().into(),
        "double.history.found.output",
    )?;
    let matcher = helpers.overlap.as_global_value().as_pointer_value();
    let helper = c_function(
        ctx,
        "opal_test_double_history_matches",
        ctx.context.i64_type().fn_type(
            &[
                pointer.into(),
                pointer.into(),
                matcher.get_type().into(),
                found.get_type().into(),
            ],
            false,
        ),
    )?;
    let identity = ctx
        .builder
        .build_call(
            helper,
            &[handle.into(), expected.into(), matcher.into(), found.into()],
            "double.history.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("history inspection omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)?;
    let value = ctx
        .builder
        .build_load(found, "double.history.found")?
        .into_int_value();
    let passed = ctx.builder.build_int_compare(
        IntPredicate::NE,
        value,
        value.get_type().const_zero(),
        "double.history.matches",
    )?;
    let function = ctx
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("history assertion has no native caller"))?;
    let failed = ctx
        .context
        .append_basic_block(function, "double.history.failed");
    let ready = ctx
        .context
        .append_basic_block(function, "double.history.ready");
    ctx.builder
        .build_conditional_branch(passed, ready, failed)?;
    ctx.builder.position_at_end(failed);
    let mismatch = super::native_assertions::record_history_failure(ctx, env, expectation)?;
    super::native_abort::emit_native_abort(ctx, env, mismatch, &[])?;
    ctx.builder.position_at_end(ready);
    Ok(())
}
