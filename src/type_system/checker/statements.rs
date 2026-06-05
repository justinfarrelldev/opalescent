#![allow(
    clippy::arithmetic_side_effects,
    clippy::missing_docs_in_private_items,
    clippy::option_if_let_else,
    clippy::pattern_type_mismatch,
    clippy::too_many_lines,
    clippy::unused_self,
    reason = "statement typing keeps multi-return destructure validation and return pass-through checks localized during blocker cleanup"
)]
//! Statement type checking for the Opalescent type system

extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardCheckRequest, GuardUsage};
use super::helpers::{
    coerce_literal_to_expected, ensure_boolean_type, ensure_integer_type, invalid_operation_error,
    is_integer_type, type_mismatch_error,
};
use crate::ast::{AstNode, Expr, LabeledValue, LetBinding, LiteralValue, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::{FallibleExpressionContext, TypeChecker};
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::{TypeError, Warning};
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::{format, string::String, vec::Vec};

impl TypeChecker {
    /// Type check a slice of statements while propagating the expected return
    /// type for the enclosing function or lambda.
    pub(super) fn type_check_statements(
        &mut self,
        statements: &[Stmt],
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        let mut terminator_seen = false;
        let mut unreachable_warning_emitted = false;
        for statement in statements {
            if terminator_seen && !unreachable_warning_emitted {
                self.push_warning(Warning::UnreachableCode {
                    span: TypeError::span_from_span(statement.span()),
                    suppression_annotation: None,
                });
                unreachable_warning_emitted = true;
            }

            self.type_check_stmt_with_return(statement, expected_return)?;

            if matches!(
                statement,
                &Stmt::Return { .. }
                    | &Stmt::Break { .. }
                    | &Stmt::Continue { .. }
                    | &Stmt::PropagateGuardError { .. }
            ) {
                terminator_seen = true;
            }
        }
        Ok(())
    }

    /// Type check a single statement, validating it within the context of an
    /// optional expected return type.
    #[expect(
        clippy::too_many_lines,
        reason = "exhaustive statement-typechecking dispatch across all Stmt variants"
    )]
    pub(crate) fn type_check_stmt_with_return(
        &mut self,
        stmt: &Stmt,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Let {
                ref binding,
                ref initializer,
                ..
            } => self.type_check_let_statement(binding, initializer.as_ref()),
            Stmt::LetDestructure {
                ref bindings,
                ref initializer,
                span,
                ..
            } => self.type_check_let_destructure(bindings.as_slice(), initializer, span),
            Stmt::Assignment {
                ref target,
                ref value,
                span,
                ..
            } => self.type_check_assignment(target, value, span),
            Stmt::Return {
                ref values, span, ..
            } => self.type_check_return(values.as_slice(), expected_return, span),
            Stmt::Expression { ref expr, .. } => {
                self.type_check_expression_statement(expr, expected_return)
            }
            Stmt::Block { ref statements, .. } => self.within_new_scope(|checker| {
                checker.type_check_statements(statements, expected_return)
            }),
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                ensure_boolean_type(&condition_type, condition.span(), "if condition")?;
                self.within_new_scope(|checker| {
                    checker.apply_true_branch_type_narrowing(condition);
                    checker.type_check_stmt_with_return(then_branch.as_ref(), expected_return)
                })?;
                if let Some(else_branch_stmt) = else_branch.as_deref() {
                    self.within_new_scope(|checker| {
                        checker.type_check_stmt_with_return(else_branch_stmt, expected_return)
                    })?;
                }
                Ok(())
            }
            Stmt::For {
                ref variable,
                ref iterable,
                ref body,
                span,
                ..
            } => {
                let iterable_type = self.type_check_expr(iterable)?;
                if let Some(element_core) = self.iterable_element_type_for(&iterable_type) {
                    let variable_name = variable.clone();
                    self.within_new_scope(move |checker| {
                        checker.symbol_table.register(SymbolInfo {
                            name: variable_name.clone(),
                            symbol_type: SymbolType::Variable,
                            core_type: element_core,
                            visibility: Visibility::Private,
                            source_location: span,
                            is_let_binding: false,
                            is_mutable: false,
                            read_count: 0,
                            is_pure: false,
                        });
                        checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                    })
                } else {
                    Err(invalid_operation_error(
                        "for loop iteration",
                        &iterable_type,
                        span,
                    ))
                }
            }
            Stmt::While {
                ref condition,
                ref body,
                ..
            } => {
                let condition_type = self.type_check_expr(condition)?;
                ensure_boolean_type(&condition_type, condition.span(), "while condition")?;
                self.within_new_scope(|checker| {
                    checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                })
            }
            Stmt::Guard {
                ref expression,
                ref success_binding,
                ref success_binding_type,
                success_binding_is_mutable,
                ref success_bindings,
                ref error_binding,
                ref else_body,
                span,
                ..
            } => {
                let uses_multi_bindings = success_bindings.len() > 1
                    || success_bindings
                        .first()
                        .is_some_and(|binding| binding.returned_label.is_some());
                if uses_multi_bindings {
                    self.type_check_guard_destructure_statement(
                        expression.as_ref(),
                        success_bindings.as_slice(),
                        error_binding.as_str(),
                        else_body.as_ref(),
                        expected_return,
                    )?;
                    return Ok(());
                }

                let fallback_binding = success_bindings.first();
                let binding_info = GuardBindingInfo {
                    name: success_binding
                        .as_deref()
                        .or_else(|| fallback_binding.map(|binding| binding.name.as_str()))
                        .unwrap_or("_"),
                    annotation: success_binding_type.as_ref().or_else(|| {
                        fallback_binding.and_then(|binding| binding.type_annotation.as_ref())
                    }),
                    is_mutable: success_binding_is_mutable
                        || fallback_binding.is_some_and(|binding| binding.is_mutable),
                    span: fallback_binding.map_or(span, |binding| binding.span),
                };
                self.type_check_guard_expr(GuardCheckRequest {
                    expr: expression.as_ref(),
                    binding: &binding_info,
                    error_binding: Some(error_binding.as_str()),
                    else_branch: else_body.as_ref(),
                    usage: GuardUsage::Statement,
                    expected_return,
                })?;
                Ok(())
            }
            Stmt::Loop { ref body, .. } => self.within_new_scope(|checker| {
                checker.context.loop_break_type_stack.push(None);
                let result = checker.type_check_stmt_with_return(body.as_ref(), expected_return);
                checker.context.loop_break_type_stack.pop();
                result
            }),
            Stmt::Break {
                ref values, span, ..
            } => {
                let mut current_types = alloc::vec::Vec::new();
                for value in values {
                    current_types.push(self.type_check_expr(&value.value)?);
                }

                let existing_break_types =
                    self.context.loop_break_type_stack.last().cloned().flatten();

                if let Some(expected_types) = existing_break_types {
                    if expected_types.len() != current_types.len() {
                        return Err(TypeError::ArityMismatch {
                            expected: expected_types.len(),
                            found: current_types.len(),
                            span: TypeError::span_from_span(span),
                        });
                    }

                    for (expected_type, found_type) in
                        expected_types.iter().zip(current_types.iter())
                    {
                        if !self.types_compatible(expected_type, found_type) {
                            return Err(type_mismatch_error(expected_type, None, found_type, span));
                        }
                    }
                } else if let Some(loop_break_types) = self.context.loop_break_type_stack.last_mut()
                {
                    *loop_break_types = Some(current_types);
                }

                Ok(())
            }
            Stmt::Continue { ref values, .. } => {
                for value in values {
                    self.type_check_expr(&value.value)?;
                }
                Ok(())
            }
            Stmt::PropagateGuardError { .. } => {
                self.type_check_guard_error_clause_statement(stmt, expected_return, false)
            }
            Stmt::Comment { .. } => Ok(()),
        }
    }

    /// Type check an expression statement, accounting for guard expressions that
    /// introduce bindings or control-flow handlers.
    fn type_check_expression_statement(
        &mut self,
        expr: &Expr,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        if let Expr::Guard {
            expr: ref guarded_expr,
            binding_name: ref guard_name,
            binding_type: ref guard_type,
            is_mutable,
            else_branch: ref guard_else,
            span: guard_span,
            ..
        } = *expr
        {
            let binding_info = GuardBindingInfo {
                name: guard_name.as_str(),
                annotation: guard_type.as_ref(),
                is_mutable,
                span: guard_span,
            };
            self.type_check_guard_expr(GuardCheckRequest {
                expr: guarded_expr.as_ref(),
                binding: &binding_info,
                error_binding: None,
                else_branch: guard_else.as_ref(),
                usage: GuardUsage::Statement,
                expected_return,
            })?;
        } else {
            self.type_check_expr(expr)?;
        }
        Ok(())
    }

    /// Validate a `let` statement by resolving optional type annotations,
    /// initializer compatibility, and registering the binding in the current
    /// scope.
    pub(super) fn type_check_let_statement(
        &mut self,
        binding: &LetBinding,
        initializer: Option<&Expr>,
    ) -> Result<(), TypeError> {
        let annotated_type = binding
            .type_annotation
            .as_ref()
            .map(
                |annotation| match ast_type_to_core_type(annotation).map_err(TypeError::from) {
                    Ok(core_type) => Ok(core_type),
                    Err(TypeError::TypeNotFound { type_name, .. }) => Ok(CoreType::Generic {
                        name: type_name,
                        type_args: Vec::new(),
                    }),
                    Err(other) => Err(other),
                },
            )
            .transpose()?;

        let initializer_info = match initializer {
            Some(expr) => Some((self.type_check_expr(expr)?, expr)),
            None => None,
        };

        let final_type = match (annotated_type, initializer_info) {
            (Some(expected), Some((actual, expr))) => {
                let reconciled = if let &Expr::If {
                    ref condition,
                    ref then_branch,
                    ref else_branch,
                    span,
                    ..
                } = expr
                {
                    self.type_check_if_expr(
                        condition.as_ref(),
                        then_branch.as_ref(),
                        else_branch.as_deref(),
                        span,
                        Some(&expected),
                    )?
                } else {
                    actual
                };

                let reconciled = if self.types_compatible(&expected, &reconciled)
                    || matches!(reconciled, CoreType::Variable(_))
                    || matches!(&expected, &CoreType::Variable(_))
                {
                    reconciled
                } else if let Some(adjusted) =
                    coerce_literal_to_expected(&expected, expr, &reconciled)
                {
                    adjusted
                } else {
                    return Err(type_mismatch_error(
                        &expected,
                        binding.type_annotation.as_ref().map(Type::span),
                        &reconciled,
                        expr.span(),
                    ));
                };
                let annotation_span = binding.type_annotation.as_ref().map(Type::span);
                self.add_constraint(TypeConstraint::equality(
                    expected.clone(),
                    reconciled,
                    annotation_span,
                    Some(expr.span()),
                ));
                expected
            }
            (Some(expected), None) => expected,
            (None, Some((actual, _))) => actual,
            (None, None) => {
                return Err(TypeError::ConstraintSolvingFailed {
                    reason: format!(
                        "Cannot infer type for binding '{}' without annotation or initializer",
                        binding.name
                    ),
                    span: TypeError::span_from_span(binding.span),
                });
            }
        };

        let symbol_type = if binding.is_mutable {
            SymbolType::Variable
        } else {
            SymbolType::Constant
        };

        self.symbol_table.register(SymbolInfo {
            name: binding.name.clone(),
            symbol_type,
            core_type: final_type,
            visibility: Visibility::Private,
            source_location: binding.span,
            is_let_binding: true,
            is_mutable: binding.is_mutable,
            read_count: 0,
            is_pure: false,
        });

        Ok(())
    }

    /// Type-check a destructuring let binding, verifying each binding against the loop break values.
    fn type_check_let_destructure(
        &mut self,
        bindings: &[LetBinding],
        initializer: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        if bindings.is_empty() {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "destructuring let requires at least one binding".to_owned(),
                span: TypeError::span_from_span(span),
            });
        }

        if let Expr::Loop { ref body, .. } = *initializer {
            self.context.loop_break_type_stack.push(None);
            self.type_check_stmt_with_return(body.as_ref(), None)?;
            let return_types = self.infer_loop_break_types(body.as_ref(), span)?;
            self.context.loop_break_type_stack.pop();
            self.validate_destructure_bindings(
                bindings,
                return_types.as_slice(),
                bindings
                    .iter()
                    .map(|binding| {
                        binding
                            .returned_label
                            .clone()
                            .unwrap_or_else(|| binding.name.clone())
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
                initializer.span(),
            )?;
            return self.register_destructure_bindings(
                bindings,
                return_types.as_slice(),
                initializer.span(),
            );
        }

        if let Expr::Propagate { ref call, .. } = *initializer {
            let resolved = self.classify_fallible_call_shape(
                call.as_ref(),
                FallibleExpressionContext::Propagate,
            )?;
            self.ensure_propagate_error_types_allowed(
                call.as_ref(),
                resolved.error_types.as_slice(),
                initializer.span(),
            )?;
            self.validate_destructure_bindings(
                bindings,
                resolved.success_types.as_slice(),
                resolved.return_labels.as_slice(),
                initializer.span(),
            )?;
            return self.register_destructure_bindings(
                bindings,
                resolved.success_types.as_slice(),
                initializer.span(),
            );
        }

        if let Expr::Call {
            ref callee,
            ref generic_args,
            ref args,
            ..
        } = *initializer
        {
            let resolved = self.resolve_call_type(
                callee.as_ref(),
                generic_args.as_deref(),
                args.as_slice(),
                span,
            )?;
            self.validate_destructure_bindings(
                bindings,
                resolved.return_types.as_slice(),
                resolved.return_labels.as_slice(),
                initializer.span(),
            )?;
            return self.register_destructure_bindings(
                bindings,
                resolved.return_types.as_slice(),
                initializer.span(),
            );
        }

        Err(TypeError::InvalidOperation {
            operation: "destructuring let initializer must be loop expression, propagate call, or multi-return call"
                .to_owned(),
            type_name: format!("{}", self.type_check_expr(initializer)?),
            span: TypeError::span_from_span(initializer.span()),
        })
    }

    pub(super) fn validate_destructure_bindings(
        &self,
        bindings: &[LetBinding],
        value_types: &[CoreType],
        returned_labels: &[String],
        span: Span,
    ) -> Result<(), TypeError> {
        if value_types.len() != bindings.len() {
            return Err(TypeError::ArityMismatch {
                expected: bindings.len(),
                found: value_types.len(),
                span: TypeError::span_from_span(span),
            });
        }

        for (index, binding) in bindings.iter().enumerate() {
            let expected_label = returned_labels
                .get(index)
                .map_or(binding.name.as_str(), String::as_str);
            let actual_label = binding
                .returned_label
                .as_deref()
                .unwrap_or(binding.name.as_str());
            if actual_label != expected_label {
                let reason = if binding.returned_label.is_some() {
                    format!(
                        "explicit destructure labels verify intent and do not reorder returned values: expected label '{expected_label}' at position {}, found '{actual_label}'",
                        index + 1,
                    )
                } else {
                    format!(
                        "destructure binding names must exactly match returned labels in order: expected label '{expected_label}' at position {}, found '{actual_label}'",
                        index + 1,
                    )
                };
                return Err(TypeError::ReturnLabelMismatch {
                    expected: reason,
                    found: actual_label.to_owned(),
                    span: TypeError::span_from_span(binding.span()),
                });
            }
        }

        Ok(())
    }

    pub(super) fn register_destructure_bindings(
        &mut self,
        bindings: &[LetBinding],
        value_types: &[CoreType],
        initializer_span: Span,
    ) -> Result<(), TypeError> {
        for (binding, value_type) in bindings.iter().zip(value_types.iter()) {
            if let Some(annotation) = binding.type_annotation.as_ref() {
                let annotated = ast_type_to_core_type(annotation).map_err(TypeError::from)?;
                if !self.types_compatible(&annotated, value_type) {
                    return Err(type_mismatch_error(
                        &annotated,
                        Some(annotation.span()),
                        value_type,
                        initializer_span,
                    ));
                }
            }

            let symbol_type = if binding.is_mutable {
                SymbolType::Variable
            } else {
                SymbolType::Constant
            };

            self.symbol_table.register(SymbolInfo {
                name: binding.name.clone(),
                symbol_type,
                core_type: value_type.clone(),
                visibility: Visibility::Private,
                source_location: binding.span,
                is_let_binding: true,
                is_mutable: binding.is_mutable,
                read_count: 0,
                is_pure: false,
            });
        }

        Ok(())
    }

    /// Infer the break value types from a loop statement body.
    fn infer_loop_break_types(
        &mut self,
        stmt: &Stmt,
        span: Span,
    ) -> Result<alloc::vec::Vec<CoreType>, TypeError> {
        if let Some(active_loop_break_types) = self.context.loop_break_type_stack.last() {
            return active_loop_break_types
                .clone()
                .ok_or_else(|| TypeError::InvalidOperation {
                    operation: "loop expression used in destructuring must break with values"
                        .to_owned(),
                    type_name: "loop".to_owned(),
                    span: TypeError::span_from_span(span),
                });
        }

        let mut found_break_types: Option<alloc::vec::Vec<CoreType>> = None;
        let mut stack = alloc::vec::Vec::from([stmt]);
        while let Some(current) = stack.pop() {
            match current {
                Stmt::Break { values, .. } => {
                    let mut current_types = alloc::vec::Vec::new();
                    for value in values {
                        current_types.push(self.type_check_expr(&value.value)?);
                    }

                    if let Some(existing) = found_break_types.as_ref() {
                        if existing.len() != current_types.len() {
                            return Err(TypeError::ArityMismatch {
                                expected: existing.len(),
                                found: current_types.len(),
                                span: TypeError::span_from_span(span),
                            });
                        }

                        for (expected, found) in existing.iter().zip(current_types.iter()) {
                            if !self.types_compatible(expected, found) {
                                return Err(type_mismatch_error(expected, None, found, span));
                            }
                        }
                    } else {
                        found_break_types = Some(current_types);
                    }
                }
                Stmt::Block { statements, .. } => {
                    for statement in statements.iter().rev() {
                        stack.push(statement);
                    }
                }
                Stmt::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    if let Some(else_stmt) = else_branch.as_deref() {
                        stack.push(else_stmt);
                    }
                    stack.push(then_branch.as_ref());
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } => {
                    stack.push(body.as_ref());
                }
                Stmt::Guard { else_body, .. } => {
                    stack.push(else_body.as_ref());
                }
                Stmt::PropagateGuardError { .. }
                | Stmt::Loop { .. }
                | Stmt::Let { .. }
                | Stmt::LetDestructure { .. }
                | Stmt::Assignment { .. }
                | Stmt::Return { .. }
                | Stmt::Expression { .. }
                | Stmt::Continue { .. }
                | Stmt::Comment { .. } => {}
            }
        }

        found_break_types.ok_or_else(|| TypeError::InvalidOperation {
            operation: "loop expression used in destructuring must break with values".to_owned(),
            type_name: "loop".to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Ensure an assignment statement has a valid target and a value that is
    /// type compatible with that target.
    fn type_check_assignment(
        &mut self,
        target: &Expr,
        value: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Expr::Identifier {
            ref name,
            span: target_span,
            ..
        } = *target
        {
            if let Some(symbol) = self.symbol_table().lookup(name) {
                if !symbol.is_mutable {
                    return Err(TypeError::ImmutableAssignment {
                        name: name.clone(),
                        assignment_span: TypeError::span_from_span(target_span),
                        declaration_span: Some(TypeError::span_from_span(symbol.source_location)),
                    });
                }
            }
        }

        let target_type = self.type_check_assignment_target(target)?;
        let value_type = self.type_check_expr(value)?;
        let reconciled_value_type = if self.types_compatible(&target_type, &value_type) {
            value_type
        } else if let Some(adjusted) = coerce_literal_to_expected(&target_type, value, &value_type)
        {
            adjusted
        } else {
            return Err(type_mismatch_error(
                &target_type,
                Some(target.span()),
                &value_type,
                value.span(),
            ));
        };
        let validity = match *target {
            Expr::Identifier { .. } | Expr::Member { .. } | Expr::Index { .. } => Ok(()),
            _ => Err(invalid_operation_error(
                "assignment target",
                &target_type,
                span,
            )),
        };

        if validity.is_ok() {
            self.add_constraint(TypeConstraint::equality(
                target_type,
                reconciled_value_type,
                Some(target.span()),
                Some(value.span()),
            ));
        }

        validity
    }

    fn type_check_assignment_target(&mut self, target: &Expr) -> Result<CoreType, TypeError> {
        match *target {
            Expr::Identifier {
                ref name,
                span: identifier_span,
                ..
            } => self
                .symbol_table()
                .lookup(name)
                .map(|symbol| symbol.core_type.clone())
                .ok_or_else(|| TypeError::SymbolNotFound {
                    name: name.clone(),
                    suggestion: None,
                    span: TypeError::span_from_span(identifier_span),
                }),
            Expr::Index {
                ref object,
                ref index,
                span,
                ..
            } => {
                let object_type = self.type_check_assignment_target(object)?;
                let index_type = self.type_check_expr(index)?;
                ensure_integer_type(&index_type, index.span(), "indexing")?;
                match object_type {
                    CoreType::Array(element_type) => Ok(*element_type),
                    CoreType::String => Ok(CoreType::String),
                    other => Err(invalid_operation_error("index assignment", &other, span)),
                }
            }
            _ => self.type_check_expr(target),
        }
    }

    /// Validate a return statement against the function's expected return type,
    /// guaranteeing both presence and compatibility.
    fn type_check_return(
        &mut self,
        values: &[LabeledValue],
        expected_return: Option<&[CoreType]>,
        span: Span,
    ) -> Result<(), TypeError> {
        let expected = expected_return.ok_or_else(|| TypeError::InvalidOperation {
            operation: "return outside of function".to_owned(),
            type_name: "<unknown>".to_owned(),
            span: TypeError::span_from_span(span),
        })?;

        let pass_through_call = if values.len() == 1 && values[0].label.is_empty() {
            if let Expr::Call {
                ref callee,
                ref generic_args,
                ref args,
                ..
            } = values[0].value
            {
                self.validate_call_preconditions(callee.as_ref())?;
                Some(self.resolve_call_type(
                    callee.as_ref(),
                    generic_args.as_deref(),
                    args.as_slice(),
                    values[0].value.span(),
                )?)
            } else {
                None
            }
        } else {
            None
        };

        let labeled_count = values
            .iter()
            .filter(|value| !value.label.is_empty())
            .count();
        if labeled_count > 0 && labeled_count != values.len() {
            return Err(TypeError::ReturnLabelMismatch {
                expected: "all values labeled or all values unlabeled".to_owned(),
                found: "mixed labeled and unlabeled values in one return".to_owned(),
                span: TypeError::span_from_span(span),
            });
        }

        let effective_labels = if let Some(resolved) = pass_through_call.as_ref() {
            resolved.return_labels.clone()
        } else if labeled_count == 0 {
            Vec::new()
        } else {
            values.iter().map(|value| value.label.clone()).collect()
        };
        self.ensure_return_label_mode(effective_labels.as_slice(), span)?;

        if let Some(resolved) = pass_through_call {
            if resolved.return_types.len() != expected.len() {
                return Err(TypeError::ArityMismatch {
                    expected: expected.len(),
                    found: resolved.return_types.len(),
                    span: TypeError::span_from_span(span),
                });
            }

            for (expected_type, value_type) in
                expected.iter().zip(resolved.return_types.into_iter())
            {
                let reconciled_type = if self.types_compatible(expected_type, &value_type)
                    || matches!(value_type, CoreType::Variable(_))
                    || matches!(expected_type, &CoreType::Variable(_))
                {
                    value_type
                } else if is_integer_type(expected_type) && is_integer_type(&value_type) {
                    expected_type.clone()
                } else {
                    return Err(type_mismatch_error(
                        expected_type,
                        None,
                        &value_type,
                        values[0].value.span(),
                    ));
                };

                self.add_constraint(TypeConstraint::equality(
                    expected_type.clone(),
                    reconciled_type,
                    None,
                    Some(values[0].value.span()),
                ));
            }

            return Ok(());
        }

        if values.is_empty() {
            if expected.len() == 1
                && (matches!(expected[0], CoreType::Unit) || self.context.guard_else_depth > 0)
            {
                return Ok(());
            }

            return Err(TypeError::ArityMismatch {
                expected: expected.len(),
                found: 0,
                span: TypeError::span_from_span(span),
            });
        }

        if values.len() != expected.len() {
            return Err(TypeError::ArityMismatch {
                expected: expected.len(),
                found: values.len(),
                span: TypeError::span_from_span(span),
            });
        }

        for (index, value) in values.iter().enumerate() {
            let expected_type = &expected[index];

            if self.context.guard_else_depth > 0
                && matches!(
                    value.value,
                    Expr::Literal {
                        value: LiteralValue::Void,
                        ..
                    }
                )
            {
                continue;
            }

            let value_type = if let &Expr::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                span: if_span,
                ..
            } = &value.value
            {
                self.type_check_if_expr(
                    condition.as_ref(),
                    then_branch.as_ref(),
                    else_branch.as_deref(),
                    if_span,
                    Some(expected_type),
                )?
            } else {
                self.type_check_expr(&value.value)?
            };
            let reconciled_type = if self.types_compatible(expected_type, &value_type)
                || matches!(value_type, CoreType::Variable(_))
                || matches!(expected_type, &CoreType::Variable(_))
            {
                value_type
            } else if let Some(adjusted) =
                coerce_literal_to_expected(expected_type, &value.value, &value_type)
            {
                adjusted
            } else if is_integer_type(expected_type) && is_integer_type(&value_type) {
                expected_type.clone()
            } else {
                return Err(type_mismatch_error(
                    expected_type,
                    None,
                    &value_type,
                    value.value.span(),
                ));
            };

            self.add_constraint(TypeConstraint::equality(
                expected_type.clone(),
                reconciled_type,
                None,
                Some(value.value.span()),
            ));
        }

        Ok(())
    }

    /// Type check a statement and update the symbol table as needed.
    ///
    /// # Errors
    /// Returns `TypeError` variants when statement typing fails.
    pub fn type_check_stmt(&mut self, stmt: &Stmt) -> Result<(), TypeError> {
        self.type_check_stmt_with_return(stmt, None)
    }
}
