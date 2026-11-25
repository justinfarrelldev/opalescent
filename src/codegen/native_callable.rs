//! Native callable values share one verified source outcome ABI.
use super::{context::CodegenContext, error::CodegenError, expressions::CodegenEnv};
use crate::{
    ast::{Expr, Type},
    type_system::types::CoreType,
};
use inkwell::values::BasicValueEnum;

/// Test functions are recognized by their explicit declaration metadata.
pub(super) fn active(ctx: &CodegenContext<'_>) -> bool {
    ctx.builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .is_some_and(|function| ctx.native_function_outcome(function).is_some())
}

/// Convert a checked named source function to the ordinary opaque function value.
pub(super) fn named<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &CodegenEnv<'ctx>,
    name: &str,
    expected: Option<&CoreType>,
) -> Result<Option<BasicValueEnum<'ctx>>, CodegenError> {
    if !active(ctx) || env.variables.contains_key(name) || env.value_accessors.contains_key(name) {
        return Ok(None);
    }
    if env.native_double_calls.contains_key(name) {
        return Err(CodegenError::new(
            "indirect interception of a doubled source import is unsupported",
        ));
    }
    let Some(
        signature @ &CoreType::Function {
            ref parameters,
            ref return_types,
            ref error_types,
            ..
        },
    ) = expected
        .filter(|ty| matches!(**ty, CoreType::Function { .. }))
        .or_else(|| env.imported_signatures.get(name))
    else {
        return Ok(None);
    };
    let symbol = env
        .imported_functions
        .get(name)
        .map_or(name, String::as_str);
    let function = ctx
        .module
        .get_function(symbol)
        .ok_or_else(|| CodegenError::new("callback must name a concrete native source function"))?;
    let (abi, _) =
        super::functions_call::native::indirect::abi(ctx, parameters, return_types, error_types)?;
    if ctx.native_function_outcome(function).is_none() || function.get_type() != abi {
        return Err(CodegenError::new(
            "foreign or incompatible source callback ABI is unsupported in native tests",
        ));
    }
    let erased = super::types::core_type_to_llvm(ctx.context, signature).into_pointer_type();
    Ok(Some(
        ctx.builder
            .build_pointer_cast(
                function.as_global_value().as_pointer_value(),
                erased,
                "source.callback.value",
            )?
            .into(),
    ))
}

