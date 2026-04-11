//! Control-flow expression typing helpers for the type checker.

extern crate alloc;

use super::helpers::{ensure_boolean_type, type_mismatch_error};
use crate::ast::{AstNode, BinaryOp, Expr, Stmt, Type};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

/// Context describing how a guard expression is consumed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardUsage {
    /// Guard result feeds into a surrounding expression.
    Expression,
    /// Guard is used for control flow, typically within a statement position.
    Statement,
}

/// Metadata describing the binding introduced by a guard expression.
#[derive(Debug, Clone)]
pub struct GuardBindingInfo<'type_ref> {
    /// Name of the binding created when the guard succeeds.
    pub name: &'type_ref str,
    /// Optional user-provided type annotation for the binding.
    pub annotation: Option<&'type_ref Type>,
    /// Whether the binding is declared as mutable.
    pub is_mutable: bool,
    /// Source span that identifies the binding declaration site.
    pub span: Span,
}

impl TypeChecker {
    /// Type-check an `if` expression and return its resulting type.
    pub(super) fn type_check_if_expr(
        &mut self,
        condition: &Expr,
        then_branch: &Stmt,
        else_branch: Option<&Stmt>,
        span: Span,
        expected_type: Option<&CoreType>,
    ) -> Result<CoreType, TypeError> {
        let condition_type = self.type_check_expr(condition)?;
        ensure_boolean_type(&condition_type, condition.span(), "if condition")?;

        let then_type = self.within_new_scope(|checker| {
            checker.apply_true_branch_type_narrowing(condition);
            checker.infer_stmt_value_type(then_branch)
        })?;

        if let Some(else_stmt) = else_branch {
            let else_type =
                self.within_new_scope(|checker| checker.infer_stmt_value_type(else_stmt))?;

            self.add_constraint(TypeConstraint::equality(
                then_type.clone(),
                else_type.clone(),
                Some(then_branch.span()),
                Some(else_stmt.span()),
            ));

            if !self.types_compatible(&then_type, &else_type) {
                return Err(type_mismatch_error(
                    &then_type,
                    Some(then_branch.span()),
                    &else_type,
                    else_stmt.span(),
                ));
            }

            Ok(then_type)
        } else {
            if let Some(required_type) = expected_type {
                if !matches!(required_type, &CoreType::Unit)
                    && !matches!(required_type, &CoreType::Variable(_))
                {
                    return Err(TypeError::MissingElseBranch {
                        expected_type: required_type.to_string(),
                        span: TypeError::span_from_span(span),
                    });
                }
            }
            self.type_check_stmt_with_return(then_branch, None)?;
            let unit = CoreType::Unit;
            self.add_constraint(TypeConstraint::equality(
                unit.clone(),
                unit.clone(),
                Some(span),
                Some(span),
            ));
            Ok(unit)
        }
    }

    /// Infer the resulting value type produced by a statement in expression context.
    pub(super) fn infer_stmt_value_type(&mut self, stmt: &Stmt) -> Result<CoreType, TypeError> {
        match *stmt {
            Stmt::Expression { ref expr, .. } => self.type_check_expr(expr),
            Stmt::Block { ref statements, .. } => self.infer_block_value_type(statements),
            Stmt::If {
                ref condition,
                ref then_branch,
                ref else_branch,
                span,
                ..
            } => {
                self.type_check_if_expr(condition, then_branch, else_branch.as_deref(), span, None)
            }
            _ => {
                self.type_check_stmt_with_return(stmt, None)?;
                Ok(CoreType::Unit)
            }
        }
    }

    /// Infer the resulting value type of a block in expression position.
    fn infer_block_value_type(&mut self, statements: &[Stmt]) -> Result<CoreType, TypeError> {
        let Some((last_stmt, prefix)) = statements.split_last() else {
            return Ok(CoreType::Unit);
        };

        for statement in prefix {
            self.type_check_stmt_with_return(statement, None)?;
        }

        self.infer_stmt_value_type(last_stmt)
    }

    /// Apply narrowing for `if x is TypeName` in the true branch scope.
    pub(super) fn apply_true_branch_type_narrowing(&mut self, condition: &Expr) {
        let narrowed = self.extract_is_type_narrowing(condition);
        let Some((variable_name, narrowed_type, source_location)) = narrowed else {
            return;
        };

        let existing_symbol = self.symbol_table().lookup(variable_name.as_str()).cloned();
        if let Some(mut symbol) = existing_symbol {
            symbol.core_type = narrowed_type;
            symbol.source_location = source_location;
            self.symbol_table.register(symbol);
        }
    }

    /// Extract `(variable_name, narrowed_type, span)` from `x is TypeName`.
    fn extract_is_type_narrowing(&self, condition: &Expr) -> Option<(String, CoreType, Span)> {
        let &Expr::Binary {
            ref left,
            operator: BinaryOp::Is,
            ref right,
            span,
            ..
        } = condition
        else {
            return None;
        };

        let Expr::Identifier {
            name: ref variable_name,
            ..
        } = *left.as_ref()
        else {
            return None;
        };

        let Expr::Identifier {
            name: ref type_name,
            ..
        } = *right.as_ref()
        else {
            return None;
        };

        self.environment()
            .lookup_type(type_name.as_str(), span)
            .ok()
            .cloned()
            .map(|narrowed_type| (variable_name.clone(), narrowed_type, span))
    }

    /// Type-check a guard expression and return its success type.
    pub(super) fn type_check_guard_expression(
        &mut self,
        expr: &Expr,
        binding_name: &str,
        binding_type: Option<&Type>,
        is_mutable: bool,
        else_branch: &Stmt,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let binding_info = GuardBindingInfo {
            name: binding_name,
            annotation: binding_type,
            is_mutable,
            span,
        };
        self.type_check_guard_expr(
            expr,
            &binding_info,
            else_branch,
            GuardUsage::Expression,
            None,
        )
    }
}
