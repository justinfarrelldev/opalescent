//! Typed native assertion lowering using normal expression and scope machinery.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::{CodegenEnv, codegen_expression};
use super::expressions_array::infer_expression_core_type;
use super::native_abort::emit_native_abort;
use crate::ast::testing::{Comparison, Expectation, Matcher, TestingStatement};
use crate::ast::{AstNode, Expr};
use crate::type_system::types::CoreType;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{FloatPredicate, IntPredicate};
#[path = "native_value_snapshots.rs"]
mod snapshots;
#[path = "native_string_assertions.rs"]
mod strings;
#[path = "native_value_equality.rs"]
mod value_equality;
#[path = "native_value_assertions.rs"]
mod values;

/// Lower an operation only in an explicitly registered native source function.
///
/// # Errors
/// Rejects production use and operations whose lowering is not implemented.
pub fn codegen_testing_statement<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    statement: &TestingStatement,
) -> Result<(), CodegenError> {
    let function = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("test operation has no enclosing function"))?;
    if codegen.native_function_outcome(function).is_none() {
        return Err(CodegenError::new(
            "unlowered test operation cannot enter production code generation",
        ));
    }
    match *statement {
        TestingStatement::Expect(ref expectation) => lower_expectation(codegen, env, expectation),
        _ => super::native_doubles::statement(codegen, env, statement),
    }
}

/// Evaluate ordinary scalar operands exactly once and branch on their result.
fn lower_expectation<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    if expectation.eventually_seconds.is_some() {
        return super::native_polling::lower(codegen, env, expectation);
    }
    if matches!(
        expectation.matcher,
        Matcher::CallCount { .. } | Matcher::CalledWith { .. }
    ) {
        return super::native_doubles::count(codegen, env, expectation);
    }
    let actual_type = operand_core_type(env, &expectation.actual);
    if actual_type == Some(CoreType::String) {
        return strings::lower(codegen, env, expectation);
    }
    if let Some(ref core @ (CoreType::Array(_) | CoreType::Generic { .. })) = actual_type {
        return values::lower(codegen, env, expectation, core);
    }
    let (Matcher::Equal {
        expected: ref expected_expression,
        ..
    }
    | Matcher::Compare {
        expected: ref expected_expression,
        ..
    }) = expectation.matcher
    else {
        return Err(CodegenError::new(
            "matcher requires native typed matcher lowering",
        ));
    };
    let actual = codegen_expression(codegen, env, &expectation.actual, actual_type.as_ref())?;
    let expected = codegen_expression(codegen, env, expected_expression, actual_type.as_ref())?;
    let passed = match expectation.matcher {
        Matcher::Equal { negated, .. } => {
            let equal = scalar_equal(codegen, actual, expected)?;
            if negated {
                codegen.builder.build_not(equal, "expect.not_equal")?
            } else {
                equal
            }
        }
        Matcher::Compare { relation, .. } => {
            scalar_compare(codegen, actual, expected, relation, actual_type.as_ref())?
        }
        _ => return Err(CodegenError::new("expected a typed scalar matcher")),
    };
    let function = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("expectation has no function"))?;
    let failure = codegen
        .context
        .append_basic_block(function, "expect.failed");
    let continuation = codegen
        .context
        .append_basic_block(function, "expect.passed");
    codegen
        .builder
        .build_conditional_branch(passed, continuation, failure)?;
    codegen.builder.position_at_end(failure);
    let fallback = if actual.is_float_value() {
        CoreType::Float64
    } else if actual.is_int_value() && actual.into_int_value().get_type().get_bit_width() == 1 {
        CoreType::Boolean
    } else {
        CoreType::Int64
    };
    let identity = snapshots::failure(
        codegen,
        env,
        expectation,
        actual_type.as_ref().unwrap_or(&fallback),
        actual,
        Some(expected),
    )?;
    emit_native_abort(codegen, env, identity, &[])?;
    codegen.builder.position_at_end(continuation);
    Ok(())
}

/// Snapshot count operands without re-reading the nominal target or invoking application code.
pub fn record_count_failure<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
    actual: IntValue<'context>,
    expected: IntValue<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    snapshots::failure(
        codegen,
        env,
        expectation,
        &CoreType::UInt64,
        actual.into(),
        Some(expected.into()),
    )
}

