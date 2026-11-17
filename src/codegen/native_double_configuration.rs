//! Compile evaluated exact/fallback rules into fixed typed owned payloads.
use super::native_callback_boundary::c_function;
use super::native_double_rules::{self, RuleLayout};
use super::{
    context::CodegenContext,
    error::CodegenError,
    expressions::{CodegenEnv, codegen_expression},
};
use crate::ast::{
    AstNode, Expr,
    testing::{SourceReference, StubBehavior},
};
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::types::StructType;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

/// A target's checked nominal signature is authoritative for all generated operations.
pub fn signature(
    env: &CodegenEnv<'_>,
    target: &Expr,
) -> Result<(String, NativeDoubleSignature), CodegenError> {
    let core = super::native_assertions::operand_core_type(env, target)
        .ok_or_else(|| CodegenError::new("double operand has no checked nominal type"))?;
    let CoreType::Generic { name, .. } = core else {
        return Err(CodegenError::new("double operand is not nominal"));
    };
    let signature = env
        .native_double_signatures
        .get(&name)
        .cloned()
        .ok_or_else(|| CodegenError::new("double operand has no compiled application signature"))?;
    Ok((name, signature))
}

/// Evaluate target, exact arguments and result once in original source order.
pub fn configure<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    arguments: Option<&[Expr]>,
    behavior: &StubBehavior,
) -> Result<(), CodegenError> {
    if let StubBehavior::Replacement(ref replacement) = *behavior {
        return super::native_double_replacements::configure(
            ctx,
            env,
            target,
            arguments,
            replacement,
        );
    }
    if arguments.is_none() {
        return Err(CodegenError::new(
            "stored value/error rules require an explicit exact argument list",
        ));
    }
    let (key, signature) = signature(env, target)?;
    if matches!(*behavior, StubBehavior::Return(_))
        && !signature
            .return_types
            .iter()
            .all(|core| *core == CoreType::Unit || native_double_rules::supported(core))
    {
        return Err(CodegenError::new(
            "stored double results require supported concrete value lowering",
        ));
    }
    let callbacks = native_double_rules::callbacks(ctx, env, &key, &signature)?;
    let plan = native_double_rules::layout(ctx, &signature)?;
    let _scope = env.enter_scope();
    let handle = codegen_expression(ctx, env, target, None)?;
    let values = if let Some(arguments) = arguments {
        evaluate_arguments(ctx, env, &signature, arguments)?
    } else {
        Vec::new()
    };
    let application = application(ctx, env, &signature, plan, behavior)?;
    let payload = allocate(ctx, env, plan.payload)?;
    let typed = ctx.builder.build_pointer_cast(
        payload,
        plan.payload.ptr_type(AddressSpace::default()),
        "double.rule.typed",
    )?;
    for (index, value) in values.into_iter().enumerate() {
        super::binding_store::retain_new_binding_value_if_needed(
            ctx,
            &signature.parameters[index],
            value,
        )?;
        store(ctx, typed, index, value)?;
    }
    if let Some(value) = application {
        super::native_double_owned::retain_application(ctx, &signature, value)?;
        store(ctx, typed, signature.parameters.len(), value)?;
    }
    let kind = match *behavior {
        StubBehavior::Return(_) => 1_u64,
        StubBehavior::ApplicationError(_) => 2_u64,
        _ => {
            return Err(CodegenError::new(
                "double behavior requires a compiled replacement adapter",
            ));
        }
    };
    register(
        ctx,
        env,
        handle,
        payload,
        callbacks,
        Registration {
            kind,
            exact: arguments.is_some(),
            span: target.span(),
        },
    )?;
    env.exit_scope_cleanup(ctx, &[])
}

/// Static source provenance and dispatch kind, not runtime ownership tracking.
#[derive(Clone, Copy)]
struct Registration {
    /// Existing runtime behavior discriminant.
    kind: u64,
    /// Exact rules require matching/overlap checks; fallbacks do not.
    exact: bool,
    /// Original target source extent for unused/duplicate configuration diagnostics.
    span: crate::token::Span,
}

