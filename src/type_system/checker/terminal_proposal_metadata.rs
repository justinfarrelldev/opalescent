//! Terminal public API prerequisite metadata and constructor visibility.
//!
//! This module wires the authoritative Task 13-22 prerequisite validator into
//! the checker while preserving constructor visibility and affine metadata.

use super::TypeChecker;
use crate::{
    ast::{DeclarationAnnotation, TypeDeclarationForm, TypeDef},
    token::Span,
    type_system::{
        errors::TypeError, terminal_public_api_prerequisites::TerminalPublicApiPrerequisite,
        type_mapping::ast_type_to_core_type, types::CoreType,
    },
};

/// Core/system prerequisite nominal types used by terminal proposal signatures.
const TERMINAL_PROPOSAL_AFFINE_RESOURCE_TYPES: &[&str] = &[
    "TerminalSession",
    "TerminalChordRouter",
    "EditorChordRuntime",
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
    "EditorBindingMap",
    "EditorChordCapacityRecovery",
    "EditorChordRuntime",
    "EditorChordTimerRecoverySlot",
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
    /// Disable one prerequisite capability for focused Task 23 gate tests.
    pub(crate) fn disable_terminal_public_api_prerequisite_for_tests(
        &mut self,
        prerequisite: TerminalPublicApiPrerequisite,
    ) {
        self.terminal_public_api_prerequisites
            .disable_for_tests(prerequisite);
    }

    /// Disable every prerequisite capability for narrow bypass tests.
    pub(crate) fn clear_terminal_public_api_prerequisites_for_tests(&mut self) {
        self.terminal_public_api_prerequisites.clear_for_tests();
    }

    /// Return whether every Task 13-22 prerequisite is currently enabled.
    pub(super) fn terminal_public_api_prerequisites_are_satisfied(&self) -> bool {
        self.terminal_public_api_prerequisites
            .allows_selected_public_api()
    }

    /// Build the precise missing-prerequisite diagnostic reason.
    pub(super) fn terminal_public_api_prerequisite_reason(&self) -> String {
        self.terminal_public_api_prerequisites.unavailable_reason()
    }

    /// Build the matching missing-prerequisite diagnostic help text.
    pub(super) fn terminal_public_api_prerequisite_help(&self) -> String {
        self.terminal_public_api_prerequisites.unavailable_help()
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

    /// Return the source type for an in-scope constrained alias type.
    pub(super) fn constrained_alias_source_core_type(
        &self,
        core_type: &CoreType,
    ) -> Option<CoreType> {
        let &CoreType::Generic {
            ref name,
            ref type_args,
        } = core_type
        else {
            return None;
        };
        if !type_args.is_empty() {
            return None;
        }

        for module_path in [
            "standard",
            "standard.terminal",
            "standard.terminal.chords",
            "standard.testing.terminal",
        ] {
            let Some(interface) = self.module_resolver.module_interface(module_path) else {
                continue;
            };
            let Some(declaration) = interface.type_declaration(name.as_str()) else {
                continue;
            };
            if declaration.form != TypeDeclarationForm::Constrained {
                continue;
            }
            let &TypeDef::Alias {
                ref target_type,
                ref constraint,
                ..
            } = &declaration.type_def
            else {
                continue;
            };
            if constraint.is_none() {
                continue;
            }
            if let Ok(source_type) = ast_type_to_core_type(target_type) {
                return Some(source_type);
            }
        }
        None
    }

    /// Register prerequisite nominal types required by the selected public API.
    pub(super) fn register_terminal_public_api_prerequisite_types(&mut self) {
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
    pub(super) fn register_terminal_proposal_affine_resources(&mut self) {
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