/// Convert a non-polymorphic lambda signature using the enclosing concrete type bindings.
pub(super) fn signature(env: &CodegenEnv<'_>, expr: &Expr) -> Result<CoreType, CodegenError> {
    let Expr::Lambda {
        ref params,
        ref return_types,
        ref error_types,
        ref generic_params,
        span,
        ..
    } = *expr
    else {
        return Err(CodegenError::new("expected source lambda"));
    };
    if generic_params
        .as_ref()
        .is_some_and(|types| !types.is_empty())
    {
        return Err(CodegenError::new(
            "generic local lambda requires concrete specialization",
        ));
    }
    let map = |ty: &Type| super::native_generics::annotation(env, ty);
    Ok(CoreType::Function {
        generic_params: Vec::new(),
        parameters: params
            .iter()
            .map(|p| map(&p.param_type))
            .collect::<Result<Vec<_>, _>>()?,
        return_types: return_types
            .iter()
            .map(map)
            .collect::<Result<Vec<_>, _>>()?,
        error_types: error_types
            .iter()
            .map(|name| {
                map(&Type::Basic {
                    name: name.clone(),
                    span,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
    })
}

/// Emit a local function in an isolated lexical environment, then resume its creator.
pub(super) fn lambda<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    expr: &Expr,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    let Expr::Lambda {
        ref params,
        ref body,
        ref captured_variables,
        span,
        ..
    } = *expr
    else {
        return Err(CodegenError::new("expected source lambda"));
    };
    if params
        .iter()
        .any(|parameter| parameter.borrow_kind != crate::ast::BorrowKind::Owned)
    {
        return Err(CodegenError::new(
            "borrowed lambda parameters are not representable in the source function-value ABI",
        ));
    }
    if captured_variables
        .iter()
        .any(|name| env.variables.contains_key(name))
    {
        return Err(CodegenError::new(
            "capturing function values require a closure representation in native tests",
        ));
    }
    let signature = signature(env, expr)?;
    let function = emit_lambda(ctx, env, params, &signature, body, span)?;
    let erased = super::types::core_type_to_llvm(ctx.context, &signature).into_pointer_type();
    Ok(ctx
        .builder
        .build_pointer_cast(
            function.as_global_value().as_pointer_value(),
            erased,
            "source.lambda.value",
        )?
        .into())
}

/// Lift lexical captures into explicit call-local parameters; they cannot escape as a closure.
pub(super) fn immediate<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    expr: &Expr,
    args: &[Expr],
    expected: Option<&CoreType>,
) -> Result<BasicValueEnum<'ctx>, CodegenError> {
    use crate::ast::{BorrowKind, NodeId, Parameter};
    let Expr::Lambda {
        ref params,
        ref body,
        ref captured_variables,
        span,
        ..
    } = *expr
    else {
        return Err(CodegenError::new("expected source lambda"));
    };
    let mut signature = signature(env, expr)?;
    let CoreType::Function {
        ref mut parameters, ..
    } = signature
    else {
        return Err(CodegenError::new("missing lambda signature"));
    };
    let mut lifted = params.clone();
    let mut inputs = args.to_vec();
    for name in captured_variables {
        let Some(binding) = env.variables.get(name) else {
            continue;
        };
        parameters.push(binding.core_type.clone());
        lifted.push(Parameter {
            name: name.clone(),
            param_type: Type::Basic {
                name: "void".to_owned(),
                span,
            },
            borrow_kind: BorrowKind::Owned,
            span,
        });
        inputs.push(Expr::Identifier {
            name: name.clone(),
            span,
            id: NodeId(0),
        });
    }
    let function = emit_lambda(ctx, env, &lifted, &signature, body, span)?;
    let alias = env.next_name("__native_immediate_lambda");
    env.imported_functions.insert(
        alias.clone(),
        function.get_name().to_string_lossy().into_owned(),
    );
    env.imported_signatures.insert(alias.clone(), signature);
    super::functions_call::codegen_call_expression(
        ctx,
        env,
        &Expr::Identifier {
            name: alias,
            span,
            id: NodeId(0),
        },
        None,
        &inputs,
        expected,
    )
}

/// Emit a typed body and restore the caller insertion point even on failure.
fn emit_lambda<'ctx>(
    ctx: &CodegenContext<'ctx>,
    env: &mut CodegenEnv<'ctx>,
    params: &[crate::ast::Parameter],
    signature: &CoreType,
    body: &crate::ast::LambdaBody,
    span: crate::token::Span,
) -> Result<inkwell::values::FunctionValue<'ctx>, CodegenError> {
    let caller = ctx
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new("source lambda has no enclosing block"))?;
    let parent = caller
        .get_parent()
        .ok_or_else(|| CodegenError::new("source lambda has no enclosing function"))?;
    let name = format!(
        "{}.{}",
        parent.get_name().to_string_lossy(),
        env.next_name("source.lambda")
    );
    let mut nested = module_environment(env);
    let body = super::native_project::source::lambda_body(body, span);
    let result = super::native_declarations::emit_source_function(
        ctx,
        &mut nested,
        &name,
        params,
        signature,
        &body,
    );
    ctx.builder.position_at_end(caller);
    result
}

/// Share only module metadata; never carry outer allocas, guard slots or cleanup scopes into a function.
fn module_environment<'ctx>(env: &CodegenEnv<'ctx>) -> CodegenEnv<'ctx> {
    let mut nested = CodegenEnv::new(env.debug_mode);
    nested
        .imported_functions
        .clone_from(&env.imported_functions);
    nested
        .imported_signatures
        .clone_from(&env.imported_signatures);
    nested.value_accessors.clone_from(&env.value_accessors);
    nested
        .owned_string_functions
        .clone_from(&env.owned_string_functions);
    nested
        .current_source_path
        .clone_from(&env.current_source_path);
    nested
        .current_source_text
        .clone_from(&env.current_source_text);
    nested.adt_field_indices.clone_from(&env.adt_field_indices);
    nested.adt_field_layouts.clone_from(&env.adt_field_layouts);
    nested
        .adt_product_layouts
        .clone_from(&env.adt_product_layouts);
    nested
        .adt_layout_aliases
        .clone_from(&env.adt_layout_aliases);
    nested
        .adt_variant_discriminants
        .clone_from(&env.adt_variant_discriminants);
    nested
        .native_assertion_types
        .clone_from(&env.native_assertion_types);
    nested
        .native_double_descriptors
        .clone_from(&env.native_double_descriptors);
    nested
        .native_double_signatures
        .clone_from(&env.native_double_signatures);
    nested
        .native_double_calls
        .clone_from(&env.native_double_calls);
    nested
        .native_generic_parameters
        .clone_from(&env.native_generic_parameters);
    nested
        .native_type_arguments
        .clone_from(&env.native_type_arguments);
    nested
}