/// Runtime registration consumes the prepared payload on success and rejection.
fn register<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    handle: BasicValueEnum<'context>,
    payload: PointerValue<'context>,
    callbacks: native_double_rules::RuleCallbacks<'context>,
    registration: Registration,
) -> Result<(), CodegenError> {
    let Registration { kind, exact, span } = registration;
    let pointer = payload.get_type();
    let release = callbacks.release.as_global_value().as_pointer_value();
    let status = if exact {
        let matcher = callbacks.matches.as_global_value().as_pointer_value();
        let overlap = callbacks.overlap.as_global_value().as_pointer_value();
        let file = ctx
            .builder
            .build_global_string_ptr(&env.current_source_path, "double.rule.file")?
            .as_pointer_value();
        let helper = c_function(
            ctx,
            "opal_test_double_add_exact",
            ctx.context.i64_type().fn_type(
                &[
                    pointer.into(),
                    ctx.context.i32_type().into(),
                    pointer.into(),
                    matcher.get_type().into(),
                    overlap.get_type().into(),
                    release.get_type().into(),
                    pointer.into(),
                    ctx.context.i64_type().into(),
                    ctx.context.i64_type().into(),
                ],
                false,
            ),
        )?;
        ctx.builder.build_call(
            helper,
            &[
                handle.into(),
                ctx.context.i32_type().const_int(kind, false).into(),
                payload.into(),
                matcher.into(),
                overlap.into(),
                release.into(),
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
            "double.rule.status",
        )?
    } else {
        let helper = c_function(
            ctx,
            "opal_test_double_fallback",
            ctx.context.i64_type().fn_type(
                &[
                    pointer.into(),
                    ctx.context.i32_type().into(),
                    pointer.into(),
                    release.get_type().into(),
                ],
                false,
            ),
        )?;
        ctx.builder.build_call(
            helper,
            &[
                handle.into(),
                ctx.context.i32_type().const_int(kind, false).into(),
                payload.into(),
                release.into(),
            ],
            "double.fallback.status",
        )?
    };
    let identity = status
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double configuration omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)
}

/// Borrow syntax configures copied logical values, not escaping source addresses.
pub fn evaluate_arguments<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    signature: &NativeDoubleSignature,
    arguments: &[Expr],
) -> Result<Vec<BasicValueEnum<'context>>, CodegenError> {
    if arguments.len() != signature.parameters.len() {
        return Err(CodegenError::new(
            "double rule argument count does not match its checked signature",
        ));
    }
    arguments
        .iter()
        .zip(&signature.parameters)
        .map(|(argument, core)| {
            let value = if let Expr::BorrowArgument { ref target, .. } = *argument {
                target.as_ref()
            } else {
                argument
            };
            super::native_double_owned::capture(ctx, env, value, core)
        })
        .collect()
}

/// Store using the exact compiler-known payload field layout.
pub fn store<'context>(
    ctx: &CodegenContext<'context>,
    data: PointerValue<'context>,
    index: usize,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    let slot = ctx.builder.build_struct_gep(
        data,
        u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
        "double.rule.slot",
    )?;
    ctx.builder.build_store(slot, value)?;
    Ok(())
}

/// Reserve ordinary framework value memory; allocation failure has no created payload.
pub fn allocate<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    layout: StructType<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let malloc = c_function(
        ctx,
        "malloc",
        pointer.fn_type(&[ctx.context.i64_type().into()], false),
    )?;
    let size = layout
        .size_of()
        .ok_or_else(|| CodegenError::new("rule payload is unsized"))?;
    let empty = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        size,
        size.get_type().const_zero(),
        "double.rule.empty",
    )?;
    let size = ctx.builder.build_select(
        empty,
        size.get_type().const_int(1, false),
        size,
        "double.rule.size",
    )?;
    let data = ctx
        .builder
        .build_call(malloc, &[size.into()], "double.rule.storage")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("rule allocation omitted its result"))?
        .into_pointer_value();
    let function = ctx
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("rule allocation has no native caller"))?;
    let failed = ctx
        .context
        .append_basic_block(function, "double.rule.allocation.failed");
    let ready = ctx
        .context
        .append_basic_block(function, "double.rule.allocation.ready");
    let missing = ctx.builder.build_is_null(data, "double.rule.missing")?;
    ctx.builder
        .build_conditional_branch(missing, failed, ready)?;
    ctx.builder.position_at_end(failed);
    let identity = infrastructure(ctx, "could not allocate typed double rule payload")?;
    super::native_abort::emit_native_abort(ctx, env, identity, &[])?;
    ctx.builder.position_at_end(ready);
    Ok(data)
}

