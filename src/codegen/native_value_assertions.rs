//! Composite assertions reuse ordinary lexical ownership and declared value ABI.
use super::{
    CodegenContext, CodegenEnv, CodegenError, emit_native_abort, record_assertion_failure,
};
use crate::ast::{
    Expr,
    testing::{Expectation, Matcher, SequenceRelation},
};
use crate::codegen::binding_store::initialize_binding_value;
use crate::codegen::expressions::{VariableBinding, codegen_expression};
use crate::codegen::expressions_array::load_array_length_from_value;
use crate::type_system::types::CoreType;
use inkwell::values::BasicValueEnum;

/// Equality and exact-order matching compare typed values; variants use nominal tags.
pub(super) fn lower<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expectation: &Expectation,
    core: &CoreType,
) -> Result<(), CodegenError> {
    let (expected, negated) = match expectation.matcher {
        Matcher::Equal {
            ref expected,
            negated,
        }
        | Matcher::Sequence {
            ref expected,
            relation: SequenceRelation::ExactlyInOrder,
            negated,
        } => (Some(expected), negated),
        Matcher::Empty { negated } => (None, negated),
        Matcher::Variant { .. } => (None, false),
        _ => return Err(CodegenError::new("unsupported typed array matcher")),
    };
    let _scope = env.enter_scope();
    let actual = capture(codegen, env, &expectation.actual, core)?;
    let mut passed = if let Some(expected) = expected {
        let value = capture(codegen, env, expected, core)?;
        super::value_equality::equal(codegen, env, core, actual, value)?
    } else if let Matcher::Variant {
        ref variant,
        ref nominal_type,
    } = expectation.matcher
    {
        let name = format!("{}.{}", nominal_type.path.join("."), variant);
        let tag = env.adt_variant_discriminant(&name).ok_or_else(|| {
            CodegenError::new(format!(
                "variant matcher has no genuine nominal discriminant for {name}"
            ))
        })?;
        let actual_tag = super::value_equality::variant_tag(codegen, actual)?;
        codegen.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            actual_tag,
            actual_tag
                .get_type()
                .const_int(crate::codegen::types::integer_literal_bits(tag)?, true),
            "expect.variant.matches",
        )?
    } else {
        let length = load_array_length_from_value(
            codegen,
            env,
            actual.into_pointer_value(),
            "expect.empty.array",
        )?;
        codegen.builder.build_int_compare(
            inkwell::IntPredicate::EQ,
            length,
            length.get_type().const_zero(),
            "expect.array.empty",
        )?
    };
    if negated {
        passed = codegen.builder.build_not(passed, "expect.array.negated")?;
    }
    let function = codegen
        .builder
        .get_insert_block()
        .and_then(inkwell::basic_block::BasicBlock::get_parent)
        .ok_or_else(|| CodegenError::new("array expectation has no enclosing native function"))?;
    let failed = codegen
        .context
        .append_basic_block(function, "expect.array.failed");
    let ready = codegen
        .context
        .append_basic_block(function, "expect.array.ready");
    codegen
        .builder
        .build_conditional_branch(passed, ready, failed)?;
    codegen.builder.position_at_end(failed);
    let identity = record_assertion_failure(codegen, env, expectation)?;
    emit_native_abort(codegen, env, identity, &[])?;
    codegen.builder.position_at_end(ready);
    env.exit_scope_cleanup(codegen, &[])
}

/// Hold the actual value before expected evaluation can abort or change a slot.
fn capture<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expression: &Expr,
    core: &CoreType,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let value = codegen_expression(codegen, env, expression, Some(core))?;
    let name = env.next_name("__native_assertion_value");
    let slot =
        crate::codegen::native_assertion_storage::entry_slot(codegen, value.get_type(), &name)?;
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
    initialize_binding_value(
        codegen,
        env,
        &name,
        value,
        "capture typed assertion value",
        borrowed(expression),
    )?;
    Ok(value)
}

/// Indexed nested arrays are borrowed elements, unlike newly constructed arrays.
fn borrowed(expression: &Expr) -> bool {
    match *expression {
        Expr::Identifier { .. } | Expr::Member { .. } | Expr::Index { .. } => true,
        Expr::Parenthesized { ref expr, .. }
        | Expr::BorrowArgument {
            target: ref expr, ..
        }
        | Expr::Constrain {
            value: ref expr, ..
        } => borrowed(expr),
        _ => false,
    }
}
