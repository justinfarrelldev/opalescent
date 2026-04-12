#![doc(hidden)]

extern crate alloc;
use crate::ast::{BinaryOp, Expr, LiteralValue, Type, UnaryOp};
use crate::codegen::adts::{
    codegen_constructor_expression, codegen_field_access_expression, codegen_match_expression,
};
use crate::codegen::context::CodegenContext;
use crate::codegen::control_flow::codegen_if_expression;
use crate::codegen::expressions_numeric::{
    codegen_cmp, codegen_div, codegen_numeric_binop, codegen_rem,
};
use crate::codegen::expressions_string::codegen_string_interpolation;
use crate::codegen::functions::{
    codegen_call_expression, codegen_guard_expression, codegen_propagate_expression,
};
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use inkwell::builder::BuilderError;
use inkwell::types::BasicType;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue,
};
use inkwell::IntPredicate;

#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
}

impl CodegenError {
    #[must_use]
    pub const fn new(message: String) -> Self {
        Self { message }
    }
}

impl core::fmt::Display for CodegenError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl core::error::Error for CodegenError {}

impl From<BuilderError> for CodegenError {
    fn from(value: BuilderError) -> Self {
        Self::new(format!("LLVM builder error: {value}"))
    }
}

#[derive(Debug, Clone)]
pub struct VariableBinding<'context> {
    pub alloca: PointerValue<'context>,
    pub core_type: CoreType,
}

pub struct CodegenEnv<'context> {
    pub variables: BTreeMap<String, VariableBinding<'context>>,
    pub imported_functions: BTreeMap<String, String>,
    pub variable_field_indices: BTreeMap<String, BTreeMap<String, u32>>,
    pub emitted_specializations: BTreeMap<(String, Vec<String>), FunctionValue<'context>>,
    pub debug_mode: bool,
    pub temp_counter: usize,
}

impl CodegenEnv<'_> {
    #[must_use]
    pub const fn new(debug_mode: bool) -> Self {
        Self {
            variables: BTreeMap::new(),
            imported_functions: BTreeMap::new(),
            variable_field_indices: BTreeMap::new(),
            emitted_specializations: BTreeMap::new(),
            debug_mode,
            temp_counter: 0,
        }
    }

    pub fn next_name(&mut self, base: &str) -> String {
        let index = self.temp_counter;
        self.temp_counter = self.temp_counter.saturating_add(1);
        format!("{base}.{index}")
    }
}

pub fn codegen_expression<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match *expr {
        Expr::Literal { ref value, .. } => {
            codegen_literal(codegen_context, env, value, expected_type)
        }
        Expr::Identifier { ref name, .. } => codegen_identifier(codegen_context, env, name),
        Expr::Parenthesized { ref expr, .. } => {
            codegen_expression(codegen_context, env, expr, expected_type)
        }
        Expr::Binary {
            ref left,
            ref operator,
            ref right,
            ..
        } => codegen_binary(codegen_context, env, left, operator, right, expected_type),
        Expr::Unary {
            ref operator,
            ref operand,
            ..
        } => codegen_unary(codegen_context, env, operator, operand, expected_type),
        Expr::Cast {
            ref expr,
            ref target_type,
            ..
        } => codegen_cast(codegen_context, env, expr, target_type),
        Expr::Array { ref elements, .. } => {
            codegen_array_literal(codegen_context, env, elements.as_slice(), expected_type)
        }
        Expr::Index {
            ref object,
            ref index,
            ..
        } => codegen_array_access(codegen_context, env, object, index, expected_type),
        Expr::Call {
            ref callee,
            ref generic_args,
            ref args,
            ..
        } => codegen_call_expression(
            codegen_context,
            env,
            callee.as_ref(),
            generic_args.as_deref(),
            args.as_slice(),
            expected_type,
        ),
        Expr::Constructor { .. } => codegen_constructor_expression(codegen_context, env, expr),
        Expr::Match { .. } => codegen_match_expression(codegen_context, env, expr),
        Expr::Loop { .. } => Err(CodegenError::new(String::from(
            "loop expressions are lowered in statement context",
        ))),
        Expr::Member { .. } => codegen_field_access_expression(codegen_context, env, expr),
        Expr::If {
            ref condition,
            ref then_branch,
            ref else_branch,
            ..
        } => codegen_if_expression(
            codegen_context,
            env,
            condition.as_ref(),
            then_branch.as_ref(),
            else_branch.as_deref(),
        ),
        Expr::Guard {
            ref expr,
            ref binding_name,
            ..
        } => codegen_guard_expression(codegen_context, env, expr.as_ref(), binding_name.as_str()),
        Expr::Propagate { ref call, .. } => {
            codegen_propagate_expression(codegen_context, env, call.as_ref())
        }
        Expr::StringInterpolation { ref parts, .. } => {
            codegen_string_interpolation(codegen_context, env, parts.as_slice())
        }
        _ => Err(CodegenError::new(String::from(
            "unsupported expression kind for task 22",
        ))),
    }
}

