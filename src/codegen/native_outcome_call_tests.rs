//! Host-executable proof that abort identity crosses distinct native return ABIs.
#![cfg(target_os = "linux")]
use super::context::CodegenContext;
use super::native_outcome::NativeOutcomeType;
use inkwell::context::Context;
use inkwell::targets::FileType;
use inkwell::values::{BasicValue, FunctionValue};
use inkwell::{AddressSpace, IntPredicate};

#[test]
fn native_calls_propagate_abort_without_reading_unrelated_application_slots() {
    let llvm = Context::create();
    let codegen = CodegenContext::new(&llvm, "native_call_chain");
    let integer = NativeOutcomeType::new(&llvm, Some(llvm.i64_type().into()));
    let text = NativeOutcomeType::new(
        &llvm,
        Some(llvm.i8_type().ptr_type(AddressSpace::default()).into()),
    );
    let leaf = emit_leaf(&codegen, integer);
    let middle = emit_middle(&codegen, integer, text, leaf);
    emit_entry(&codegen, text, middle);
    codegen
        .module
        .verify()
        .expect("verified independent channel");
    execute_module(&codegen);
}

/// A non-fallible integer-producing function aborts without an integer value.
fn emit_leaf<'context>(
    codegen: &CodegenContext<'context>,
    outcome: NativeOutcomeType<'context>,
) -> FunctionValue<'context> {
    let function =
        codegen
            .module
            .add_function("leaf", outcome.llvm_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(codegen.context.append_basic_block(function, "entry"));
    let aborted = outcome
        .build_aborted(codegen, codegen.context.i64_type().const_int(41, false))
        .expect("leaf abort");
    codegen
        .builder
        .build_return(Some(&aborted))
        .expect("leaf return");
    function
}

/// Propagation into a string-producing caller cannot invent a pointer result.
fn emit_middle<'context>(
    codegen: &CodegenContext<'context>,
    integer: NativeOutcomeType<'context>,
    text: NativeOutcomeType<'context>,
    leaf: FunctionValue<'context>,
) -> FunctionValue<'context> {
    let function =
        codegen
            .module
            .add_function("middle", text.llvm_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(codegen.context.append_basic_block(function, "entry"));
    let result = codegen
        .builder
        .build_call(leaf, &[], "leaf_result")
        .expect("call")
        .try_as_basic_value()
        .basic()
        .expect("result")
        .into_struct_value();
    let branch = integer
        .branch_after_call(codegen, result)
        .expect("independent abort branch");
    integer
        .completed_application_value(codegen, result)
        .expect("completed-only read")
        .expect("integer");
    let literal = codegen
        .builder
        .build_global_string_ptr("completed", "text")
        .expect("string");
    let completed = text
        .build_completed(
            codegen,
            Some(literal.as_pointer_value().as_basic_value_enum()),
        )
        .expect("middle complete");
    codegen
        .builder
        .build_return(Some(&completed))
        .expect("completed return");
    codegen.builder.position_at_end(branch.aborted);
    let propagated = text
        .build_aborted(codegen, branch.failure_identity)
        .expect("same identity, different result");
    codegen
        .builder
        .build_return(Some(&propagated))
        .expect("abort return");
    function
}

/// The native entry observes the original failure without reading either slot.
fn emit_entry<'context>(
    codegen: &CodegenContext<'context>,
    text: NativeOutcomeType<'context>,
    middle: FunctionValue<'context>,
) {
    let llvm = codegen.context;
    let main = codegen
        .module
        .add_function("main", llvm.i32_type().fn_type(&[], false), None);
    codegen
        .builder
        .position_at_end(llvm.append_basic_block(main, "entry"));
    let result = codegen
        .builder
        .build_call(middle, &[], "middle_result")
        .expect("outer call")
        .try_as_basic_value()
        .basic()
        .expect("result")
        .into_struct_value();
    let branch = text
        .branch_after_call(codegen, result)
        .expect("outer abort branch");
    codegen
        .builder
        .build_return(Some(&llvm.i32_type().const_int(1, false)))
        .expect("unexpected completion");
    codegen.builder.position_at_end(branch.aborted);
    let wrong = codegen
        .builder
        .build_int_compare(
            IntPredicate::NE,
            branch.failure_identity,
            llvm.i64_type().const_int(41, false),
            "wrong_identity",
        )
        .expect("identity comparison");
    let status = codegen
        .builder
        .build_int_z_extend(wrong, llvm.i32_type(), "status")
        .expect("status");
    codegen
        .builder
        .build_return(Some(&status))
        .expect("main return");
}

/// Link and execute real machine code rather than interpreting callback data.
pub(super) fn execute_module(codegen: &CodegenContext<'_>) {
    execute_module_with_sources(codegen, &[]);
}

/// Link native LLVM with explicit test-only runtime/fixture translation units.
pub(super) fn execute_module_with_sources(
    codegen: &CodegenContext<'_>,
    sources: &[std::path::PathBuf],
) {
    let directory = tempfile::tempdir().expect("artifact directory");
    let object = directory.path().join("calls.o");
    let executable = directory.path().join("calls");
    codegen
        .target_machine
        .as_ref()
        .expect("native target")
        .write_to_file(&codegen.module, FileType::Object, &object)
        .expect("native object");
    let linked = std::process::Command::new("cc")
        .arg("-no-pie")
        .arg(&object)
        .args(sources)
        .arg("-I")
        .arg(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("runtime"))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("system linker");
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let status = std::process::Command::new(executable)
        .status()
        .expect("compiled native calls");
    assert!(
        status.success(),
        "failure identity was not preserved through native calls: {status}"
    );
}
