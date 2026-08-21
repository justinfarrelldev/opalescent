//! Affine owner and second-class borrow validation for the checker.

extern crate alloc;

use crate::ast::{AstNode, BorrowKind, Expr, LambdaBody, Parameter, Stmt};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::String,
    vec::Vec,
};

/// Stored state for an affine owner binding.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OwnerBinding {
    /// Nominal display name for the affine owner type.
    type_name: String,
    /// Source span where this binding was introduced.
    source_location: Span,
    /// Source span of the consuming move, once the owner has been moved.
    moved_at: Option<Span>,
}

/// Stored state for a second-class borrow binding.
#[derive(Debug, Clone, PartialEq, Eq)]
struct BorrowBinding {
    /// Borrow mode carried by this binding.
    borrow_kind: BorrowKind,
    /// Source span where this borrow binding was introduced.
    source_location: Span,
}

/// Prior ownership metadata shadowed by one lexical scope.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct OwnershipScope {
    /// Owner bindings changed inside this scope and their previous values.
    owners: BTreeMap<String, Option<OwnerBinding>>,
    /// Borrow bindings changed inside this scope and their previous values.
    borrows: BTreeMap<String, Option<BorrowBinding>>,
    /// Function borrow metadata changed inside this scope and its previous value.
    function_borrows: BTreeMap<String, Option<Vec<BorrowKind>>>,
}

/// Checker-owned affine resource and borrow metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct OwnershipState {
    /// Nominal type names declared as compiler-registered affine resources.
    affine_resource_types: BTreeSet<String>,
    /// Live owner bindings keyed by source name.
    owners: BTreeMap<String, OwnerBinding>,
    /// Live second-class borrow bindings keyed by source name.
    borrows: BTreeMap<String, BorrowBinding>,
    /// Parameter borrow modes keyed by locally visible function symbol name.
    function_borrows: BTreeMap<String, Vec<BorrowKind>>,
    /// Stack used to restore shadowed metadata at lexical scope exit.
    scopes: Vec<OwnershipScope>,
}

impl OwnershipState {
    /// Enter a lexical scope for ownership metadata.
    fn enter_scope(&mut self) {
        self.scopes.push(OwnershipScope::default());
    }

    /// Exit a lexical scope and restore shadowed ownership metadata.
    fn exit_scope(&mut self) {
        let Some(scope) = self.scopes.pop() else {
            return;
        };
        for (name, previous) in scope.owners {
            restore_entry(&mut self.owners, name, previous);
        }
        for (name, previous) in scope.borrows {
            restore_entry(&mut self.borrows, name, previous);
        }
        for (name, previous) in scope.function_borrows {
            restore_entry(&mut self.function_borrows, name, previous);
        }
    }

    /// Record the previous owner value before this scope changes it.
    fn remember_owner(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope
                .owners
                .entry(name.to_owned())
                .or_insert_with(|| self.owners.get(name).cloned());
        }
    }

    /// Record the previous borrow value before this scope changes it.
    fn remember_borrow(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope
                .borrows
                .entry(name.to_owned())
                .or_insert_with(|| self.borrows.get(name).cloned());
        }
    }

    /// Record the previous function metadata before this scope changes it.
    fn remember_function_borrow(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope
                .function_borrows
                .entry(name.to_owned())
                .or_insert_with(|| self.function_borrows.get(name).cloned());
        }
    }
}

/// Restore a scope-shadowed metadata entry.
fn restore_entry<T>(entries: &mut BTreeMap<String, T>, name: String, previous: Option<T>) {
    if let Some(value) = previous {
        entries.insert(name, value);
    } else {
        entries.remove(&name);
    }
}

impl TypeChecker {
    /// Enter a lexical ownership scope in lockstep with the symbol table.
    pub(super) fn enter_ownership_scope(&mut self) {
        self.ownership.enter_scope();
    }