fn codegen_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    literal: &LiteralValue,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    match *literal {
        LiteralValue::Integer(number) => {
            let int_type = match expected_type {
                Some(core_type) => match *core_type {
                    CoreType::Int8 | CoreType::UInt8 => codegen_context.context.i8_type(),
                    CoreType::Int16 | CoreType::UInt16 => codegen_context.context.i16_type(),
                    CoreType::Int32 | CoreType::UInt32 => codegen_context.context.i32_type(),
                    CoreType::Int64 | CoreType::UInt64 => codegen_context.context.i64_type(),
                    _ => {
                        return Err(CodegenError::new(format!(
                            "integer literal cannot target type {core_type}"
                        )));
                    }
                },
                None => codegen_context.context.i64_type(),
            };
            let bits = integer_literal_bits(number)?;
            Ok(int_type.const_int(bits, true).as_basic_value_enum())
        }
        LiteralValue::Float(number) => expected_type.map_or_else(
            || {
                Ok(codegen_context
                    .context
                    .f64_type()
                    .const_float(number)
                    .as_basic_value_enum())
            },
            |core_type| match *core_type {
                CoreType::Float32 => Ok(codegen_context
                    .context
                    .f32_type()
                    .const_float(number)
                    .as_basic_value_enum()),
                CoreType::Float64 => Ok(codegen_context
                    .context
                    .f64_type()
                    .const_float(number)
                    .as_basic_value_enum()),
                _ => Err(CodegenError::new(format!(
                    "float literal cannot target type {core_type}"
                ))),
            },
        ),
        LiteralValue::Boolean(value) => Ok(codegen_context
            .context
            .bool_type()
            .const_int(u64::from(value), false)
            .as_basic_value_enum()),
        LiteralValue::String(ref text) => {
            let name = env.next_name("str");
            let ptr = codegen_context
                .builder
                .build_global_string_ptr(text, &name)?
                .as_pointer_value();
            Ok(ptr.as_basic_value_enum())
        }
        LiteralValue::Void => Ok(codegen_context
            .context
            .struct_type(&[], false)
            .const_zero()
            .as_basic_value_enum()),
    }
}

fn codegen_identifier<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &CodegenEnv<'context>,
    name: &str,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let Some(binding) = env.variables.get(name) else {
        return Err(CodegenError::new(format!("unknown variable '{name}'")));
    };
    Ok(codegen_context.builder.build_load(binding.alloca, name)?)
}

fn codegen_binary<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    left: &Expr,
    operator: &BinaryOp,
    right: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let lhs = codegen_expression(codegen_context, env, left, expected_type)?;
    let rhs = codegen_expression(codegen_context, env, right, expected_type)?;

    match *operator {
        BinaryOp::Add => codegen_add(codegen_context, env, lhs, rhs, expected_type),
        BinaryOp::Subtract => codegen_sub(codegen_context, env, lhs, rhs, expected_type),
        BinaryOp::Multiply => codegen_mul(codegen_context, env, lhs, rhs, expected_type),
        BinaryOp::Divide => codegen_div(codegen_context, env, lhs, rhs, expected_type),
        BinaryOp::Modulo => codegen_rem(codegen_context, env, lhs, rhs, expected_type),
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual
        | BinaryOp::Is
        | BinaryOp::IsNot => codegen_cmp(codegen_context, lhs, rhs, operator, expected_type),
        BinaryOp::And | BinaryOp::Or | BinaryOp::Xor => {
            codegen_bool(codegen_context, lhs, rhs, operator)
        }
        BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor => {
            codegen_bitwise(codegen_context, lhs, rhs, operator)
        }
        BinaryOp::BitShiftLeft | BinaryOp::BitShiftRight | BinaryOp::BitUnsignedShiftRight => {
            codegen_shift(codegen_context, lhs, rhs, operator)
        }
        BinaryOp::Power | BinaryOp::Assign => Err(CodegenError::new(format!(
            "binary operator {operator} is unsupported in task 22"
        ))),
    }
}

