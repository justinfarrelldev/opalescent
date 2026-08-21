//! Internal terminal proposal typechecker gates and metadata.
//!
//! This module keeps proposal-only APIs unavailable to production programs while
//! allowing focused typechecker tests to inspect future signatures.

use super::TypeChecker;
use crate::{
    ast::DeclarationAnnotation,
    token::Span,
    type_system::{errors::TypeError, types::CoreType},
};

/// Core/system prerequisite nominal types used by terminal proposal signatures.
const TERMINAL_PROPOSAL_AFFINE_RESOURCE_TYPES: &[&str] = &[
    "TerminalSession",
    "TerminalChordRouter",
    "TerminalTestScenario",
    "TerminalTestBackendActivation",
    "SystemWaitSet",
    "SystemOwnedWaitRegistration",
    "ProcessControlSource",
    "MonotonicTimer",
    "CancellationSource",
];

/// Core/system prerequisite nominal types used by terminal proposal signatures.
const TERMINAL_PROPOSAL_PREREQUISITE_TYPES: &[&str] = &[
    "AllocationFailureError",
    "CancellationSource",
    "CancellationToken",
    "ConstraintObservedValue",
    "ConstraintViolationError",
    "Error",
    "ErrorAttachmentAbsentError",
    "ErrorAttachmentTruncation",
    "MonotonicDeadline",
    "MonotonicTimer",
    "MonotonicTimerError",
    "MonotonicTimerNotArmedError",
    "ProcessControlAcknowledgementError",
    "ProcessControlError",
    "ProcessControlNotification",
    "ProcessControlPollResult",
    "ProcessControlResumeError",
    "ProcessControlSource",
    "ProcessControlUnavailableError",
    "SystemOwnedWaitRegistration",
    "SystemReadinessSource",
    "SystemWaitRegistration",
    "SystemWaitSet",
    "SystemWaitSetError",
    "SystemWaitWake",
];

impl TypeChecker {
    /// Permit proposal-only terminal imports for focused typechecker tests.
    ///
    /// This is deliberately separate from [`enable_test_only_imports`](Self::enable_test_only_imports):
    /// selected public terminal and chord APIs stay future-gated for production,
    /// while `standard.testing.terminal` remains test-only.
    pub(crate) fn enable_terminal_proposal_imports_for_tests(&mut self) {
        self.allow_terminal_proposal_imports = true;
        self.register_terminal_proposal_prerequisite_types();
        self.register_core_prerequisite_affine_resources();
        self.register_terminal_proposal_affine_resources();
    }

    /// Record constructor visibility metadata for one locally visible type name.
    pub(super) fn register_constructor_visibility(
        &mut self,
        type_name: String,
        annotations: &[DeclarationAnnotation],
    ) {
        if let Some(visibility) = constructor_visibility(annotations) {
            self.constructor_visibilities.insert(type_name, visibility);
        }
    }

    /// Copy constructor visibility metadata from an imported proposal type.
    pub(super) fn register_imported_constructor_visibility(
        &mut self,
        source: &str,
        imported_name: &str,
        local_name: &str,
    ) {
        let Some(interface) = self.module_resolver.module_interface(source) else {
            return;
        };
        let Some(declaration) = interface.type_declaration(imported_name) else {
            return;
        };
        self.register_constructor_visibility(local_name.to_owned(), &declaration.annotations);
    }

    /// Reject constructor expressions for types whose declaration says that
    /// construction is reserved to the runtime, standard library, or test runner.
    pub(super) fn validate_public_constructor_visibility(
        &self,
        owner_name: &str,
        span: Span,
    ) -> Result<(), TypeError> {
        let type_name = constructor_type_name(owner_name);
        let Some(visibility) = self.constructor_visibilities.get(type_name) else {
            return Ok(());
        };
        if visibility == "public" {
            return Ok(());
        }
        Err(TypeError::ConstructorUnavailable {
            type_name: type_name.to_owned(),
            visibility: visibility.clone(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Register prerequisite nominal types for internal gated signature tests.
    fn register_terminal_proposal_prerequisite_types(&mut self) {
        for name in TERMINAL_PROPOSAL_PREREQUISITE_TYPES {
            self.environment.register_type(
                (*name).to_owned(),
                CoreType::Generic {
                    name: (*name).to_owned(),
                    type_args: Vec::new(),
                },
            );
        }
    }

    /// Register canonical proposal affine resources independent of type imports.
    fn register_terminal_proposal_affine_resources(&mut self) {
        for name in TERMINAL_PROPOSAL_AFFINE_RESOURCE_TYPES {
            self.register_affine_resource_type((*name).to_owned());
        }
    }
}

/// Extract constructor visibility metadata from parsed declaration annotations.
fn constructor_visibility(annotations: &[DeclarationAnnotation]) -> Option<String> {
    annotations.iter().find_map(|annotation| match *annotation {
        DeclarationAnnotation::ConstructorVisibility { ref value, .. } => Some(value.clone()),
        _ => None,
    })
}

/// Reduce variant-qualified constructor owners to their nominal type name.
fn constructor_type_name(owner_name: &str) -> &str {
    owner_name
        .split_once('.')
        .map_or(owner_name, |(type_name, _)| type_name)
}
