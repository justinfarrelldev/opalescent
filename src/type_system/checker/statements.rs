//! Statement type checking for the Opalescent type system

extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardUsage};
use super::helpers::{
    coerce_literal_to_expected, ensure_boolean_type, invalid_operation_error, is_integer_type,
    type_mismatch_error,
};
use crate::ast::{AstNode, Expr, LabeledValue, LetBinding, LiteralValue, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::{TypeError, Warning};
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::format;

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
                &Stmt::Return { .. } | &Stmt::Break { .. } | &Stmt::Continue { .. }
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
                        checker.symbol_table.register(SymbolInfo { name: variable_name.clone(),
                        symbol_type: SymbolType::Variable,
                        core_type: element_core,
                        visibility: Visibility::Private,
                        source_location: span,
                        is_let_binding: false,
                        is_mutable: false, read_count: 0, is_pure: false, });
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
                ref error_binding,
                ref else_body,
                span,
                ..
            } => self.type_check_guard_stmt_with_return(
                expression.as_ref(),
                success_binding.as_str(),
                error_binding.as_str(),
                else_body.as_ref(),
                span,
                expected_return,
            ),
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

                let existing_break_types = self.context.loop_break_type_stack.last().cloned().flatten();

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
                } else if let Some(loop_break_types) = self.context.loop_break_type_stack.last_mut() {
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
            self.type_check_guard_expr(
                guarded_expr.as_ref(),
                &binding_info,
                guard_else.as_ref(),
                GuardUsage::Statement,
                expected_return,
            )?;
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

        self.symbol_table.register(SymbolInfo { name: binding.name.clone(),
        symbol_type,
        core_type: final_type,
        visibility: Visibility::Private,
        source_location: binding.span,
        is_let_binding: true,
        is_mutable: binding.is_mutable, read_count: 0, is_pure: false, });

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

        let Expr::Loop { ref body, .. } = *initializer else {
            return Err(TypeError::InvalidOperation {
                operation: "destructuring let initializer must be loop expression".to_owned(),
                type_name: format!("{}", self.type_check_expr(initializer)?),
                span: TypeError::span_from_span(initializer.span()),
            });
        };

        self.context.loop_break_type_stack.push(None);
        self.type_check_stmt_with_return(body.as_ref(), None)?;
        let return_types = self.infer_loop_break_types(body.as_ref(), span)?;
        self.context.loop_break_type_stack.pop();

        if return_types.len() != bindings.len() {
            return Err(TypeError::ArityMismatch {
                expected: bindings.len(),
                found: return_types.len(),
                span: TypeError::span_from_span(initializer.span()),
            });
        }

        for (binding, value_type) in bindings.iter().zip(return_types.into_iter()) {
            if let Some(annotation) = binding.type_annotation.as_ref() {
                let annotated = ast_type_to_core_type(annotation).map_err(TypeError::from)?;
                if !self.types_compatible(&annotated, &value_type) {
                    return Err(type_mismatch_error(
                        &annotated,
                        Some(annotation.span()),
                        &value_type,
                        initializer.span(),
                    ));
                }
            }

            let symbol_type = if binding.is_mutable {
                SymbolType::Variable
            } else {
                SymbolType::Constant
            };

            self.symbol_table.register(SymbolInfo { name: binding.name.clone(),
            symbol_type,
            core_type: value_type,
            visibility: Visibility::Private,
            source_location: binding.span,
            is_let_binding: true,
            is_mutable: binding.is_mutable, read_count: 0, is_pure: false, });
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
        self.collect_break_types(stmt, &mut found_break_types, span)?;
        found_break_types.ok_or_else(|| TypeError::InvalidOperation {
            operation: "loop expression used in destructuring must break with values".to_owned(),
            type_name: "loop".to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Recursively collect break value types from all break statements within the loop body.
    fn collect_break_types(
        &mut self,
        stmt: &Stmt,
        found_break_types: &mut Option<alloc::vec::Vec<CoreType>>,
        span: Span,
    ) -> Result<(), TypeError> {
        match *stmt {
            Stmt::Break { ref values, .. } => {
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
                    *found_break_types = Some(current_types);
                }
                Ok(())
            }
            Stmt::Block { ref statements, .. } => {
                for statement in statements {
                    self.collect_break_types(statement, found_break_types, span)?;
                }
                Ok(())
            }
            Stmt::If {
                ref then_branch,
                ref else_branch,
                ..
            } => {
                self.collect_break_types(then_branch, found_break_types, span)?;
                if let Some(else_stmt) = else_branch.as_deref() {
                    self.collect_break_types(else_stmt, found_break_types, span)?;
                }
                Ok(())
            }
            Stmt::For { ref body, .. } | Stmt::While { ref body, .. } => {
                self.collect_break_types(body, found_break_types, span)
            }
            Stmt::Guard { ref else_body, .. } => {
                self.collect_break_types(else_body.as_ref(), found_break_types, span)
            }
            Stmt::Loop { .. }
            | Stmt::Let { .. }
            | Stmt::LetDestructure { .. }
            | Stmt::Assignment { .. }
            | Stmt::Return { .. }
            | Stmt::Expression { .. }
            | Stmt::Continue { .. }
            | Stmt::Comment { .. } => Ok(()),
        }
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

        let target_type = match *target {
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
                })?,
            _ => self.type_check_expr(target)?,
        };
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

    /// Type-check a guard statement by binding success/error names and checking the else body.
    fn type_check_guard_statement(
        &mut self,
        expression: &Expr,
        success_binding: &str,
        error_binding: &str,
        else_body: &Stmt,
        span: Span,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        let previous_guard_subject_context = self.context.in_guard_subject_context;
        self.context.in_guard_subject_context = true;
        let success_result = self.type_check_expr(expression);
        self.context.in_guard_subject_context = previous_guard_subject_context;
        let success_type = success_result?;

        self.symbol_table.register(SymbolInfo { name: success_binding.to_owned(),
        symbol_type: SymbolType::Constant,
        core_type: success_type,
        visibility: Visibility::Private,
        source_location: span,
        is_let_binding: true,
        is_mutable: false, read_count: 0, is_pure: false, });

        self.within_new_scope(|checker| {
            if let Some(existing_success) = checker.symbol_table.lookup(success_binding).cloned() {
                checker.symbol_table.register(SymbolInfo { name: success_binding.to_owned(),
                symbol_type: SymbolType::Constant,
                core_type: existing_success.core_type,
                visibility: Visibility::Private,
                source_location: span,
                is_let_binding: true,
                is_mutable: false, read_count: 0, is_pure: false, });
            }

            checker.symbol_table.register(SymbolInfo { name: error_binding.to_owned(),
            symbol_type: SymbolType::Constant,
            core_type: CoreType::String,
            visibility: Visibility::Private,
            source_location: span,
            is_let_binding: true,
            is_mutable: false, read_count: 0, is_pure: false, });

            checker.type_check_stmt_with_return(else_body, expected_return)
        })
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

        if labeled_count == 0 {
            self.ensure_return_label_mode(&[], span)?;
        } else {
            let labels: alloc::vec::Vec<String> =
                values.iter().map(|value| value.label.clone()).collect();
            self.ensure_return_label_mode(labels.as_slice(), span)?;
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

    /// Delegate guard statement typing while preserving expected return context.
    fn type_check_guard_stmt_with_return(
        &mut self,
        expression: &Expr,
        success_binding: &str,
        error_binding: &str,
        else_body: &Stmt,
        span: Span,
        expected_return: Option<&[CoreType]>,
    ) -> Result<(), TypeError> {
        self.type_check_guard_statement(
            expression,
            success_binding,
            error_binding,
            else_body,
            span,
            expected_return,
        )
    }
}
