extern crate alloc;

use crate::ast::BinaryOp;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use crate::codegen::types::is_signed_core_type;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::module::Linkage;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue,
};
use inkwell::{FloatPredicate, IntPredicate};

pub fn codegen_numeric_binop<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
    op: &str,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if lhs.is_float_value() {
        let value = match op {
            "add" => codegen_context
                .builder
                .build_float_add(lhs.into_float_value(), rhs.into_float_value(), "fadd")?
                .as_basic_value_enum(),
            "sub" => codegen_context
                .builder
                .build_float_sub(lhs.into_float_value(), rhs.into_float_value(), "fsub")?
                .as_basic_value_enum(),
            "mul" => codegen_context
                .builder
                .build_float_mul(lhs.into_float_value(), rhs.into_float_value(), "fmul")?
                .as_basic_value_enum(),
            _ => {
                return Err(CodegenError::new(format!("unsupported float op '{op}'")));
            }
        };
        return Ok(value);
    }

    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    // Always use checked overflow intrinsics in all build modes
    codegen_checked_overflow_intrinsic(codegen_context, env, lhs_int, rhs_int, expected_type, op)
}

pub fn codegen_checked_overflow_intrinsic<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: IntValue<'context>,
    rhs: IntValue<'context>,
    expected_type: Option<&CoreType>,
    op: &str,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs, rhs) = normalize_int_operands(codegen_context, lhs, rhs, signed)?;
    let op_family = if signed { "s" } else { "u" };
    let bits = lhs.get_type().get_bit_width();
    let intrinsic_name = format!("llvm.{op_family}{op}.with.overflow.i{bits}");

    let function = codegen_context
        .module
        .get_function(&intrinsic_name)
        .map_or_else(
            || {
                let int_type = lhs.get_type();
                let result_type = codegen_context.context.struct_type(
                    &[int_type.into(), codegen_context.context.bool_type().into()],
                    false,
                );
                let params: [BasicMetadataTypeEnum<'context>; 2] =
                    [int_type.into(), int_type.into()];
                let function_type = result_type.fn_type(&params, false);
                codegen_context
                    .module
                    .add_function(&intrinsic_name, function_type, None)
            },
            |existing| existing,
        );

    let args: [BasicMetadataValueEnum<'context>; 2] = [lhs.into(), rhs.into()];
    let call =
        codegen_context
            .builder
            .build_call(function, &args, &env.next_name("overflow.call"))?;
    let aggregate = call
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("overflow intrinsic returned void")))?
        .into_struct_value();
    let result = codegen_context
        .builder
        .build_extract_value(aggregate, 0, &env.next_name("overflow.value"))?
        .into_int_value();
    let flag = codegen_context
        .builder
        .build_extract_value(aggregate, 1, &env.next_name("overflow.flag"))?
        .into_int_value();

    let current_fn = super::expressions::current_function(codegen_context)?;
    let trap_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("overflow.trap"));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("overflow.cont"));
    let _branch = codegen_context
        .builder
        .build_conditional_branch(flag, trap_block, cont_block)?;

    codegen_context.builder.position_at_end(trap_block);
    let msg = codegen_context
        .builder
        .build_global_string_ptr("integer overflow", &env.next_name("ovr.msg"))?
        .as_pointer_value();
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let trap_args: [BasicMetadataValueEnum<'context>; 1] = [msg.into()];
    let _call =
        codegen_context
            .builder
            .build_call(runtime_fn, &trap_args, &env.next_name("ovr.trap"))?;
    let _unreachable = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(cont_block);
    Ok(result.as_basic_value_enum())
}

pub fn codegen_div<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if lhs.is_float_value() {
        return Ok(codegen_context
            .builder
            .build_float_div(lhs.into_float_value(), rhs.into_float_value(), "fdiv")?
            .as_basic_value_enum());
    }
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let value = if signed {
        codegen_context
            .builder
            .build_int_signed_div(lhs_int, rhs_int, "sdiv")?
            .as_basic_value_enum()
    } else {
        codegen_context
            .builder
            .build_int_unsigned_div(lhs_int, rhs_int, "udiv")?
            .as_basic_value_enum()
    };
    Ok(value)
}

pub fn codegen_rem<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let value = if signed {
        codegen_context
            .builder
            .build_int_signed_rem(lhs_int, rhs_int, "srem")?
            .as_basic_value_enum()
    } else {
        codegen_context
            .builder
            .build_int_unsigned_rem(lhs_int, rhs_int, "urem")?
            .as_basic_value_enum()
    };
    Ok(value)
}

