//! Source assertions lower through LLVM into the independent runtime channel.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::expressions::CodegenEnv;
use super::native_outcome_call_tests::execute_module_with_sources;
use crate::ast::testing::TestItem;
use crate::ast::{Expr, LambdaBody, Stmt};
use crate::type_system::types::CoreType;
use inkwell::context::Context;

#[test]
fn parsed_boolean_expectation_reports_original_source_span_and_aborts() {
    exercise("expect false to be true", true);
}

#[test]
fn parsed_numeric_relations_lower_to_typed_comparisons() {
    for assertion in [
        "expect 3 to be greater than 2",
        "expect 2 to be less than 3",
        "expect 3 to be at least 3",
        "expect 3 to be at most 3",
        "expect 2.5 to be greater than 2.0",
    ] {
        exercise(assertion, false);
    }
    exercise("expect 2 to be greater than 3", true);
}

#[test]
fn scalar_operands_execute_once_in_source_order_on_success_and_failure() {
    exercise("expect read_actual() to be read_expected()", false);
    exercise("expect read_actual() to be read_expected()", true);
}

#[test]
fn unsigned_comparisons_keep_their_type_through_cast_observations() {
    exercise(
        "let maximum: uint64 = 9223372036854775807\n        let higher: uint64 = maximum + (1 as uint64)\n        expect (higher as uint64) to be greater than (0 as uint64)",
        false,
    );
}

#[test]
fn parsed_numeric_equality_preserves_ordinary_expression_evaluation() {
    exercise("expect 2 + 3 to be 5", false);
}

/// Ordinary scalar observation functions used by the evaluation-order fixture.
fn observation_signature() -> CoreType {
    CoreType::Function {
        generic_params: Vec::new(),
        parameters: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    }
}

/// Register real external fixture function signatures before checking source.
fn register_observation_signatures(checker: &mut crate::type_system::checker::TypeChecker) {
    let (tokens, lex_errors) = crate::lexer::Lexer::new("let read_actual = f(): int64 =>\n    return 7\nlet read_expected = f(): int64 =>\n    return 7\n").tokenize();
    assert!(lex_errors.is_empty());
    let (program, parse_errors) = crate::parser::Parser::new(tokens).parse();
    assert!(parse_errors.is_empty());
    let errors = checker
        .type_check_program(&program.expect("observation signatures"))
        .expect_err("helper module has no entry");
    assert!(
        errors.iter().all(|error| matches!(
            *error,
            crate::type_system::errors::TypeError::MissingEntryPoint { .. }
        )),
        "{errors:?}"
    );
}

/// Extract and type-check a real managed callback through the test frontend.
fn callback_statements(source: &str) -> Vec<Stmt> {
    let (tokens, lex_errors) = crate::lexer::Lexer::new_test_module(source).tokenize();
    assert!(lex_errors.is_empty(), "{lex_errors:?}");
    let (module, parse_errors) = crate::parser::Parser::new(tokens).parse_test_module();
    assert!(parse_errors.is_empty());
    let callback = module
        .expect("module")
        .items
        .into_iter()
        .find_map(|item| match item {
            TestItem::Scenario(scenario) => {
                scenario.items.into_iter().find_map(|child| match child {
                    TestItem::Test(test) => Some(test.callback),
                    _ => None,
                })
            }
            _ => None,
        })
        .expect("callback");
    let mut checker = crate::type_system::checker::TypeChecker::new();
    register_observation_signatures(&mut checker);
    checker
        .type_check_native_case_callback(&callback, &[], None)
        .expect("typed assertion");
    match callback.function {
        Expr::Lambda {
            body: LambdaBody::Block(statements),
            ..
        } => Some(statements),
        _ => None,
    }
    .expect("block callback")
}

