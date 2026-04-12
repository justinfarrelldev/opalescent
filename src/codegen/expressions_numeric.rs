extern crate alloc;

use crate::ast::BinaryOp;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{CodegenEnv, CodegenError};
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::{BasicMetadataValueEnum, BasicValue, BasicValueEnum, IntValue};
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

    let lhs_int = lhs.into_int_value();
    let rhs_int = rhs.into_int_value();
    if env.debug_mode {
        return codegen_checked_overflow_intrinsic(
            codegen_context,
            env,
            lhs_int,
            rhs_int,
            expected_type,
            op,
        );
    }

    let value = match op {
        "add" => codegen_context
            .builder
            .build_int_add(lhs_int, rhs_int, "iadd")?
            .as_basic_value_enum(),
        "sub" => codegen_context
            .builder
            .build_int_sub(lhs_int, rhs_int, "isub")?
            .as_basic_value_enum(),
        "mul" => codegen_context
            .builder
            .build_int_mul(lhs_int, rhs_int, "imul")?
            .as_basic_value_enum(),
        _ => {
            return Err(CodegenError::new(format!("unsupported integer op '{op}'")));
        }
    };
    Ok(value)
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
    super::expressions::emit_trap_call(codegen_context)?;
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
    let lhs_int = lhs.into_int_value();
    let rhs_int = rhs.into_int_value();
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let value = if expected_type.is_none_or(is_signed_core_type) {
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
    let lhs_int = lhs.into_int_value();
    let rhs_int = rhs.into_int_value();
    super::expressions::emit_div_by_zero_check(codegen_context, env, rhs_int)?;
    let value = if expected_type.is_none_or(is_signed_core_type) {
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
    expected_type: Option<&CoreType>,
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

    let signed = expected_type.is_none_or(is_signed_core_type);
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
            )))
        }
    };

    Ok(codegen_context
        .builder
        .build_int_compare(pred, lhs.into_int_value(), rhs.into_int_value(), "icmp")?
        .as_basic_value_enum())
}

/// Returns true when the core type is a signed integer type (i8, i16, i32, i64).
const fn is_signed_core_type(core_type: &CoreType) -> bool {
    matches!(
        *core_type,
        CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64
    )
}
