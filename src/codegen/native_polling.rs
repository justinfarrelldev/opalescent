//! Synchronous assertion mismatch retries with ordinary lexical value ownership.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::{CodegenEnv, codegen_expression};
use super::native_abort::propagate_runtime_failure as check_abort;
use super::native_assertion_storage::entry_slot;
use super::native_callback_boundary::c_function;
use crate::ast::{
    AstNode, Expr,
    testing::{Expectation, Matcher, TestingStatement},
};
use crate::type_system::types::CoreType;
use inkwell::basic_block::BasicBlock;
use inkwell::values::IntValue;

/// Compiler-only retry destination, never runtime resource/lifetime tracking.
#[derive(Clone, Copy)]
pub struct NativePollingContext<'context> {
    /// Reevaluate only the original observation expression.
    pub head: BasicBlock<'context>,
    /// Once-evaluated monotonic deadline.
    pub deadline: IntValue<'context>,
    /// Existing lexical scope depth holding expected configuration.
    pub root_depth: usize,
}

/// Capture expected/deadline once, then reuse ordinary typed matcher lowering.
///
/// # Errors
/// Rejects unsupported matcher plans and propagates independent native aborts.
pub fn lower<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    let deadline = expectation
        .eventually_seconds
        .as_ref()
        .ok_or_else(|| CodegenError::new("polling expectation has no deadline"))?;
    let core =
        super::native_assertions::operand_core_type(env, &expectation.actual).ok_or_else(|| {
            CodegenError::new("polling observation has no authoritative checked type")
        })?;
    let _scope = env.enter_scope();
    let mut lowered = expectation.clone();
    lowered.eventually_seconds = None;
    capture_expected(codegen, env, &mut lowered.matcher, &core)?;
    let deadline = begin(codegen, env, deadline)?;
    let function = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("polling requires a native source function"))?;
    let head = codegen
        .context
        .append_basic_block(function, "expect.poll.observation");
    codegen.builder.build_unconditional_branch(head)?;
    codegen.builder.position_at_end(head);
    env.native_polling.push(NativePollingContext {
        head,
        deadline,
        root_depth: env.scope_stack.len(),
    });
    let result = super::native_assertions::codegen_testing_statement(
        codegen,
        env,
        &TestingStatement::Expect(lowered),
    );
    env.native_polling.pop();
    result?;
    env.exit_scope_cleanup(codegen, &[])
}

/// Called only on mismatch edges, before diagnostics or abort identities exist.
pub(crate) fn retry_mismatch<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
) -> Result<(), CodegenError> {
    let Some(context) = env.native_polling.last().copied() else {
        return Ok(());
    };
    let llvm = codegen.context;
    let output = entry_slot(codegen, llvm.i32_type().into(), "expect.poll.retry.output")?;
    let helper = c_function(
        codegen,
        "opal_test_poll_retry",
        llvm.i64_type()
            .fn_type(&[llvm.i64_type().into(), output.get_type().into()], false),
    )?;
    let identity = codegen
        .builder
        .build_call(
            helper,
            &[context.deadline.into(), output.into()],
            "expect.poll.retry.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("polling wait omitted its independent outcome"))?
        .into_int_value();
    check_abort(codegen, env, identity)?;
    let retry = codegen
        .builder
        .build_load(output, "expect.poll.retry")?
        .into_int_value();
    let retry = codegen.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        retry,
        llvm.i32_type().const_zero(),
        "expect.poll.can_retry",
    )?;
    let function = context
        .head
        .get_parent()
        .ok_or_else(|| CodegenError::new("polling retry destination has no function"))?;
    let again = llvm.append_basic_block(function, "expect.poll.again");
    let expired = llvm.append_basic_block(function, "expect.poll.expired");
    codegen
        .builder
        .build_conditional_branch(retry, again, expired)?;
    codegen.builder.position_at_end(again);
    super::scope_tracker::cleanup_scopes_to_depth_preserving_codegen_env(
        codegen,
        env,
        context.root_depth,
        &[],
    )?;
    codegen.builder.build_unconditional_branch(context.head)?;
    codegen.builder.position_at_end(expired);
    Ok(())
}