    /// Exit a lexical ownership scope in lockstep with the symbol table.
    pub(super) fn exit_ownership_scope(&mut self) {
        self.ownership.exit_scope();
    }

    /// Register one compiler-declared affine resource type by its local visible name.
    pub(super) fn register_affine_resource_type(&mut self, type_name: String) {
        self.ownership.affine_resource_types.insert(type_name);
    }

    /// Register core prerequisite affine resources that do not yet have parsed declarations.
    pub(super) fn register_core_prerequisite_affine_resources(&mut self) {
        for type_name in [
            "SystemWaitSet",
            "SystemOwnedWaitRegistration",
            "ProcessControlSource",
            "MonotonicTimer",
            "CancellationSource",
        ] {
            self.register_affine_resource_type(type_name.to_owned());
        }
    }
    /// Record local function parameter borrow modes.
    pub(super) fn register_function_borrow_kinds_for_symbol(
        &mut self,
        name: String,
        parameters: &[Parameter],
    ) {
        self.ownership.remember_function_borrow(&name);
        let borrow_kinds = parameters
            .iter()
            .map(|parameter| parameter.borrow_kind)
            .collect::<Vec<_>>();
        self.ownership.function_borrows.insert(name, borrow_kinds);
    }
    /// Record imported function parameter borrow modes.
    pub(super) fn register_function_borrow_modes_for_symbol(
        &mut self,
        name: String,
        borrow_kinds: &[BorrowKind],
    ) {
        self.ownership.remember_function_borrow(&name);
        self.ownership
            .function_borrows
            .insert(name, borrow_kinds.to_vec());
    }
    /// Return visible function parameter borrow modes.
    pub(super) fn function_borrow_kinds_for_symbol(&self, name: &str) -> Option<&[BorrowKind]> {
        self.ownership.function_borrows.get(name).map(Vec::as_slice)
    }
    /// Register owner or borrow metadata for one function/lambda parameter.
    pub(super) fn register_parameter_ownership(
        &mut self,
        parameter: &Parameter,
        core_type: &CoreType,
    ) {
        match parameter.borrow_kind {
            BorrowKind::Owned => {
                self.clear_binding_ownership(parameter.name.as_str());
                self.register_owner_binding_if_affine(
                    parameter.name.clone(),
                    core_type,
                    parameter.span,
                );
            }
            BorrowKind::Ref | BorrowKind::MutableRef => {
                self.register_borrow_binding(
                    parameter.name.clone(),
                    parameter.borrow_kind,
                    parameter.span,
                );
            }
        }
    }

    /// Register an owner binding when its type contains an affine resource.
    pub(super) fn register_owner_binding_if_affine(
        &mut self,
        name: String,
        core_type: &CoreType,
        span: Span,
    ) {
        if !self.core_type_contains_affine_resource(core_type) {
            return;
        }
        self.ownership.remember_owner(&name);
        self.ownership.remember_borrow(&name);
        self.ownership.borrows.remove(&name);
        self.ownership.owners.insert(
            name,
            OwnerBinding {
                type_name: core_type.to_string(),
                source_location: span,
                moved_at: None,
            },
        );
    }
    /// Clear owner and borrow metadata for a newly shadowed non-affine binding.
    pub(super) fn clear_binding_ownership(&mut self, name: &str) {
        self.ownership.remember_owner(name);
        self.ownership.remember_borrow(name);
        self.ownership.remember_function_borrow(name);
        self.ownership.owners.remove(name);
        self.ownership.borrows.remove(name);
        self.ownership.function_borrows.remove(name);
    }
    /// Register a second-class borrow binding.
    fn register_borrow_binding(&mut self, name: String, borrow_kind: BorrowKind, span: Span) {
        self.ownership.remember_borrow(&name);
        self.ownership.remember_owner(&name);
        self.ownership.owners.remove(&name);
        self.ownership.borrows.insert(
            name,
            BorrowBinding {
                borrow_kind,
                source_location: span,
            },
        );
    }

