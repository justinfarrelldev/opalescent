//! `using` cleanup registration and statement type checking.

extern crate alloc;

use super::helpers::type_mismatch_error;
use crate::ast::{AstNode, BorrowKind, Expr, LetBinding, Stmt, Type};
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

/// One active compiler-owned cleanup obligation for a scoped `using` binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct UsingCleanupObligation {
    /// Binding whose resource owns the cleanup obligation.
    binding_name: String,
    /// Declared cleanup operation that can consume this obligation.
    cleanup_operation: &'static str,
    /// Optional exact cleanup-authority transfer result.
    transfer: Option<CleanupAuthorityTransfer>,
    /// Whether successful explicit close already consumed the obligation.
    consumed: bool,
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

        self.push_using_cleanup_obligation(binding.name.clone(), registration);
        let result = self.within_new_scope(|checker| {
            checker.register_using_owner_binding(binding, binding_type.clone());
            checker.type_check_stmt_with_return(body.as_ref(), expected_return)
        });
        let _obligation = self.context.using_cleanup_obligations.pop();
        result
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

    /// Push one active cleanup obligation for a `using` binding.
    fn push_using_cleanup_obligation(
        &mut self,
        binding_name: String,
        registration: &'static CleanupRegistration,
    ) {
        self.context
            .using_cleanup_obligations
            .push(UsingCleanupObligation {
                binding_name,
                cleanup_operation: registration.cleanup_operation,
                transfer: registration.transfer,
                consumed: false,
            });
    }

    /// Record successful explicit cleanup if `call` is the registered operation for a live obligation.
    pub(super) fn consume_using_cleanup_obligation_after_success(&mut self, call: &Expr) {
        let Expr::Call {
            ref callee,
            ref args,
            ..
        } = *call
        else {
            return;
        };
        let Some(operation_name) = cleanup_operation_name(callee.as_ref()) else {
            return;
        };
        let Some(binding_name) = cleanup_mutable_ref_binding(args.as_slice()) else {
            return;
        };
        if let Some(obligation) = self
            .context
            .using_cleanup_obligations
            .iter_mut()
            .rev()
            .find(|obligation| obligation.binding_name == binding_name)
        {
            if obligation.cleanup_operation == operation_name {
                obligation.consumed = true;
            }
        }
    }

    /// Return whether a cleanup result is the exact registered authority transfer.
    fn cleanup_result_transfers_authority(
        &self,
        binding_name: &str,
        operation: &str,
        error_family: &str,
        variant: &str,
    ) -> bool {
        self.context
            .using_cleanup_obligations
            .iter()
            .rev()
            .find(|obligation| obligation.binding_name == binding_name)
            .and_then(|obligation| obligation.transfer)
            .is_some_and(|transfer| {
                transfer.operation == operation
                    && transfer.error_family == error_family
                    && transfer.variant == variant
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

/// Extract a called operation name from a direct or module-qualified callee.
fn cleanup_operation_name(callee: &Expr) -> Option<&str> {
    match *callee {
        Expr::Identifier { ref name, .. } => Some(name.as_str()),
        Expr::Member { ref member, .. } => Some(member.as_str()),
        _ => None,
    }
}

/// Extract the binding name from a cleanup operation's `mutable ref` argument.
fn cleanup_mutable_ref_binding(args: &[Expr]) -> Option<&str> {
    let Expr::BorrowArgument {
        ref target,
        borrow_kind: BorrowKind::MutableRef,
        ..
    } = *args.first()?
    else {
        return None;
    };
    let Expr::Identifier { ref name, .. } = **target else {
        return None;
    };
    Some(name.as_str())
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

    fn test_span() -> Span {
        Span::single(crate::token::Position::new(1, 1, 0))
    }

    fn mutable_ref_call(operation: &str, binding_name: &str) -> Expr {
        Expr::Call {
            callee: Box::new(Expr::Identifier {
                name: operation.to_owned(),
                span: test_span(),
                id: crate::ast::NodeId(91_000),
            }),
            generic_args: None,
            args: vec![Expr::BorrowArgument {
                target: Box::new(Expr::Identifier {
                    name: binding_name.to_owned(),
                    span: test_span(),
                    id: crate::ast::NodeId(91_001),
                }),
                borrow_kind: BorrowKind::MutableRef,
                span: test_span(),
                id: crate::ast::NodeId(91_002),
            }],
            span: test_span(),
            id: crate::ast::NodeId(91_003),
        }
    }

    #[test]
    fn using_cleanup_consumes_only_registered_explicit_close() {
        let mut checker = TypeChecker::new();
        let registration = cleanup_registration("TerminalSession")
            .expect("TerminalSession cleanup registration must exist");
        checker.push_using_cleanup_obligation(String::from("session"), registration);
        checker.consume_using_cleanup_obligation_after_success(&mutable_ref_call(
            "terminal_session_state",
            "session",
        ));
        assert!(
            !checker.context.using_cleanup_obligations[0].consumed,
            "ordinary mutable-ref operations must not consume cleanup authority"
        );
        checker.consume_using_cleanup_obligation_after_success(&mutable_ref_call(
            "terminal_session_close_sync",
            "other_session",
        ));
        assert!(
            !checker.context.using_cleanup_obligations[0].consumed,
            "registered cleanup operation on another binding must not consume this obligation"
        );
        checker.consume_using_cleanup_obligation_after_success(&mutable_ref_call(
            "terminal_session_close_sync",
            "session",
        ));
        assert!(
            checker.context.using_cleanup_obligations[0].consumed,
            "successful registered close must consume exactly the live using obligation"
        );
    }

    #[test]
    fn using_cleanup_transfer_requires_exact_registered_result() {
        let mut checker = TypeChecker::new();
        let registration = cleanup_registration("TerminalSession")
            .expect("TerminalSession cleanup registration must exist");
        checker.push_using_cleanup_obligation(String::from("session"), registration);
        assert!(checker.cleanup_result_transfers_authority(
            "session",
            "terminal_session_close_sync",
            "TerminalSessionRestoreError",
            "CloseRestorePending",
        ));
        assert!(!checker.cleanup_result_transfers_authority(
            "session",
            "terminal_session_close_sync",
            "TerminalSessionRestoreError",
            "PendingCloseRestoreFailed",
        ));
        assert!(!checker.cleanup_result_transfers_authority(
            "session",
            "terminal_session_state",
            "TerminalSessionRestoreError",
            "CloseRestorePending",
        ));
    }

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
