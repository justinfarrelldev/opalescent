extern crate alloc;

use crate::ast::{Expr, StringPart};
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::{CodegenEnv, codegen_expression};
use crate::codegen::expressions_array::infer_expression_core_type;
use crate::codegen::scope_tracker::runtime_returns_owned_string;
use crate::codegen::string_ownership::{
    declare_or_get_opal_string_alloc, declare_or_get_opal_string_release,
};
use crate::type_system::types::CoreType;
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::AddressSpace;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue,
};

/// Lowers a `StringInterpolation` AST node to LLVM IR, using dynamically-sized
/// `snprintf` for mixed literal and expression parts, or a global string
/// constant for literal-only parts.
pub fn codegen_string_interpolation<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    parts: &[StringPart],
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if let Some(literal_text) = interpolation_literal_only(parts) {
        let ptr = codegen_context
            .builder
            .build_global_string_ptr(literal_text.as_str(), &env.next_name("interp.literal"))?
            .as_pointer_value();
        return Ok(ptr.as_basic_value_enum());
    }

    let mut format_text = String::new();
    let mut format_args: Vec<BasicMetadataValueEnum<'context>> = Vec::new();
    let mut temporary_string_allocations: Vec<PointerValue<'context>> = Vec::new();

    for part in parts {
        match *part {
            StringPart::Literal(ref text) => {
                format_text.push_str(escape_printf_literal(text).as_str());
            }
            StringPart::Expression(ref expr) => {
                let value = codegen_expression(codegen_context, env, expr, None)?;
                let (lowered, temporary_allocation) = lower_interpolation_argument(
                    codegen_context,
                    env,
                    expr,
                    value,
                    &mut format_text,
                )?;
                format_args.push(lowered.into());
                if let Some(temporary_string_ptr) = temporary_allocation {
                    temporary_string_allocations.push(temporary_string_ptr);
                }
            }
        }
    }

    let format_ptr = codegen_context
        .builder
        .build_global_string_ptr(format_text.as_str(), &env.next_name("interp.fmt"))?
        .as_pointer_value();
    let snprintf_function = ensure_snprintf_function(codegen_context);
    let (buffer_ptr, buffer_size) = allocate_interpolation_buffer(
        codegen_context,
        env,
        snprintf_function,
        format_ptr,
        format_args.as_slice(),
    )?;

    let mut call_args = Vec::with_capacity(format_args.len().saturating_add(3_usize));
    call_args.push(buffer_ptr.into());
    call_args.push(buffer_size.into());
    call_args.push(format_ptr.into());
    call_args.extend(format_args);

    let _snprintf_call = codegen_context.builder.build_call(
        snprintf_function,
        call_args.as_slice(),
        &env.next_name("interp.snprintf"),
    )?;

    if !temporary_string_allocations.is_empty() {
        let release_function = declare_or_get_opal_string_release(codegen_context);
        for temporary_string_ptr in temporary_string_allocations {
            let _release_call = codegen_context.builder.build_call(
                release_function,
                &[temporary_string_ptr.into()],
                &env.next_name("interp.release"),
            )?;
        }
    }

    Ok(buffer_ptr.as_basic_value_enum())
}

/// Performs the first `snprintf` sizing pass and allocates a sufficiently sized
/// heap buffer (including trailing NUL) for interpolation output.
fn allocate_interpolation_buffer<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    snprintf_function: FunctionValue<'context>,
    format_ptr: PointerValue<'context>,
    format_args: &[BasicMetadataValueEnum<'context>],
) -> Result<(PointerValue<'context>, inkwell::values::IntValue<'context>), CodegenError> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let null_buffer = i8_ptr_type.const_null();
    let mut size_call_args = Vec::with_capacity(format_args.len().saturating_add(3_usize));
    size_call_args.push(null_buffer.into());
    size_call_args.push(codegen_context.context.i64_type().const_zero().into());
    size_call_args.push(format_ptr.into());
    size_call_args.extend(format_args.iter().copied());
    let required_length = codegen_context
        .builder
        .build_call(
            snprintf_function,
            size_call_args.as_slice(),
            &env.next_name("interp.snprintf.size"),
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("snprintf returned void".to_owned()))?
        .into_int_value();
    let is_negative_required_length = codegen_context.builder.build_int_compare(
        inkwell::IntPredicate::SLT,
        required_length,
        codegen_context.context.i32_type().const_zero(),
        &env.next_name("interp.snprintf.neg"),
    )?;
    let clamped_required_length = codegen_context
        .builder
        .build_select(
            is_negative_required_length,
            codegen_context.context.i32_type().const_zero(),
            required_length,
            &env.next_name("interp.snprintf.clamp"),
        )?
        .into_int_value();
    let non_negative_required_length = codegen_context.builder.build_int_s_extend(
        clamped_required_length,
        codegen_context.context.i64_type(),
        &env.next_name("interp.snprintf.i64"),
    )?;
    let buffer_size = codegen_context.builder.build_int_add(
        non_negative_required_length,
        codegen_context.context.i64_type().const_int(1_u64, false),
        &env.next_name("interp.snprintf.size_plus_nul"),
    )?;
    let alloc_fn = declare_or_get_opal_string_alloc(codegen_context);
    let buffer_ptr: PointerValue<'context> = codegen_context
        .builder
        .build_call(
            alloc_fn,
            &[buffer_size.into()],
            &env.next_name("interp.buf"),
        )?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("opal_string_alloc returned void".to_owned()))?
        .into_pointer_value();
    Ok((buffer_ptr, buffer_size))
}

