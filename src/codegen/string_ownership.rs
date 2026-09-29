//! Helpers for compiler-owned string value semantics.
//!
//! Opalescent strings lower to `i8*` for C ABI compatibility, but source-level
//! `string` values are owned values. Runtime helpers retain/release managed heap
//! strings and no-op for static string literals.

extern crate alloc;

use crate::ast::Expr;
use crate::codegen::context::CodegenContext;
use crate::codegen::error::CodegenError;
use crate::codegen::expressions::CodegenEnv;
use alloc::string::String;
use inkwell::AddressSpace;
use inkwell::values::{BasicValueEnum, FunctionValue, PointerValue};

/// Declare or return the runtime helper that retains managed strings.
pub(crate) fn declare_or_get_opal_string_retain<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_string_retain") {
        return function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_string_retain", function_type, None)
}

/// Declare or return the runtime helper that adopts freshly allocated strings.
pub(crate) fn declare_or_get_opal_string_adopt<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_string_adopt") {
        return function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_string_adopt", function_type, None)
}

/// Declare or return the runtime helper that releases managed strings.
pub(crate) fn declare_or_get_opal_string_release<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_string_release") {
        return function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = codegen_context
        .context
        .void_type()
        .fn_type(&[i8_ptr_type.into()], false);
    module.add_function("opal_string_release", function_type, None)
}

/// Declare or return the runtime helper that allocates managed strings.
pub(crate) fn declare_or_get_opal_string_alloc<'context>(
    codegen_context: &CodegenContext<'context>,
) -> FunctionValue<'context> {
    let module = &codegen_context.module;
    if let Some(function) = module.get_function("opal_string_alloc") {
        return function;
    }
    let i8_ptr_type = codegen_context
        .context
        .i8_type()
        .ptr_type(AddressSpace::default());
    let function_type = i8_ptr_type.fn_type(&[codegen_context.context.i64_type().into()], false);
    module.add_function("opal_string_alloc", function_type, None)
}

/// Emit a call that adopts a raw string pointer and returns the managed pointer.
pub(crate) fn emit_string_adopt<'context>(
    codegen_context: &CodegenContext<'context>,
    value: BasicValueEnum<'context>,
    name: &str,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if !value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "string adopt expected pointer value",
        )));
    }
    let adopt_fn = declare_or_get_opal_string_adopt(codegen_context);
    let call =
        codegen_context
            .builder
            .build_call(adopt_fn, &[value.into_pointer_value().into()], name)?;
    call.try_as_basic_value().basic().ok_or_else(|| {
        CodegenError::new(String::from(
            "opal_string_adopt should return pointer value",
        ))
    })
}

/// Emit a retain for a basic value known to contain a string pointer.
pub(crate) fn emit_string_retain<'context>(
    codegen_context: &CodegenContext<'context>,
    value: BasicValueEnum<'context>,
    name: &str,
) -> Result<(), CodegenError> {
    if !value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "string retain expected pointer value",
        )));
    }
    emit_string_retain_pointer(codegen_context, value.into_pointer_value(), name)
}

/// Emit a retain for a raw string pointer value.
pub(crate) fn emit_string_retain_pointer<'context>(
    codegen_context: &CodegenContext<'context>,
    value: PointerValue<'context>,
    name: &str,
) -> Result<(), CodegenError> {
    let retain_fn = declare_or_get_opal_string_retain(codegen_context);
    let _call = codegen_context
        .builder
        .build_call(retain_fn, &[value.into()], name)?;
    Ok(())
}

/// Emit a release for a basic value known to contain a string pointer.
pub(crate) fn emit_string_release<'context>(
    codegen_context: &CodegenContext<'context>,
    value: BasicValueEnum<'context>,
    name: &str,
) -> Result<(), CodegenError> {
    if !value.is_pointer_value() {
        return Err(CodegenError::new(String::from(
            "string release expected pointer value",
        )));
    }
    emit_string_release_pointer(codegen_context, value.into_pointer_value(), name)
}

/// Emit a release for a raw string pointer value.
pub(crate) fn emit_string_release_pointer<'context>(
    codegen_context: &CodegenContext<'context>,
    value: PointerValue<'context>,
    name: &str,
) -> Result<(), CodegenError> {
    let release_fn = declare_or_get_opal_string_release(codegen_context);
    let _call = codegen_context
        .builder
        .build_call(release_fn, &[value.into()], name)?;
    Ok(())
}

/// Return true when evaluating `expr` yields a borrowed string pointer that
/// needs a retain before it is stored or returned as an independently-owned
/// string value.
#[must_use]
pub(crate) fn string_expr_needs_retain_at_ownership_boundary(expr: &Expr) -> bool {
    match expr.clone() {
        Expr::Identifier { .. } | Expr::Member { .. } => true,
        Expr::BorrowArgument { target, .. }
        | Expr::Parenthesized { expr: target, .. }
        | Expr::Constrain { value: target, .. } => {
            string_expr_needs_retain_at_ownership_boundary(target.as_ref())
        }
        _ => false,
    }
}

/// Retain a string result when crossing an ownership boundary if the expression is borrowed.
pub(crate) fn retain_string_for_expr_boundary<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    value: BasicValueEnum<'context>,
) -> Result<(), CodegenError> {
    if string_expr_needs_retain_at_ownership_boundary(expr) {
        emit_string_retain(
            codegen_context,
            value,
            env.next_name("string.boundary.retain").as_str(),
        )?;
    }
    Ok(())
}

/// Retain a returned string unless its local binding is transferred by the return.
pub(crate) fn retain_string_return_value_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    expr: &Expr,
    value: BasicValueEnum<'context>,
    transferred_names: &[String],
) -> Result<(), CodegenError> {
    let transferred = match expr.clone() {
        Expr::Identifier { name, .. } => {
            transferred_names.iter().any(|candidate| candidate == &name)
        }
        _ => false,
    };
    if !transferred {
        retain_string_for_expr_boundary(codegen_context, env, expr, value)?;
    }
    Ok(())
}

/// Adopt a direct or fallible call result when the callee returns a fresh string.
pub(crate) fn adopt_string_call_result_if_needed<'context>(
    codegen_context: &CodegenContext<'context>,
    env: &mut CodegenEnv<'context>,
    call_result: BasicValueEnum<'context>,
    should_adopt: bool,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    if !should_adopt {
        return Ok(call_result);
    }
    if call_result.is_pointer_value() {
        return emit_string_adopt(
            codegen_context,
            call_result,
            env.next_name("call.string.adopt").as_str(),
        );
    }
    if call_result.is_struct_value() {
        let struct_value = call_result.into_struct_value();
        if struct_value.get_type().count_fields() >= 1 {
            let success_value = codegen_context.builder.build_extract_value(
                struct_value,
                0,
                env.next_name("call.string.adopt.value").as_str(),
            )?;
            if success_value.is_pointer_value() {
                let _adopted_success_value = emit_string_adopt(
                    codegen_context,
                    success_value,
                    env.next_name("call.string.adopt").as_str(),
                )?;
            }
        }
    }
    Ok(call_result)
}