/// Lower, link and execute the callback without interpreting assertion data.
fn exercise(assertion: &str, should_fail: bool) {
    let source = format!(
        "scenario compiled assertions:\n    it should execute native source: f(): void =>\n        {assertion}\n        return void\n"
    );
    let statements = callback_statements(&source);
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "compiled_assertion");
    for name in ["read_actual", "read_expected"] {
        codegen
            .module
            .add_function(name, llvm.i64_type().fn_type(&[], false), None);
    }
    let function = codegen
        .declare_native_test_function("callback", llvm.void_type().fn_type(&[], false), None)
        .expect("native callback");
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(function, "entry"));
    let mut environment = CodegenEnv::new(false);
    environment.current_source_path = "assertion.test.op".to_owned();
    environment.current_source_text.clone_from(&source);
    environment
        .current_function_return_types
        .push(vec![CoreType::Unit]);
    for name in ["read_actual", "read_expected"] {
        environment
            .imported_signatures
            .insert(name.to_owned(), observation_signature());
    }
    for statement in &statements {
        super::statements::codegen_statement(&codegen, &mut environment, statement)
            .expect("native assertion lowering");
    }
    let outcome = codegen.native_function_outcome(function).expect("outcome");
    let bridge =
        codegen
            .module
            .add_function("run_assertion", llvm.i64_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(bridge, "entry"));
    let result = codegen
        .builder
        .build_call(function, &[], "callback.result")
        .expect("callback")
        .try_as_basic_value()
        .basic()
        .expect("outcome")
        .into_struct_value();
    let branch = outcome
        .branch_after_call(&codegen, result)
        .expect("callback boundary");
    codegen
        .builder
        .build_return(Some(&llvm.i64_type().const_zero()))
        .expect("completed");
    codegen.builder.position_at_end(branch.aborted);
    codegen
        .builder
        .build_return(Some(&branch.failure_identity))
        .expect("failed");
    codegen
        .module
        .verify()
        .expect("valid source assertion module");
    let directory = tempfile::tempdir().expect("fixture");
    let driver = directory.path().join("driver.c");
    std::fs::write(&driver, driver_source(&source, should_fail)).expect("driver");
    let runtime = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime");
    execute_module_with_sources(
        &codegen,
        &[
            driver,
            runtime.join("opal_testing_runtime.c"),
            runtime.join("opal_error.c"),
        ],
    );
}

/// A C bridge observes the actual structured native failure and original spans.
fn driver_source(source: &str, should_fail: bool) -> String {
    let expected_offset = source.find("expect").expect("assertion offset");
    format!(
        r#"
#include "opal_testing_runtime.h"
#include <string.h>
uint64_t run_assertion(void);
static int actual_count, expected_count, order;
int64_t read_actual(void) {{ ++actual_count; order = order == 0 ? 1 : -1; return 7; }}
int64_t read_expected(void) {{ ++expected_count; order = order == 1 ? 2 : -1; return {observation_expected}; }}
int main(void) {{
    if (!opal_test_attempt_begin("compiled assertion", 1)) return 1;
    opal_test_set_phase(OPAL_TEST_PHASE_BODY);
    uint64_t failure = run_assertion();
    if ({expected}) {{
        if (!failure || failure != opal_test_primary_failure_id()) return 2;
        const OpalTestFailure *record = opal_test_failure_at(0);
        if (!record || record->kind != OPAL_TEST_FAILURE_ASSERTION) return 3;
        if (record->phase != OPAL_TEST_PHASE_BODY || record->offset != {expected_offset}) return 4;
        if (!record->length || strcmp(record->file, "assertion.test.op")) return 5;
    }} else if (failure || opal_test_attempt_failed()) return 6;
    if ({probe} && (actual_count != 1 || expected_count != 1 || order != 2)) return 7;
    opal_test_attempt_end();
    return 0;
}}
"#,
        expected = u8::from(should_fail),
        observation_expected = if should_fail { 8 } else { 7 },
        probe = u8::from(source.contains("read_actual()"))
    )
}
