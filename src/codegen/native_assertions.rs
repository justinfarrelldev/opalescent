//! Typed native assertion lowering using normal expression and scope machinery.
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::{CodegenEnv, codegen_expression};
use super::expressions_array::infer_expression_core_type;
use super::native_abort::emit_native_abort;
use crate::ast::Expr;
use crate::ast::testing::{Comparison, Expectation, Matcher, TestingStatement};
use crate::type_system::types::CoreType;
use inkwell::values::{BasicValueEnum, IntValue};
use inkwell::{AddressSpace, FloatPredicate, IntPredicate};
#[path = "native_string_assertions.rs"]
mod strings;

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
        _ => Err(CodegenError::new(
            "typed double operation requires native interception lowering",
        )),
    }
}

/// Evaluate ordinary scalar operands exactly once and branch on their result.
fn lower_expectation<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<(), CodegenError> {
    if expectation.eventually_seconds.is_some() {
        return Err(CodegenError::new(
            "eventual assertion requires native polling lowering",
        ));
    }
    if operand_core_type(env, &expectation.actual) == Some(CoreType::String) {
        return strings::lower(codegen, env, expectation);
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
    let actual_type = operand_core_type(env, &expectation.actual);
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
    let identity = record_assertion_failure(codegen, env, expectation)?;
    emit_native_abort(codegen, env, identity, &[])?;
    codegen.builder.position_at_end(continuation);
    Ok(())
}

/// Projection wrappers preserve their ordinary scalar type, including unsigned
/// casts whose LLVM integer representation alone does not retain signedness.
fn operand_core_type(env: &CodegenEnv<'_>, expression: &Expr) -> Option<CoreType> {
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

/// Record original source coordinates without executing operand expressions again.
fn record_assertion_failure<'context>(
    codegen: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    expectation: &Expectation,
) -> Result<IntValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let signature = llvm.i64_type().fn_type(
        &[
            llvm.i32_type().into(),
            pointer.into(),
            pointer.into(),
            llvm.i64_type().into(),
            llvm.i64_type().into(),
        ],
        false,
    );
    let name = "opal_test_record_failure";
    let record = codegen
        .module
        .get_function(name)
        .unwrap_or_else(|| codegen.module.add_function(name, signature, None));
    if record.get_type() != signature {
        return Err(CodegenError::new(
            "native failure runtime ABI declaration mismatch",
        ));
    }
    let message = codegen
        .builder
        .build_global_string_ptr("expectation failed", "expect.message")?
        .as_pointer_value();
    let file = codegen
        .builder
        .build_global_string_ptr(&env.current_source_path, "expect.file")?
        .as_pointer_value();
    let offset = u64::try_from(expectation.span.start.offset)
        .map_err(|error| CodegenError::new(error.to_string()))?;
    let length = u64::try_from(
        expectation
            .span
            .end
            .offset
            .saturating_sub(expectation.span.start.offset),
    )
    .map_err(|error| CodegenError::new(error.to_string()))?;
    Ok(codegen
        .builder
        .build_call(
            record,
            &[
                llvm.i32_type().const_int(1, false).into(),
                message.into(),
                file.into(),
                llvm.i64_type().const_int(offset, false).into(),
                llvm.i64_type().const_int(length, false).into(),
            ],
            "expect.failure",
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("native failure runtime did not return an identity"))?
        .into_int_value())
}
