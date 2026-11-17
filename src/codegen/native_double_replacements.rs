//! Statically resolved replacement functions retain explicit native outcome metadata.
use super::native_callback_boundary::c_function;
use super::{
    context::CodegenContext,
    error::CodegenError,
    expressions::{CodegenEnv, codegen_expression},
};
use crate::ast::Expr;
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::values::{FunctionValue, PointerValue};
use inkwell::{AddressSpace, types::BasicType};

/// Configure a typed function fallback without requiring argument-history support.
pub fn configure<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    arguments: Option<&[Expr]>,
    replacement: &Expr,
) -> Result<(), CodegenError> {
    if arguments.is_some() {
        return Err(CodegenError::new(
            "replacement handlers configure an explicit fallback, not an exact rule",
        ));
    }
    let (key, signature) = super::native_double_configuration::signature(env, target)?;
    let plan = super::native_double_rules::layout(ctx, &signature)?;
    let handle = codegen_expression(ctx, env, target, None)?;
    let function = resolve(ctx, env, &key, &signature, replacement)?;
    let release_name = super::native_project::symbol("replacement_drop", &key)?;
    let release = super::native_double_rules::free_callback(ctx, &release_name)?
        .as_global_value()
        .as_pointer_value();
    let payload = super::native_double_configuration::allocate(ctx, env, plan.payload)?;
    let typed = ctx.builder.build_pointer_cast(
        payload,
        plan.payload.ptr_type(AddressSpace::default()),
        "replacement.payload.typed",
    )?;
    let index = usize::try_from(plan.replacement_index()?)
        .map_err(|error| CodegenError::new(error.to_string()))?;
    super::native_double_configuration::store(ctx, typed, index, function.into())?;
    let pointer = payload.get_type();
    let fallback = c_function(
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
    let identity = ctx
        .builder
        .build_call(
            fallback,
            &[
                handle.into(),
                ctx.context.i32_type().const_int(4, false).into(),
                payload.into(),
                release.into(),
            ],
            "replacement.configuration.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| {
            CodegenError::new("replacement configuration omitted its independent outcome")
        })?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)
}

/// Resolve only known source functions; unknown callback ABI paths never silently pass.
fn resolve<'context>(
    ctx: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    key: &str,
    signature: &NativeDoubleSignature,
    replacement: &Expr,
) -> Result<PointerValue<'context>, CodegenError> {
    if let Expr::Parenthesized { ref expr, .. } = *replacement {
        return resolve(ctx, env, key, signature, expr);
    }
    let Expr::Identifier { ref name, .. } = *replacement else {
        return Err(CodegenError::new(
            "replacement requires a statically resolved native source function; indirect and foreign callback ABIs are unsupported",
        ));
    };
    let symbol = env
        .imported_functions
        .get(name)
        .map_or(name.as_str(), String::as_str);
    let source = ctx.module.get_function(symbol).ok_or_else(|| {
        CodegenError::new("replacement source function is not declared in this artifact")
    })?;
    if ctx.native_function_outcome(source).is_none() || source.get_call_conventions() != 0 {
        return Err(CodegenError::new(
            "replacement requires explicit native outcome metadata and C calling convention",
        ));
    }
    let plan = super::native_double_rules::layout(ctx, signature)?;
    if source.as_global_value().as_pointer_value().get_type() == plan.replacement {
        return Ok(source.as_global_value().as_pointer_value());
    }
    let Some(&CoreType::Function {
        ref error_types, ..
    }) = env.imported_signatures.get(name)
    else {
        return Err(CodegenError::new(
            "replacement has no authoritative checked application signature",
        ));
    };
    if !error_types.is_empty() || signature.error_types.is_empty() {
        return Err(CodegenError::new(
            "replacement application ABI adaptation requires a nonfallible source and a declared fallible target",
        ));
    }
    adapter(ctx, key, source, signature)
}