/// The generated local replaces only expected AST reads, not original source bytes.
fn capture_expected<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    matcher: &mut Matcher,
    core: &CoreType,
) -> Result<(), CodegenError> {
    use crate::ast::testing::ExpectedCallCount;
    match *matcher {
        Matcher::Equal {
            ref mut expected, ..
        }
        | Matcher::Compare {
            ref mut expected, ..
        }
        | Matcher::Sequence {
            ref mut expected, ..
        } => capture_once(codegen, env, expected, core),
        Matcher::Empty { .. }
        | Matcher::Variant { .. }
        | Matcher::CallCount {
            count: ExpectedCallCount::Once | ExpectedCallCount::Never,
        } => Ok(()),
        Matcher::CallCount {
            count: ExpectedCallCount::Exactly(ref mut expected),
        } => {
            let expected_core = super::native_assertions::operand_core_type(env, expected)
                .unwrap_or(CoreType::Int64);
            capture_once(codegen, env, expected, &expected_core)
        }
        Matcher::CalledWith { ref mut arguments } => {
            let CoreType::Generic { ref name, .. } = *core else {
                return Err(CodegenError::new(
                    "polling history requires a checked nominal double",
                ));
            };
            let signature = env
                .native_double_signatures
                .get(name)
                .cloned()
                .ok_or_else(|| {
                    CodegenError::new("polling history has no checked double signature")
                })?;
            if arguments.len() != signature.parameters.len() {
                return Err(CodegenError::new(
                    "polling history argument arity does not match its signature",
                ));
            }
            for (expression, expected_core) in arguments.iter_mut().zip(&signature.parameters) {
                capture_once(codegen, env, expression, expected_core)?;
            }
            Ok(())
        }
    }
}

/// Generated identifiers reference ordinary owned capture slots, not repeatable source calls.
fn capture_once<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &mut Expr,
    core: &CoreType,
) -> Result<(), CodegenError> {
    let (name, _) = super::native_double_owned::capture_binding(codegen, env, expression, core)?;
    *expression = Expr::Identifier {
        name,
        span: expression.span(),
        id: expression.node_id(),
    };
    Ok(())
}

/// Deadlines use the existing signedness rules before conversion to seconds.
fn begin<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
) -> Result<IntValue<'context>, CodegenError> {
    let core = super::native_assertions::operand_core_type(env, expression);
    let value = codegen_expression(codegen, env, expression, core.as_ref())?;
    let llvm = codegen.context;
    let seconds = if value.is_float_value() {
        let value = value.into_float_value();
        if value.get_type() == llvm.f64_type() {
            value
        } else {
            codegen
                .builder
                .build_float_ext(value, llvm.f64_type(), "expect.poll.seconds")?
        }
    } else if value.is_int_value() {
        let value = value.into_int_value();
        if core.is_some_and(|core| {
            matches!(
                core,
                CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64
            )
        }) {
            codegen.builder.build_unsigned_int_to_float(
                value,
                llvm.f64_type(),
                "expect.poll.seconds",
            )?
        } else {
            codegen.builder.build_signed_int_to_float(
                value,
                llvm.f64_type(),
                "expect.poll.seconds",
            )?
        }
    } else {
        return Err(CodegenError::new("polling deadline is not numeric seconds"));
    };
    let output = entry_slot(
        codegen,
        llvm.i64_type().into(),
        "expect.poll.deadline.output",
    )?;
    let helper = c_function(
        codegen,
        "opal_test_poll_begin",
        llvm.i64_type()
            .fn_type(&[llvm.f64_type().into(), output.get_type().into()], false),
    )?;
    let identity = codegen
        .builder
        .build_call(
            helper,
            &[seconds.into(), output.into()],
            "expect.poll.begin.status",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("polling clock omitted its independent outcome"))?
        .into_int_value();
    check_abort(codegen, env, identity)?;
    Ok(codegen
        .builder
        .build_load(output, "expect.poll.deadline")?
        .into_int_value())
}
