//! Terminal proposal parameter borrow-mode metadata.

extern crate alloc;

use crate::ast::BorrowKind;
use alloc::vec::Vec;

use super::terminal_proposal_symbols::TerminalApiFunctionSpec;

/// Return parameter borrow metadata when a proposal function has non-owned parameters.
pub(super) fn function_borrow_kinds(spec: &TerminalApiFunctionSpec) -> Option<Vec<BorrowKind>> {
    let first_kind = first_parameter_borrow_kind(spec.name)?;
    Some(
        spec.parameters
            .iter()
            .enumerate()
            .map(|(index, _)| {
                if index == 0 {
                    first_kind
                } else {
                    BorrowKind::Owned
                }
            })
            .collect(),
    )
}

/// Return the authoritative first-parameter borrow mode for proposal functions.
fn first_parameter_borrow_kind(function_name: &str) -> Option<BorrowKind> {
    if REF_FIRST_PARAMETER_FUNCTIONS.contains(&function_name) {
        return Some(BorrowKind::Ref);
    }
    if MUTABLE_REF_FIRST_PARAMETER_FUNCTIONS.contains(&function_name) {
        return Some(BorrowKind::MutableRef);
    }
    None
}

/// Proposal functions whose first parameter is `ref`.
const REF_FIRST_PARAMETER_FUNCTIONS: &[&str] = &[
    "terminal_session_state",
    "terminal_session_capabilities",
    "terminal_session_readiness_source",
    "terminal_session_size_sync",
    "terminal_session_write_sync",
    "terminal_session_write_diagnostic_sync",
    "terminal_session_flush_sync",
    "terminal_session_set_cursor_visible_sync",
    "terminal_session_set_cursor_shape_sync",
    "cancellation_token",
    "monotonic_timer_readiness_source",
    "monotonic_timer_generation",
    "monotonic_timer_deadline",
    "process_control_readiness_source",
];

/// Proposal functions whose first parameter is `mutable ref`.
const MUTABLE_REF_FIRST_PARAMETER_FUNCTIONS: &[&str] = &[
    "terminal_session_read_event_sync",
    "terminal_session_clear_screen_sync",
    "terminal_session_move_cursor_sync",
    "terminal_session_draw_rows_sync",
    "terminal_session_bell_sync",
    "terminal_session_pause_sync",
    "terminal_session_resume_sync",
    "terminal_session_close_sync",
    "system_wait_set_register",
    "system_wait_set_remove",
    "system_wait_set_register_owned",
    "system_wait_set_wait_sync",
    "system_owned_wait_registration_retarget",
    "system_owned_wait_registration_remove",
    "cancellation_request",
    "monotonic_timer_arm",
    "monotonic_timer_disarm",
    "process_control_poll",
    "process_control_acknowledge_suspend",
    "process_control_resume_application",
    "terminal_chord_router_register",
    "terminal_chord_router_unregister",
    "terminal_chord_router_replace",
    "terminal_chord_router_process",
    "terminal_chord_router_expire_sync",
    "terminal_chord_router_reset",
    "terminal_test_event_id_new",
    "terminal_test_composition_id_new",
    "terminal_test_trusted_paste_evidence",
    "terminal_test_key_event",
    "terminal_test_text_input_event",
    "terminal_test_composition_started_event",
    "terminal_test_composition_updated_event",
    "terminal_test_composition_ended_event",
    "terminal_test_paste_event",
    "terminal_test_mouse_event",
    "terminal_test_resize_event",
    "terminal_test_focus_gained_event",
    "terminal_test_focus_lost_event",
    "terminal_test_unknown_bytes_event",
    "terminal_test_unknown_native_event",
    "terminal_test_input_reset_event",
    "terminal_test_capabilities",
    "terminal_test_diagnostic",
    "terminal_test_bind_fake_backend",
    "terminal_test_activate_backend",
];
