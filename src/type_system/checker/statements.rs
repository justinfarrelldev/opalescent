//! Statement type checking for the Opalescent type system

extern crate alloc;

use super::control_flow::{GuardBindingInfo, GuardUsage};
use super::helpers::{
    coerce_literal_to_expected, ensure_boolean_type, invalid_operation_error, type_mismatch_error,
};
use crate::ast::{AstNode, Expr, LabeledValue, LetBinding, LiteralValue, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::{TypeError, Warning};
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
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
                match iterable_type {
                    CoreType::Array(element_type) => {
                        let element_core = *element_type;
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
                            });
                            checker.type_check_stmt_with_return(body.as_ref(), expected_return)
                        })
                    }
                    _ => Err(invalid_operation_error(
                        "for loop iteration",
                        &iterable_type,
                        span,
                    )),
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
            Stmt::Loop { ref body, .. } => self.within_new_scope(|checker| {
                checker.type_check_stmt_with_return(body.as_ref(), expected_return)
            }),
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
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
            .map(Self::ast_type_to_core_type)
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
        });

        Ok(())
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
                && (matches!(expected[0], CoreType::Unit) || self.guard_else_depth > 0)
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

            if self.guard_else_depth > 0
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