    /// Return whether `name` is currently a second-class borrow parameter/binding.
    pub(super) fn is_ref_param(&self, name: &str) -> bool {
        self.ownership.borrows.contains_key(name)
    }

    /// Reject use of an affine owner after it has been moved.
    pub(super) fn ensure_owner_not_moved(&self, name: &str, span: Span) -> Result<(), TypeError> {
        let Some(owner) = self.ownership.owners.get(name) else {
            return Ok(());
        };
        if owner.moved_at.is_none() {
            return Ok(());
        }
        Err(Self::affine_use_after_move_error(name, owner, span))
    }

    /// Type-check one call argument with ownership and borrow-mode awareness.
    pub(super) fn type_check_call_argument_for_param(
        &mut self,
        callee: &Expr,
        index: usize,
        argument: &Expr,
        parameter_type: &CoreType,
    ) -> Result<CoreType, TypeError> {
        let expected_borrow = self.expected_borrow_kind_for_call(callee, index);
        if matches!(
            expected_borrow,
            Some(BorrowKind::Ref | BorrowKind::MutableRef)
        ) {
            return self.type_check_explicit_borrow_argument(
                argument,
                expected_borrow.unwrap_or(BorrowKind::Owned),
            );
        }
        if matches!(*argument, Expr::BorrowArgument { .. }) {
            return Err(Self::borrow_escape_error(
                "temporary borrow",
                "be passed to an owned parameter",
                argument.span(),
            ));
        }

        let argument_type = self.type_check_expr(argument)?;
        if !self.core_type_contains_affine_resource(&argument_type) {
            return Ok(argument_type);
        }

        if matches!(expected_borrow, Some(BorrowKind::Owned)) {
            self.move_affine_argument(argument, parameter_type)?;
        } else {
            self.ensure_affine_argument_available(argument)?;
        }
        Ok(argument_type)
    }

    /// Reject an expression that would store, return, capture, or otherwise escape an owner/borrow.
    pub(super) fn check_ref_escape_in_expr(
        &self,
        expr: &Expr,
        context_description: &str,
        _span: Span,
    ) -> Result<(), TypeError> {
        self.check_affine_or_borrow_escape(expr, context_description)
    }

    /// Reject typed value escapes, optionally allowing fresh owner-producing calls for `let`.
    pub(super) fn check_value_escape(
        &self,
        expr: &Expr,
        value_type: &CoreType,
        context_description: &str,
        allow_new_owner_call: bool,
    ) -> Result<(), TypeError> {
        if self.core_type_contains_affine_resource(value_type) {
            if allow_new_owner_call && self.is_fresh_owner_binding_result(expr, value_type) {
                return Ok(());
            }
            self.check_affine_or_borrow_escape(expr, context_description)?;
            return Err(Self::affine_escape_error(
                "<expression>",
                &value_type.to_string(),
                context_description,
                expr.span(),
            ));
        }
        if Self::is_direct_borrow_escape_shape(expr) {
            return self.check_affine_or_borrow_escape(expr, context_description);
        }
        Ok(())
    }

    /// Reject typed value escapes without allowing fresh owner-producing calls.
    pub(super) fn check_value_escape_in(
        &self,
        expr: &Expr,
        value_type: &CoreType,
        context_description: &str,
    ) -> Result<(), TypeError> {
        self.check_value_escape(expr, value_type, context_description, false)
    }

    /// Reject lambdas that close over affine owners or second-class borrows.
    pub(super) fn check_lambda_captures_no_ref_params(
        &self,
        parameters: &[Parameter],
        body: &LambdaBody,
        span: Span,
    ) -> Result<(), TypeError> {
        let local_names = parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        self.check_lambda_body_for_capture(body, local_names.as_slice(), span)
    }

