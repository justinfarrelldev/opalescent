//! Attempt-local lazy values use ordinary typed initializers and preserve native aborts.
use super::{CodegenContext, CodegenError, source::SourceFunction, symbol};
use crate::codegen::{
    binding_store::binding_requires_rc_cleanup,
    expressions::{CodegenEnv, ValueAccessorBinding},
    rc_emitter::RcEmitter,
    types::core_type_to_llvm,
};
use crate::type_system::types::CoreType;
use inkwell::module::Linkage;

/// Expose the same checked value type through local names and import aliases.
pub(super) fn bind(
    env: &mut CodegenEnv<'_>,
    name: &str,
    value: &SourceFunction,
) -> Result<(), CodegenError> {
    env.value_accessors.insert(
        name.to_owned(),
        ValueAccessorBinding {
            accessor_name: symbol("source", &value.identity)?,
            core_type: core(value)?.clone(),
        },
    );
    Ok(())
}

/// Extract the single cached application value from the synthetic function signature.
fn core(value: &SourceFunction) -> Result<&CoreType, CodegenError> {
    if let CoreType::Function {
        ref return_types, ..
    } = value.signature
    {
        if let [ref result] = *return_types.as_slice() {
            return Ok(result);
        }
    }
    Err(CodegenError::new(
        "module value requires exactly one checked result",
    ))
}

/// Cache only a completed result. Aborted initialization never publishes a value.
pub(super) fn emit<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    value: &SourceFunction,
) -> Result<(), CodegenError> {
    let name = symbol("source", &value.identity)?;
    let accessor = ctx
        .module
        .get_function(&name)
        .ok_or_else(|| CodegenError::new("missing native value accessor"))?;
    let outcome = ctx
        .native_function_outcome(accessor)
        .ok_or_else(|| CodegenError::new("missing native value outcome"))?;
    let init = crate::codegen::native_declarations::emit_source_function(
        ctx,
        env,
        &symbol("value_init", &value.identity)?,
        &[],
        &value.signature,
        &value.body,
    )?;
    let result_type = core_type_to_llvm(ctx.context, core(value)?);
    let cache = ctx
        .module
        .add_global(result_type, None, &format!("{name}.cache"));
    cache.set_linkage(Linkage::Internal);
    cache.set_initializer(&result_type.const_zero());
    let flag_type = ctx.context.bool_type();
    let ready = ctx
        .module
        .add_global(flag_type, None, &format!("{name}.ready"));
    ready.set_linkage(Linkage::Internal);
    ready.set_initializer(&flag_type.const_zero());
    let entry = ctx.context.append_basic_block(accessor, "entry");
    let cached = ctx.context.append_basic_block(accessor, "cached");
    let initialize = ctx.context.append_basic_block(accessor, "initialize");
    ctx.builder.position_at_end(entry);
    let flag = ctx
        .builder
        .build_load(ready.as_pointer_value(), "ready")?
        .into_int_value();
    ctx.builder
        .build_conditional_branch(flag, cached, initialize)?;
    ctx.builder.position_at_end(initialize);
    let result = ctx
        .builder
        .build_call(init, &[], "initializer")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("missing native initializer result"))?
        .into_struct_value();
    let branches = outcome.branch_after_call(ctx, result)?;
    ctx.builder.position_at_end(branches.aborted);
    ctx.builder.build_return(Some(&result))?;
    ctx.builder.position_at_end(branches.completed);
    let initialized = outcome
        .completed_application_value(ctx, result)?
        .ok_or_else(|| CodegenError::new("initializer omitted its module value"))?;
    ctx.builder
        .build_store(cache.as_pointer_value(), initialized)?;
    ctx.builder
        .build_store(ready.as_pointer_value(), flag_type.const_all_ones())?;
    ctx.builder.build_unconditional_branch(cached)?;
    ctx.builder.position_at_end(cached);
    let loaded = ctx
        .builder
        .build_load(cache.as_pointer_value(), "module.value")?;
    if binding_requires_rc_cleanup(core(value)?) {
        RcEmitter::new(&ctx.builder, &ctx.module).emit_inc(loaded.into_pointer_value())?;
    }
    let completed = outcome.build_completed(ctx, Some(loaded))?;
    ctx.builder.build_return(Some(&completed))?;
    Ok(())
}
