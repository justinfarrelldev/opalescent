#![allow(
    clippy::all,
    clippy::panic,
    reason = "test helper module"
)]
extern crate alloc;
use super::{CodegenEnv, LoopContext};
use inkwell::context::Context;
use alloc::string::String;

#[test]
fn with_loop_isolated_clears_and_restores_loop_stack() {
    let llvm_context = Context::create();
    let module = llvm_context.create_module("loop_stack_tests");
    let builder = llvm_context.create_builder();
    let function_type = llvm_context.void_type().fn_type(&[], false);
    let function = module.add_function("loop_test", function_type, None);
    let continue_target = llvm_context.append_basic_block(function, "continue");
    let break_target = llvm_context.append_basic_block(function, "break");
    builder.position_at_end(continue_target);
    let break_slot = builder
        .build_alloca(llvm_context.i64_type(), "break.slot")
        .expect("alloca should succeed for loop stack test");

    let loop_context = LoopContext {
        continue_target,
        break_target,
        break_slots: vec![break_slot],
        break_labels: vec![String::from("value")],
    };

    let mut env = CodegenEnv::new(false);
    env.push_loop(loop_context);
    assert_eq!(env.loop_stack.len(), 1, "push_loop should add a frame");

    env.with_loop_isolated(|isolated_env| {
        assert!(
            isolated_env.current_loop().is_none(),
            "with_loop_isolated should clear stack inside closure"
        );
        assert!(
            isolated_env.loop_stack.is_empty(),
            "loop stack should be empty while isolated"
        );
    });

    assert_eq!(
        env.loop_stack.len(),
        1,
        "with_loop_isolated should restore prior stack"
    );
    assert!(
        env.current_loop().is_some(),
        "restored stack should expose current loop"
    );
}