/// Nonfallible success may enter a fallible target without inventing new errors.
fn adapter<'context>(
    ctx: &CodegenContext<'context>,
    key: &str,
    source: FunctionValue<'context>,
    signature: &NativeDoubleSignature,
) -> Result<PointerValue<'context>, CodegenError> {
    let plan = super::native_double_rules::layout(ctx, signature)?;
    let parameters = plan
        .physical
        .get_field_types()
        .into_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    if source.get_type().get_param_types() != parameters {
        return Err(CodegenError::new(
            "replacement parameter ABI does not match the checked reference modes",
        ));
    }
    let name = super::native_project::symbol(
        "replacement_adapter",
        &(key, source.get_name().to_string_lossy().as_ref()),
    )?;
    let original = plan
        .application
        .ok_or_else(|| {
            CodegenError::new("fallible replacement target omitted its application ABI")
        })?
        .fn_type(&parameters, false);
    let function =
        ctx.declare_native_test_function(&name, original, Some(inkwell::module::Linkage::Private))?;
    if function.count_basic_blocks() != 0 {
        return Ok(function.as_global_value().as_pointer_value());
    }
    let previous = ctx.builder.get_insert_block();
    let result = adapter_body(ctx, source, function);
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result?;
    Ok(function.as_global_value().as_pointer_value())
}

/// Abort branches are handled before extracting any application success payload.
fn adapter_body<'context>(
    ctx: &CodegenContext<'context>,
    source: FunctionValue<'context>,
    function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(function, "entry"));
    let arguments = function
        .get_param_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    let result = ctx
        .builder
        .build_call(source, &arguments, "replacement.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("replacement omitted its explicit native outcome"))?
        .into_struct_value();
    let metadata = ctx
        .native_function_outcome(source)
        .ok_or_else(|| CodegenError::new("replacement lost native outcome metadata"))?;
    let branches = metadata.branch_after_call(ctx, result)?;
    ctx.builder.position_at_end(branches.aborted);
    super::native_abort::emit_native_abort(
        ctx,
        &mut CodegenEnv::new(true),
        branches.failure_identity,
        &[],
    )?;
    ctx.builder.position_at_end(branches.completed);
    let application = if let Some(value) = metadata.completed_application_value(ctx, result)? {
        super::error_abi::build_success_aggregate(ctx, value)?
    } else {
        super::error_abi::build_void_success_aggregate(ctx)?
    };
    ctx.emit_application_return(Some(application.into()))?;
    Ok(())
}

/// Invoke a precompiled target-native function pointer and forward its whole outcome.
pub fn invoke<'context>(
    ctx: &CodegenContext<'context>,
    function: FunctionValue<'context>,
    selection: PointerValue<'context>,
    signature: &NativeDoubleSignature,
) -> Result<(), CodegenError> {
    let plan = super::native_double_rules::layout(ctx, signature)?;
    let payload_slot = ctx
        .builder
        .build_struct_gep(selection, 1, "replacement.payload.slot")?;
    let payload = ctx
        .builder
        .build_load(payload_slot, "replacement.payload")?
        .into_pointer_value();
    let payload = ctx.builder.build_pointer_cast(
        payload,
        plan.payload.ptr_type(AddressSpace::default()),
        "replacement.payload.typed",
    )?;
    let function_slot = ctx.builder.build_struct_gep(
        payload,
        plan.replacement_index()?,
        "replacement.function.slot",
    )?;
    let replacement = ctx
        .builder
        .build_load(function_slot, "replacement.function")?
        .into_pointer_value();
    let callable = inkwell::values::CallableValue::try_from(replacement).map_err(|()| {
        CodegenError::new("replacement storage is not a callable target-native function pointer")
    })?;
    let arguments = function
        .get_param_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    let result = ctx
        .builder
        .build_call(callable, &arguments, "replacement.native.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("replacement invocation omitted its native outcome"))?;
    ctx.builder.build_return(Some(&result))?;
    Ok(())
}
