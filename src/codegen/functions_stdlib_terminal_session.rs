extern crate alloc;

use crate::codegen::context::CodegenContext;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

/// Runtime-ready selected terminal session/data-model functions.
pub(super) const TERMINAL_SESSION_RUNTIME_NAMES: &[&str] = &[
    "terminal_session_options_default",
    "terminal_session_options_with_feature_policy",
    "terminal_session_options_with_resource_limits",
    "terminal_session_options_validate",
    "trusted_terminal_output_from_application_text",
    "terminal_session_open_sync",
    "terminal_session_state",
    "terminal_session_capabilities",
    "terminal_capabilities_feature",
    "terminal_capabilities_trusted_paste_framing",
    "terminal_capabilities_color",
    "terminal_session_size_sync",
    "terminal_session_read_event_sync",
    "terminal_session_write_sync",
    "terminal_session_flush_sync",
    "terminal_session_clear_screen_sync",
    "terminal_session_move_cursor_sync",
    "terminal_session_draw_rows_sync",
    "terminal_session_bell_sync",
    "terminal_session_set_cursor_visible_sync",
    "terminal_session_set_cursor_shape_sync",
    "terminal_session_close_sync",
];

/// Return whether `name` is a generated-runtime-ready selected terminal symbol.
#[must_use]
pub(super) fn is_terminal_session_runtime_name(name: &str) -> bool {
    TERMINAL_SESSION_RUNTIME_NAMES.contains(&name)
}

/// Declare an implemented selected terminal runtime function.
#[expect(
    clippy::too_many_lines,
    reason = "terminal runtime declarations are centralized for ABI readability"
)]
pub(super) fn declare_terminal_session_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
    let i64_type = ctx.i64_type();
    let pointer_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let void_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    match name {
        "terminal_session_options_default" => module
            .get_function(name)
            .or_else(|| Some(module.add_function(name, i8_ptr.fn_type(&[], false), None))),
        "terminal_session_options_with_feature_policy"
        | "terminal_session_options_with_resource_limits" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "terminal_session_options_validate"
        | "trusted_terminal_output_from_application_text"
        | "terminal_session_open_sync"
        | "terminal_session_size_sync"
        | "terminal_session_close_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_state" | "terminal_session_capabilities" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(name, i8_ptr.fn_type(&[i8_ptr.into()], false), None))
            })
        }
        "terminal_capabilities_trusted_paste_framing" | "terminal_capabilities_color" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(name, i8_ptr.fn_type(&[i8_ptr.into()], false), None))
            })
        }
        "terminal_capabilities_feature" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_read_event_sync" => module.get_function(name).or_else(|| {
            Some(
                module.add_function(
                    name,
                    pointer_error_result_type
                        .fn_type(&[i8_ptr.into(), i8_ptr.into(), i8_ptr.into()], false),
                    None,
                ),
            )
        }),
        "terminal_session_write_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_flush_sync"
        | "terminal_session_clear_screen_sync"
        | "terminal_session_bell_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(&[i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_move_cursor_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(
                    &[i8_ptr.into(), ctx.i32_type().into(), ctx.i32_type().into()],
                    false,
                ),
                None,
            ))
        }),
        "terminal_session_set_cursor_visible_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(&[i8_ptr.into(), ctx.bool_type().into()], false),
                None,
            ))
        }),
        "terminal_session_set_cursor_shape_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_draw_rows_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(
                    &[
                        i8_ptr.into(),
                        i8_ptr.ptr_type(AddressSpace::default()).into(),
                        i64_type.into(),
                    ],
                    false,
                ),
                None,
            ))
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::is_terminal_session_runtime_name;

    #[test]
    fn generated_high_level_rendering_v1_includes_cursor_visibility_and_shape() {
        assert!(is_terminal_session_runtime_name(
            "terminal_session_clear_screen_sync"
        ));
        assert!(is_terminal_session_runtime_name(
            "terminal_session_move_cursor_sync"
        ));
        assert!(is_terminal_session_runtime_name(
            "terminal_session_draw_rows_sync"
        ));
        assert!(is_terminal_session_runtime_name(
            "terminal_session_bell_sync"
        ));
        assert!(is_terminal_session_runtime_name(
            "terminal_session_set_cursor_visible_sync"
        ));
        assert!(is_terminal_session_runtime_name(
            "terminal_session_set_cursor_shape_sync"
        ));
    }
}
