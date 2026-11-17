//! Ordinary source return lowering uses explicit native function ABI metadata.
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use crate::ast::{Decl, Expr, LambdaBody, Stmt};
use crate::type_system::types::CoreType;
use inkwell::context::Context;

/// Parse an ordinary function body without rewriting its source text.
fn body(source: &str) -> Stmt {
    let (tokens, lex_errors) = crate::lexer::Lexer::new(source).tokenize();
    assert!(lex_errors.is_empty());
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse();
    assert!(parse_errors.is_empty());
    program
        .expect("source program")
        .declarations
        .into_iter()
        .find_map(|declaration| {
            if let Decl::Let {
                initializer:
                    Expr::Lambda {
                        body: LambdaBody::Block(statements),
                        ..
                    },
                ..
            } = declaration
            {
                statements.into_iter().next()
            } else {
                None
            }
        })
        .expect("source return")
}

#[test]
fn ordinary_integer_return_uses_completed_native_result_without_changing_production() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_source_return");
    let original = llvm.i64_type().fn_type(&[], false);
    let native = codegen
        .declare_native_test_function("native", original, None)
        .expect("native declaration");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(native, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Int64]);
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): int64 =>\n    return 42\n"),
    )
    .expect("native source return");
    let production = codegen.module.add_function("production", original, None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(production, "entry"));
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): int64 =>\n    return 42\n"),
    )
    .expect("production source return");
    assert_eq!(production.get_type(), original);
    assert_ne!(native.get_type(), original);
    codegen
        .module
        .verify()
        .expect("both ABIs have valid source returns");
}

#[test]
fn ordinary_application_error_return_remains_a_completed_result() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_error_return");
    let application =
        super::error_abi::build_error_return_type(&llvm, Some(llvm.i64_type().into()));
    let native = codegen
        .declare_native_test_function("native_error", application.fn_type(&[], false), None)
        .expect("native declaration");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(native, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Int64]);
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): int64 errors ParseError =>\n    return err: ParseError\n"),
    )
    .expect("ordinary error return");
    codegen
        .module
        .verify()
        .expect("valid completed application error");
    let ir = codegen.module.print_to_string().to_string();
    assert!(
        ir.contains("i1 false"),
        "application errors must not become test aborts: {ir}"
    );
}

#[test]
fn ordinary_propagation_returns_application_errors_inside_completed_outcomes() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_propagation");
    let application =
        super::error_abi::build_error_return_type(&llvm, Some(llvm.i64_type().into()));
    codegen
        .module
        .add_function("step", application.fn_type(&[], false), None);
    let function = codegen
        .declare_native_test_function("native_propagation", application.fn_type(&[], false), None)
        .expect("native declaration");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Int64]);
    environment.imported_signatures.insert(
        "step".to_owned(),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Int64],
            error_types: vec![CoreType::Generic {
                name: "ParseError".to_owned(),
                type_args: Vec::new(),
            }],
        },
    );
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): int64 errors ParseError =>\n    return propagate step()\n"),
    )
    .expect("ordinary propagation");
    codegen
        .module
        .verify()
        .expect("valid completed propagated application error");
    assert!(
        !codegen
            .module
            .print_to_string()
            .to_string()
            .contains("missing return statement"),
        "propagated application errors must not enter missing-return handling"
    );
}

#[test]
fn guard_error_forwarding_preserves_the_application_channel_in_native_functions() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_guard_error");
    let application =
        super::error_abi::build_error_return_type(&llvm, Some(llvm.i64_type().into()));
    codegen
        .module
        .add_function("step", application.fn_type(&[], false), None);
    let function = codegen
        .declare_native_test_function("forward", application.fn_type(&[], false), None)
        .expect("native declaration");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Int64]);
    environment.imported_signatures.insert(
        "step".to_owned(),
        CoreType::Function {
            generic_params: Vec::new(),
            parameters: Vec::new(),
            return_types: vec![CoreType::Int64],
            error_types: vec![CoreType::Generic {
                name: "ParseError".to_owned(),
                type_args: Vec::new(),
            }],
        },
    );
    super::statements::codegen_statement(&codegen, &mut environment, &body("let value = f(): int64 errors ParseError =>\n    guard step() else failure =>\n        propagate failure\n    return 1\n")).expect("guard error forwarding");
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): int64 errors ParseError =>\n    return 1\n"),
    )
    .expect("success continuation");
    codegen
        .module
        .verify()
        .expect("valid native guard forwarding");
}

#[test]
fn implicit_void_completion_uses_original_application_abi() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_default_void");
    for (name, original) in [
        ("infallible", llvm.void_type().fn_type(&[], false)),
        (
            "fallible",
            super::error_abi::build_error_return_type(&llvm, None).fn_type(&[], false),
        ),
    ] {
        let function = codegen
            .declare_native_test_function(name, original, None)
            .expect("declaration");
        codegen
            .builder
            .position_at_end(llvm.append_basic_block(function, "entry"));
        super::functions_call::emit_default_return(
            &codegen,
            &mut CodegenEnv::new(false),
            &[CoreType::Unit],
        )
        .expect("default source return");
    }
    codegen
        .module
        .verify()
        .expect("valid implicit completed results");
}

#[test]
fn native_function_declarations_reject_abi_conflicts_and_variadic_paths() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "declaration_contract");
    let original = llvm.i64_type().fn_type(&[], false);
    let declared = codegen
        .declare_native_test_function("native", original, None)
        .expect("declaration");
    assert_eq!(
        codegen
            .declare_native_test_function("native", original, None)
            .expect("idempotent declaration"),
        declared
    );
    assert!(
        codegen
            .declare_native_test_function("native", llvm.i32_type().fn_type(&[], false), None)
            .is_err()
    );
    assert!(
        codegen
            .declare_native_test_function(
                "foreign_variadic",
                llvm.i64_type().fn_type(&[], true),
                None
            )
            .is_err()
    );
    codegen.module.add_function("ordinary", original, None);
    assert!(
        codegen
            .declare_native_test_function("ordinary", original, None)
            .is_err()
    );
}

#[test]
fn ordinary_void_return_completes_without_an_application_value() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_void_return");
    let original = llvm.void_type().fn_type(&[], false);
    let native = codegen
        .declare_native_test_function("native_void", original, None)
        .expect("native declaration");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(native, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment
        .current_function_return_types
        .push(vec![CoreType::Unit]);
    super::statements::codegen_statement(
        &codegen,
        &mut environment,
        &body("let value = f(): void =>\n    return void\n"),
    )
    .expect("void source return");
    codegen.module.verify().expect("valid completed void ABI");
}
