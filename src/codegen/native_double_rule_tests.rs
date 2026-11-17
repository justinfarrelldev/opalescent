//! Reject inconsistent rule metadata before allocating or reading argument packs.
use super::{context::CodegenContext, expressions::CodegenEnv};
use crate::ast::BorrowKind;
use crate::type_system::{NativeDoubleSignature, types::CoreType};
use inkwell::context::Context;

#[test]
fn double_rule_layout_rejects_missing_reference_mode_metadata() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "invalid_rule_layout");
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: Vec::new(),
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    assert!(super::native_double_rules::layout(&codegen, &signature).is_err());
}

#[test]
fn double_argument_evaluation_rejects_missing_values_before_codegen() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "invalid_rule_arguments");
    let mut env = CodegenEnv::new(true);
    let signature = NativeDoubleSignature {
        parameters: vec![CoreType::Int64],
        parameter_modes: vec![BorrowKind::Owned],
        return_types: vec![CoreType::Int64],
        error_types: Vec::new(),
    };
    assert!(
        super::native_double_configuration::evaluate_arguments(&codegen, &mut env, &signature, &[])
            .is_err()
    );
}