fn codegen_unary<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    operator: &UnaryOp,
    operand: &Expr,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let value = codegen_expression(codegen_context, env, operand, expected_type)?;
    match *operator {
        UnaryOp::Negate => {
            if value.is_float_value() {
                Ok(codegen_context
                    .builder
                    .build_float_neg(value.into_float_value(), "fneg")?
                    .as_basic_value_enum())
            } else {
                Ok(codegen_context
                    .builder
                    .build_int_neg(value.into_int_value(), "ineg")?
                    .as_basic_value_enum())
            }
        }
        UnaryOp::Not | UnaryOp::BitNot => Ok(codegen_context
            .builder
            .build_not(value.into_int_value(), "inot")?
            .as_basic_value_enum()),
        UnaryOp::Plus => Ok(value),
    }
}

fn codegen_cast<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    target: &Type,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let target_type = ast_type_to_core_type(target)?;
    let value = codegen_expression(codegen_context, env, expr, None)?;

    if value.is_int_value() {
        let int_value = value.into_int_value();
        if is_integer_core_type(&target_type) {
            let out_type = integer_type_for(codegen_context, &target_type)?;
            let in_bits = int_value.get_type().get_bit_width();
            let out_bits = out_type.get_bit_width();
            let casted = match in_bits.cmp(&out_bits) {
                core::cmp::Ordering::Greater => codegen_context
                    .builder
                    .build_int_truncate(int_value, out_type, "trunc")?,
                core::cmp::Ordering::Less => {
                    if is_signed_core_type(&target_type) {
                        codegen_context
                            .builder
                            .build_int_s_extend(int_value, out_type, "sext")?
                    } else {
                        codegen_context
                            .builder
                            .build_int_z_extend(int_value, out_type, "zext")?
                    }
                }
                core::cmp::Ordering::Equal => int_value,
            };
            return Ok(casted.as_basic_value_enum());
        }

        if is_float_core_type(&target_type) {
            let float_type = float_type_for(codegen_context, &target_type)?;
            let casted = codegen_context
                .builder
                .build_signed_int_to_float(int_value, float_type, "sitofp")?;
            return Ok(casted.as_basic_value_enum());
        }
    }

    if value.is_float_value() {
        let float_value = value.into_float_value();
        if is_float_core_type(&target_type) {
            let out_type = float_type_for(codegen_context, &target_type)?;
            let in_bits = float_value.get_type().get_bit_width();
            let out_bits = out_type.get_bit_width();
            let casted = match in_bits.cmp(&out_bits) {
                core::cmp::Ordering::Greater => {
                    codegen_context
                        .builder
                        .build_float_trunc(float_value, out_type, "fptrunc")?
                }
                core::cmp::Ordering::Less => {
                    codegen_context
                        .builder
                        .build_float_ext(float_value, out_type, "fpext")?
                }
                core::cmp::Ordering::Equal => float_value,
            };
            return Ok(casted.as_basic_value_enum());
        }

        if is_integer_core_type(&target_type) {
            let int_type = integer_type_for(codegen_context, &target_type)?;
            let casted = if is_signed_core_type(&target_type) {
                codegen_context.builder.build_float_to_signed_int(
                    float_value,
                    int_type,
                    "fptosi",
                )?
            } else {
                codegen_context.builder.build_float_to_unsigned_int(
                    float_value,
                    int_type,
                    "fptoui",
                )?
            };
            return Ok(casted.as_basic_value_enum());
        }
    }

    Err(CodegenError::new(format!(
        "unsupported cast to type {target_type}"
    )))
}

fn codegen_array_literal<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    elements: &[Expr],
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let element_core = expected_type.map_or(&CoreType::Int64, |core_type| match *core_type {
        CoreType::Array(ref element) => element.as_ref(),
        _ => &CoreType::Int64,
    });
    let element_type = core_type_to_llvm(codegen_context.context, element_core);
    let count = u32::try_from(elements.len()).map_err(|conversion_error| {
        CodegenError::new(format!("array literal is too large: {conversion_error}"))
    })?;
    let array_type = element_type.array_type(count);
    let array_alloca = codegen_context
        .builder
        .build_alloca(array_type, &env.next_name("array.alloca"))?;

    for (index, element_expr) in elements.iter().enumerate() {
        let idx = u64::try_from(index).map_err(|conversion_error| {
            CodegenError::new(format!("array index conversion failed: {conversion_error}"))
        })?;
        // SAFETY: array_alloca refers to stack memory for this array and indices are bounded by iteration.
        let ptr = unsafe {
            codegen_context.builder.build_in_bounds_gep(
                array_alloca,
                &[
                    codegen_context.context.i32_type().const_zero(),
                    codegen_context.context.i32_type().const_int(idx, false),
                ],
                &env.next_name("array.store.ptr"),
            )?
        };
        let value = codegen_expression(codegen_context, env, element_expr, Some(element_core))?;
        let _store_instruction = codegen_context.builder.build_store(ptr, value)?;
    }

    // SAFETY: zero-offset GEP into the alloca points to the start of the contiguous array payload.
    let base_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            array_alloca,
            &[
                codegen_context.context.i32_type().const_zero(),
                codegen_context.context.i32_type().const_zero(),
            ],
            &env.next_name("array.base.ptr"),
        )?
    };

    Ok(base_ptr.as_basic_value_enum())
}

