//! Checker support for private transactional affine aggregate metadata.

extern crate alloc;

use super::TypeChecker;
use crate::ast::{AstNode, BorrowKind, Expr, Stmt};
use crate::token::Span;
use crate::type_system::affine_aggregates::{
    AffineAggregateSpec, nominal_type, terminal_affine_aggregate_specs,
    validate_affine_aggregate_spec,
};
use crate::type_system::errors::TypeError;
#[cfg(test)]
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
#[cfg(test)]
use alloc::vec::Vec;

/// A successful fallible retarget that must be followed by an infallible commit.
pub(super) struct PendingAggregateCommit {
    /// Aggregate root binding whose same-owner layout is awaiting commit.
    aggregate_root: String,
    /// Fallible operation that created this obligation.
    retarget_operation: String,
    /// Required immediately-following commit helper.
    commit_operation: String,
    /// Source span for diagnostics if interposition occurs.
    span: Span,
}

impl TypeChecker {
    /// Register terminal proposal aggregate metadata for focused internal tests.
    pub(super) fn register_terminal_affine_aggregates(&mut self) {
        for spec in terminal_affine_aggregate_specs() {
            let _registered = self.register_affine_aggregate_spec(spec);
        }
        self.register_editor_chord_runtime_shape();
    }

    /// Register one aggregate spec after validating exact obligation coverage.
    pub(super) fn register_affine_aggregate_spec(
        &mut self,
        spec: AffineAggregateSpec,
    ) -> Result<(), TypeError> {
        validate_affine_aggregate_spec(&spec).map_err(|reason| {
            TypeError::ConstraintSolvingFailed {
                reason,
                span: TypeError::unknown_span(),
            }
        })?;
        self.register_affine_resource_type(spec.type_name.clone());
        self.affine_aggregate_specs
            .insert(spec.type_name.clone(), spec);
        Ok(())
    }

    /// Register minimal private `EditorChordRuntime` field metadata needed by chord examples.
    fn register_editor_chord_runtime_shape(&mut self) {
        for type_name in [
            "EditorBindingMap",
            "EditorChordCapacityRecovery",
            "EditorChordRuntime",
            "EditorChordTimerRecoverySlot",
        ] {
            self.environment_mut()
                .register_type(type_name.to_owned(), nominal_type(type_name));
        }
        self.register_adt_fields(
            "EditorChordRuntime".to_owned(),
            field_map(&[
                ("binding_map", "EditorBindingMap"),
                ("router", "TerminalChordRouter"),
                ("prefix_timer", "MonotonicTimer"),
                ("active_registration", "SystemOwnedWaitRegistration"),
                ("timer_recovery_slot", "EditorChordTimerRecoverySlot"),
                ("capacity_recovery", "EditorChordCapacityRecovery"),
            ]),
        );
        self.register_adt_fields(
            "EditorChordTimerRecoverySlot".to_owned(),
            field_map(&[
                ("owned_registration", "SystemOwnedWaitRegistration"),
                ("candidate_source", "SystemReadinessSource"),
            ]),
        );
    }

    /// Allow a constructor field only when an opt-in aggregate owns the fresh acquisition.
    pub(super) fn allow_affine_aggregate_constructor_field(
        &self,
        owner_name: &str,
        field_name: &str,
        field_value: &Expr,
        field_type: &CoreType,
    ) -> Result<bool, TypeError> {
        if !self.core_type_contains_affine_resource(field_type) {
            return Ok(false);
        }
        let Some(spec) = self.affine_aggregate_specs.get(owner_name) else {
            return Ok(false);
        };
        if !spec.has_slot(field_name) {
            return Err(aggregate_error(
                format!(
                    "transactional affine aggregate '{}' has no declared slot '{}'",
                    spec.type_name, field_name
                ),
                field_value.span(),
            ));
        }
        if !Self::is_new_owner_call_result(field_value) {
            return Err(aggregate_error(
                format!(
                    "transactional affine aggregate '{}' slot '{}' must be initialized from a fresh acquisition",
                    spec.type_name, field_name
                ),
                field_value.span(),
            ));
        }
        if let Some(expected_type_name) = spec.slot_type(field_name) {
            if expected_type_name != field_type.to_string() {
                return Err(aggregate_error(
                    format!(
                        "transactional affine aggregate '{}' slot '{}' expected obligation '{}', found '{}'",
                        spec.type_name, field_name, expected_type_name, field_type
                    ),
                    field_value.span(),
                ));
            }
        }
        Ok(true)
    }

