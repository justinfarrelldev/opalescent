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
    "terminal_session_write_diagnostic_sync",
    "terminal_session_flush_sync",
    "terminal_session_clear_screen_sync",
    "terminal_session_move_cursor_sync",
    "terminal_session_draw_rows_sync",
    "terminal_session_bell_sync",
    "terminal_session_set_cursor_visible_sync",
    "terminal_session_set_cursor_shape_sync",
    "terminal_session_pause_sync",
    "terminal_session_resume_sync",
    "terminal_session_close_sync",
    "safe_terminal_diagnostic_format",
    "safe_terminal_diagnostic_collection_format",
    "terminal_pause_events_length",
    "terminal_pause_events_at",
    "terminal_diagnostic_backend",
    "terminal_diagnostic_operation",
    "terminal_diagnostic_stage",
    "terminal_diagnostic_coordinator_state",
    "terminal_diagnostic_session_state",
    "terminal_diagnostic_os_code",
    "terminal_diagnostic_detail",
    "terminal_diagnostic_retryability",
    "terminal_diagnostic_was_truncated",
    "terminal_diagnostics_length",
    "terminal_diagnostics_at",
    "terminal_diagnostics_retained_count",
    "terminal_diagnostics_omitted_count",
    "terminal_diagnostics_retained_bytes",
    "terminal_diagnostics_omitted_bytes",
    "terminal_diagnostics_was_truncated",
    "terminal_chord_modifiers",
    "terminal_chord_new",
    "terminal_chord_with_lock_modifier_mask",
    "terminal_chord_sequence_single",
    "terminal_chord_sequence_append",
    "terminal_chord_router_new",
    "terminal_chord_router_register",
    "terminal_chord_router_unregister",
    "terminal_chord_router_replace",
    "terminal_chord_binding_id_ordinal",
    "terminal_chord_router_process",
    "terminal_chord_router_expire_sync",
    "terminal_chord_router_reset",
    "terminal_chord_released_input_length",
    "terminal_chord_released_input_at",
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
        | "terminal_session_pause_sync"
        | "terminal_session_close_sync"
        | "terminal_chord_sequence_single"
        | "terminal_chord_router_expire_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_chord_router_new" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_session_state"
        | "terminal_session_capabilities"
        | "safe_terminal_diagnostic_format"
        | "safe_terminal_diagnostic_collection_format"
        | "terminal_diagnostic_backend"
        | "terminal_diagnostic_operation"
        | "terminal_diagnostic_stage"
        | "terminal_diagnostic_coordinator_state"
        | "terminal_diagnostic_session_state"
        | "terminal_diagnostic_os_code"
        | "terminal_diagnostic_detail"
        | "terminal_diagnostic_retryability" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, i8_ptr.fn_type(&[i8_ptr.into()], false), None))
        }),
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
        "terminal_session_write_sync" | "terminal_session_write_diagnostic_sync" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    void_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "terminal_session_flush_sync"
        | "terminal_session_clear_screen_sync"
        | "terminal_session_bell_sync"
        | "terminal_session_resume_sync" => module.get_function(name).or_else(|| {
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
        "terminal_pause_events_length" | "terminal_chord_released_input_length" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(name, i64_type.fn_type(&[i8_ptr.into()], false), None))
            })
        }
        "terminal_diagnostic_was_truncated" | "terminal_diagnostics_was_truncated" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    ctx.bool_type().fn_type(&[i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "terminal_diagnostics_retained_count"
        | "terminal_diagnostics_omitted_count"
        | "terminal_diagnostics_retained_bytes"
        | "terminal_diagnostics_omitted_bytes"
        | "terminal_chord_binding_id_ordinal" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, ctx.i64_type().fn_type(&[i8_ptr.into()], false), None))
        }),
        "terminal_pause_events_at"
        | "terminal_diagnostics_at"
        | "terminal_chord_released_input_at" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into(), i64_type.into()], false),
                None,
            ))
        }),
        "terminal_chord_modifiers" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                i8_ptr.fn_type(
                    &[
                        ctx.bool_type().into(),
                        ctx.bool_type().into(),
                        ctx.bool_type().into(),
                        ctx.bool_type().into(),
                    ],
                    false,
                ),
                None,
            ))
        }),
        "terminal_chord_new" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_chord_with_lock_modifier_mask" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                i8_ptr.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_chord_sequence_append" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "terminal_chord_router_register" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(
                    &[i8_ptr.into(), i8_ptr.into(), i8_ptr.into(), i8_ptr.into()],
                    false,
                ),
                None,
            ))
        }),
        "terminal_chord_router_unregister" | "terminal_chord_router_reset" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "terminal_chord_router_replace" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(
                    &[
                        i8_ptr.into(),
                        i8_ptr.into(),
                        i8_ptr.into(),
                        i8_ptr.into(),
                        i8_ptr.into(),
                    ],
                    false,
                ),
                None,
            ))
        }),
        "terminal_chord_router_process" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{declare_terminal_session_function, is_terminal_session_runtime_name};
    use crate::codegen::context::CodegenContext;
    use inkwell::context::Context;

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

    #[test]
    fn generated_chord_declarations_use_exact_non_vararg_abi() {
        let context = Context::create();
        let codegen_context = CodegenContext::new(&context, "terminal_chord_exact_abi");
        for (name, expected_parameter_count) in [
            ("terminal_chord_new", 3_usize),
            ("terminal_chord_router_new", 2_usize),
            ("terminal_chord_router_register", 4_usize),
        ] {
            let function = declare_terminal_session_function(&codegen_context, name)
                .expect("terminal chord declaration should be runtime-ready");
            let function_type = function.get_type();
            assert!(
                !function_type.is_var_arg(),
                "{name} should use an exact C ABI declaration"
            );
            assert_eq!(
                function_type.get_param_types().len(),
                expected_parameter_count,
                "{name} should declare every C ABI parameter explicitly"
            );
        }
    }
}