pub fn codegen_cmp<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    operator: &BinaryOp,
    operand_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if lhs.is_float_value() {
        let pred = match *operator {
            BinaryOp::Equal | BinaryOp::Is => FloatPredicate::OEQ,
            BinaryOp::NotEqual | BinaryOp::IsNot => FloatPredicate::ONE,
            BinaryOp::Less => FloatPredicate::OLT,
            BinaryOp::LessEqual => FloatPredicate::OLE,
            BinaryOp::Greater => FloatPredicate::OGT,
            BinaryOp::GreaterEqual => FloatPredicate::OGE,
            _ => return Err(CodegenError::new(String::from("unsupported float compare"))),
        };
        return Ok(codegen_context
            .builder
            .build_float_compare(pred, lhs.into_float_value(), rhs.into_float_value(), "fcmp")?
            .as_basic_value_enum());
    }

    if lhs.is_pointer_value() {
        return codegen_pointer_cmp(codegen_context, lhs, rhs, operator, operand_type);
    }

    let signed = operand_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    let pred = match *operator {
        BinaryOp::Equal | BinaryOp::Is => IntPredicate::EQ,
        BinaryOp::NotEqual | BinaryOp::IsNot => IntPredicate::NE,
        BinaryOp::Less => {
            if signed {
                IntPredicate::SLT
            } else {
                IntPredicate::ULT
            }
        }
        BinaryOp::LessEqual => {
            if signed {
                IntPredicate::SLE
            } else {
                IntPredicate::ULE
            }
        }
        BinaryOp::Greater => {
            if signed {
                IntPredicate::SGT
            } else {
                IntPredicate::UGT
            }
        }
        BinaryOp::GreaterEqual => {
            if signed {
                IntPredicate::SGE
            } else {
                IntPredicate::UGE
            }
        }
        _ => {
            return Err(CodegenError::new(String::from(
                "unsupported integer compare",
            )));
        }
    };

    Ok(codegen_context
        .builder
        .build_int_compare(pred, lhs_int, rhs_int, "icmp")?
        .as_basic_value_enum())
}

/// Declares or retrieves the strcmp external function declaration from the LLVM module.
fn ensure_strcmp_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let i8_ptr = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i32_type = codegen_context.context.i32_type();
    let fn_type = i32_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false);
    codegen_context
        .module
        .get_function("strcmp")
        .unwrap_or_else(|| {
            codegen_context
                .module
                .add_function("strcmp", fn_type, Some(Linkage::External))
        })
}

/// Lower pointer comparisons for supported pointer-typed operands.
///
/// String pointers use `strcmp` and compare the result against zero.
/// Function pointers are converted to integers and compared via integer predicates.
fn codegen_pointer_cmp<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    operator: &BinaryOp,
    operand_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let lhs_ptr = lhs.into_pointer_value();
    let rhs_ptr = rhs.into_pointer_value();

    // Strings use strcmp; function pointers use direct pointer icmp.
    let is_string = matches!(operand_type, Some(&CoreType::String) | None);

    if is_string {
        let strcmp_fn = ensure_strcmp_function(codegen_context);
        let strcmp_call = codegen_context.builder.build_call(
            strcmp_fn,
            &[lhs_ptr.into(), rhs_ptr.into()],
            "strcmp_result",
        )?;
        let strcmp_result = strcmp_call
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| CodegenError::new(String::from("strcmp returned void")))?
            .into_int_value();

        let zero = codegen_context.context.i32_type().const_int(0, false);
        let pred = match *operator {
            BinaryOp::Equal | BinaryOp::Is => IntPredicate::EQ,
            BinaryOp::NotEqual | BinaryOp::IsNot => IntPredicate::NE,
            _ => {
                return Err(CodegenError::new(String::from(
                    "unsupported string comparison operator",
                )));
            }
        };

        Ok(codegen_context
            .builder
            .build_int_compare(pred, strcmp_result, zero, "str_cmp")?
            .as_basic_value_enum())
    } else {
        // Function pointer comparison: ptrtoint + icmp.
        let pred = match *operator {
            BinaryOp::Equal | BinaryOp::Is => IntPredicate::EQ,
            BinaryOp::NotEqual | BinaryOp::IsNot => IntPredicate::NE,
            _ => {
                return Err(CodegenError::new(String::from(
                    "unsupported pointer comparison operator",
                )));
            }
        };

        let ptr_int_type = codegen_context.context.i64_type();
        let lhs_int =
            codegen_context
                .builder
                .build_ptr_to_int(lhs_ptr, ptr_int_type, "lhs_ptr_int")?;
        let rhs_int =
            codegen_context
                .builder
                .build_ptr_to_int(rhs_ptr, ptr_int_type, "rhs_ptr_int")?;

        Ok(codegen_context
            .builder
            .build_int_compare(pred, lhs_int, rhs_int, "ptr_cmp")?
            .as_basic_value_enum())
    }
}

