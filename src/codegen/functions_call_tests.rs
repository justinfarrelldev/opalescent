use super::resolve_callee_function;
use crate::ast::{Expr, NodeId};
use crate::codegen::context::CodegenContext;
use crate::codegen::expressions::CodegenEnv;
use crate::token::{Position, Span};
use inkwell::context::Context;

#[doc = "Create a deterministic test span for function-resolution unit tests."]
fn test_span() -> Span {
    Span::single(Position::new(1, 1, 0))
}

#[doc = "Create an identifier expression for callee-resolution tests."]
fn identifier(name: &str) -> Expr {
    Expr::Identifier {
        name: name.to_owned(),
        span: test_span(),
        id: NodeId(1),
    }
}

#[test]
fn resolve_print_to_puts() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "resolve_print_to_puts");
    let mut env = CodegenEnv::new(true);

    let result = resolve_callee_function(&codegen_context, &mut env, &identifier("print"), None);
    assert!(
        result.is_ok(),
        "print should resolve successfully to stdlib puts"
    );

    let Ok(function) = result else {
        return;
    };
    assert_eq!(
        function.get_name().to_str(),
        Ok("puts"),
        "print should resolve to module function named puts"
    );

    let function_type = function.get_type();
    assert!(
        !function_type.is_var_arg(),
        "puts prototype should not be variadic"
    );

    let return_type_text = function_type
        .get_return_type()
        .map_or_else(String::new, |return_type| return_type.print_to_string().to_string());
    assert_eq!(return_type_text, "i32", "puts return type should be i32");

    let parameter_types = function_type.get_param_types();
    assert_eq!(
        parameter_types.len(),
        1,
        "puts should accept exactly one parameter"
    );
    let parameter_type_text = parameter_types[0].print_to_string().to_string();
    assert_eq!(
        parameter_type_text, "i8*",
        "puts first parameter should be i8 pointer"
    );
}

#[test]
fn resolve_printf_to_variadic_printf() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "resolve_printf_to_variadic_printf");
    let mut env = CodegenEnv::new(true);

    let result =
        resolve_callee_function(&codegen_context, &mut env, &identifier("printf"), None);
    assert!(
        result.is_ok(),
        "printf should resolve successfully to libc printf"
    );

    let Ok(function) = result else {
        return;
    };
    assert_eq!(
        function.get_name().to_str(),
        Ok("printf"),
        "printf should resolve to module function named printf"
    );

    let function_type = function.get_type();
    assert!(
        function_type.is_var_arg(),
        "printf prototype should be variadic"
    );

    let return_type_text = function_type
        .get_return_type()
        .map_or_else(String::new, |return_type| return_type.print_to_string().to_string());
    assert_eq!(return_type_text, "i32", "printf return type should be i32");

    let parameter_types = function_type.get_param_types();
    assert_eq!(
        parameter_types.len(),
        1,
        "printf should declare one fixed parameter"
    );
    let parameter_type_text = parameter_types[0].print_to_string().to_string();
    assert_eq!(
        parameter_type_text, "i8*",
        "printf first fixed parameter should be i8 pointer"
    );
}

#[test]
fn unknown_function_produces_error() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "unknown_function_produces_error");
    let mut env = CodegenEnv::new(true);

    let result = resolve_callee_function(
        &codegen_context,
        &mut env,
        &identifier("definitely_not_registered"),
        None,
    );
    assert!(
        result.is_err(),
        "unknown function names should return CodegenError"
    );

    let error_text = result
        .err()
        .map_or_else(String::new, |error| error.to_string());
    assert!(
        error_text.contains("unknown function: definitely_not_registered"),
        "error should include unknown function name, got: {error_text}"
    );
}

#[test]
fn resolve_print_int64_declaration() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "resolve_print_int");
    let mut env = CodegenEnv::new(true);

    let result =
        resolve_callee_function(&codegen_context, &mut env, &identifier("print_int64"), None);
    assert!(result.is_ok(), "print_int64 should resolve successfully");

    let Ok(function) = result else {
        return;
    };
    assert_eq!(
        function.get_name().to_str(),
        Ok("print_int64"),
        "print_int64 should resolve to module function named print_int64"
    );

    let function_type = function.get_type();
    assert!(
        function_type.get_return_type().is_none(),
        "print_int64 should return void in LLVM"
    );

    let parameter_types = function_type.get_param_types();
    assert_eq!(
        parameter_types.len(),
        1,
        "print_int64 should accept exactly one parameter"
    );
    let parameter_type_text = parameter_types[0].print_to_string().to_string();
    assert_eq!(
        parameter_type_text, "i64",
        "print_int64 first parameter should be i64"
    );
}

#[test]
fn opal_runtime_error_is_stdlib_name() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "opal_runtime_error_is_stdlib_name");
    let mut env = CodegenEnv::new(true);

    let result = resolve_callee_function(
        &codegen_context,
        &mut env,
        &identifier("opal_runtime_error"),
        None,
    );
    assert!(
        result.is_ok(),
        "opal_runtime_error should resolve successfully to stdlib function"
    );

    let Ok(function) = result else {
        return;
    };
    assert_eq!(
        function.get_name().to_str(),
        Ok("opal_runtime_error"),
        "opal_runtime_error should resolve to module function named opal_runtime_error"
    );

    let function_type = function.get_type();
    assert!(
        function_type.get_return_type().is_none(),
        "opal_runtime_error should return void in LLVM"
    );

    let parameter_types = function_type.get_param_types();
    assert_eq!(
        parameter_types.len(),
        1,
        "opal_runtime_error should accept exactly one parameter"
    );
    let parameter_type_text = parameter_types[0].print_to_string().to_string();
    assert_eq!(
        parameter_type_text, "i8*",
        "opal_runtime_error first parameter should be i8 pointer (string message)"
    );
}
