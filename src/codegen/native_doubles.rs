//! Compiler-only nominal activation and inspection of attempt-local double state.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::{CodegenEnv, codegen_expression};
use super::native_assertion_storage::entry_slot;
use super::native_callback_boundary::c_function;
use crate::ast::{
    Expr,
    testing::{Expectation, ExpectedCallCount, Matcher, TestingStatement},
};
use crate::testing::native::checking::CheckedNativeProject;
use crate::type_system::types::CoreType;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::{AddressSpace, IntPredicate};

/// Install immutable target-layout descriptors; aliases retain canonical handle types.
pub fn install<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    checked: &CheckedNativeProject,
) -> Result<(), CodegenError> {
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let clone_type = pointer
        .fn_type(&[pointer.into()], false)
        .ptr_type(AddressSpace::default());
    let drop_type = ctx
        .context
        .void_type()
        .fn_type(&[pointer.into()], false)
        .ptr_type(AddressSpace::default());
    let layout = ctx.context.struct_type(
        &[
            pointer.into(),
            pointer.into(),
            clone_type.into(),
            drop_type.into(),
        ],
        false,
    );
    for (identity, core) in &checked.double_types {
        let CoreType::Generic { ref name, .. } = *core else {
            return Err(CodegenError::new(
                "double descriptor has no canonical nominal type",
            ));
        };
        let signature = checked
            .double_signatures
            .get(identity)
            .ok_or_else(|| CodegenError::new("double has no checked application signature"))?;
        env.native_double_signatures
            .insert(name.clone(), signature.clone());
        let callbacks = signature
            .parameters
            .iter()
            .all(super::native_double_rules::supported)
            .then(|| super::native_double_rules::callbacks(ctx, env, name, signature))
            .transpose()?;
        let symbol = super::native_project::symbol("double_descriptor", identity)?;
        let descriptor = if let Some(global) = ctx.module.get_global(&symbol) {
            if global.as_pointer_value().get_type() != layout.ptr_type(AddressSpace::default()) {
                return Err(CodegenError::new(
                    "double descriptor conflicts with an existing global ABI",
                ));
            }
            global
        } else {
            let declaration = &checked.doubles.declarations[identity];
            let edge = serde_json::to_string(&declaration.edge)
                .map_err(|error| CodegenError::new(error.to_string()))?;
            let key = serde_json::to_string(identity)
                .map_err(|error| CodegenError::new(error.to_string()))?;
            let edge = constant_string(ctx, &format!("{symbol}.edge"), &edge);
            let key = constant_string(ctx, &format!("{symbol}.identity"), &key);
            let global = ctx.module.add_global(layout, None, &symbol);
            global.set_initializer(
                &layout.const_named_struct(&[
                    edge.into(),
                    key.into(),
                    callbacks
                        .map_or_else(
                            || clone_type.const_null(),
                            |helpers| helpers.snapshot.as_global_value().as_pointer_value(),
                        )
                        .into(),
                    callbacks
                        .map_or_else(
                            || drop_type.const_null(),
                            |helpers| helpers.release.as_global_value().as_pointer_value(),
                        )
                        .into(),
                ]),
            );
            global.set_constant(true);
            global.set_linkage(inkwell::module::Linkage::Private);
            global
        };
        env.native_double_descriptors.insert(
            name.clone(),
            descriptor.as_pointer_value().const_cast(pointer),
        );
    }
    Ok(())
}

/// Static data construction does not require a source builder insertion position.
fn constant_string<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
    value: &str,
) -> PointerValue<'context> {
    let value = ctx.context.const_string(value.as_bytes(), true);
    let global = ctx.module.add_global(value.get_type(), None, name);
    global.set_initializer(&value);
    global.set_constant(true);
    global.set_linkage(inkwell::module::Linkage::Private);
    global
        .as_pointer_value()
        .const_cast(ctx.context.i8_type().ptr_type(AddressSpace::default()))
}

/// Recognize nominal double construction before ordinary record construction.
pub fn constructor<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
) -> Result<Option<BasicValueEnum<'context>>, CodegenError> {
    let Expr::Constructor {
        ref callee,
        ref fields,
        ..
    } = *expression
    else {
        return Ok(None);
    };
    let Expr::Identifier { ref name, .. } = **callee else {
        return Ok(None);
    };
    let Some(descriptor) = env
        .native_double_descriptors
        .get(env.adt_layout_key(name))
        .copied()
    else {
        return Ok(None);
    };
    if !fields.is_empty() {
        return Err(CodegenError::new(
            "nominal double activation takes no record fields",
        ));
    }
    let pointer = descriptor.get_type();
    let output = entry_slot(ctx, pointer.into(), "double.activation.output")?;
    let helper = c_function(
        ctx,
        "opal_test_double_activate",
        ctx.context
            .i64_type()
            .fn_type(&[pointer.into(), output.get_type().into()], false),
    )?;
    let identity = ctx
        .builder
        .build_call(
            helper,
            &[descriptor.into(), output.into()],
            "double.activation.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double activation omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)?;
    Ok(Some(ctx.builder.build_load(output, "double.handle")?))
}

