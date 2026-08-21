//! `using` cleanup registration and statement type checking.

extern crate alloc;

use super::helpers::type_mismatch_error;
use crate::ast::{AstNode, Expr, LetBinding, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::{format, vec::Vec};

/// One compiler-visible cleanup-authority transfer result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CleanupAuthorityTransfer {
    /// Operation whose cleanup-only result transfers authority.
    operation: &'static str,
    /// Nominal error family that carries the transfer result.
    error_family: &'static str,
    /// Exact variant that transfers cleanup authority.
    variant: &'static str,
}

/// One compiler-visible affine resource cleanup registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CleanupRegistration {
    /// Registered affine resource type name.
    resource_type: &'static str,
    /// Declared cleanup operation name.
    cleanup_operation: &'static str,
    /// Nominal cleanup error families.
    cleanup_errors: &'static [&'static str],
    /// Optional exact cleanup-authority transfer result.
    transfer: Option<CleanupAuthorityTransfer>,
}

/// The exact Task 14 compiler-visible cleanup registry.
const CLEANUP_REGISTRATIONS: &[CleanupRegistration] = &[
    CleanupRegistration {
        resource_type: "SystemWaitSet",
        cleanup_operation: "system_wait_set_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "SystemOwnedWaitRegistration",
        cleanup_operation: "system_owned_wait_registration_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "ProcessControlSource",
        cleanup_operation: "process_control_source_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "MonotonicTimer",
        cleanup_operation: "monotonic_timer_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "CancellationSource",
        cleanup_operation: "cancellation_source_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "TerminalSession",
        cleanup_operation: "terminal_session_close_sync",
        cleanup_errors: &["TerminalSessionRestoreError"],
        transfer: Some(CleanupAuthorityTransfer {
            operation: "terminal_session_close_sync",
            error_family: "TerminalSessionRestoreError",
            variant: "CloseRestorePending",
        }),
    },
    CleanupRegistration {
        resource_type: "TerminalChordRouter",
        cleanup_operation: "terminal_chord_router_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "TerminalTestScenario",
        cleanup_operation: "terminal_test_scenario_drop",
        cleanup_errors: &[],
        transfer: None,
    },
    CleanupRegistration {
        resource_type: "TerminalTestBackendActivation",
        cleanup_operation: "terminal_test_backend_activation_drop",
        cleanup_errors: &[],
        transfer: None,
    },
];