/// Infrastructure failures stay independent of allowed application error sets.
pub fn infrastructure<'context>(
    ctx: &CodegenContext<'context>,
    message: &str,
) -> Result<IntValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let helper = c_function(
        ctx,
        "opal_test_record_failure",
        ctx.context.i64_type().fn_type(
            &[
                ctx.context.i32_type().into(),
                pointer.into(),
                pointer.into(),
                ctx.context.i64_type().into(),
                ctx.context.i64_type().into(),
            ],
            false,
        ),
    )?;
    let message = ctx
        .builder
        .build_global_string_ptr(message, "double.infrastructure.message")?
        .as_pointer_value();
    let file = ctx
        .builder
        .build_global_string_ptr("", "double.infrastructure.file")?
        .as_pointer_value();
    Ok(ctx
        .builder
        .build_call(
            helper,
            &[
                ctx.context.i32_type().const_int(5, false).into(),
                message.into(),
                file.into(),
                ctx.context.i64_type().const_zero().into(),
                ctx.context.i64_type().const_zero().into(),
            ],
            "double.infrastructure.identity",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("infrastructure failure omitted its identity"))?
        .into_int_value())
}

/// Build the unchanged ordinary application ABI, including deliberate nominal errors.
fn application<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    signature: &NativeDoubleSignature,
    plan: RuleLayout<'context>,
    behavior: &StubBehavior,
) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
    match *behavior {
        StubBehavior::Return(ref expression) => {
            let void =
                signature.return_types.is_empty() || signature.return_types == [CoreType::Unit];
            let hint = if signature.return_types.len() == 1 {
                signature.return_types.first()
            } else {
                None
            };
            let value = if !void && signature.return_types.len() == 1 {
                super::native_double_owned::capture(
                    ctx,
                    env,
                    expression,
                    &signature.return_types[0],
                )?
            } else {
                let value = codegen_expression(ctx, env, expression, hint)?;
                if !void {
                    for (index, core) in signature.return_types.iter().enumerate() {
                        let field = ctx.builder.build_extract_value(
                            value.into_struct_value(),
                            u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
                            "double.return.field",
                        )?;
                        super::native_double_owned::hold(ctx, env, field, core, false)?;
                    }
                }
                value
            };
            if signature.error_types.is_empty() {
                Ok((!void).then_some(value))
            } else if void {
                Ok(Some(
                    super::error_abi::build_void_success_aggregate(ctx)?.into(),
                ))
            } else {
                Ok(Some(
                    super::error_abi::build_success_aggregate(ctx, value)?.into(),
                ))
            }
        }
        StubBehavior::ApplicationError(ref reference) => {
            let name = error_name(env, reference);
            let error = super::error_abi::intern_variant_name(ctx, env, &name)?;
            let application = plan
                .application
                .ok_or_else(|| {
                    CodegenError::new(
                        "configured error has no ordinary application error representation",
                    )
                })?
                .into_struct_type();
            Ok(Some(
                super::error_abi::build_error_aggregate_for_return_type(ctx, application, error)?
                    .into(),
            ))
        }
        _ => Err(CodegenError::new(
            "unsupported stored double result behavior",
        )),
    }
}

/// Imported error aliases retain the genuine canonical nominal family identity.
fn error_name(env: &CodegenEnv<'_>, reference: &SourceReference) -> String {
    let Some((variant, family)) = reference.path.split_last() else {
        return String::new();
    };
    if family.is_empty() {
        env.adt_layout_key(variant).to_owned()
    } else {
        format!("{}.{variant}", env.adt_layout_key(&family.join(".")))
    }
}