/// Projection wrappers preserve their ordinary scalar type, including unsigned
/// casts whose LLVM integer representation alone does not retain signedness.
pub fn operand_core_type(env: &CodegenEnv<'_>, expression: &Expr) -> Option<CoreType> {
    let extent = expression.span();
    if let Some(core) = env
        .native_assertion_types
        .get(&(extent.start.offset, extent.end.offset))
    {
        return Some(core.clone());
    }
    infer_expression_core_type(env, expression).or_else(|| match *expression {
        Expr::Literal {
            value: crate::ast::LiteralValue::String(_),
            ..
        }
        | Expr::StringInterpolation { .. } => Some(CoreType::String),
        Expr::Call { ref callee, .. } => {
            if let Expr::Identifier { ref name, .. } = **callee {
                if let Some(&CoreType::Function {
                    ref return_types, ..
                }) = env.imported_signatures.get(name)
                {
                    return_types.first().cloned()
                } else {
                    None
                }
            } else {
                None
            }
        }
        Expr::Cast {
            ref target_type, ..
        } => super::functions_call::ast_type_to_core_type_for_signature(target_type).ok(),
        Expr::Parenthesized { ref expr, .. } => operand_core_type(env, expr),
        _ => None,
    })
}

/// Compare actual typed values, never their diagnostic string representations.
fn scalar_equal<'context>(
    codegen: &CodegenContext<'context>,
    actual: BasicValueEnum<'context>,
    expected: BasicValueEnum<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    if actual.get_type() != expected.get_type() {
        return Err(CodegenError::new(
            "typed equality operands have incompatible LLVM layouts",
        ));
    }
    match (actual, expected) {
        (BasicValueEnum::IntValue(actual), BasicValueEnum::IntValue(expected)) => Ok(codegen
            .builder
            .build_int_compare(IntPredicate::EQ, actual, expected, "expect.equal")?),
        (BasicValueEnum::FloatValue(actual), BasicValueEnum::FloatValue(expected)) => Ok(codegen
            .builder
            .build_float_compare(FloatPredicate::OEQ, actual, expected, "expect.equal")?),
        _ => Err(CodegenError::new(
            "operand type requires native value equality lowering; pointer identity is not equality",
        )),
    }
}

/// Preserve ordinary signed/unsigned integer and ordered floating semantics.
fn scalar_compare<'context>(
    codegen: &CodegenContext<'context>,
    actual: BasicValueEnum<'context>,
    expected: BasicValueEnum<'context>,
    relation: Comparison,
    actual_type: Option<&CoreType>,
) -> Result<IntValue<'context>, CodegenError> {
    if actual.get_type() != expected.get_type() {
        return Err(CodegenError::new(
            "numeric matcher operands have incompatible LLVM layouts",
        ));
    }
    let unsigned = actual_type.is_some_and(|core| {
        matches!(
            *core,
            CoreType::UInt8 | CoreType::UInt16 | CoreType::UInt32 | CoreType::UInt64
        )
    });
    match (actual, expected) {
        (BasicValueEnum::IntValue(actual), BasicValueEnum::IntValue(expected)) => {
            let predicate = match (relation, unsigned) {
                (Comparison::Greater, false) => IntPredicate::SGT,
                (Comparison::Less, false) => IntPredicate::SLT,
                (Comparison::AtLeast, false) => IntPredicate::SGE,
                (Comparison::AtMost, false) => IntPredicate::SLE,
                (Comparison::Greater, true) => IntPredicate::UGT,
                (Comparison::Less, true) => IntPredicate::ULT,
                (Comparison::AtLeast, true) => IntPredicate::UGE,
                (Comparison::AtMost, true) => IntPredicate::ULE,
            };
            Ok(codegen
                .builder
                .build_int_compare(predicate, actual, expected, "expect.compare")?)
        }
        (BasicValueEnum::FloatValue(actual), BasicValueEnum::FloatValue(expected)) => {
            let predicate = match relation {
                Comparison::Greater => FloatPredicate::OGT,
                Comparison::Less => FloatPredicate::OLT,
                Comparison::AtLeast => FloatPredicate::OGE,
                Comparison::AtMost => FloatPredicate::OLE,
            };
            Ok(codegen.builder.build_float_compare(
                predicate,
                actual,
                expected,
                "expect.compare",
            )?)
        }
        _ => Err(CodegenError::new(
            "numeric matcher requires ordinary numeric operands",
        )),
    }
}
