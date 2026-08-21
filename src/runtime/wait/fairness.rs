//! Bounded-fair wait-set cursor and selection helpers.

use super::{SystemWaitWake, WaitSetState};

/// Adjust the fairness cursor after removing `removed_index`.
pub(super) fn adjust_cursor_after_remove(state: &mut WaitSetState, removed_index: usize) {
    if state.entries.is_empty() {
        state.cursor = 0;
    } else if state.cursor > removed_index {
        state.cursor = state.cursor.saturating_sub(1);
    } else if state.cursor >= state.entries.len() {
        state.cursor = 0;
    }
}

/// Select an eligible pending wake in bounded-fair registration order.
pub(super) fn select_pending_wake(
    state: &mut WaitSetState,
    cancel_sequence: Option<u64>,
) -> Option<SystemWaitWake> {
    let entry_count = state.entries.len();
    if entry_count == 0 {
        return None;
    }
    let start = wrap_index(state.cursor, entry_count);
    for offset in 0..entry_count {
        let index = offset_index(start, offset, entry_count);
        let registration_id = state.entries[index].registration_id;
        let Some(pending_index) = state.pending.iter().position(|wake| {
            wake.registration_id == registration_id
                && cancel_sequence.is_none_or(|sequence| wake.sequence < sequence)
        }) else {
            continue;
        };
        let pending = state.pending.remove(pending_index)?;
        state.cursor = cursor_after(index, entry_count);
        return Some(SystemWaitWake::Ready {
            source: pending.source,
            generation: pending.generation,
        });
    }
    None
}

/// Select a currently level-ready source in bounded-fair registration order.
pub(super) fn select_level_ready_wake(state: &mut WaitSetState) -> Option<SystemWaitWake> {
    let entry_count = state.entries.len();
    if entry_count == 0 {
        return None;
    }
    let start = wrap_index(state.cursor, entry_count);
    for offset in 0..entry_count {
        let index = offset_index(start, offset, entry_count);
        let Some(generation) = state.entries[index].source.current_ready_generation() else {
            continue;
        };
        state.cursor = cursor_after(index, entry_count);
        return Some(SystemWaitWake::Ready {
            source: state.entries[index].source.clone(),
            generation,
        });
    }
    None
}

/// Return `value` modulo nonzero `entry_count` using checked arithmetic.
fn wrap_index(value: usize, entry_count: usize) -> usize {
    value.checked_rem(entry_count).unwrap_or(0)
}

/// Return `(start + offset) % entry_count` using checked arithmetic.
fn offset_index(start: usize, offset: usize, entry_count: usize) -> usize {
    start
        .checked_add(offset)
        .and_then(|sum| sum.checked_rem(entry_count))
        .unwrap_or(0)
}

/// Return the next cursor position after `index`.
fn cursor_after(index: usize, entry_count: usize) -> usize {
    index
        .checked_add(1)
        .and_then(|next| next.checked_rem(entry_count))
        .unwrap_or(0)
}