    /// Allow publishing a freshly sealed registered aggregate constructor into its owner binding.
    pub(super) fn is_affine_aggregate_constructor_result(
        &self,
        expr: &Expr,
        value_type: &CoreType,
    ) -> bool {
        let &CoreType::Generic { ref name, .. } = value_type else {
            return false;
        };
        let Some(spec) = self.affine_aggregate_specs.get(name) else {
            return false;
        };
        let &Expr::Constructor {
            ref callee,
            ref fields,
            ..
        } = expr
        else {
            return false;
        };
        let Expr::Identifier {
            name: ref constructor,
            ..
        } = **callee
        else {
            return false;
        };
        constructor == &spec.type_name
            && fields.iter().all(|field| {
                spec.slots.iter().any(|slot| slot.name == field.name)
                    && Self::is_new_owner_call_result(&field.value)
            })
    }

    /// Return whether a value may initialize a new owner binding.
    pub(super) fn is_fresh_owner_binding_result(&self, expr: &Expr, value_type: &CoreType) -> bool {
        Self::is_new_owner_call_result(expr)
            || self.is_affine_aggregate_constructor_result(expr, value_type)
    }

    /// Ensure no retarget/commit barrier is already pending before `statement` interposes.
    pub(super) fn check_affine_aggregate_statement_before(
        &self,
        statement: &Stmt,
    ) -> Result<(), TypeError> {
        let Some(pending) = self.context.pending_affine_aggregate_commit.as_ref() else {
            return Ok(());
        };
        if matching_commit_call_from_statement(
            statement,
            pending.commit_operation.as_str(),
            pending.aggregate_root.as_str(),
        )
        .is_some()
        {
            return Ok(());
        }
        Err(aggregate_error(
            format!(
                "fallible or observable operation cannot interpose after successful '{}' retarget before matching '{}' commit on aggregate '{}'",
                pending.retarget_operation, pending.commit_operation, pending.aggregate_root
            ),
            pending.span,
        ))
    }