fn codegen_array_access<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    object: &Expr,
    index: &Expr,
    _expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let base_ptr = if let Expr::Identifier { ref name, .. } = *object {
        let Some(binding) = env.variables.get(name) else {
            return Err(CodegenError::new(format!(
                "unknown array variable '{name}'"
            )));
        };
        binding.alloca
    } else {
        codegen_expression(codegen_context, env, object, None)?.into_pointer_value()
    };

    let index_value =
        codegen_expression(codegen_context, env, index, Some(&CoreType::Int64))?.into_int_value();
    // SAFETY: base_ptr is a valid pointer to contiguous elements and index_value is an LLVM integer index.
    let element_ptr = unsafe {
        codegen_context.builder.build_in_bounds_gep(
            base_ptr,
            &[index_value],
            &env.next_name("array.load.ptr"),
        )?
    };

    Ok(codegen_context
        .builder
        .build_load(element_ptr, &env.next_name("array.load"))?)
}

fn codegen_add<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    codegen_numeric_binop(codegen_context, env, lhs, rhs, expected_type, "add")
}

fn codegen_sub<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    codegen_numeric_binop(codegen_context, env, lhs, rhs, expected_type, "sub")
}

fn codegen_mul<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    expected_type: Option<&CoreType>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    codegen_numeric_binop(codegen_context, env, lhs, rhs, expected_type, "mul")
}

fn codegen_bool<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    operator: &BinaryOp,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let l = lhs.into_int_value();
    let r = rhs.into_int_value();
    let value = match *operator {
        BinaryOp::And => codegen_context.builder.build_and(l, r, "land")?,
        BinaryOp::Or => codegen_context.builder.build_or(l, r, "lor")?,
        BinaryOp::Xor => codegen_context.builder.build_xor(l, r, "lxor")?,
        _ => return Err(CodegenError::new(String::from("unsupported logical op"))),
    };
    Ok(value.as_basic_value_enum())
}

fn codegen_bitwise<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    operator: &BinaryOp,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let l = lhs.into_int_value();
    let r = rhs.into_int_value();
    let value = match *operator {
        BinaryOp::BitAnd => codegen_context.builder.build_and(l, r, "band")?,
        BinaryOp::BitOr => codegen_context.builder.build_or(l, r, "bor")?,
        BinaryOp::BitXor => codegen_context.builder.build_xor(l, r, "bxor")?,
        _ => return Err(CodegenError::new(String::from("unsupported bitwise op"))),
    };
    Ok(value.as_basic_value_enum())
}

fn codegen_shift<'context>(
    codegen_context: &CodegenContext<'context>,
    lhs: BasicValueEnum<'context>,
    rhs: BasicValueEnum<'context>,
    operator: &BinaryOp,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    let l = lhs.into_int_value();
    let r = rhs.into_int_value();
    let value = match *operator {
        BinaryOp::BitShiftLeft => codegen_context.builder.build_left_shift(l, r, "bshl")?,
        BinaryOp::BitShiftRight => codegen_context
            .builder
            .build_right_shift(l, r, true, "bshr")?,
        BinaryOp::BitUnsignedShiftRight => codegen_context
            .builder
            .build_right_shift(l, r, false, "bushr")?,
        _ => return Err(CodegenError::new(String::from("unsupported shift op"))),
    };
    Ok(value.as_basic_value_enum())
}

pub fn emit_div_by_zero_check<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    divisor: IntValue<'context>,
) -> Result<(), CodegenError> {
    let zero = divisor.get_type().const_zero();
    let is_zero = codegen_context.builder.build_int_compare(
        IntPredicate::EQ,
        divisor,
        zero,
        &env.next_name("div.zero"),
    )?;
    let current_fn = current_function(codegen_context)?;
    let trap_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("div.trap"));
    let cont_block = codegen_context
        .context
        .append_basic_block(current_fn, &env.next_name("div.cont"));
    let _branch = codegen_context
        .builder
        .build_conditional_branch(is_zero, trap_block, cont_block)?;

    codegen_context.builder.position_at_end(trap_block);
    emit_trap_call(codegen_context)?;
    let _unreachable = codegen_context.builder.build_unreachable()?;

    codegen_context.builder.position_at_end(cont_block);
    Ok(())
}

