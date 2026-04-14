extern crate alloc;

use crate::ast::StringPart;
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::{codegen_expression, CodegenEnv, CodegenError};
use alloc::string::String;
use alloc::vec::Vec;
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, BasicValueEnum, FunctionValue, PointerValue,
};
use inkwell::AddressSpace;

/// Lowers a `StringInterpolation` AST node to LLVM IR, using sprintf for mixed literal and expression parts, or a global string constant for literal-only parts.
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

    // Heap-allocate a 256-byte buffer via malloc so the returned pointer is valid
    // after the current LLVM stack frame is retired.  Callers are responsible for
    // the lifetime of the allocated string; Opalescent strings are not currently
    // freed (consistent with how string constants are handled in the runtime).
    let malloc_fn = ensure_malloc_function(codegen_context);
    let buf_size = codegen_context.context.i64_type().const_int(256_u64, false);
    let buffer_ptr: PointerValue<'context> = codegen_context
        .builder
        .build_call(malloc_fn, &[buf_size.into()], &env.next_name("interp.buf"))?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("malloc returned void".to_owned()))?
        .into_pointer_value();

    let mut format_text = String::new();
    let mut format_args: Vec<BasicMetadataValueEnum<'context>> = Vec::new();

    for part in parts {
        match *part {
            StringPart::Literal(ref text) => {
                format_text.push_str(escape_printf_literal(text).as_str());
            }
            StringPart::Expression(ref expr) => {
                let value = codegen_expression(codegen_context, env, expr, None)?;
                let lowered =
                    lower_interpolation_argument(codegen_context, env, value, &mut format_text)?;
                format_args.push(lowered.into());
            }
        }
    }

    let format_ptr = codegen_context
        .builder
        .build_global_string_ptr(format_text.as_str(), &env.next_name("interp.fmt"))?
        .as_pointer_value();
    let sprintf_function = ensure_sprintf_function(codegen_context);
    let mut call_args = Vec::with_capacity(format_args.len().saturating_add(2_usize));
    call_args.push(buffer_ptr.into());
    call_args.push(format_ptr.into());
    call_args.extend(format_args);

    let _sprintf_call = codegen_context.builder.build_call(
        sprintf_function,
        call_args.as_slice(),
        &env.next_name("interp.sprintf"),
    )?;
    Ok(buffer_ptr.as_basic_value_enum())
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

/// Coerces a codegen'd expression value to a sprintf-compatible argument type, appending the appropriate format specifier to `format_text`.
fn lower_interpolation_argument<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    value: BasicValueEnum<'context>,
    format_text: &mut String,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if value.is_pointer_value() {
        format_text.push_str("%s");
        return Ok(value);
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
            return Ok(widened.as_basic_value_enum());
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
        return Ok(lowered.as_basic_value_enum());
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
        return Ok(lowered.as_basic_value_enum());
    }

    Err(CodegenError::new(String::from(
        "unsupported interpolation expression value type",
    )))
}

/// Declares or retrieves the sprintf external function declaration from the LLVM module.
fn ensure_sprintf_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i32_type = codegen_context.context.i32_type();
    codegen_context.module.get_function("sprintf").map_or_else(
        || {
            let sprintf_type = i32_type.fn_type(&[i8_ptr_type.into(), i8_ptr_type.into()], true);
            codegen_context
                .module
                .add_function("sprintf", sprintf_type, None)
        },
        |existing| existing,
    )
}

/// Declares or retrieves the `malloc` external function declaration from the LLVM module.
fn ensure_malloc_function<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let i64_type = codegen_context.context.i64_type();
    codegen_context.module.get_function("malloc").map_or_else(
        || {
            let malloc_type = i8_ptr_type.fn_type(&[i64_type.into()], false);
            codegen_context
                .module
                .add_function("malloc", malloc_type, None)
        },
        |existing| existing,
    )
}
