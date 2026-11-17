//! Rule/history ownership reuses ordinary lexical captures and RC value operations.
use super::{
    context::CodegenContext,
    error::CodegenError,
    expressions::{CodegenEnv, VariableBinding, codegen_expression},
};
use crate::ast::Expr;
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::AddressSpace;
use inkwell::values::{BasicValueEnum, FunctionValue};

/// Capture a source operand before any subsequent expression can overwrite its slot.
pub fn capture<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
    core: &CoreType,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    capture_binding(ctx, env, expression, core).map(|(_, value)| value)
}

/// A private binding lets polling reuse a captured operand without reevaluating source.
pub fn capture_binding<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
    core: &CoreType,
) -> Result<(String, BasicValueEnum<'context>), CodegenError> {
    let value = codegen_expression(ctx, env, expression, Some(core))?;
    if super::native_path_values::is_path(core) {
        let copy = super::native_path_values::copy(ctx, value)?;
        let function = ctx
            .builder
            .get_insert_block()
            .and_then(inkwell::basic_block::BasicBlock::get_parent)
            .ok_or_else(|| CodegenError::new("path capture has no native caller"))?;
        let failed = ctx
            .context
            .append_basic_block(function, "path.snapshot.failed");
        let ready = ctx
            .context
            .append_basic_block(function, "path.snapshot.ready");
        let missing = ctx.builder.build_is_null(copy, "path.snapshot.missing")?;
        ctx.builder
            .build_conditional_branch(missing, failed, ready)?;
        ctx.builder.position_at_end(failed);
        let identity = super::native_double_configuration::infrastructure(
            ctx,
            "could not allocate owned path snapshot",
        )?;
        super::native_abort::emit_native_abort(ctx, env, identity, &[])?;
        ctx.builder.position_at_end(ready);
        return hold_binding(ctx, env, copy.into(), &CoreType::String, false);
    }
    hold_binding(ctx, env, value, core, borrowed(expression, core))
}

/// Hold one already-evaluated value under the existing lexical cleanup machinery.
pub fn hold<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: BasicValueEnum<'context>,
    core: &CoreType,
    retain: bool,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    hold_binding(ctx, env, value, core, retain).map(|(_, value)| value)
}

/// Register a capture in the existing lexical scope, with its true storage ownership.
fn hold_binding<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: BasicValueEnum<'context>,
    core: &CoreType,
    retain: bool,
) -> Result<(String, BasicValueEnum<'context>), CodegenError> {
    let name = env.next_name("__native_double_value");
    let slot = super::native_assertion_storage::entry_slot(ctx, value.get_type(), &name)?;
    env.variables.insert(
        name.clone(),
        VariableBinding {
            alloca: slot,
            core_type: core.clone(),
            length: None,
            capacity: None,
            is_mutable: false,
        },
    );
    env.register_scope_binding(&name);
    super::binding_store::initialize_binding_value(
        ctx,
        env,
        &name,
        value,
        "capture double value",
        retain,
    )?;
    Ok((name, value))
}

/// String indexing produces fresh values; array projections are ordinarily borrowed.
fn borrowed(expression: &Expr, core: &CoreType) -> bool {
    if *core == CoreType::String {
        return super::string_ownership::string_expr_needs_retain_at_ownership_boundary(expression);
    }
    match *expression {
        Expr::Identifier { .. } | Expr::Member { .. } | Expr::Index { .. } => true,
        Expr::Parenthesized { ref expr, .. }
        | Expr::BorrowArgument {
            target: ref expr, ..
        }
        | Expr::Constrain {
            value: ref expr, ..
        } => borrowed(expr, core),
        _ => false,
    }
}

/// Retain exactly the existing RC-bearing success values; void has no value to retain.
pub fn retain_application<'context>(
    ctx: &CodegenContext<'context>,
    signature: &NativeDoubleSignature,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    for (index, core) in signature.return_types.iter().enumerate() {
        if *core == CoreType::Unit {
            continue;
        }
        let field = if signature.error_types.is_empty() && signature.return_types.len() == 1 {
            value
        } else {
            ctx.builder.build_extract_value(
                value.into_struct_value(),
                u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
                "double.application.value",
            )?
        };
        super::binding_store::retain_new_binding_value_if_needed(ctx, core, field)?;
    }
    Ok(())
}

/// Generate ordinary value-memory release callbacks, not user cleanup/destructors.
pub fn drop_callback<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
    signature: &NativeDoubleSignature,
    payload: bool,
) -> Result<FunctionValue<'context>, CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let function = super::native_callback_boundary::c_function(
        ctx,
        name,
        ctx.context.void_type().fn_type(&[pointer.into()], false),
    )?;
    if function.count_basic_blocks() != 0 {
        return Ok(function);
    }
    let previous = ctx.builder.get_insert_block();
    function.set_linkage(inkwell::module::Linkage::Private);
    ctx.builder
        .position_at_end(ctx.context.append_basic_block(function, "entry"));
    let result = drop_body(ctx, function, signature, payload);
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result?;
    Ok(function)
}

/// A snapshot contains only logical arguments; a rule additionally owns application values.
fn drop_body<'context>(
    ctx: &CodegenContext<'context>,
    function: FunctionValue<'context>,
    signature: &NativeDoubleSignature,
    payload: bool,
) -> Result<(), CodegenError> {
    let raw = function
        .get_first_param()
        .ok_or_else(|| CodegenError::new("double value release omitted its storage"))?
        .into_pointer_value();
    let plan = super::native_double_rules::layout(ctx, signature)?;
    for (index, core) in signature.parameters.iter().enumerate() {
        let value = super::native_double_rules::argument(ctx, plan.logical, raw, index, false)?;
        super::binding_store::release_binding_value_if_needed(
            ctx,
            &super::native_path_values::snapshot_core(core),
            value,
            "release double argument",
        )?;
    }
    if payload && plan.application.is_some() {
        let data = ctx.builder.build_pointer_cast(
            raw,
            plan.payload.ptr_type(AddressSpace::default()),
            "double.release.typed",
        )?;
        let slot = ctx.builder.build_struct_gep(
            data,
            u32::try_from(signature.parameters.len())
                .map_err(|e| CodegenError::new(e.to_string()))?,
            "double.release.application.slot",
        )?;
        let application = ctx.builder.build_load(slot, "double.release.application")?;
        for (index, core) in signature.return_types.iter().enumerate() {
            if *core == CoreType::Unit {
                continue;
            }
            let value = if signature.error_types.is_empty() && signature.return_types.len() == 1 {
                application
            } else {
                ctx.builder.build_extract_value(
                    application.into_struct_value(),
                    u32::try_from(index).map_err(|e| CodegenError::new(e.to_string()))?,
                    "double.release.value",
                )?
            };
            super::binding_store::release_binding_value_if_needed(
                ctx,
                core,
                value,
                "release double result",
            )?;
        }
    }
    let free = super::native_callback_boundary::c_function(
        ctx,
        "free",
        ctx.context
            .void_type()
            .fn_type(&[raw.get_type().into()], false),
    )?;
    ctx.builder.build_call(free, &[raw.into()], "")?;
    ctx.builder.build_return(None)?;
    Ok(())
}