/// Returns Some(concatenated text) if all parts are literal strings, or None if any expression parts exist.
fn interpolation_literal_only(parts: &[StringPart]) -> Option<String> {
    let mut output = String::new();
    for part in parts {
        match *part {
            StringPart::Literal(ref text) => output.push_str(text),
            StringPart::Expression(_) => return None,
        }
    }
    Some(output)
}

/// Escapes percent signs to double-percents so literal text can safely be used in printf-family format strings.
fn escape_printf_literal(text: &str) -> String {
    let mut escaped = String::new();
    for ch in text.chars() {
        if ch == '%' {
            escaped.push('%');
            escaped.push('%');
        } else {
            escaped.push(ch);
        }
    }
    escaped
}

/// Coerces a codegen'd expression value to a printf-compatible argument type,
/// appending the appropriate format specifier to `format_text`.
fn lower_interpolation_argument<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    value: BasicValueEnum<'context>,
    format_text: &mut String,
) -> Result<(BasicValueEnum<'context>, Option<PointerValue<'context>>), CodegenError> {
    if value.is_pointer_value() {
        format_text.push_str("%s");
        let temporary_allocation = should_release_interpolation_pointer_argument(env, expr)
            .then(|| value.into_pointer_value());
        return Ok((value, temporary_allocation));
    }

    if value.is_int_value() {
        let int_value = value.into_int_value();
        let bit_width = int_value.get_type().get_bit_width();
        if bit_width == 1_u32 {
            format_text.push_str("%d");
            let widened = codegen_context.builder.build_int_z_extend(
                int_value,
                codegen_context.context.i32_type(),
                &env.next_name("interp.bool.i32"),
            )?;
            return Ok((widened.as_basic_value_enum(), None));
        }

        format_text.push_str("%lld");
        let lowered = match bit_width.cmp(&64_u32) {
            core::cmp::Ordering::Less => codegen_context.builder.build_int_s_extend(
                int_value,
                codegen_context.context.i64_type(),
                &env.next_name("interp.int.i64"),
            )?,
            core::cmp::Ordering::Greater => codegen_context.builder.build_int_truncate(
                int_value,
                codegen_context.context.i64_type(),
                &env.next_name("interp.int.i64"),
            )?,
            core::cmp::Ordering::Equal => int_value,
        };
        return Ok((lowered.as_basic_value_enum(), None));
    }

    if value.is_float_value() {
        format_text.push_str("%f");
        let float_value = value.into_float_value();
        let bit_width = float_value.get_type().get_bit_width();
        let lowered = match bit_width.cmp(&64_u32) {
            core::cmp::Ordering::Less => codegen_context.builder.build_float_ext(
                float_value,
                codegen_context.context.f64_type(),
                &env.next_name("interp.float.f64"),
            )?,
            core::cmp::Ordering::Greater => codegen_context.builder.build_float_trunc(
                float_value,
                codegen_context.context.f64_type(),
                &env.next_name("interp.float.f64"),
            )?,
            core::cmp::Ordering::Equal => float_value,
        };
        return Ok((lowered.as_basic_value_enum(), None));
    }

    Err(CodegenError::new(String::from(
        "unsupported interpolation expression value type",
    )))
}

/// Returns whether a pointer interpolation argument is a temporary owned string
/// that should be released immediately after `snprintf` use.
fn should_release_interpolation_pointer_argument(env: &CodegenEnv<'_>, expr: &Expr) -> bool {
    match *expr {
        Expr::StringInterpolation { .. } => true,
        Expr::Call { .. } if expression_is_string(env, expr) => true,
        Expr::Call { ref callee, .. } => match *callee.as_ref() {
            Expr::Identifier { ref name, .. } => {
                let runtime_name = env
                    .imported_functions
                    .get(name.as_str())
                    .map_or(name.as_str(), String::as_str);
                name.ends_with("_to_string")
                    || runtime_returns_owned_string(runtime_name)
                    || env
                        .owned_string_functions
                        .get(name)
                        .copied()
                        .unwrap_or(false)
            }
            Expr::Member {
                ref object,
                ref member,
                ..
            } if member == "at" || member == "pop" => {
                expression_yields_owned_string_value(env, object.as_ref())
            }
            _ => false,
        },
        Expr::Index { ref object, .. } => {
            expression_yields_owned_string_value(env, object.as_ref())
        }
        Expr::Propagate { ref call, .. } => {
            should_release_interpolation_pointer_argument(env, call.as_ref())
        }
        Expr::Parenthesized { ref expr, .. } => {
            should_release_interpolation_pointer_argument(env, expr.as_ref())
        }
        Expr::Constrain { ref value, .. } => {
            should_release_interpolation_pointer_argument(env, value.as_ref())
        }
        _ => false,
    }
}

/// Determine whether an interpolation argument has string type.
fn expression_is_string(env: &CodegenEnv<'_>, expr: &Expr) -> bool {
    matches!(
        infer_expression_core_type(env, expr),
        Some(CoreType::String)
    )
}

/// Determine whether an expression produces an owned string or string container.
fn expression_yields_owned_string_value(env: &CodegenEnv<'_>, expr: &Expr) -> bool {
    match infer_expression_core_type(env, expr) {
        Some(CoreType::String) => true,
        Some(CoreType::Array(element_type)) => element_type.as_ref() == &CoreType::String,
        _ => false,
    }
}

/// Declares or retrieves the snprintf external function declaration from the LLVM module.
fn ensure_snprintf_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i32_type = codegen_context.context.i32_type();
    let i64_type = codegen_context.context.i64_type();
    codegen_context.module.get_function("snprintf").map_or_else(
        || {
            let snprintf_type = i32_type.fn_type(
                &[i8_ptr_type.into(), i64_type.into(), i8_ptr_type.into()],
                true,
            );
            codegen_context
                .module
                .add_function("snprintf", snprintf_type, None)
        },
        |existing| existing,
    )
}
