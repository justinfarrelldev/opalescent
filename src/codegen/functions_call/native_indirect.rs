//! Ordinary typed source function parameters use the native outcome ABI.
use super::{
    BasicValueEnum, CodegenContext, CodegenEnv, CodegenError, CoreType, Expr,
    lower_native_argument, unwrap,
};
use crate::codegen::{native_outcome::NativeOutcomeType, types::core_type_to_llvm};
use inkwell::{AddressSpace, types::BasicMetadataTypeEnum, values::CallableValue};

/// Derive the ordinary owned-parameter callback ABI, then wrap its result.
pub(in crate::codegen) fn abi<'ctx>(
    ctx: &CodegenContext<'ctx>,
    parameters: &[CoreType],
    returns: &[CoreType],
    errors: &[CoreType],
) -> Result<(inkwell::types::FunctionType<'ctx>, NativeOutcomeType<'ctx>), CodegenError> {
    let llvm: Vec<BasicMetadataTypeEnum<'ctx>> = parameters
        .iter()
        .map(|ty| core_type_to_llvm(ctx.context, ty).into())
        .collect();
    let original = super::super::build_function_type(ctx, &llvm, returns, errors)?;
    let outcome = NativeOutcomeType::new(ctx.context, original.get_return_type());
    Ok((outcome.llvm_type().fn_type(&llvm, false), outcome))
}

/// Indirect calls are permitted only within the instrumented source ABI domain.
pub(super) fn lower<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    callee: &Expr,
    name: &str,
    args: &[Expr],
) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
    let Some(binding) = env.variables.get(name).cloned() else {
        return Ok(None);
    };
    if !matches!(binding.core_type, CoreType::Function { .. })
        || !crate::codegen::native_callable::active(ctx)
    {
        return Ok(None);
    }
    let pointer = ctx
        .builder
        .build_load(binding.alloca, "source.callback")?
        .into_pointer_value();
    invoke(ctx, env, callee, &binding.core_type, pointer, args).map(Some)
}

/// Call an evaluated source callable, without retaining its lexical storage address.
pub(super) fn invoke<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    callee: &Expr,
    signature: &CoreType,
    pointer: inkwell::values::PointerValue<'ctx>,
    args: &[Expr],
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let CoreType::Function {
        ref parameters,
        ref return_types,
        ref error_types,
        ..
    } = *signature
    else {
        return Err(CodegenError::new(
            "indirect source call requires a function signature",
        ));
    };
    if args.len() != parameters.len() {
        return Err(CodegenError::new(
            "typed source callback argument count mismatch",
        ));
    }
    let (function_type, outcome) = abi(ctx, parameters, return_types, error_types)?;
    let typed = ctx.builder.build_pointer_cast(
        pointer,
        function_type.ptr_type(AddressSpace::default()),
        "source.callback.abi",
    )?;
    let function = CallableValue::try_from(typed)
        .map_err(|()| CodegenError::new("invalid native source callback ABI"))?;
    let depth = env.current_scope_depth();
    let _scope = env.enter_scope();
    let mut cleanup = Vec::new();
    let mut lowered = Vec::new();
    for (index, (arg, ty)) in args.iter().zip(parameters).enumerate() {
        lowered.push(
            lower_native_argument(ctx, env, callee, index, arg, Some(ty), &mut cleanup)?.into(),
        );
    }
    let value = ctx
        .builder
        .build_call(function, &lowered, "source.callback.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("typed source callback omitted its outcome"))?
        .into_struct_value();
    let completed = unwrap(ctx, env, outcome, value, &cleanup)?;
    super::super::call_arg_cleanup::cleanup_call_argument_temporaries(ctx, env, depth, &cleanup)?;
    Ok(completed)
}
