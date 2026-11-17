//! Invalid descriptor identities and callback ABIs fail before native execution.
use super::context::CodegenContext;
use super::native_descriptors::{
    NativeCallbackDescriptor, NativeCaseDescriptorPlan, NativeWorldDescriptor,
};
use crate::testing::native::identity::{DeclarationIdentity, WorldIdentity};
use crate::token::{Position, Span};
use inkwell::context::Context;

#[test]
fn case_descriptor_rejects_a_conflicting_function_symbol() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "descriptor_symbol_conflict");
    codegen.module.add_function(
        "case_descriptor",
        llvm.void_type().fn_type(&[], false),
        None,
    );
    let plan = plan(&codegen);
    assert!(
        super::native_descriptors::emit_case_descriptor(&codegen, "case_descriptor", &plan)
            .is_err()
    );
}

#[test]
fn duplicate_canonical_worlds_cannot_create_multiple_attempt_instances() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "duplicate_descriptor_provider");
    let mut plan = plan(&codegen);
    for _ in 0_u8..2_u8 {
        plan.worlds.push(NativeWorldDescriptor {
            identity: WorldIdentity(DeclarationIdentity {
                module: "worlds.op".to_owned(),
                name: "Shared".to_owned(),
            }),
            setup: plan.body.clone(),
            cleanup: None,
            drop_value: None,
        });
    }
    assert!(
        super::native_descriptors::emit_case_descriptor(&codegen, "case_descriptor", &plan)
            .is_err()
    );
}

#[test]
fn lifecycle_descriptor_rejects_non_c_callback_calling_conventions() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "descriptor_calling_convention");
    let plan = plan(&codegen);
    plan.body.function.set_call_conventions(8);
    assert!(
        super::native_descriptors::emit_case_descriptor(&codegen, "case_descriptor", &plan)
            .is_err()
    );
}

/// A real scalar void wrapper is sufficient for descriptor validation only.
fn plan<'context>(codegen: &CodegenContext<'context>) -> NativeCaseDescriptorPlan<'context> {
    let source = codegen
        .declare_native_test_function(
            "source",
            codegen.context.void_type().fn_type(&[], false),
            None,
        )
        .expect("source");
    codegen
        .builder
        .position_at_end(codegen.context.append_basic_block(source, "entry"));
    codegen
        .emit_application_return(None)
        .expect("source return");
    let function =
        super::native_callbacks::emit_void_callback_wrapper(codegen, "callback", source, false)
            .expect("wrapper");
    NativeCaseDescriptorPlan {
        worlds: Vec::new(),
        scopes: Vec::new(),
        body: NativeCallbackDescriptor {
            function,
            file: "fixture.test.op".to_owned(),
            span: Span::single(Position::start()),
        },
        construct_example: None,
        drop_example: None,
    }
}