/// Normalize integer operands to matching bit widths for LLVM integer ops.
fn normalize_int_operands<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: IntValue<'context>,
    rhs: IntValue<'context>,
    signed: bool,
) -> Result<(IntValue<'context>, IntValue<'context>), CodegenError> {
    let lhs_bits = lhs.get_type().get_bit_width();
    let rhs_bits = rhs.get_type().get_bit_width();
    if lhs_bits == rhs_bits {
        return Ok((lhs, rhs));
    }

    if lhs_bits > rhs_bits {
        let widened_rhs = if signed {
            codegen_context
                .builder
                .build_int_s_extend(rhs, lhs.get_type(), "int.widen.rhs")?
        } else {
            codegen_context
                .builder
                .build_int_z_extend(rhs, lhs.get_type(), "int.widen.rhs")?
        };
        Ok((lhs, widened_rhs))
    } else {
        let widened_lhs = if signed {
            codegen_context
                .builder
                .build_int_s_extend(lhs, rhs.get_type(), "int.widen.lhs")?
        } else {
            codegen_context
                .builder
                .build_int_z_extend(lhs, rhs.get_type(), "int.widen.lhs")?
        };
        Ok((widened_lhs, rhs))
    }
}

/// Lower `base ^ exp` for integer types using a loop-based repeated multiplication.
///
/// The exponent is treated as a non-negative i64. Negative exponents produce 0
/// (integer power with negative exponent is 0 for any non-zero base in integer arithmetic).
/// Emits LLVM basic blocks: entry → `loop_header` → `loop_body` → done.
pub fn codegen_power<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if lhs.is_float_value() {
        return codegen_float_power(codegen_context, lhs, rhs);
    }
    codegen_int_power(codegen_context, env, lhs, rhs, expected_type)
}

/// Lower `base ^ exp` for float types using the `llvm.pow` intrinsic.
fn codegen_float_power<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let lhs_float = lhs.into_float_value();
    let rhs_float = rhs.into_float_value();
    let bits = lhs_float.get_type().get_bit_width();
    let intrinsic_name = format!("llvm.pow.f{bits}");

    let float_type = lhs_float.get_type();
    let fn_type = float_type.fn_type(
        &[
            BasicMetadataTypeEnum::from(float_type),
            BasicMetadataTypeEnum::from(float_type),
        ],
        false,
    );
    let function = codegen_context
        .module
        .get_function(&intrinsic_name)
        .unwrap_or_else(|| {
            codegen_context
                .module
                .add_function(&intrinsic_name, fn_type, None)
        });

    let args: [BasicMetadataValueEnum<'context>; 2] = [lhs_float.into(), rhs_float.into()];
    let call = codegen_context
        .builder
        .build_call(function, &args, "fpow")?;
    call.try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new(String::from("llvm.pow returned void")))
}

/// Lower `base ^ exp` for integer types using PHI-node loop: acc=1, count down exponent, multiply each iteration.
fn codegen_int_power<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (base, exponent) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    let int_type = base.get_type();
    let zero = int_type.const_zero();
    let one = int_type.const_int(1, false);

    let entry_block = codegen_context
        .builder
        .get_insert_block()
        .ok_or_else(|| CodegenError::new(String::from("no insert block for power loop")))?;

    let current_fn = super::expressions::current_function(codegen_context)?;
    let loop_header = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("pow.header"));
    let loop_body = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("pow.body"));
    let exit_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("pow.exit"));

    codegen_context
        .builder
        .build_unconditional_branch(loop_header)?;

    codegen_context.builder.position_at_end(loop_header);
    let remaining_phi = codegen_context
        .builder
        .build_phi(int_type, "pow.remaining")?;
    let acc_phi = codegen_context.builder.build_phi(int_type, "pow.acc")?;
    let compare_predicate = if signed {
        IntPredicate::SLE
    } else {
        IntPredicate::ULE
    };
    let cond = codegen_context.builder.build_int_compare(
        compare_predicate,
        remaining_phi.as_basic_value().into_int_value(),
        zero,
        "pow.done",
    )?;
    codegen_context
        .builder
        .build_conditional_branch(cond, exit_block, loop_body)?;

    codegen_context.builder.position_at_end(loop_body);
    let new_acc = codegen_context.builder.build_int_mul(
        acc_phi.as_basic_value().into_int_value(),
        base,
        "pow.mul",
    )?;
    let next_remaining = codegen_context.builder.build_int_sub(
        remaining_phi.as_basic_value().into_int_value(),
        one,
        "pow.dec",
    )?;
    codegen_context
        .builder
        .build_unconditional_branch(loop_header)?;

    remaining_phi.add_incoming(&[(&exponent, entry_block), (&next_remaining, loop_body)]);
    acc_phi.add_incoming(&[(&one, entry_block), (&new_acc, loop_body)]);

    codegen_context.builder.position_at_end(exit_block);
    Ok(acc_phi.as_basic_value())
}

