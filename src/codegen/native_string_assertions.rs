//! String matcher operands live in existing lexical ownership scopes.
use super::{CodegenContext, CodegenEnv, CodegenError, emit_native_abort};
use crate::ast::{
    Expr,
    testing::{Expectation, Matcher, SequenceRelation},
};
use crate::codegen::binding_store::initialize_binding_value;
use crate::codegen::expressions::{VariableBinding, codegen_expression};
use crate::codegen::native_callback_boundary::c_function;
use crate::type_system::types::CoreType;
use inkwell::values::PointerValue;

/// Actual and expected values are captured once, before any failure formatting.
pub(super) fn lower<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    let (relation, negated, expected, label) = match expectation.matcher {
        Matcher::Equal {
            ref expected,
            negated,
        } => (
            0_u64,
            negated,
            Some(expected),
            if negated { "not to be" } else { "to be" },
        ),
        Matcher::Sequence {
            ref expected,
            relation,
            negated,
        } => match relation {
            SequenceRelation::Contain => (
                1,
                negated,
                Some(expected),
                if negated {
                    "not to contain"
                } else {
                    "to contain"
                },
            ),
            SequenceRelation::StartWith => (2, negated, Some(expected), "to start with"),
            SequenceRelation::EndWith => (
                3,
                negated,
                Some(expected),
                if negated {
                    "not to end with"
                } else {
                    "to end with"
                },
            ),
            SequenceRelation::ExactlyInOrder => {
                return Err(CodegenError::new(
                    "exact ordered string matching requires an array operand",
                ));
            }
        },
        Matcher::Empty { negated } => (
            4,
            negated,
            None,
            if negated {
                "not to be empty"
            } else {
                "to be empty"
            },
        ),
        _ => return Err(CodegenError::new("unsupported typed string matcher")),
    };
    let _scope = env.enter_scope();
    let actual = capture(codegen, env, &expectation.actual)?;
    let expected = expected
        .map(|expression| capture(codegen, env, expression))
        .transpose()?
        .unwrap_or_else(|| actual.get_type().const_null());
    let llvm = codegen.context;
    let matched = compare_values(codegen, actual, expected, relation)?;
    let mut passed = codegen.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        matched,
        llvm.i32_type().const_zero(),
        "expect.string.passed",
    )?;
    if negated {
        passed = codegen.builder.build_not(passed, "expect.string.negated")?;
    }
    let function = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("string expectation has no enclosing native function"))?;
    let failed = llvm.append_basic_block(function, "expect.string.failed");
    let ready = llvm.append_basic_block(function, "expect.string.ready");
    codegen
        .builder
        .build_conditional_branch(passed, ready, failed)?;
    codegen.builder.position_at_end(failed);
    let identity = record_failure(codegen, env, expectation, label, actual, expected)?;
    emit_native_abort(codegen, env, identity, &[])?;
    codegen.builder.position_at_end(ready);
    env.exit_scope_cleanup(codegen, &[])
}

/// String comparison consumes already-evaluated ordinary ABI values.
fn compare_values<'context>(
    codegen: &CodegenContext<'context>,
    actual: PointerValue<'context>,
    expected: PointerValue<'context>,
    relation: u64,
) -> Result<inkwell::values::IntValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let pointer = actual.get_type();
    let compare = c_function(
        codegen,
        "opal_test_string_matches",
        llvm.i32_type().fn_type(
            &[llvm.i32_type().into(), pointer.into(), pointer.into()],
            false,
        ),
    )?;
    Ok(codegen
        .builder
        .build_call(
            compare,
            &[
                llvm.i32_type().const_int(relation, false).into(),
                actual.into(),
                expected.into(),
            ],
            "expect.string.matches",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("string matcher returned no comparison"))?
        .into_int_value())
}

/// Capture source coordinates and still-live operand bytes before scope cleanup.
fn record_failure<'context>(
    codegen: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expectation: &Expectation,
    label: &str,
    actual: PointerValue<'context>,
    expected: PointerValue<'context>,
) -> Result<inkwell::values::IntValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let pointer = actual.get_type();
    let record = c_function(
        codegen,
        "opal_test_record_string_mismatch",
        llvm.i64_type().fn_type(
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                pointer.into(),
                llvm.i64_type().into(),
                llvm.i64_type().into(),
            ],
            false,
        ),
    )?;
    let label = codegen
        .builder
        .build_global_string_ptr(label, "expect.string.matcher")?
        .as_pointer_value();
    let file = codegen
        .builder
        .build_global_string_ptr(&env.current_source_path, "expect.string.file")?
        .as_pointer_value();
    let offset = u64::try_from(expectation.span.start.offset)
        .map_err(|error| CodegenError::new(error.to_string()))?;
    let length = u64::try_from(expectation.span.len())
        .map_err(|error| CodegenError::new(error.to_string()))?;
    Ok(codegen
        .builder
        .build_call(
            record,
            &[
                label.into(),
                actual.into(),
                expected.into(),
                file.into(),
                llvm.i64_type().const_int(offset, false).into(),
                llvm.i64_type().const_int(length, false).into(),
            ],
            "expect.string.failure",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("string diagnostic returned no failure identity"))?
        .into_int_value())
}

/// A borrowed operand is retained and an owned result is transferred into a
/// normal lexical binding before later expressions can abort or mutate values.
fn capture<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
) -> Result<PointerValue<'context>, CodegenError> {
    let value = codegen_expression(codegen, env, expression, Some(&CoreType::String))?;
    if !value.is_pointer_value() {
        return Err(CodegenError::new(
            "checked string operand has no string representation",
        ));
    }
    let name = env.next_name("__native_assertion_string");
    let slot = codegen.builder.build_alloca(value.get_type(), &name)?;
    env.variables.insert(
        name.clone(),
        VariableBinding {
            alloca: slot,
            core_type: CoreType::String,
            length: None,
            capacity: None,
            is_mutable: false,
        },
    );
    env.register_scope_binding(&name);
    initialize_binding_value(
        codegen,
        env,
        &name,
        value,
        "capture assertion string",
        crate::codegen::string_ownership::string_expr_needs_retain_at_ownership_boundary(
            expression,
        ),
    )?;
    Ok(value.into_pointer_value())
}