impl TypeChecker {
    /// Type-check a scoped affine `using` statement.
    pub(super) fn type_check_using_statement(
        &mut self,
        stmt: &Stmt,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        let &Stmt::Using {
            ref binding,
            ref acquisition,
            ref body,
            span,
            ..
        } = stmt
        else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "internal checker error: expected using statement".to_owned(),
                span: TypeError::span_from_span(stmt.span()),
            });
        };

        let acquisition_type = self.type_check_expr(acquisition)?;
        let binding_type =
            self.reconcile_using_binding_type(binding, acquisition, acquisition_type)?;
        let registration = self.cleanup_registration_for_using(&binding_type, span)?;
        self.ensure_using_cleanup_errors_allowed(registration, span)?;
        self.check_value_escape(
            acquisition,
            &binding_type,
            "escape through using acquisition",
            true,
        )?;

        self.within_new_scope(|checker| {
            checker.register_using_owner_binding(binding, binding_type.clone());
            checker.type_check_stmt_with_return(body.as_ref(), expected_return)
        })
    }

    /// Reconcile an optional `using` binding annotation with its acquisition type.
    fn reconcile_using_binding_type(
        &self,
        binding: &LetBinding,
        acquisition: &Expr,
        acquisition_type: CoreType,
    ) -> Result<CoreType, TypeError> {
        let Some(annotation) = binding.type_annotation.as_ref() else {
            return Ok(acquisition_type);
        };
        let annotated_type = using_annotation_to_core_type(annotation)?;
        if self.types_compatible(&annotated_type, &acquisition_type) {
            return Ok(annotated_type);
        }
        Err(type_mismatch_error(
            &annotated_type,
            Some(annotation.span()),
            &acquisition_type,
            acquisition.span(),
        ))
    }

    /// Find the cleanup registration for a direct affine resource type.
    fn cleanup_registration_for_using(
        &self,
        core_type: &CoreType,
        span: Span,
    ) -> Result<&'static CleanupRegistration, TypeError> {
        if !self.core_type_contains_affine_resource(core_type) {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "using acquisition must produce an affine resource, found {core_type}"
                ),
                span: TypeError::span_from_span(span),
            });
        }
        let Some(resource_name) = direct_resource_type_name(core_type) else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "using acquisition type {core_type} contains affine state but has no direct cleanup registration"
                ),
                span: TypeError::span_from_span(span),
            });
        };
        cleanup_registration(resource_name).ok_or_else(|| TypeError::ConstraintSolvingFailed {
            reason: format!(
                "affine resource '{resource_name}' has no declared using cleanup registration"
            ),
            span: TypeError::span_from_span(span),
        })
    }

    /// Ensure cleanup errors are covered by the enclosing function's `errors` clause.
    fn ensure_using_cleanup_errors_allowed(
        &self,
        registration: &CleanupRegistration,
        span: Span,
    ) -> Result<(), TypeError> {
        let cleanup_errors = registration.cleanup_error_types();
        if cleanup_errors.is_empty() {
            return Ok(());
        }
        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(),
        };
        let cleanup_errors_are_allowed = cleanup_errors.iter().all(|cleanup_error| {
            current_fn_error_types.iter().any(|declared_error| {
                Self::declared_error_type_covers(cleanup_error, declared_error)
            })
        });
        if cleanup_errors_are_allowed {
            return Ok(());
        }
        Err(TypeError::PropagateErrorMismatch {
            expected: Self::format_error_type_list(&current_fn_error_types),
            found: Self::format_error_type_list(cleanup_errors.as_slice()),
            span: TypeError::span_from_span(
                self.symbol_table.current_function_span().unwrap_or(span),
            ),
            callee_span: TypeError::span_from_span(span),
        })
    }

    /// Register the scoped owner binding introduced by `using`.
    fn register_using_owner_binding(&mut self, binding: &LetBinding, core_type: CoreType) {
        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };
        self.clear_binding_ownership(binding.name.as_str());
        self.register_owner_binding_if_affine(binding.name.clone(), &core_type, binding.span);
        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type,
            visibility: Visibility::Private,
            source_location: binding.span,
            is_let_binding: true,
            is_mutable: binding.is_mutable,
            read_count: 0,
            is_pure: false,
        });
    }
}

impl CleanupRegistration {
    /// Convert cleanup error family names to nominal core types.
    fn cleanup_error_types(&self) -> Vec<CoreType> {
        self.cleanup_errors
            .iter()
            .map(|error| CoreType::Generic {
                name: (*error).to_owned(),
                type_args: Vec::new(),
            })
            .collect()
    }
}

/// Resolve a `using` binding annotation into a core type.
fn using_annotation_to_core_type(annotation: &Type) -> Result<CoreType, TypeError> {
    match ast_type_to_core_type(annotation).map_err(TypeError::from) {
        Ok(core_type) => Ok(core_type),
        Err(TypeError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
            name: type_name,
            type_args: Vec::new(),
        }),
        Err(other) => Err(other),
    }
}

/// Extract a direct nominal affine resource type name.
fn direct_resource_type_name(core_type: &CoreType) -> Option<&str> {
    let &CoreType::Generic {
        ref name,
        ref type_args,
    } = core_type
    else {
        return None;
    };
    if type_args.is_empty() {
        return Some(name.as_str());
    }
    None
}

/// Return the registered cleanup entry for one resource type.
fn cleanup_registration(resource_type: &str) -> Option<&'static CleanupRegistration> {
    CLEANUP_REGISTRATIONS
        .iter()
        .find(|registration| registration.resource_type == resource_type)
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::missing_docs_in_private_items,
        reason = "unit tests are named for behavior"
    )]

    use super::*;

    #[test]
    fn using_cleanup_registry_has_exact_terminal_transfer() {
        let terminal = cleanup_registration("TerminalSession")
            .expect("TerminalSession cleanup registration must exist");
        assert_eq!(
            terminal.transfer,
            Some(CleanupAuthorityTransfer {
                operation: "terminal_session_close_sync",
                error_family: "TerminalSessionRestoreError",
                variant: "CloseRestorePending",
            })
        );
        for resource in [
            "SystemWaitSet",
            "SystemOwnedWaitRegistration",
            "ProcessControlSource",
            "MonotonicTimer",
            "CancellationSource",
            "TerminalChordRouter",
            "TerminalTestScenario",
            "TerminalTestBackendActivation",
        ] {
            assert_eq!(
                cleanup_registration(resource).and_then(|registration| registration.transfer),
                None,
                "{resource} must not declare cleanup-authority transfer"
            );
        }
    }
}
