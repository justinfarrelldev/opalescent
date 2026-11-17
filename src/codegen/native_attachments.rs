//! Test-context diagnostics use independent runtime failures, never application payloads.
use super::{
    context::CodegenContext,
    error::CodegenError,
    expressions::{CodegenEnv, codegen_expression},
};
use crate::ast::Expr;
use crate::type_system::types::CoreType;
use inkwell::{AddressSpace, values::BasicValueEnum};

/// Capture context/name/text once, then propagate infrastructure abort before continuing.
pub fn lower<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    arguments: &[Expr],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if arguments.len() != 3 {
        return Err(CodegenError::new(
            "attach_text requires context, name and text",
        ));
    }
    let context = codegen_expression(ctx, env, &arguments[0], None)?;
    let name = super::native_double_owned::capture(ctx, env, &arguments[1], &CoreType::String)?;
    let text = super::native_double_owned::capture(ctx, env, &arguments[2], &CoreType::String)?;
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let helper = super::native_callback_boundary::c_function(
        ctx,
        "opal_test_attach_text",
        ctx.context
            .i64_type()
            .fn_type(&[pointer.into(), pointer.into(), pointer.into()], false),
    )?;
    let identity = ctx
        .builder
        .build_call(
            helper,
            &[context.into(), name.into(), text.into()],
            "attachment.failure",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| {
            CodegenError::new("attachment runtime omitted its independent failure identity")
        })?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)?;
    Ok(ctx.context.i8_type().const_zero().into())
}
