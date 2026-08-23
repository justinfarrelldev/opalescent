extern crate alloc;

use crate::codegen::context::CodegenContext;
use inkwell::AddressSpace;
use inkwell::values::FunctionValue;

/// Proposal core prerequisite runtime names that already lower to generated runtime targets.
pub(super) const TERMINAL_CORE_PREREQUISITE_RUNTIME_NAMES: &[&str] = &[
    "system_wait_set_new",
    "system_wait_set_register",
    "system_wait_set_remove",
    "system_wait_set_register_owned",
    "system_owned_wait_registration_retarget",
    "system_owned_wait_registration_remove",
    "system_wait_set_wait_sync",
    "cancellation_source_new",
    "cancellation_token",
    "cancellation_request",
    "monotonic_timer_new",
    "monotonic_timer_readiness_source",
    "monotonic_timer_arm",
    "monotonic_timer_disarm",
    "monotonic_timer_generation",
    "monotonic_timer_deadline",
    "monotonic_clock_now",
    "process_control_source_new",
    "process_control_readiness_source",
    "process_control_poll",
    "process_control_acknowledge_suspend",
    "process_control_resume_application",
];

/// Return whether `name` is a generated-runtime-ready core prerequisite symbol.
#[must_use]
pub(super) fn is_terminal_core_prerequisite_runtime_name(name: &str) -> bool {
    TERMINAL_CORE_PREREQUISITE_RUNTIME_NAMES.contains(&name)
}

/// Declare an implemented `standard.system` core prerequisite runtime function.
pub(super) fn declare_terminal_core_prerequisite_function<'context>(
    codegen_context: &CodegenContext<'context>,
    name: &str,
) -> Option<FunctionValue<'context>> {
    let ctx = codegen_context.context;
    let module = &codegen_context.module;
    let i8_ptr = ctx.i8_type().ptr_type(AddressSpace::default());
    let i64_type = ctx.i64_type();
    let parse_result_u64_type = ctx.struct_type(&[i64_type.into(), i8_ptr.into()], false);
    let pointer_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let void_error_result_type = ctx.struct_type(&[i8_ptr.into(), i8_ptr.into()], false);
    let void_type = ctx.void_type();

    match name {
        "system_wait_set_new"
        | "cancellation_source_new"
        | "monotonic_timer_new"
        | "process_control_source_new" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, pointer_error_result_type.fn_type(&[], false), None))
        }),
        "system_wait_set_register"
        | "system_wait_set_register_owned"
        | "system_wait_set_wait_sync" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                pointer_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "monotonic_timer_deadline" | "process_control_poll" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    pointer_error_result_type.fn_type(&[i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "system_wait_set_remove" | "system_owned_wait_registration_retarget" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    void_error_result_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                    None,
                ))
            })
        }
        "system_owned_wait_registration_remove" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                void_error_result_type.fn_type(&[i8_ptr.into()], false),
                None,
            ))
        }),
        "cancellation_token"
        | "monotonic_timer_readiness_source"
        | "process_control_readiness_source" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, i8_ptr.fn_type(&[i8_ptr.into()], false), None))
        }),
        "monotonic_clock_now" => module
            .get_function(name)
            .or_else(|| Some(module.add_function(name, i8_ptr.fn_type(&[], false), None))),
        "cancellation_request" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, void_type.fn_type(&[i8_ptr.into()], false), None))
        }),
        "monotonic_timer_arm" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                parse_result_u64_type.fn_type(&[i8_ptr.into(), i8_ptr.into()], false),
                None,
            ))
        }),
        "monotonic_timer_disarm" => module.get_function(name).or_else(|| {
            Some(module.add_function(
                name,
                parse_result_u64_type.fn_type(&[i8_ptr.into()], false),
                None,
            ))
        }),
        "monotonic_timer_generation" => module.get_function(name).or_else(|| {
            Some(module.add_function(name, i64_type.fn_type(&[i8_ptr.into()], false), None))
        }),
        "process_control_acknowledge_suspend" | "process_control_resume_application" => {
            module.get_function(name).or_else(|| {
                Some(module.add_function(
                    name,
                    void_error_result_type.fn_type(&[i8_ptr.into(), i64_type.into()], false),
                    None,
                ))
            })
        }
        _ => None,
    }
}