pub fn emit_trap_call<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<(), CodegenError> {
    let trap_fn = codegen_context
        .module
        .get_function("llvm.trap")
        .map_or_else(
            || {
                let fn_type = codegen_context.context.void_type().fn_type(&[], false);
                codegen_context
                    .module
                    .add_function("llvm.trap", fn_type, None)
            },
            |existing| existing,
        );
    let args: [BasicMetadataValueEnum<'context>; 0] = [];
    let _call = codegen_context
        .builder
        .build_call(trap_fn, &args, "trap.call")?;
    Ok(())
}

pub fn current_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> Result<FunctionValue<'context>, CodegenError> {
    let Some(block) = codegen_context.builder.get_insert_block() else {
        return Err(CodegenError::new(String::from(
            "builder is not positioned in a block",
        )));
    };
    let Some(function) = block.get_parent() else {
        return Err(CodegenError::new(String::from(
            "insert block does not have a parent function",
        )));
    };
    Ok(function)
}

fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, CodegenError> {
    match *ast_type {
        Type::Basic { ref name, .. } => match name.as_str() {
            "int8" => Ok(CoreType::Int8),
            "int16" => Ok(CoreType::Int16),
            "int32" => Ok(CoreType::Int32),
            "int64" => Ok(CoreType::Int64),
            "uint8" => Ok(CoreType::UInt8),
            "uint16" => Ok(CoreType::UInt16),
            "uint32" => Ok(CoreType::UInt32),
            "uint64" => Ok(CoreType::UInt64),
            "float32" => Ok(CoreType::Float32),
            "float64" => Ok(CoreType::Float64),
            "string" => Ok(CoreType::String),
            "boolean" => Ok(CoreType::Boolean),
            "void" | "unit" => Ok(CoreType::Unit),
            _ => Err(CodegenError::new(format!("unsupported type '{name}'"))),
        },
        Type::Array {
            ref element_type, ..
        } => Ok(CoreType::Array(alloc::boxed::Box::new(
            ast_type_to_core_type(element_type)?,
        ))),
        _ => Err(CodegenError::new(String::from(
            "function/generic cast targets unsupported in task 22",
        ))),
    }
}

fn integer_type_for<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
) -> Result<inkwell::types::IntType<'context>, CodegenError> {
    match *core_type {
        CoreType::Int8 | CoreType::UInt8 => Ok(codegen_context.context.i8_type()),
        CoreType::Int16 | CoreType::UInt16 => Ok(codegen_context.context.i16_type()),
        CoreType::Int32 | CoreType::UInt32 => Ok(codegen_context.context.i32_type()),
        CoreType::Int64 | CoreType::UInt64 => Ok(codegen_context.context.i64_type()),
        _ => Err(CodegenError::new(format!(
            "{core_type} is not an integer type"
        ))),
    }
}

fn float_type_for<'context>(
    codegen_context: &CodegenContext<'context>,
    core_type: &CoreType,
) -> Result<inkwell::types::FloatType<'context>, CodegenError> {
    match *core_type {
        CoreType::Float32 => Ok(codegen_context.context.f32_type()),
        CoreType::Float64 => Ok(codegen_context.context.f64_type()),
        _ => Err(CodegenError::new(format!(
            "{core_type} is not a float type"
        ))),
    }
}

const fn is_integer_core_type(core_type: &CoreType) -> bool {
    matches!(
        *core_type,
        CoreType::Int8
            | CoreType::Int16
            | CoreType::Int32
            | CoreType::Int64
            | CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
    )
}

const fn is_float_core_type(core_type: &CoreType) -> bool {
    matches!(*core_type, CoreType::Float32 | CoreType::Float64)
}

const fn is_signed_core_type(core_type: &CoreType) -> bool {
    matches!(
        *core_type,
        CoreType::Int8 | CoreType::Int16 | CoreType::Int32 | CoreType::Int64
    )
}

fn integer_literal_bits(number: i64) -> Result<u64, CodegenError> {
    if number >= 0 {
        return u64::try_from(number).map_err(|conversion_error| {
            CodegenError::new(format!(
                "failed converting non-negative integer literal to u64: {conversion_error}"
            ))
        });
    }

    let magnitude = number.unsigned_abs();
    Ok((!magnitude).wrapping_add(1))
}