    /// Return whether a core type contains an affine resource directly or through known fields.
    pub(super) fn core_type_contains_affine_resource(&self, core_type: &CoreType) -> bool {
        self.core_type_contains_affine_resource_inner(core_type, &mut BTreeSet::new())
    }
    /// Return whether `name` is an affine resource type.
    pub(super) fn is_affine_resource_type_name(&self, name: &str) -> bool {
        self.ownership.affine_resource_types.contains(name)
    }
    /// Return the expected borrow kind for a call argument.
    fn expected_borrow_kind_for_call(&self, callee: &Expr, index: usize) -> Option<BorrowKind> {
        let name = Self::callee_borrow_metadata_name(callee)?;
        self.ownership
            .function_borrows
            .get(name.as_str())
            .and_then(|borrow_kinds| borrow_kinds.get(index))
            .copied()
    }
    /// Return the local symbol used for callee borrow metadata.
    fn callee_borrow_metadata_name(callee: &Expr) -> Option<String> {
        match *callee {
            Expr::Identifier { ref name, .. } => Some(name.clone()),
            Expr::Member {
                ref object,
                ref member,
                ..
            } => {
                let Expr::Identifier {
                    name: ref module_alias,
                    ..
                } = **object
                else {
                    return None;
                };
                Some(format!("{module_alias}.{member}"))
            }
            _ => None,
        }
    }
    /// Type-check explicit `ref` or `mutable ref` call syntax.
    fn type_check_explicit_borrow_argument(
        &mut self,
        argument: &Expr,
        expected: BorrowKind,
    ) -> Result<CoreType, TypeError> {
        let Expr::BorrowArgument {
            ref target,
            borrow_kind,
            span,
            ..
        } = *argument
        else {
            return Err(Self::borrow_required_error(expected, argument.span()));
        };
        if borrow_kind != expected {
            return Err(Self::borrow_mode_mismatch_error(
                expected,
                borrow_kind,
                span,
            ));
        }
        self.validate_borrow_target(target.as_ref(), expected, span)?;
        self.type_check_expr(target.as_ref())
    }