/// Lower `a div_euclid b` — floor division where the result rounds toward negative infinity.
///
/// For unsigned operands this is identical to truncating division.
/// For signed operands the result is adjusted by -1 when the remainder is
/// non-zero and has a different sign from the divisor, matching Rust's
/// `i64::div_euclid` semantics.
pub fn codegen_div_euclid<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let int_type = lhs_int.get_type();

    // Unsigned: plain truncating division is already floor division.
    if !signed {
        return Ok(codegen_context
            .builder
            .build_int_unsigned_div(lhs_int, rhs_int, "diveuc.uq")?
            .as_basic_value_enum());
    }

    let q = codegen_context
        .builder
        .build_int_signed_div(lhs_int, rhs_int, "diveuc.q")?;
    let r = codegen_context
        .builder
        .build_int_signed_rem(lhs_int, rhs_int, "diveuc.r")?;
    let zero = int_type.const_zero();
    let neg_one = int_type.const_all_ones();

    // Adjust q by -1 when remainder is non-zero AND (r XOR b) < 0  (i.e. signs differ).
    let r_nonzero =
        codegen_context
            .builder
            .build_int_compare(IntPredicate::NE, r, zero, "diveuc.r_ne_zero")?;
    let r_xor_b = codegen_context
        .builder
        .build_xor(r, rhs_int, "diveuc.r_xor_b")?;
    let signs_differ = codegen_context.builder.build_int_compare(
        IntPredicate::SLT,
        r_xor_b,
        zero,
        "diveuc.signs_differ",
    )?;
    let need_adjust =
        codegen_context
            .builder
            .build_and(r_nonzero, signs_differ, "diveuc.need_adjust")?;
    let adjust =
        codegen_context
            .builder
            .build_select(need_adjust, neg_one, zero, "diveuc.adjust")?;
    Ok(codegen_context
        .builder
        .build_int_add(q, adjust.into_int_value(), "diveuc.result")?
        .as_basic_value_enum())
}

/// Lower `a mod_euclid b` — remainder that is always non-negative (matches Rust's
/// `i64::rem_euclid` semantics).
///
/// For unsigned operands this is the ordinary unsigned remainder.
/// For signed operands the result is `r + abs(b)` when `r < 0`, otherwise `r`.
pub fn codegen_mod_euclid<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let signed = expected_type.is_none_or(is_signed_core_type);
    let (lhs_int, rhs_int) = normalize_int_operands(
        codegen_context,
        lhs.into_int_value(),
        rhs.into_int_value(),
        signed,
    )?;
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let int_type = lhs_int.get_type();
    let zero = int_type.const_zero();

    // Unsigned: ordinary remainder is always non-negative.
    if !signed {
        return Ok(codegen_context
            .builder
            .build_int_unsigned_rem(lhs_int, rhs_int, "modeuc.urem")?
            .as_basic_value_enum());
    }

    let r = codegen_context
        .builder
        .build_int_signed_rem(lhs_int, rhs_int, "modeuc.r")?;
    let r_negative =
        codegen_context
            .builder
            .build_int_compare(IntPredicate::SLT, r, zero, "modeuc.r_neg")?;
    let b_neg = codegen_context
        .builder
        .build_int_neg(rhs_int, "modeuc.neg_b")?;
    let b_negative = codegen_context.builder.build_int_compare(
        IntPredicate::SLT,
        rhs_int,
        zero,
        "modeuc.b_neg",
    )?;
    let abs_b = codegen_context
        .builder
        .build_select(b_negative, b_neg, rhs_int, "modeuc.abs_b")?
        .into_int_value();
    let r_adjusted = codegen_context
        .builder
        .build_int_add(r, abs_b, "modeuc.adjusted")?;
    Ok(codegen_context
        .builder
        .build_select(r_negative, r_adjusted, r, "modeuc.result")?
        .into_int_value()
        .as_basic_value_enum())
}
