//! Typed value equality, never diagnostic stringification or pointer identity.
use super::{CodegenContext, CodegenEnv, CodegenError, scalar_equal};
use crate::codegen::expressions_array::{
    load_array_data_ptr_for_element_type, load_array_length_from_value,
};
use crate::codegen::native_callback_boundary::c_function;
use crate::type_system::types::CoreType;
use inkwell::IntPredicate;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};

/// Dispatch exclusively through the checked ordinary operand type.
pub(super) fn equal<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    core: &CoreType,
    actual: BasicValueEnum<'context>,
    expected: BasicValueEnum<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    match *core {
        CoreType::String => {
            let pointer = actual.into_pointer_value().get_type();
            let helper = c_function(
                codegen,
                "opal_test_string_matches",
                codegen.context.i32_type().fn_type(
                    &[
                        codegen.context.i32_type().into(),
                        pointer.into(),
                        pointer.into(),
                    ],
                    false,
                ),
            )?;
            let result = codegen
                .builder
                .build_call(
                    helper,
                    &[
                        codegen.context.i32_type().const_zero().into(),
                        actual.into(),
                        expected.into(),
                    ],
                    "expect.element.string",
                )?
                .try_as_basic_value()
                .basic()
                .ok_or_else(|| CodegenError::new("string equality returned no result"))?
                .into_int_value();
            Ok(codegen.builder.build_int_compare(
                IntPredicate::NE,
                result,
                result.get_type().const_zero(),
                "expect.element.equal",
            )?)
        }
        CoreType::Array(ref element) => array_equal(
            codegen,
            env,
            element,
            actual.into_pointer_value(),
            expected.into_pointer_value(),
        ),
        _ => scalar_equal(codegen, actual, expected),
    }
}

/// Length is checked before elements; order and multiplicity follow the index loop.
fn array_equal<'context>(
    codegen: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    element: &CoreType,
    actual: PointerValue<'context>,
    expected: PointerValue<'context>,
) -> Result<IntValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let actual_length = load_array_length_from_value(codegen, env, actual, "expect.actual.array")?;
    let expected_length =
        load_array_length_from_value(codegen, env, expected, "expect.expected.array")?;
    let actual_data =
        load_array_data_ptr_for_element_type(codegen, env, actual, element, "expect.actual.array")?;
    let expected_data = load_array_data_ptr_for_element_type(
        codegen,
        env,
        expected,
        element,
        "expect.expected.array",
    )?;
    let entry = codegen
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new("array equality has no entry block"))?;
    let function = entry
        .get_parent()
        .ok_or_else(|| CodegenError::new("array equality has no function"))?;
    let loop_head = llvm.append_basic_block(function, "expect.array.loop");
    let compare = llvm.append_basic_block(function, "expect.array.element");
    let increment = llvm.append_basic_block(function, "expect.array.next");
    let different = llvm.append_basic_block(function, "expect.array.different");
    let same = llvm.append_basic_block(function, "expect.array.same");
    let merge = llvm.append_basic_block(function, "expect.array.done");
    let lengths_match = codegen.builder.build_int_compare(
        IntPredicate::EQ,
        actual_length,
        expected_length,
        "expect.array.same_length",
    )?;
    codegen
        .builder
        .build_conditional_branch(lengths_match, loop_head, different)?;
    codegen.builder.position_at_end(loop_head);
    let index = codegen
        .builder
        .build_phi(actual_length.get_type(), "expect.array.index")?;
    index.add_incoming(&[(&actual_length.get_type().const_zero(), entry)]);
    let current = index.as_basic_value().into_int_value();
    let in_range = codegen.builder.build_int_compare(
        IntPredicate::ULT,
        current,
        actual_length,
        "expect.array.in_range",
    )?;
    codegen
        .builder
        .build_conditional_branch(in_range, compare, same)?;
    codegen.builder.position_at_end(compare);
    // SAFETY: Equal lengths and the loop condition prove both element indices
    // are in range in arrays with the checked ordinary element representation.
    let actual_slot = unsafe {
        codegen
            .builder
            .build_in_bounds_gep(actual_data, &[current], "expect.actual.element")?
    };
    // SAFETY: The same checked length/index proof applies to the expected array.
    let expected_slot = unsafe {
        codegen
            .builder
            .build_in_bounds_gep(expected_data, &[current], "expect.expected.element")?
    };
    let actual_value = codegen
        .builder
        .build_load(actual_slot, "expect.actual.value")?;
    let expected_value = codegen
        .builder
        .build_load(expected_slot, "expect.expected.value")?;
    let matches = equal(codegen, env, element, actual_value, expected_value)?;
    codegen
        .builder
        .build_conditional_branch(matches, increment, different)?;
    codegen.builder.position_at_end(increment);
    let next = codegen.builder.build_int_add(
        current,
        current.get_type().const_int(1, false),
        "expect.array.increment",
    )?;
    index.add_incoming(&[(&next, increment)]);
    codegen.builder.build_unconditional_branch(loop_head)?;
    codegen.builder.position_at_end(same);
    codegen.builder.build_unconditional_branch(merge)?;
    codegen.builder.position_at_end(different);
    codegen.builder.build_unconditional_branch(merge)?;
    codegen.builder.position_at_end(merge);
    let result = codegen
        .builder
        .build_phi(llvm.bool_type(), "expect.array.equal")?;
    result.add_incoming(&[
        (&llvm.bool_type().const_int(1, false), same),
        (&llvm.bool_type().const_zero(), different),
    ]);
    Ok(result.as_basic_value().into_int_value())
}