    /// Validate that a borrow argument names an in-scope non-moved binding.
    fn validate_borrow_target(
        &self,
        target: &Expr,
        expected: BorrowKind,
        span: Span,
    ) -> Result<(), TypeError> {
        let Expr::Identifier { ref name, .. } = *target else {
            return Err(Self::borrow_escape_error(
                "temporary borrow",
                "borrow a non-binding expression",
                span,
            ));
        };
        self.ensure_owner_not_moved(name, target.span())?;
        if expected == BorrowKind::MutableRef {
            if let Some(binding) = self.ownership.borrows.get(name) {
                if binding.borrow_kind != BorrowKind::MutableRef {
                    return Err(Self::borrow_mode_mismatch_error(
                        BorrowKind::MutableRef,
                        binding.borrow_kind,
                        span,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Consume an affine owner when it is passed to an owned parameter.
    fn move_affine_argument(
        &mut self,
        argument: &Expr,
        parameter_type: &CoreType,
    ) -> Result<(), TypeError> {
        let Expr::Identifier { ref name, span, .. } = *argument else {
            return Err(Self::affine_escape_error(
                "<expression>",
                &parameter_type.to_string(),
                "move through a non-binding expression",
                argument.span(),
            ));
        };
        if self.is_ref_param(name) {
            return Err(Self::borrow_escape_error(
                name,
                "be moved into an owned parameter",
                span,
            ));
        }
        self.ensure_owner_not_moved(name, span)?;
        let Some(owner) = self.ownership.owners.get_mut(name) else {
            return Ok(());
        };
        owner.moved_at = Some(span);
        Ok(())
    }

    /// Ensure an affine argument treated as an implicit proposal borrow is available.
    fn ensure_affine_argument_available(&self, argument: &Expr) -> Result<(), TypeError> {
        if let Expr::Identifier { ref name, span, .. } = *argument {
            self.ensure_owner_not_moved(name, span)?;
        }
        Ok(())
    }

    /// Return whether an expression syntactically produces a fresh call result.
    pub(super) fn is_new_owner_call_result(expr: &Expr) -> bool {
        match *expr {
            Expr::Call { .. } | Expr::Guard { .. } => true,
            Expr::Propagate { ref call, .. } => Self::is_new_owner_call_result(call.as_ref()),
            Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
                Self::is_new_owner_call_result(expr.as_ref())
            }
            _ => false,
        }
    }

    /// Return whether a non-affine expression can still carry a second-class borrow value.
    fn is_direct_borrow_escape_shape(expr: &Expr) -> bool {
        match *expr {
            Expr::BorrowArgument { .. } | Expr::Identifier { .. } => true,
            Expr::Parenthesized { ref expr, .. } | Expr::Cast { ref expr, .. } => {
                Self::is_direct_borrow_escape_shape(expr.as_ref())
            }
            _ => false,
        }
    }

    /// Recursively detect owner or borrow escape through an expression.
    #[expect(
        clippy::too_many_lines,
        reason = "Escape analysis exhaustively walks every expression variant in one match"
    )]
    fn check_affine_or_borrow_escape(
        &self,
        expr: &Expr,
        context_description: &str,
    ) -> Result<(), TypeError> {
        match *expr {
            Expr::Identifier { ref name, span, .. } => {
                if self.ownership.borrows.contains_key(name) {
                    return Err(Self::borrow_escape_error(name, context_description, span));
                }
                if let Some(owner) = self.ownership.owners.get(name) {
                    if owner.moved_at.is_some() {
                        return Err(Self::affine_use_after_move_error(name, owner, span));
                    }
                    return Err(Self::affine_escape_error(
                        name,
                        &owner.type_name,
                        context_description,
                        span,
                    ));
                }
                Ok(())
            }
            Expr::BorrowArgument { span, .. } => Err(Self::borrow_escape_error(
                "temporary borrow",
                context_description,
                span,
            )),
            Expr::Parenthesized { ref expr, .. }
            | Expr::TypeOf { ref expr, .. }
            | Expr::Cast { ref expr, .. } => {
                self.check_affine_or_borrow_escape(expr.as_ref(), context_description)
            }
            Expr::Unary { ref operand, .. } => {
                self.check_affine_or_borrow_escape(operand.as_ref(), context_description)
            }
            Expr::Binary {
                ref left,
                ref right,
                ..
            } => {
                self.check_affine_or_borrow_escape(left.as_ref(), context_description)?;
                self.check_affine_or_borrow_escape(right.as_ref(), context_description)
            }
            Expr::Call {
                ref callee,
                ref args,
                ..
            } => {
                self.check_affine_or_borrow_escape(callee.as_ref(), context_description)?;
                for argument in args {
                    if !matches!(*argument, Expr::BorrowArgument { .. }) {
                        self.check_affine_or_borrow_escape(argument, context_description)?;
                    }
                }
                Ok(())
            }
            Expr::Constructor {
                ref callee,
                ref fields,
                ..
            } => {
                self.check_affine_or_borrow_escape(callee.as_ref(), context_description)?;
                for field in fields {
                    self.check_affine_or_borrow_escape(&field.value, context_description)?;
                }
                Ok(())
            }
            Expr::Index {
                ref object,
                ref index,
                ..
            } => {
                self.check_affine_or_borrow_escape(object.as_ref(), context_description)?;
                self.check_affine_or_borrow_escape(index.as_ref(), context_description)
            }
            Expr::Member { ref object, .. } => {
                self.check_affine_or_borrow_escape(object.as_ref(), context_description)
            }
            Expr::Constrain { ref value, .. } => {
                self.check_affine_or_borrow_escape(value.as_ref(), context_description)
            }
            Expr::Refinement {
                ref value,
                ref variant,
                ..
            } => {
                self.check_affine_or_borrow_escape(value.as_ref(), context_description)?;
                self.check_affine_or_borrow_escape(variant.as_ref(), context_description)
            }
            Expr::StringInterpolation { ref parts, .. } => {
                for part in parts {
                    if let crate::ast::StringPart::Expression(ref expression) = *part {
                        self.check_affine_or_borrow_escape(expression, context_description)?;
                    }
                }
                Ok(())
            }
            Expr::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                self.check_affine_or_borrow_escape(condition.as_ref(), context_description)?;
                self.check_stmt_for_ref_capture(then_branch.as_ref(), then_branch.span())?;
                if let Some(branch) = else_branch.as_deref() {
                    self.check_stmt_for_ref_capture(branch, branch.span())?;
                }
                Ok(())
            }
            Expr::Array { ref elements, .. } => {
                for element in elements {
                    self.check_affine_or_borrow_escape(element, context_description)?;
                }
                Ok(())
            }
            Expr::Match {
                ref scrutinee,
                ref arms,
                ..
            } => {
                self.check_affine_or_borrow_escape(scrutinee.as_ref(), context_description)?;
                for arm in arms {
                    self.check_affine_or_borrow_escape(&arm.body, context_description)?;
                }
                Ok(())
            }
            Expr::Loop { ref body, .. } => {
                self.check_stmt_for_ref_capture(body.as_ref(), body.span())
            }
            Expr::Lambda {
                ref params,
                ref body,
                span,
                ..
            } => self.check_lambda_captures_no_ref_params(params.as_slice(), body, span),
            Expr::Guard {
                ref expr,
                ref else_branch,
                ..
            } => {
                self.check_affine_or_borrow_escape(expr.as_ref(), context_description)?;
                self.check_stmt_for_ref_capture(else_branch.as_ref(), else_branch.span())
            }
            Expr::Propagate {
                ref call,
                ref cause,
                ..
            } => {
                self.check_affine_or_borrow_escape(call.as_ref(), context_description)?;
                if let Some(cause_expr) = cause.as_deref() {
                    self.check_affine_or_borrow_escape(cause_expr, context_description)?;
                }
                Ok(())
            }
            Expr::Literal { .. } => Ok(()),
        }
    }

    /// Check if an expression captures a second-class borrow or affine owner.
    fn check_expr_for_ref_capture(&self, expr: &Expr, _span: Span) -> Result<(), TypeError> {
        self.check_affine_or_borrow_escape(expr, "be captured")
    }

    /// Check if a statement captures a second-class borrow or affine owner.
    fn check_stmt_for_ref_capture(&self, stmt: &Stmt, _span: Span) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Let {
                ref initializer, ..
            } => {
                if let Some(expr) = initializer.as_ref() {
                    self.check_expr_for_ref_capture(expr, expr.span())?;
                }
                Ok(())
            }
            Stmt::LetDestructure {
                ref initializer, ..
            } => self.check_expr_for_ref_capture(initializer, initializer.span()),
            Stmt::Assignment { ref value, .. } => {
                self.check_expr_for_ref_capture(value, value.span())
            }
            Stmt::Return { ref values, .. }
            | Stmt::Break { ref values, .. }
            | Stmt::Continue { ref values, .. } => {
                for value in values {
                    self.check_expr_for_ref_capture(&value.value, value.value.span())?;
                }
                Ok(())
            }
            Stmt::Expression { ref expr, .. } => self.check_expr_for_ref_capture(expr, expr.span()),
            Stmt::Block { ref statements, .. } => {
                for statement in statements {
                    self.check_stmt_for_ref_capture(statement, statement.span())?;
                }
                Ok(())
            }
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                self.check_expr_for_ref_capture(condition, condition.span())?;
                self.check_stmt_for_ref_capture(then_branch.as_ref(), then_branch.span())?;
                if let Some(branch) = else_branch.as_deref() {
                    self.check_stmt_for_ref_capture(branch, branch.span())?;
                }
                Ok(())
            }
            Stmt::For {
                ref iterable,
                ref body,
                ..
            } => {
                self.check_expr_for_ref_capture(iterable, iterable.span())?;
                self.check_stmt_for_ref_capture(body.as_ref(), body.span())
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                self.check_expr_for_ref_capture(condition, condition.span())?;
                self.check_stmt_for_ref_capture(body.as_ref(), body.span())
            }
            Stmt::Guard {
                ref expression,
                ref else_body,
                ..
            } => {
                self.check_expr_for_ref_capture(expression.as_ref(), expression.span())?;
                self.check_stmt_for_ref_capture(else_body.as_ref(), else_body.span())
            }
            Stmt::Using {
                ref acquisition,
                ref body,
                ..
            } => {
                self.check_expr_for_ref_capture(acquisition, acquisition.span())?;
                self.check_stmt_for_ref_capture(body.as_ref(), body.span())
            }
            Stmt::Loop { ref body, .. } => {
                self.check_stmt_for_ref_capture(body.as_ref(), body.span())
            }
            Stmt::PropagateGuardError { .. } | Stmt::Comment { .. } => Ok(()),
        }
    }

    /// Check lambda body capture while ignoring names declared as lambda parameters.
    fn check_lambda_body_for_capture(
        &self,
        body: &LambdaBody,
        local_names: &[&str],
        span: Span,
    ) -> Result<(), TypeError> {
        match *body {
            LambdaBody::Expression(ref expr) => {
                self.check_expr_for_capture_skipping_locals(expr, local_names)
            }
            LambdaBody::Block(ref statements) => {
                for statement in statements {
                    self.check_stmt_for_capture_skipping_locals(statement, local_names, span)?;
                }
                Ok(())
            }
        }
    }

    /// Check expression capture while ignoring lambda-local parameter names.
    fn check_expr_for_capture_skipping_locals(
        &self,
        expr: &Expr,
        local_names: &[&str],
    ) -> Result<(), TypeError> {
        match *expr {
            Expr::Identifier { ref name, .. } if local_names.contains(&name.as_str()) => Ok(()),
            Expr::BorrowArgument { ref target, .. } => {
                self.check_expr_for_capture_skipping_locals(target.as_ref(), local_names)
            }
            Expr::Call {
                ref callee,
                ref args,
                ..
            } => {
                self.check_expr_for_capture_skipping_locals(callee.as_ref(), local_names)?;
                for argument in args {
                    self.check_expr_for_capture_skipping_locals(argument, local_names)?;
                }
                Ok(())
            }
            Expr::Parenthesized { ref expr, .. }
            | Expr::TypeOf { ref expr, .. }
            | Expr::Cast { ref expr, .. } => {
                self.check_expr_for_capture_skipping_locals(expr.as_ref(), local_names)
            }
            Expr::Unary { ref operand, .. } => {
                self.check_expr_for_capture_skipping_locals(operand.as_ref(), local_names)
            }
            Expr::Binary {
                ref left,
                ref right,
                ..
            } => {
                self.check_expr_for_capture_skipping_locals(left.as_ref(), local_names)?;
                self.check_expr_for_capture_skipping_locals(right.as_ref(), local_names)
            }
            _ => self.check_affine_or_borrow_escape(expr, "be captured by a lambda"),
        }
    }

    /// Check statement capture while ignoring lambda-local parameter names.
    fn check_stmt_for_capture_skipping_locals(
        &self,
        stmt: &Stmt,
        local_names: &[&str],
        span: Span,
    ) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Let {
                ref initializer, ..
            } => {
                if let Some(expr) = initializer.as_ref() {
                    self.check_expr_for_capture_skipping_locals(expr, local_names)?;
                }
                Ok(())
            }
            Stmt::Expression { ref expr, .. } => {
                self.check_expr_for_capture_skipping_locals(expr, local_names)
            }
            Stmt::Return { ref values, .. }
            | Stmt::Break { ref values, .. }
            | Stmt::Continue { ref values, .. } => {
                for value in values {
                    self.check_expr_for_capture_skipping_locals(&value.value, local_names)?;
                }
                Ok(())
            }
            Stmt::Block { ref statements, .. } => {
                for statement in statements {
                    self.check_stmt_for_capture_skipping_locals(statement, local_names, span)?;
                }
                Ok(())
            }
            _ => self.check_stmt_for_ref_capture(stmt, span),
        }
    }

    /// Recursively determine whether a core type contains affine resource state.
    fn core_type_contains_affine_resource_inner(
        &self,
        core_type: &CoreType,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        match *core_type {
            CoreType::Array(ref element_type) => {
                self.core_type_contains_affine_resource_inner(element_type, visited)
            }
            CoreType::Generic {
                ref name,
                ref type_args,
            } => {
                self.ownership.affine_resource_types.contains(name)
                    || type_args.iter().any(|type_arg| {
                        self.core_type_contains_affine_resource_inner(type_arg, visited)
                    })
                    || self.known_fields_contain_affine_resource(name, visited)
            }
            CoreType::Function { .. }
            | CoreType::Int8
            | CoreType::Int16
            | CoreType::Int32
            | CoreType::Int64
            | CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
            | CoreType::Float32
            | CoreType::Float64
            | CoreType::String
            | CoreType::Boolean
            | CoreType::Unit
            | CoreType::Variable(_) => false,
        }
    }

    /// Check registered fields/variants for affine resource containment.
    fn known_fields_contain_affine_resource(
        &self,
        type_name: &str,
        visited: &mut BTreeSet<String>,
    ) -> bool {
        if !visited.insert(type_name.to_owned()) {
            return false;
        }
        if let Some(fields) = self.adt_fields_for_owner(type_name) {
            if fields.values().any(|field_type| {
                self.core_type_contains_affine_resource_inner(field_type, visited)
            }) {
                return true;
            }
        }
        self.adt_variants.get(type_name).is_some_and(|variants| {
            variants.iter().any(|variant| {
                self.adt_fields_for_owner(variant).is_some_and(|fields| {
                    fields.values().any(|field_type| {
                        self.core_type_contains_affine_resource_inner(field_type, visited)
                    })
                })
            })
        })
    }

    /// Build a precise use-after-move diagnostic.
    fn affine_use_after_move_error(name: &str, owner: &OwnerBinding, span: Span) -> TypeError {
        let moved_at = owner.moved_at.unwrap_or(owner.source_location);
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "affine resource '{name}' of type '{}' was already moved at byte {} and cannot be used again",
                owner.type_name, moved_at.start.offset,
            ),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a precise affine escape diagnostic.
    fn affine_escape_error(name: &str, type_name: &str, context: &str, span: Span) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!("affine resource '{name}' of type '{type_name}' cannot {context}"),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a precise borrow escape diagnostic.
    fn borrow_escape_error(name: &str, context: &str, span: Span) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!("second-class borrow '{name}' cannot {context}"),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a diagnostic for a missing call-site borrow argument.
    fn borrow_required_error(expected: BorrowKind, span: Span) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "parameter requires a '{}' call-site borrow argument",
                borrow_kind_label(expected),
            ),
            span: TypeError::span_from_span(span),
        }
    }

    /// Build a diagnostic for incompatible borrow modes.
    fn borrow_mode_mismatch_error(
        expected: BorrowKind,
        found: BorrowKind,
        span: Span,
    ) -> TypeError {
        TypeError::ConstraintSolvingFailed {
            reason: format!(
                "borrow argument mismatch: expected '{}', found '{}'",
                borrow_kind_label(expected),
                borrow_kind_label(found),
            ),
            span: TypeError::span_from_span(span),
        }
    }
}

/// Human-readable borrow kind for diagnostics.
const fn borrow_kind_label(kind: BorrowKind) -> &'static str {
    match kind {
        BorrowKind::Owned => "owned",
        BorrowKind::Ref => "ref",
        BorrowKind::MutableRef => "mutable ref",
    }
}