    /// Update aggregate transition state after a statement has otherwise type checked.
    pub(super) fn update_affine_aggregate_statement_after(
        &mut self,
        statement: &Stmt,
    ) -> Result<(), TypeError> {
        if let Some(pending) = self.context.pending_affine_aggregate_commit.as_ref() {
            if matching_commit_call_from_statement(
                statement,
                pending.commit_operation.as_str(),
                pending.aggregate_root.as_str(),
            )
            .is_some()
            {
                self.context.pending_affine_aggregate_commit = None;
                return Ok(());
            }
        }

        if let Some((operation, span, aggregate_root)) =
            retarget_call_from_statement(self, statement)
        {
            if let Some((spec, rule)) = self.affine_aggregate_specs.values().find_map(|spec| {
                spec.retarget_rule(operation.as_str())
                    .map(|rule| (spec, rule))
            }) {
                self.context.pending_affine_aggregate_commit = Some(PendingAggregateCommit {
                    aggregate_root,
                    retarget_operation: operation,
                    commit_operation: rule.commit_operation.clone(),
                    span,
                });
                if !spec.has_transition_operation(rule.commit_operation.as_str()) {
                    return Err(aggregate_error(
                        format!(
                            "undeclared layout transition '{}' for transactional affine aggregate '{}'",
                            rule.commit_operation, spec.type_name
                        ),
                        span,
                    ));
                }
                return Ok(());
            }
        }

        if let Some((operation, span, aggregate_root)) = transition_call_from_statement(statement) {
            if let Some(spec) = aggregate_spec_for_root(self, aggregate_root.as_str()) {
                if spec.has_transition_operation(operation.as_str()) {
                    return Ok(());
                }
                if spec.helper_name_is_reserved_transition(operation.as_str()) {
                    return Err(aggregate_error(
                        format!(
                            "undeclared layout transition '{}' for transactional affine aggregate '{}'",
                            operation, spec.type_name
                        ),
                        span,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Reject a scope/function that exits while a retarget commit is still pending.
    pub(super) fn finish_affine_aggregate_statement_sequence(&self) -> Result<(), TypeError> {
        let Some(pending) = self.context.pending_affine_aggregate_commit.as_ref() else {
            return Ok(());
        };
        Err(aggregate_error(
            format!(
                "missing matching '{}' commit after successful '{}' retarget on aggregate '{}'",
                pending.commit_operation, pending.retarget_operation, pending.aggregate_root
            ),
            pending.span,
        ))
    }

    /// Register synthetic private aggregate symbols for unit tests.
    #[cfg(test)]
    pub(crate) fn register_terminal_aggregate_fixture_for_tests(
        &mut self,
    ) -> Result<(), TypeError> {
        let spec = crate::type_system::affine_aggregates::terminal_aggregate_fixture_spec();
        self.environment_mut().register_type(
            "TerminalAggregateMember".to_owned(),
            nominal_type("TerminalAggregateMember"),
        );
        self.environment_mut().register_type(
            "TerminalAggregateFixture".to_owned(),
            nominal_type("TerminalAggregateFixture"),
        );
        self.register_affine_resource_type("TerminalAggregateMember".to_owned());
        self.register_affine_resource_type("TerminalAggregateFixture".to_owned());
        self.register_adt_fields(
            "TerminalAggregateFixture".to_owned(),
            field_map(&[
                ("first", "TerminalAggregateMember"),
                ("second", "TerminalAggregateMember"),
            ]),
        );
        self.register_fixture_aggregate_function(
            "terminal_aggregate_member_new",
            &[],
            &[nominal_type("TerminalAggregateMember")],
            &[],
        );
        self.register_fixture_aggregate_function(
            "terminal_aggregate_retarget_member",
            &[
                nominal_type("TerminalAggregateFixture"),
                nominal_type("TerminalAggregateMember"),
            ],
            &[CoreType::Unit],
            &[BorrowKind::MutableRef, BorrowKind::Owned],
        );
        self.register_fixture_aggregate_function(
            "terminal_aggregate_commit_candidate",
            &[nominal_type("TerminalAggregateFixture")],
            &[CoreType::Unit],
            &[BorrowKind::MutableRef],
        );
        self.register_affine_aggregate_spec(spec)
    }

    /// Register one synthetic external operation used by aggregate source tests.
    #[cfg(test)]
    fn register_fixture_aggregate_function(
        &mut self,
        name: &str,
        parameters: &[CoreType],
        return_types: &[CoreType],
        borrow_kinds: &[BorrowKind],
    ) {
        self.register_symbol(SymbolInfo {
            name: name.to_owned(),
            symbol_type: SymbolType::Function,
            core_type: CoreType::Function {
                generic_params: Vec::new(),
                parameters: parameters.to_vec(),
                return_types: return_types.to_vec(),
                error_types: Vec::new(),
            },
            visibility: Visibility::Private,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
        if !borrow_kinds.is_empty() {
            self.register_function_borrow_modes_for_symbol(name.to_owned(), borrow_kinds);
        }
    }
}

/// Build a small field map from nominal type names.
fn field_map(entries: &[(&str, &str)]) -> BTreeMap<String, CoreType> {
    entries
        .iter()
        .map(|entry| (entry.0.to_owned(), nominal_type(entry.1)))
        .collect()
}

/// Build a compact affine aggregate diagnostic.
fn aggregate_error(reason: String, span: Span) -> TypeError {
    TypeError::ConstraintSolvingFailed {
        reason,
        span: TypeError::span_from_span(span),
    }
}

/// Return whether a statement directly calls `operation`.
fn statement_calls_operation(statement: &Stmt, operation: &str) -> bool {
    statement_call(statement)
        .and_then(call_operation_name)
        .is_some_and(|found| found == operation)
}

/// Return the commit call span only when its first argument mutably borrows the expected root.
fn matching_commit_call_from_statement(
    statement: &Stmt,
    operation: &str,
    aggregate_root: &str,
) -> Option<Span> {
    let call = statement_call(statement)?;
    if call_operation_name(call)? != operation {
        return None;
    }
    let first_root = call_args(call)
        .and_then(|args| args.first())
        .and_then(mutable_root_identifier)?;
    (first_root == aggregate_root).then(|| call.span())
}

/// Return the root identifier only for explicit `mutable ref` aggregate arguments.
fn mutable_root_identifier(expr: &Expr) -> Option<&str> {
    match *expr {
        Expr::BorrowArgument {
            ref target,
            borrow_kind: BorrowKind::MutableRef,
            ..
        } => root_identifier_ref(target.as_ref()),
        Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
            mutable_root_identifier(expr.as_ref())
        }
        _ => None,
    }
}

/// Borrowed variant of `root_identifier` for exact root comparisons.
fn root_identifier_ref(expr: &Expr) -> Option<&str> {
    match *expr {
        Expr::Identifier { ref name, .. } => Some(name.as_str()),
        Expr::Member { ref object, .. } => root_identifier_ref(object.as_ref()),
        Expr::BorrowArgument { ref target, .. } => root_identifier_ref(target.as_ref()),
        Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
            root_identifier_ref(expr.as_ref())
        }
        _ => None,
    }
}

/// Extract a fallible retarget call rooted in its first mutable aggregate argument.
fn retarget_call_from_statement(
    checker: &TypeChecker,
    statement: &Stmt,
) -> Option<(String, Span, String)> {
    let call = statement_call(statement)?;
    let operation = call_operation_name(call)?.to_owned();
    let aggregate_root = call_args(call)
        .and_then(|args| args.first())
        .and_then(mutable_root_identifier)?;
    aggregate_spec_for_root(checker, aggregate_root)
        .map(|_| (operation, call.span(), aggregate_root.to_owned()))
}

/// Extract an infallible transition helper call rooted in its first mutable aggregate argument.
fn transition_call_from_statement(statement: &Stmt) -> Option<(String, Span, String)> {
    let call = statement_call(statement)?;
    let operation = call_operation_name(call)?.to_owned();
    let aggregate_root = call_args(call)
        .and_then(|args| args.first())
        .and_then(mutable_root_identifier)?;
    Some((operation, call.span(), aggregate_root.to_owned()))
}

/// Return the call expression represented by a statement.
fn statement_call(statement: &Stmt) -> Option<&Expr> {
    match *statement {
        Stmt::Expression { ref expr, .. } => match *expr {
            Expr::Call { .. } => Some(expr),
            Expr::Propagate { ref call, .. } => Some(call.as_ref()),
            _ => None,
        },
        Stmt::Let {
            initializer: Some(Expr::Propagate { ref call, .. }),
            ..
        } => Some(call.as_ref()),
        _ => None,
    }
}

/// Return the identifier operation name for a call expression.
fn call_operation_name(call: &Expr) -> Option<&str> {
    let Expr::Call { ref callee, .. } = *call else {
        return None;
    };
    let Expr::Identifier { ref name, .. } = *callee.as_ref() else {
        return None;
    };
    Some(name.as_str())
}

/// Return call arguments for a call expression.
fn call_args(call: &Expr) -> Option<&[Expr]> {
    let Expr::Call { ref args, .. } = *call else {
        return None;
    };
    Some(args.as_slice())
}

/// Return the aggregate root when an expression is rooted in a registered aggregate owner.
fn aggregate_root_for_expr(checker: &TypeChecker, expr: &Expr) -> Option<String> {
    let root = root_identifier(expr)?;
    aggregate_spec_for_root(checker, root.as_str()).map(|_| root)
}

/// Return the aggregate spec for a root binding.
fn aggregate_spec_for_root<'checker>(
    checker: &'checker TypeChecker,
    root: &str,
) -> Option<&'checker AffineAggregateSpec> {
    let symbol = checker.symbol_table().lookup(root)?;
    let CoreType::Generic { ref name, .. } = symbol.core_type else {
        return None;
    };
    checker.affine_aggregate_specs.get(name)
}

/// Return the root identifier for identifiers, member chains, and borrow wrappers.
fn root_identifier(expr: &Expr) -> Option<String> {
    match *expr {
        Expr::Identifier { ref name, .. } => Some(name.clone()),
        Expr::Member { ref object, .. } => root_identifier(object.as_ref()),
        Expr::BorrowArgument { ref target, .. } => root_identifier(target.as_ref()),
        Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
            root_identifier(expr.as_ref())
        }
        _ => None,
    }
}

#[cfg(test)]
mod test_affine_aggregates;