/// Explicit restore and verification never become inferred scope cleanup.
pub fn statement<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    statement: &TestingStatement,
) -> Result<(), CodegenError> {
    let (target, name) = match *statement {
        TestingStatement::Stub {
            ref target,
            ref arguments,
            ref behavior,
        } => {
            if matches!(
                *behavior,
                crate::ast::testing::StubBehavior::SpyOriginal
                    | crate::ast::testing::StubBehavior::FailTest
            ) && arguments.is_none()
            {
                return fallback(ctx, env, target, behavior);
            }
            return super::native_double_configuration::configure(
                ctx,
                env,
                target,
                arguments.as_deref(),
                behavior,
            );
        }
        TestingStatement::Restore(ref target) => (target, "opal_test_double_restore"),
        TestingStatement::Verify(ref target) => (target, "opal_test_double_verify"),
        TestingStatement::Expect(_) => {
            return Err(CodegenError::new(
                "expected a double configuration or lifecycle statement",
            ));
        }
    };
    let target = codegen_expression(ctx, env, target, None)?;
    let helper = c_function(
        ctx,
        name,
        ctx.context
            .i64_type()
            .fn_type(&[target.get_type().into()], false),
    )?;
    let identity = ctx
        .builder
        .build_call(helper, &[target.into()], "double.operation.status")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double operation omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)
}

/// Explicit original/fail fallbacks require no fabricated result payloads.
fn fallback<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    target: &Expr,
    behavior: &crate::ast::testing::StubBehavior,
) -> Result<(), CodegenError> {
    let kind = match *behavior {
        crate::ast::testing::StubBehavior::SpyOriginal => 0_u64,
        crate::ast::testing::StubBehavior::FailTest => 3_u64,
        _ => {
            return Err(CodegenError::new(
                "typed double fallback requires compiled owned payload lowering",
            ));
        }
    };
    let target = codegen_expression(ctx, env, target, None)?.into_pointer_value();
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let release = ctx
        .context
        .void_type()
        .fn_type(&[pointer.into()], false)
        .ptr_type(AddressSpace::default());
    let helper = c_function(
        ctx,
        "opal_test_double_fallback",
        ctx.context.i64_type().fn_type(
            &[
                pointer.into(),
                ctx.context.i32_type().into(),
                pointer.into(),
                release.into(),
            ],
            false,
        ),
    )?;
    let status = ctx
        .builder
        .build_call(
            helper,
            &[
                target.into(),
                ctx.context.i32_type().const_int(kind, false).into(),
                pointer.const_null().into(),
                release.const_null().into(),
            ],
            "double.fallback.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double fallback omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, status)
}

/// Count-only inspection remains available independently of argument snapshots.
pub fn count<'context>(
    ctx: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    if matches!(expectation.matcher, Matcher::CalledWith { .. }) {
        return super::native_double_history::lower(ctx, env, expectation);
    }
    let Matcher::CallCount { ref count } = expectation.matcher else {
        return Err(CodegenError::new(
            "typed argument history requires compiled snapshot matchers",
        ));
    };
    let target = codegen_expression(ctx, env, &expectation.actual, None)?;
    let output = entry_slot(ctx, ctx.context.i64_type().into(), "double.count.output")?;
    let helper = c_function(
        ctx,
        "opal_test_double_count",
        ctx.context
            .i64_type()
            .fn_type(&[target.get_type().into(), output.get_type().into()], false),
    )?;
    let identity = ctx
        .builder
        .build_call(
            helper,
            &[target.into(), output.into()],
            "double.count.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("double count omitted its independent outcome"))?
        .into_int_value();
    super::native_abort::propagate_runtime_failure(ctx, env, identity)?;
    let actual = ctx
        .builder
        .build_load(output, "double.count")?
        .into_int_value();
    let expected = match *count {
        ExpectedCallCount::Once => ctx.context.i64_type().const_int(1, false),
        ExpectedCallCount::Never => ctx.context.i64_type().const_zero(),
        ExpectedCallCount::Exactly(ref expression) => {
            let value = codegen_expression(ctx, env, expression, None)?.into_int_value();
            if value.get_type() == ctx.context.i64_type() {
                value
            } else {
                let core = super::native_assertions::operand_core_type(env, expression);
                if core.is_some_and(|core| {
                    matches!(
                        core,
                        CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64
                    )
                }) {
                    ctx.builder.build_int_z_extend(
                        value,
                        ctx.context.i64_type(),
                        "double.expected.unsigned",
                    )?
                } else {
                    ctx.builder.build_int_s_extend(
                        value,
                        ctx.context.i64_type(),
                        "double.expected.signed",
                    )?
                }
            }
        }
    };
    let matches = ctx.builder.build_int_compare(
        IntPredicate::EQ,
        actual,
        expected,
        "double.count.matches",
    )?;
    let function = ctx
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("double assertion has no function"))?;
    let failed = ctx
        .context
        .append_basic_block(function, "double.count.failed");
    let ready = ctx
        .context
        .append_basic_block(function, "double.count.ready");
    ctx.builder
        .build_conditional_branch(matches, ready, failed)?;
    ctx.builder.position_at_end(failed);
    let mismatch =
        super::native_assertions::record_count_failure(ctx, env, expectation, actual, expected)?;
    super::native_abort::emit_native_abort(ctx, env, mismatch, &[])?;
    ctx.builder.position_at_end(ready);
    Ok(())
}
