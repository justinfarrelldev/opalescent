//! Pure typed argument rendering over already-owned history/configuration values.
use super::{context::CodegenContext, error::CodegenError, expressions::CodegenEnv};
use crate::ast::testing::Expectation;
use crate::type_system::NativeDoubleSignature;
use inkwell::AddressSpace;
use inkwell::values::{FunctionValue, IntValue, PointerValue};

/// Retry only mismatch before constructing final owned history diagnostics.
pub fn failure<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
    handle: PointerValue<'context>,
    expected: PointerValue<'context>,
    key: &str,
    signature: &NativeDoubleSignature,
) -> Result<IntValue<'context>, CodegenError> {
    super::native_polling::retry_mismatch(ctx, env)?;
    let describe = renderer(ctx, env, key, signature)?
        .as_global_value()
        .as_pointer_value();
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let helper = super::native_callback_boundary::c_function(
        ctx,
        "opal_test_double_history_failure",
        ctx.context.i64_type().fn_type(
            &[
                pointer.into(),
                pointer.into(),
                describe.get_type().into(),
                pointer.into(),
                ctx.context.i64_type().into(),
                ctx.context.i64_type().into(),
            ],
            false,
        ),
    )?;
    let file = ctx
        .builder
        .build_global_string_ptr(&env.current_source_path, "history.file")?
        .as_pointer_value();
    let span = expectation.span;
    Ok(ctx
        .builder
        .build_call(
            helper,
            &[
                handle.into(),
                expected.into(),
                describe.into(),
                file.into(),
                ctx.context
                    .i64_type()
                    .const_int(
                        u64::try_from(span.start.offset)
                            .map_err(|e| CodegenError::new(e.to_string()))?,
                        false,
                    )
                    .into(),
                ctx.context
                    .i64_type()
                    .const_int(
                        u64::try_from(span.end.offset.saturating_sub(span.start.offset))
                            .map_err(|e| CodegenError::new(e.to_string()))?,
                        false,
                    )
                    .into(),
            ],
            "history.failure.identity",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| {
            CodegenError::new("history diagnostic omitted its independent failure identity")
        })?
        .into_int_value())
}

/// Generated callbacks do not evaluate source expressions or invoke application code.
fn renderer<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    key: &str,
    signature: &NativeDoubleSignature,
) -> Result<FunctionValue<'context>, CodegenError> {
    let name = super::native_project::symbol("history_render", &key)?;
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let function = super::native_callback_boundary::c_function(
        ctx,
        &name,
        ctx.context
            .void_type()
            .fn_type(&[pointer.into(), pointer.into()], false),
    )?;
    if function.count_basic_blocks() != 0 {
        return Ok(function);
    }
    let previous = ctx.builder.get_insert_block();
    function.set_linkage(inkwell::module::Linkage::Private);
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(function, "entry"));
    let result = body(ctx, env, function, signature);
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result?;
    Ok(function)
}

/// Logical snapshot layouts are the same for configured expectations and retained calls.
fn body<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    function: FunctionValue<'context>,
    signature: &NativeDoubleSignature,
) -> Result<(), CodegenError> {
    let diagnostic = function
        .get_nth_param(0)
        .ok_or_else(|| CodegenError::new("history renderer omitted diagnostic storage"))?
        .into_pointer_value();
    let arguments = function
        .get_nth_param(1)
        .ok_or_else(|| CodegenError::new("history renderer omitted owned arguments"))?
        .into_pointer_value();
    let layout = super::native_double_rules::layout(ctx, signature)?;
    super::native_assertions::snapshot_text(ctx, diagnostic, "(")?;
    for (index, core) in signature.parameters.iter().enumerate() {
        if index != 0 {
            super::native_assertions::snapshot_text(ctx, diagnostic, ", ")?;
        }
        let value =
            super::native_double_rules::argument(ctx, layout.logical, arguments, index, false)?;
        super::native_assertions::snapshot_value(ctx, env, diagnostic, core, value)?;
    }
    super::native_assertions::snapshot_text(ctx, diagnostic, ")")?;
    ctx.builder.build_return(None)?;
    Ok(())
}
