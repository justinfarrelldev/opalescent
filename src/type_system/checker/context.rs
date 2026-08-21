//! Transient type-checking context stacks.

extern crate alloc;

use super::{ActiveGuardErrorBinding, ReturnLabelMode, affine_aggregates, using_cleanup};
use crate::type_system::types::CoreType;
use alloc::vec::Vec;

/// Ad-hoc context stacks pushed and popped while descending nested constructs.
#[derive(Default)]
pub(super) struct TypeCheckContext {
    /// Nesting depth of guard `else` handlers currently being type checked.
    pub(super) guard_else_depth: usize,
    /// Stack tracking the error types handled by active guard else branches.
    pub(super) guard_error_stack: Vec<Vec<CoreType>>,
    /// Stack of guard success bindings intentionally hidden while typing the active error clause.
    pub(super) pending_guard_success_bindings: Vec<String>,
    /// Stack of active guard error binding metadata currently in scope.
    pub(super) active_guard_error_bindings: Vec<ActiveGuardErrorBinding>,
    /// Tracks whether calls are being checked from within a propagate expression.
    pub(super) in_propagate_context: bool,
    /// Tracks whether calls are being checked as the subject expression of a guard.
    pub(super) in_guard_subject_context: bool,
    /// Stack tracking return label mode for active function/lambda bodies.
    pub(super) return_label_modes: Vec<ReturnLabelMode>,
    /// Stack of inferred break payload types for nested loop analysis.
    pub(super) loop_break_type_stack: Vec<Option<Vec<CoreType>>>,
    /// Pending cleanup obligations introduced by active `using` bindings.
    pub(super) using_cleanup_obligations: Vec<using_cleanup::UsingCleanupObligation>,
    /// Pending aggregate retarget commit that must be observed immediately.
    pub(super) pending_affine_aggregate_commit: Option<affine_aggregates::PendingAggregateCommit>,
}
