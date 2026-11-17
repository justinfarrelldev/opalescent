//! Checked source callbacks and canonical per-case typed invocation plans.
extern crate alloc;
mod planner;
use super::checking::CheckedNativeProject;
use super::identity::{CaseIdentity, WorldIdentity};
use crate::ast::testing::{ExampleDeclaration, ManagedCallback};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use std::path::PathBuf;
use thiserror::Error;

/// One source boundary, shared independently of case or example ordering.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeCallbackIdentity {
    /// Defining module used for type metadata and source environments.
    pub module_path: PathBuf,
    /// Original source start offset.
    pub start: usize,
    /// Original source end offset.
    pub end: usize,
}

/// Why the compiler invokes a callback; ordinary helper functions are separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCallbackRole {
    /// Required World state constructor.
    WorldSetup,
    /// Explicit optional initialized-state cleanup.
    WorldCleanup,
    /// Shared test body, compiled once even with several examples.
    Body,
    /// Source-order scope entry hook.
    BeforeHook,
    /// Reverse-order scope exit hook.
    AfterHook,
}

/// Authoritative source and checked application signature for one native body.
#[derive(Debug, Clone)]
pub struct NativeCallbackDefinition {
    /// Original typed function, not rewritten source.
    pub callback: ManagedCallback,
    /// Concrete checked signature; does not include the internal abort channel.
    pub signature: CoreType,
    /// Lifecycle role with an independent invocation boundary.
    pub role: NativeCallbackRole,
}

/// Exact compiler-resolved references, not names looked up during execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInjection {
    /// Borrow a nominal handle into the case's initialized World vector.
    WorldReference {
        /// Canonical provider, retained for validation and reporting.
        identity: WorldIdentity,
        /// Deterministic case-local initialization position.
        index: usize,
    },
    /// Borrow the actual state of the World receiving declared cleanup.
    StateReference {
        /// Canonical state provider, never inferred from its concrete state type.
        identity: WorldIdentity,
        /// The same initialized slot used by dependent handles.
        index: usize,
    },
    /// Borrow this case's independently constructed typed example.
    ExampleReference,
    /// Final runner-owned context reference.
    ContextReference,
}

/// A reusable source body plus one case-specific ordered injection plan.
#[derive(Debug, Clone)]
pub struct NativeInvocationPlan {
    /// Key into the deduplicated source definition table.
    pub callback: NativeCallbackIdentity,
    /// Positional reference arguments in checked source signature order.
    pub arguments: Vec<NativeInjection>,
}

/// State is initialized once, only after successful setup.
#[derive(Debug, Clone)]
pub struct NativeWorldPlan {
    /// Canonical provider identity.
    pub identity: WorldIdentity,
    /// Concrete ordinary source state value, not the nominal World type.
    pub state_type: CoreType,
    /// Required source setup with explicit dependency handles.
    pub setup: NativeInvocationPlan,
    /// Optional declared cleanup, with state first and dependency subset after.
    pub cleanup: Option<NativeInvocationPlan>,
}

/// Hook order is preserved here; the lifecycle owns reverse exit ordering.
#[derive(Debug, Clone, Default)]
pub struct NativeScopePlan {
    /// Source-order entry hooks.
    pub before: Vec<NativeInvocationPlan>,
    /// Source-order exit hooks.
    pub after: Vec<NativeInvocationPlan>,
}

/// Independent case descriptor input, not yet a compiled executable artifact.
#[derive(Debug, Clone)]
pub struct LoweredNativeCase {
    /// Stable complete structured case identity.
    pub identity: CaseIdentity,
    /// Shared body/association metadata key in the checked source suite.
    pub body_identity: CaseIdentity,
    /// Unique providers in deterministic topological initialization order.
    pub worlds: Vec<NativeWorldPlan>,
    /// Outermost-to-innermost hook scopes, including empty entered scopes.
    pub scopes: Vec<NativeScopePlan>,
    /// Independently invoked shared body.
    pub body: NativeInvocationPlan,
    /// Constant typed example data, constructed fresh for each attempt.
    pub example: Option<ExampleDeclaration>,
    /// Concrete checked record schema for the example.
    pub example_type: Option<CoreType>,
}

/// Source definitions are shared; invocation plans remain case-specific.
#[derive(Debug, Clone, Default)]
pub struct LoweredNativeProject {
    /// Source functions compiled once for all applicable descriptors.
    pub callbacks: BTreeMap<NativeCallbackIdentity, NativeCallbackDefinition>,
    /// Independent, stable case plans with no execution performed at discovery.
    pub cases: Vec<LoweredNativeCase>,
}

/// Compilation consistency failure, never a test assertion or application error.
#[derive(Debug, Error)]
#[error("native case lowering failed: {0}")]
pub struct NativeLoweringError(
    /// Actionable compiler consistency diagnostic.
    pub String,
);

/// Lower a fully checked suite into explicit typed case/callback plans.
///
/// This performs no setup, callback execution, dynamic fixture lookup or builds.
///
/// # Errors
/// Rejects inconsistent checked metadata rather than inventing missing slots.
pub fn lower_checked_project(
    checked: &CheckedNativeProject,
) -> Result<LoweredNativeProject, NativeLoweringError> {
    planner::lower(checked)
}
