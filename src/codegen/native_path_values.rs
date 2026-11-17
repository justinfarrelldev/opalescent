//! Explicit `FilesystemPath` value semantics for test-only equality and owned snapshots.
use super::{context::CodegenContext, error::CodegenError};
use crate::type_system::types::CoreType;
use inkwell::values::{BasicValueEnum, PointerValue};

/// Only the existing builtin nominal identity has this documented runtime representation.
pub fn is_path(core: &CoreType) -> bool {
    crate::type_system::native_double_signature::is_filesystem_path(core)
}

/// Framework path snapshots use existing immutable managed-string value storage.
pub fn snapshot_core(core: &CoreType) -> CoreType {
    if is_path(core) {
        CoreType::String
    } else {
        core.clone()
    }
}

/// Copy path value bytes without invoking filesystem operations or retaining addresses.
pub fn copy<'context>(
    ctx: &CodegenContext<'context>,
    value: BasicValueEnum<'context>,
) -> Result<PointerValue<'context>, CodegenError> {
    let value = value.into_pointer_value();
    let function = super::native_callback_boundary::c_function(
        ctx,
        "opal_test_path_snapshot",
        value.get_type().fn_type(&[value.get_type().into()], false),
    )?;
    Ok(ctx
        .builder
        .build_call(function, &[value.into()], "path.owned.snapshot")?
        .try_as_basic_value()
        .basic()
        .ok_or_else(|| CodegenError::new("path snapshot omitted its owned data pointer"))?
        .into_pointer_value())
}
