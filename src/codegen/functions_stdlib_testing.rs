//! Ordinary C/application-error ABIs for explicit test-only workspace helpers.
use crate::codegen::{context::CodegenContext, functions_stdlib::declare_fs_result_function};
use inkwell::{AddressSpace, values::FunctionValue};

/// Only test artifacts link these symbols; availability is enforced by imports.
pub(super) fn declare<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    if !matches!(
        name,
        "temporary_directory_create" | "temporary_directory_file" | "temporary_directory_remove"
    ) {
        return None;
    }
    if let Some(function) = ctx.module.get_function(name) {
        return Some(function);
    }
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let result = ctx
        .context
        .struct_type(&[pointer.into(), pointer.into()], false);
    match name {
        "temporary_directory_create" => Some(declare_fs_result_function(ctx, name, result, &[])),
        "temporary_directory_remove" => Some(declare_fs_result_function(
            ctx,
            name,
            result,
            &[pointer.into()],
        )),
        "temporary_directory_file" => Some(ctx.module.get_function(name).unwrap_or_else(|| {
            ctx.module.add_function(
                name,
                pointer.fn_type(&[pointer.into(), pointer.into()], false),
                None,
            )
        })),
        _ => None,
    }
}
