//! Canonical source import-edge dispatch before ordinary native call execution.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::CodegenEnv;
use super::native_callback_boundary::c_function;
use crate::testing::native::checking::CheckedNativeProject;
use crate::testing::native::identity::ResolvedImportEdge;
use inkwell::types::BasicType;
use inkwell::values::{FunctionValue, IntValue};
use inkwell::{AddressSpace, IntPredicate};

/// Override only the resolved importing module's exact local edge aliases.
pub fn install<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    checked: &CheckedNativeProject,
) -> Result<(), CodegenError> {
    super::native_doubles::install(ctx, env, checked)?;
    for declaration in checked.doubles.declarations.values() {
        if declaration.edge.importing_module != env.current_source_path {
            continue;
        }
        let source =
            super::native_project::symbol("source", &declaration.edge.imported_function.0)?;
        let original = ctx.module.get_function(&source).ok_or_else(|| CodegenError::new("double boundary requires a compiled native source or explicit standard-library adapter"))?;
        if ctx.native_function_outcome(original).is_none() {
            return Err(CodegenError::new(
                "double boundary cannot silently intercept an unsupported foreign ABI",
            ));
        }
        let dispatcher = emit(ctx, &declaration.edge, original)?;
        let symbol = dispatcher
            .get_name()
            .to_str()
            .map_err(|error| CodegenError::new(error.to_string()))?
            .to_owned();
        for alias in &declaration.import_names {
            env.imported_functions.insert(alias.clone(), symbol.clone());
        }
    }
    Ok(())
}

/// A compiler wrapper forwards the original native ABI unchanged, including aborts.
/// No source parameter ownership is acquired: caller temporaries stay live and
/// the actual source callee applies its existing parameter retain/drop rules.
fn emit<'context>(
    ctx: &CodegenContext<'context>,
    edge: &ResolvedImportEdge,
    original: FunctionValue<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let name = super::native_project::symbol("double_edge", edge)?;
    let parameters = original.get_type().get_param_types();
    let application = ctx.application_return_type(original);
    let signature = application.map_or_else(
        || ctx.context.void_type().fn_type(&parameters, false),
        |result| result.fn_type(&parameters, false),
    );
    let function = ctx.declare_native_test_function(
        &name,
        signature,
        Some(inkwell::module::Linkage::Private),
    )?;
    if function.count_basic_blocks() != 0 {
        return Ok(function);
    }
    let previous = ctx.builder.get_insert_block();
    let result = emit_body(ctx, edge, original, function);
    if let Some(previous) = previous {
        ctx.builder.position_at_end(previous);
    } else {
        ctx.builder.clear_insertion_position();
    }
    result?;
    Ok(function)
}

/// Inactive boundaries forward; active unconfigured calls abort without payload reads.
fn emit_body<'context>(
    ctx: &CodegenContext<'context>,
    edge: &ResolvedImportEdge,
    original: FunctionValue<'context>,
    function: FunctionValue<'context>,
) -> Result<(), CodegenError> {
    let llvm = ctx.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    ctx.builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let selection_type = llvm.struct_type(&[llvm.i32_type().into(), pointer.into()], false);
    let selection = ctx
        .builder
        .build_alloca(selection_type, "double.selection")?;
    let edge = serde_json::to_string(edge).map_err(|error| CodegenError::new(error.to_string()))?;
    let edge = ctx
        .builder
        .build_global_string_ptr(&edge, "double.edge")?
        .as_pointer_value();
    let select = c_function(
        ctx,
        "opal_test_double_select",
        llvm.i64_type().fn_type(
            &[pointer.into(), pointer.into(), selection.get_type().into()],
            false,
        ),
    )?;
    let identity = ctx
        .builder
        .build_call(
            select,
            &[edge.into(), pointer.const_null().into(), selection.into()],
            "double.dispatch.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double dispatch omitted its independent outcome"))?
        .into_int_value();
    let mut env = CodegenEnv::new(true);
    super::native_abort::propagate_runtime_failure(ctx, &mut env, identity)?;
    let kind_slot = ctx
        .builder
        .build_struct_gep(selection, 0, "double.selection.kind")?;
    let kind = ctx
        .builder
        .build_load(kind_slot, "double.kind")?
        .into_int_value();
    let is_original = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        kind,
        llvm.i32_type().const_zero(),
        "double.original.selected",
    )?;
    let forward = llvm.append_basic_block(function, "double.original");
    let unsupported = llvm.append_basic_block(function, "double.typed.rule.required");
    ctx.builder
        .build_conditional_branch(is_original, forward, unsupported)?;
    ctx.builder.position_at_end(unsupported);
    let failure = unsupported_rule(ctx, pointer)?;
    super::native_abort::emit_native_abort(ctx, &mut env, failure, &[])?;
    ctx.builder.position_at_end(forward);
    let arguments = function
        .get_param_iter()
        .map(Into::into)
        .collect::<Vec<_>>();
    let outcome = ctx
        .builder
        .build_call(original, &arguments, "double.original.outcome")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("native original omitted its explicit outcome"))?;
    ctx.builder.build_return(Some(&outcome))?;
    Ok(())
}

/// Unimplemented rule payloads are never interpreted as a completed application value.
fn unsupported_rule<'context>(
    ctx: &CodegenContext<'context>,
    pointer: inkwell::types::PointerType<'context>,
) -> Result<IntValue<'context>, CodegenError> {
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
        .build_global_string_ptr(
            "selected double rule requires compiled typed payload dispatch",
            "double.unsupported.rule",
        )?
        .as_pointer_value();
    let file = ctx
        .builder
        .build_global_string_ptr("", "double.internal.file")?
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
            "double.rule.abort",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double rule rejection omitted failure identity"))?
        .into_int_value())
}
