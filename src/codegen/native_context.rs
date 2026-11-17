//! Runner-owned ordinary metadata for the generated nominal TestContext.
extern crate alloc;
use super::context::CodegenContext;
use super::error::CodegenError;
use super::expressions::CodegenEnv;
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use inkwell::AddressSpace;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, PointerValue};

/// Install the physical field order shared by context reads and materialization.
pub fn install_layout(env: &mut CodegenEnv<'_>) {
    let name = "__opal_testing::TestContext".to_owned();
    env.adt_field_layouts.insert(
        name.clone(),
        vec![
            ("failed".to_owned(), CoreType::Boolean),
            ("case_id".to_owned(), CoreType::String),
            ("attempt".to_owned(), CoreType::Int64),
        ],
    );
    env.adt_field_indices.insert(
        name,
        BTreeMap::from([
            ("failed".to_owned(), 0),
            ("case_id".to_owned(), 1),
            ("attempt".to_owned(), 2),
        ]),
    );
}

/// A callback-boundary snapshot is sufficient: failures abort source execution,
/// and each subsequent hook receives a fresh view of the retained attempt state.
/// The case-id string remains valid until the entire attempt ends.
pub(super) fn reference_address<'context>(
    codegen: &CodegenContext<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let llvm = codegen.context;
    let failed = runtime_value(codegen, "opal_test_attempt_failed", llvm.i32_type().into())?
        .into_int_value();
    let failed = codegen.builder.build_int_compare(
        inkwell::IntPredicate::NE,
        failed,
        llvm.i32_type().const_zero(),
        "context.failed",
    )?;
    let pointer = llvm.i8_type().ptr_type(AddressSpace::default());
    let case_id = runtime_value(codegen, "opal_test_case_id", pointer.into())?;
    let attempt = runtime_value(codegen, "opal_test_attempt_number", llvm.i64_type().into())?;
    let structure = llvm.struct_type(
        &[
            llvm.bool_type().into(),
            pointer.into(),
            llvm.i64_type().into(),
        ],
        false,
    );
    let mut value = structure.get_undef();
    for (index, field) in [(0, failed.into()), (1, case_id), (2, attempt)] {
        value = codegen
            .builder
            .build_insert_value(value, field, index, "context.field")?
            .into_struct_value();
    }
    let storage = codegen.builder.build_alloca(structure, "context.storage")?;
    codegen.builder.build_store(storage, value)?;
    let handle = codegen
        .builder
        .build_pointer_cast(storage, pointer, "context.handle")?;
    let reference = codegen.builder.build_alloca(pointer, "context.reference")?;
    codegen.builder.build_store(reference, handle)?;
    Ok(reference)
}

/// These metadata getters use their unchanged scalar C ABI, not source outcomes.
fn runtime_value<'context>(
    codegen: &CodegenContext<'context>,
    name: &str,
    result: BasicTypeEnum<'context>,
) -> Result<BasicValueEnum<'context>, CodegenError> {
    use inkwell::types::BasicType;
    let function = codegen.module.get_function(name).unwrap_or_else(|| {
        codegen
            .module
            .add_function(name, result.fn_type(&[], false), None)
    });
    if function.get_type() != result.fn_type(&[], false) {
        return Err(CodegenError::new(
            "test context runtime getter conflicts with an incompatible source ABI",
        ));
    }
    codegen
        .builder
        .build_call(function, &[], "context.metadata")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("test context getter returned no metadata"))
}
