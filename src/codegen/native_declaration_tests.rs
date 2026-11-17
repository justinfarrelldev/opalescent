//! Checked source function definitions use the private native ABI end to end.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::ast::{Decl, Expr, LambdaBody, Parameter, Stmt};
use crate::type_system::types::CoreType;
use inkwell::IntPredicate;
use inkwell::context::Context;

#[test]
fn compiled_native_reference_parameters_update_the_callers_original_binding() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_reference_definition");
    let (increment_parameters, increment_body) = function(
        "let increment = f(mutable ref value: int64): void =>\n    value = value + 1\n    return void\n",
    );
    let signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: vec![CoreType::Int64],
        return_types: vec![CoreType::Unit],
        error_types: Vec::new(),
    };
    super::native_declarations::emit_source_function(
        &codegen,
        &mut CodegenEnv::new(false),
        "increment",
        &increment_parameters,
        &signature,
        &increment_body,
    )
    .expect("native mutable-reference function");
    let (parameters, statements) = function(
        "let caller = f(): int64 =>\n    let mutable original: int64 = 41\n    increment(mutable ref original)\n    return original\n",
    );
    let caller_signature = CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    let mut environment = CodegenEnv::new(false);
    environment
        .imported_signatures
        .insert("increment".to_owned(), signature);
    let caller = super::native_declarations::emit_source_function(
        &codegen,
        &mut environment,
        "caller",
        &parameters,
        &caller_signature,
        &statements,
    )
    .expect("native reference call");
    let main = codegen
        .module
        .add_function("main", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(main, "entry"));
    let value = codegen
        .builder
        .build_call(caller, &[], "caller.outcome")
        .expect("call")
        .try_as_basic_value()
        .basic()
        .expect("outcome")
        .into_struct_value();
    let outcome = codegen.native_function_outcome(caller).expect("metadata");
    let branches = outcome.branch_after_call(&codegen, value).expect("branch");
    let actual = outcome
        .completed_application_value(&codegen, value)
        .expect("completed")
        .expect("value")
        .into_int_value();
    let wrong = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            actual,
            llvm.i64_type().const_int(42, false),
            "wrong",
        )
        .expect("comparison");
    let status = codegen
        .builder
        .build_int_z_extend(wrong, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("completed status");
    codegen.builder.position_at_end(branches.aborted);
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(1, false)))
        .expect("unexpected abort");
    codegen.module.verify().expect("valid native reference ABI");
    execute_module_with_sources(
        &codegen,
        &[std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime/opal_error.c")],
    );
}

/// Read ordinary AST function parameters and body without rewriting source.
fn function(source: &str) -> (Vec<Parameter>, Stmt) {
    let (tokens, lex_errors) = crate::lexer::Lexer::new(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse();
    assert!(parse_errors.is_empty(), "{parse_errors:?}");
    program
        .expect("source")
        .declarations
        .into_iter()
        .find_map(|declaration| match declaration {
            Decl::Let {
                initializer:
                    Expr::Lambda {
                        params,
                        body: LambdaBody::Block(statements),
                        span,
                        id,
                        ..
                    },
                ..
            } => Some((
                params,
                Stmt::Block {
                    statements,
                    span,
                    id,
                },
            )),
            _ => None,
        })
        .expect("function")
}
