use crate::codegen::context::CodegenContext;
use crate::codegen::types::core_type_to_llvm;
use crate::type_system::types::{CoreType, GenericTypeParameter, TypeVar};
extern crate alloc;
use alloc::boxed::Box;
use alloc::vec::Vec;
use inkwell::context::Context;
use inkwell::types::AnyType;

#[test]
fn test_core_type_mapping_covers_all_variants() {
    let context = Context::create();

    let type_variable = TypeVar {
        id: 1,
        name: "T".to_owned(),
    };
    let generic_param = GenericTypeParameter {
        name: "T".to_owned(),
        type_var: type_variable.clone(),
        constraints: Vec::new(),
    };

    let cases = [
        (CoreType::Int8, "i8"),
        (CoreType::Int16, "i16"),
        (CoreType::Int32, "i32"),
        (CoreType::Int64, "i64"),
        (CoreType::UInt8, "i8"),
        (CoreType::UInt16, "i16"),
        (CoreType::UInt32, "i32"),
        (CoreType::UInt64, "i64"),
        (CoreType::Float32, "float"),
        (CoreType::Float64, "double"),
        (CoreType::Boolean, "i1"),
        (CoreType::String, "i8*"),
        (CoreType::Array(Box::new(CoreType::Int32)), "[0 x i32]"),
        (CoreType::Unit, "{}"),
        (CoreType::Variable(type_variable), "i8*"),
        (
            CoreType::Function {
                generic_params: vec![generic_param],
                parameters: vec![CoreType::Int32],
                return_types: vec![CoreType::Int32],
                error_types: Vec::new(),
            },
            "i8*",
        ),
        (
            CoreType::Generic {
                name: "List".to_owned(),
                type_args: vec![CoreType::Int32],
            },
            "i8*",
        ),
    ];

    for (core_type, expected_llvm_text) in cases {
        let llvm_type = core_type_to_llvm(&context, &core_type);
        let llvm_type_text = llvm_type.as_any_type_enum().print_to_string().to_string();
        assert_eq!(
            llvm_type_text, expected_llvm_text,
            "unexpected LLVM type mapping for {core_type}"
        );
    }
}

#[test]
fn test_codegen_context_new_creates_module_and_builder() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "task21_module");

    assert_eq!(
        codegen_context.module.get_name().to_str(),
        Ok("task21_module"),
        "module name should match constructor input"
    );
    assert!(
        codegen_context.target_machine.is_some(),
        "target machine should be created for the default target triple"
    );
}

#[test]
fn test_codegen_context_sets_target_triple() {
    let context = Context::create();
    let codegen_context = CodegenContext::new(&context, "triple_module");
    let configured_triple = codegen_context.target_triple();
    let default_triple = inkwell::targets::TargetMachine::get_default_triple();

    assert_eq!(
        configured_triple, default_triple,
        "module target triple must match LLVM default triple"
    );
}
