#![doc(hidden)]

extern crate alloc;
use crate::ast::{Expr, LiteralValue};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, current_function};
use crate::codegen::types::integer_literal_bits;
use crate::type_system::types::CoreType;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use inkwell::IntPredicate;
use inkwell::values::IntValue;

const fn integer_core_type_name(bits: u32, signed: bool) -> &'static str {
    match (bits, signed) {
        (8, true) => "int8",
        (16, true) => "int16",
        (32, true) => "int32",
        (64, true) => "int64",
        (8, false) => "uint8",
        (16, false) => "uint16",
        (32, false) => "uint32",
        _ => "uint64",
    }
}

const fn signed_bounds(bits: u32) -> (i64, i64) {
    match bits {
        8 => (-128, 127),
        16 => (-0x8000, 0x7FFF),
        32 => (-0x8000_0000, 0x7FFF_FFFF),
        _ => (i64::MIN, i64::MAX),
    }
}

const fn unsigned_max(bits: u32) -> u64 {
    match bits {
        8 => 255,
        16 => 0xFFFF,
        32 => 0xFFFF_FFFF,
        _ => u64::MAX,
    }
}

fn compile_time_cast_in_range(
    expr: &Expr,
    source_bits: u32,
    target_signed: bool,
    target_bits: u32,
) -> Option<bool> {
    let Expr::Literal {
        value: LiteralValue::Integer(number),
        ..
    } = *expr
    else {
        return None;
    };

    let value = i128::from(number);
    if source_bits > target_bits {
        if target_signed {
            let (min, max) = signed_bounds(target_bits);
            return Some(value >= i128::from(min) && value <= i128::from(max));
        }
        return Some(value >= 0 && value <= i128::from(unsigned_max(target_bits)));
    }

    if target_signed {
        let (_, max) = signed_bounds(target_bits);
        return Some(value <= i128::from(max));
    }
    Some(value >= 0)
}

#[expect(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    reason = "Integer cast guard generation depends on type widths/signedness and emits trap CFG"
)]
pub fn emit_integer_cast_range_guard<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    int_value: IntValue<'context>,
    in_bits: u32,
    out_bits: u32,
    source_signed: bool,
    target_signed: bool,
    source_core_type: Option<&CoreType>,
    target_type: &CoreType,
) -> Result<(), CodegenError> {
    let current_fn = current_function(codegen_context)?;
    let trap_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("cast.trap"));
    let ok_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("cast.ok"));

    let compile_time_in_range = compile_time_cast_in_range(expr, in_bits, target_signed, out_bits);

    let in_type = int_value.get_type();
    let in_range = if let Some(in_range_const) = compile_time_in_range {
        codegen_context
            .context
            .bool_type()
            .const_int(u64::from(in_range_const), false)
    } else if in_bits > out_bits {
        if target_signed {
            let (min, max) = signed_bounds(out_bits);
            let min_bits = integer_literal_bits(min)?;
            let max_bits = u64::try_from(max).map_err(|conversion_error| {
                CodegenError::new(format!(
                    "signed cast max conversion failed: {conversion_error}"
                ))
            })?;
            let min_const = in_type.const_int(min_bits, true);
            let max_const = in_type.const_int(max_bits, false);
            let ge_min = codegen_context.builder.build_int_compare(
                IntPredicate::SGE,
                int_value,
                min_const,
                &env.next_name("cast.ge_min"),
            )?;
            let le_max = codegen_context.builder.build_int_compare(
                IntPredicate::SLE,
                int_value,
                max_const,
                &env.next_name("cast.le_max"),
            )?;
            codegen_context
                .builder
                .build_and(ge_min, le_max, &env.next_name("cast.in_range"))?
        } else {
            let max_const = in_type.const_int(unsigned_max(out_bits), false);
            if source_signed {
                let ge_zero = codegen_context.builder.build_int_compare(
                    IntPredicate::SGE,
                    int_value,
                    in_type.const_zero(),
                    &env.next_name("cast.ge_zero"),
                )?;
                let le_max = codegen_context.builder.build_int_compare(
                    IntPredicate::SLE,
                    int_value,
                    max_const,
                    &env.next_name("cast.le_max"),
                )?;
                codegen_context.builder.build_and(
                    ge_zero,
                    le_max,
                    &env.next_name("cast.in_range"),
                )?
            } else {
                codegen_context.builder.build_int_compare(
                    IntPredicate::ULE,
                    int_value,
                    max_const,
                    &env.next_name("cast.le_max"),
                )?
            }
        }
    } else if source_signed {
        codegen_context.builder.build_int_compare(
            IntPredicate::SGE,
            int_value,
            in_type.const_zero(),
            &env.next_name("cast.ge_zero"),
        )?
    } else {
        let (_, max) = signed_bounds(out_bits);
        let max_const = in_type.const_int(
            u64::try_from(max).map_err(|conversion_error| {
                CodegenError::new(format!(
                    "same-width signed cast max conversion failed: {conversion_error}"
                ))
            })?,
            false,
        );
        codegen_context.builder.build_int_compare(
            IntPredicate::ULE,
            int_value,
            max_const,
            &env.next_name("cast.le_max"),
        )?
    };

    let _branch = codegen_context
        .builder
        .build_conditional_branch(in_range, ok_block, trap_block)?;

    codegen_context.builder.position_at_end(trap_block);
    let source_name = source_core_type.map_or_else(
        || String::from(integer_core_type_name(in_bits, source_signed)),
        ToString::to_string,
    );
    let message = format!("cast out of range: {source_name} to {target_type}");
    let msg_ptr = codegen_context
        .builder
        .build_global_string_ptr(&message, &env.next_name("cast.msg"))?
        .as_pointer_value();
    let runtime_fn = crate::codegen::functions_stdlib::declare_stdlib_function(
        codegen_context,
        "opal_runtime_error",
    )
    .ok_or_else(|| CodegenError::new(String::from("opal_runtime_error declaration missing")))?;
    let _runtime_error_call = codegen_context.builder.build_call(
        runtime_fn,
        &[msg_ptr.into()],
        &env.next_name("cast.trap.call"),
    )?;
    let _unreachable_instruction = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(ok_block);
    Ok(())
}
