//! Ordinary C/application-error ABIs for explicit test-only support helpers.
use crate::codegen::{context::CodegenContext, functions_stdlib::declare_fs_result_function};
use inkwell::{AddressSpace, values::FunctionValue};

/// Exact finite test-library runtime surface, never a production import wildcard.
pub(super) fn is_name(name: &str) -> bool {
    matches!(
        name,
        "attach_text"
            | "temporary_directory_create"
            | "temporary_directory_file"
            | "temporary_directory_remove"
            | "application_executable"
            | "terminal_process_launch"
            | "terminal_process_send_text"
            | "terminal_process_send_key"
            | "terminal_process_row"
            | "terminal_process_cursor_line"
            | "terminal_process_cursor_column"
            | "terminal_process_exited"
            | "terminal_process_exit_code"
            | "terminal_process_transcript"
            | "terminal_process_stop"
    )
}

/// Availability is enforced by imports; only native test artifacts link these symbols.
pub(super) fn declare<'context>(
    ctx: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    if !is_name(name) {
        return None;
    }
    if let Some(function) = ctx.module.get_function(name) {
        return Some(function);
    }
    let pointer = ctx.context.i8_type().ptr_type(AddressSpace::default());
    let integer = ctx.context.i64_type();
    let result = ctx
        .context
        .struct_type(&[pointer.into(), pointer.into()], false);
    let function = match name {
        "attach_text" => ctx
            .module
            .get_function("opal_test_attach_text")
            .unwrap_or_else(|| {
                ctx.module.add_function(
                    "opal_test_attach_text",
                    integer.fn_type(&[pointer.into(), pointer.into(), pointer.into()], false),
                    None,
                )
            }),
        "temporary_directory_create" => declare_fs_result_function(ctx, name, result, &[]),
        "temporary_directory_remove" | "terminal_process_stop" => {
            declare_fs_result_function(ctx, name, result, &[pointer.into()])
        }
        "terminal_process_send_text" | "terminal_process_send_key" => {
            declare_fs_result_function(ctx, name, result, &[pointer.into(), pointer.into()])
        }
        "terminal_process_launch" => declare_fs_result_function(
            ctx,
            name,
            result,
            &[
                pointer.into(),
                pointer.into(),
                pointer.into(),
                integer.into(),
                integer.into(),
            ],
        ),
        "temporary_directory_file" => ctx.module.add_function(
            name,
            pointer.fn_type(&[pointer.into(), pointer.into()], false),
            None,
        ),
        "application_executable" => {
            ctx.module
                .add_function(name, pointer.fn_type(&[], false), None)
        }
        "terminal_process_row" => ctx.module.add_function(
            name,
            pointer.fn_type(&[pointer.into(), integer.into()], false),
            None,
        ),
        "terminal_process_transcript" => {
            ctx.module
                .add_function(name, pointer.fn_type(&[pointer.into()], false), None)
        }
        "terminal_process_exited" => ctx.module.add_function(
            name,
            ctx.context.bool_type().fn_type(&[pointer.into()], false),
            None,
        ),
        "terminal_process_exit_code"
        | "terminal_process_cursor_line"
        | "terminal_process_cursor_column" => {
            ctx.module
                .add_function(name, integer.fn_type(&[pointer.into()], false), None)
        }
        _ => return None,
    };
    Some(function)
}
