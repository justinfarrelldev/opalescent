#![allow(
    clippy::missing_docs_in_private_items,
    clippy::pattern_type_mismatch,
    clippy::shadow_unrelated,
    reason = "fallible expression helpers intentionally match on borrowed AST nodes"
)]
//! Fallible-expression classification helpers extracted from `expressions.rs`.

extern crate alloc;

use super::helpers::{coerce_literal_to_expected, type_mismatch_error};
use super::{
    FallibleCallShape, FallibleExpressionContext, FallibleExpressionInfo, FallibleExpressionKind,
    TypeChecker,
};
use crate::ast::{AstNode, Expr, Type, TypeDeclarationForm, TypeDef};
use crate::token::Span;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::fallible_constructors::{
    CanonicalTypeIdentity, lookup_fallible_constructor,
};
use crate::type_system::type_mapping::ast_type_to_core_type;
use crate::type_system::types::CoreType;
use alloc::{format, string::String};

impl TypeChecker {
    /// Type-check a `propagate` expression.
    ///
    /// This function ensures that:
    /// 1. The `propagate` expression is used inside a function that declares error types.
    /// 2. The inner expression is a fallible call, registered fallible constructor, or
    ///    already-evaluated immutable error binding.
    /// 3. The error types produced by the inner expression are a subset of the error types
    ///    declared by the enclosing function.
    ///
    /// # Errors
    ///
    /// - `PropagateOutsideErrorFunction`: If used outside a function declaring errors.
    /// - `PropagateErrorMismatch`: If the propagated errors are not a subset of the
    ///   enclosing function's declared errors.
    pub(super) fn type_check_propagate_expr(
        &mut self,
        call: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if let Some(error_types) = self.propagated_error_value_types(call)? {
            self.ensure_propagate_error_types_allowed(call, error_types.as_slice(), span)?;
            return Ok(CoreType::Unit);
        }

        let fallible_info =
            self.classify_fallible_expression(call, FallibleExpressionContext::Propagate)?;
        self.ensure_propagate_error_types_allowed(
            call,
            fallible_info.error_types.as_slice(),
            span,
        )?;
        self.consume_using_cleanup_obligation_after_success(call);
        Ok(fallible_info.success_type)
    }

    /// Validate the optional `cause` operand for a propagate expression.
    pub(super) fn type_check_propagate_cause_expr(
        &self,
        primary: &Expr,
        cause: &Expr,
        span: Span,
    ) -> Result<(), TypeError> {
        let Expr::Identifier {
            name: cause_name,
            span: cause_span,
            ..
        } = cause
        else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "propagate cause must be an already-evaluated immutable error binding"
                    .to_owned(),
                span: TypeError::span_from_span(cause.span()),
            });
        };

        if Self::identifier_name(primary).is_some_and(|primary_name| primary_name == cause_name) {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: "propagate cause cannot reference the same immutable error instance as the primary"
                    .to_owned(),
                span: TypeError::span_from_span(span),
            });
        }

        let Some(symbol) = self.symbol_table().lookup(cause_name) else {
            return Err(TypeError::SymbolNotFound {
                name: cause_name.clone(),
                suggestion: self.suggest_visible_identifier(cause_name),
                span: TypeError::span_from_span(*cause_span),
            });
        };

        if symbol.is_mutable || !Self::core_type_can_be_error_value(&symbol.core_type) {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "propagate cause binding '{cause_name}' must be an immutable error value, found '{}'",
                    symbol.core_type
                ),
                span: TypeError::span_from_span(*cause_span),
            });
        }

        Ok(())
    }

    /// Return the nominal families represented by an already-evaluated error value.
    fn propagated_error_value_types(
        &self,
        expr: &Expr,
    ) -> Result<Option<Vec<CoreType>>, TypeError> {
        match expr {
            Expr::Parenthesized { expr, .. } | Expr::BorrowArgument { target: expr, .. } => {
                self.propagated_error_value_types(expr.as_ref())
            }
            Expr::Identifier { name, span, .. } => {
                let Some(symbol) = self.symbol_table().lookup(name) else {
                    return Err(TypeError::SymbolNotFound {
                        name: name.clone(),
                        suggestion: self.suggest_visible_identifier(name),
                        span: TypeError::span_from_span(*span),
                    });
                };
                Ok(self.propagated_error_types_from_core_type(name, &symbol.core_type))
            }
            _ => Ok(None),
        }
    }

    /// Map a core type for an error binding back to its propagatable nominal families.
    fn propagated_error_types_from_core_type(
        &self,
        binding_name: &str,
        core_type: &CoreType,
    ) -> Option<Vec<CoreType>> {
        if self
            .context
            .active_guard_error_bindings
            .last()
            .is_some_and(|binding| binding.name == binding_name)
        {
            return Some(self.active_guard_propagation_error_types(binding_name));
        }

        if let CoreType::Generic { name, type_args } = core_type {
            if name == "GuardErrorContext" {
                return Some(type_args.clone());
            }
            if name == "Error" {
                return None;
            }
            if let Some(narrowed_family) = self.narrowed_guard_error_family(core_type) {
                return Some(vec![narrowed_family]);
            }
            if type_args.is_empty() && Self::type_name_can_be_error_value(name) {
                return Some(vec![core_type.clone()]);
            }
        }

        None
    }

    /// Return whether a source expression is an identifier and borrow its name.
    fn identifier_name(expr: &Expr) -> Option<&str> {
        match expr {
            Expr::Parenthesized { expr, .. } | Expr::BorrowArgument { target: expr, .. } => {
                Self::identifier_name(expr.as_ref())
            }
            Expr::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        }
    }

    /// Return whether a core type can be used as an immutable error cause value.
    fn core_type_can_be_error_value(core_type: &CoreType) -> bool {
        matches!(
            core_type,
            CoreType::Generic { name, type_args }
                if (name == "GuardErrorContext" || name == "Error" || Self::type_name_can_be_error_value(name))
                    && (name == "GuardErrorContext" || type_args.is_empty())
        )
    }

    /// Return whether a nominal type name is error-like enough for cause retention.
    fn type_name_can_be_error_value(name: &str) -> bool {
        name.ends_with("Error") || name.contains("Error.")
    }

    pub(super) fn ensure_propagate_error_types_allowed(
        &self,
        call: &Expr,
        error_types: &[CoreType],
        span: Span,
    ) -> Result<(), TypeError> {
        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(),
        };

        let is_subset = error_types.iter().all(|error_type| {
            current_fn_error_types
                .iter()
                .any(|declared_error| Self::declared_error_type_covers(error_type, declared_error))
        });

        if !is_subset {
            return Err(TypeError::PropagateErrorMismatch {
                expected: Self::format_error_type_list(&current_fn_error_types),
                found: Self::format_error_type_list(error_types),
                span: TypeError::span_from_span(
                    self.symbol_table.current_function_span().unwrap_or(span),
                ),
                callee_span: TypeError::span_from_span(call.span()),
            });
        }

        Ok(())
    }

    pub(super) fn classify_fallible_call_shape(
        &mut self,
        expr: &Expr,
        context: FallibleExpressionContext,
    ) -> Result<FallibleCallShape, TypeError> {
        let Expr::Call {
            callee,
            generic_args,
            args,
            span,
            ..
        } = expr
        else {
            return Err(Self::non_error_expression_type_error(context, expr.span()));
        };

        let callee_type = self.type_check_expr(callee.as_ref())?;
        let CoreType::Function { error_types, .. } = callee_type else {
            return Err(Self::non_error_expression_type_error(context, expr.span()));
        };

        if error_types.is_empty() {
            return Err(Self::non_error_expression_type_error(context, expr.span()));
        }

        let previous_propagate_context = self.context.in_propagate_context;
        let previous_guard_subject_context = self.context.in_guard_subject_context;
        match context {
            FallibleExpressionContext::Propagate => self.context.in_propagate_context = true,
            FallibleExpressionContext::Guard => self.context.in_guard_subject_context = true,
        }
        let resolved = self.resolve_call_type(
            callee.as_ref(),
            generic_args.as_deref(),
            args.as_slice(),
            *span,
        );
        self.context.in_propagate_context = previous_propagate_context;
        self.context.in_guard_subject_context = previous_guard_subject_context;
        let resolved = resolved?;

        Ok(FallibleCallShape {
            success_types: resolved.return_types,
            return_labels: resolved.return_labels,
            error_types,
        })
    }

    /// Classify a fallible expression used by `propagate` and `guard`.
    pub(super) fn classify_fallible_expression(
        &mut self,
        expr: &Expr,
        context: FallibleExpressionContext,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        match expr {
            Expr::Parenthesized { expr, .. } | Expr::BorrowArgument { target: expr, .. } => {
                self.classify_fallible_expression(expr.as_ref(), context)
            }
            Expr::Call {
                callee,
                generic_args,
                args,
                span,
                ..
            } => self.classify_call_fallible_expression(
                expr,
                callee,
                generic_args.as_deref(),
                args.as_slice(),
                *span,
                context,
            ),
            Expr::Constructor {
                callee,
                fields,
                span,
                ..
            } => self.classify_constructor_fallible_expression(
                callee,
                fields.as_slice(),
                *span,
                context,
            ),
            Expr::Constrain {
                target_type,
                value,
                span,
                ..
            } => self.classify_constrain_fallible_expression(target_type, value, *span),
            _ => Err(Self::non_error_expression_type_error(context, expr.span())),
        }
    }

    /// Classify a fallible function call.
    fn classify_call_fallible_expression(
        &mut self,
        expr: &Expr,
        _callee: &Expr,
        _generic_args: Option<&[Type]>,
        _args: &[Expr],
        _span: Span,
        context: FallibleExpressionContext,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        let resolved = self.classify_fallible_call_shape(expr, context)?;
        if resolved.success_types.len() != 1 {
            return Err(TypeError::ArityMismatch {
                expected: 1,
                found: resolved.success_types.len(),
                span: TypeError::span_from_span(expr.span()),
            });
        }

        let success_type = resolved.success_types.first().cloned().ok_or_else(|| {
            TypeError::ConstraintSolvingFailed {
                reason: "fallible call has no declared return type".to_owned(),
                span: TypeError::span_from_span(expr.span()),
            }
        })?;

        Ok(FallibleExpressionInfo {
            success_type,
            error_types: resolved.error_types,
            expression_kind: FallibleExpressionKind::Call,
            constructor_entry: None,
        })
    }

    /// Classify a registered fallible constructor expression.
    fn classify_constructor_fallible_expression(
        &mut self,
        callee: &Expr,
        fields: &[crate::ast::ConstructorField],
        span: Span,
        context: FallibleExpressionContext,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        let resolved_constructor_type = self.resolve_constructor_target_type(callee, span)?;
        let Some(identity) = CanonicalTypeIdentity::from_core_type(&resolved_constructor_type)
        else {
            return Err(Self::non_error_expression_type_error(context, span));
        };

        let Some(entry) = lookup_fallible_constructor(identity) else {
            return match context {
                FallibleExpressionContext::Propagate => {
                    Err(TypeError::PropagateOnNonFallibleConstructor {
                        type_name: Self::constructor_display_name(callee),
                        span: TypeError::span_from_span(span),
                    })
                }
                FallibleExpressionContext::Guard => Err(TypeError::GuardOnNonErrorExpression {
                    span: TypeError::span_from_span(span),
                }),
            };
        };

        self.type_check_registered_constructor_fields(&entry, fields, span)?;

        Ok(FallibleExpressionInfo {
            success_type: entry.success_type.clone(),
            error_types: entry.error_types.clone(),
            expression_kind: FallibleExpressionKind::RegisteredConstructor,
            constructor_entry: Some(entry),
        })
    }

    /// Type-check a bare `constrain` expression and report its unhandled error surface.
    pub(super) fn type_check_unhandled_constrain_expr(
        &mut self,
        target_type: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let _success_type = self.type_check_constrain_expr(target_type, value, span)?;
        Err(TypeError::UnhandledCallError {
            name: "constrain".to_owned(),
            error_types: "ConstraintViolationError".to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Classify `constrain Type from value` as a fallible constrained construction.
    fn classify_constrain_fallible_expression(
        &mut self,
        target_type: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        let success_type = self.type_check_constrain_expr(target_type, value, span)?;
        let error_type = self.constraint_violation_error_type(span)?;
        Ok(FallibleExpressionInfo {
            success_type,
            error_types: vec![error_type],
            expression_kind: FallibleExpressionKind::Call,
            constructor_entry: None,
        })
    }

    /// Type-check the constrained target and runtime source value.
    fn type_check_constrain_expr(
        &mut self,
        target_type: &Type,
        value: &Expr,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let target_core_type = ast_type_to_core_type(target_type).map_err(TypeError::from)?;
        let source_type = self.constrained_alias_source_type(&target_core_type, span)?;
        let value_type = self.type_check_expr(value)?;
        let reconciled_value_type = if self.types_compatible(&source_type, &value_type) {
            value_type
        } else if let Some(adjusted) = coerce_literal_to_expected(&source_type, value, &value_type)
        {
            adjusted
        } else {
            return Err(type_mismatch_error(
                &source_type,
                None,
                &value_type,
                value.span(),
            ));
        };
        self.add_constraint(TypeConstraint::equality(
            source_type,
            reconciled_value_type,
            None,
            Some(value.span()),
        ));
        Ok(target_core_type)
    }

    /// Resolve the underlying source type for a constrained proposal alias.
    fn constrained_alias_source_type(
        &self,
        target_core_type: &CoreType,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let Some(type_name) = Self::constrained_type_name(target_core_type) else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!("constrain target '{target_core_type}' is not a nominal type"),
                span: TypeError::span_from_span(span),
            });
        };

        for module_path in [
            "standard.terminal",
            "standard.terminal.chords",
            "standard.testing.terminal",
        ] {
            let Some(interface) = self.module_resolver.module_interface(module_path) else {
                continue;
            };
            let Some(declaration) = interface.type_declaration(type_name) else {
                continue;
            };
            return Self::constrained_declaration_source_type(declaration, span);
        }

        Err(TypeError::ConstraintSolvingFailed {
            reason: format!("constrain target '{type_name}' is not a constrained type"),
            span: TypeError::span_from_span(span),
        })
    }

    /// Resolve a parsed constrained alias declaration to its source representation type.
    fn constrained_declaration_source_type(
        declaration: &crate::type_system::module_resolver::ModuleTypeDeclaration,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if declaration.form != TypeDeclarationForm::Constrained {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "constrain target '{}' is not a constrained type",
                    declaration.name
                ),
                span: TypeError::span_from_span(span),
            });
        }

        let TypeDef::Alias {
            target_type,
            constraint: Some(_),
            ..
        } = &declaration.type_def
        else {
            return Err(TypeError::ConstraintSolvingFailed {
                reason: format!(
                    "constrained type '{}' has no runtime-checkable where clause",
                    declaration.name
                ),
                span: TypeError::span_from_span(span),
            });
        };

        ast_type_to_core_type(target_type).map_err(TypeError::from)
    }

    /// Return the core error type produced by runtime constrained construction.
    fn constraint_violation_error_type(&self, span: Span) -> Result<CoreType, TypeError> {
        if let Ok(core_type) = self
            .environment
            .lookup_type("ConstraintViolationError", span)
        {
            return Ok(core_type.clone());
        }
        if let Some(symbol) = self.symbol_table.lookup("ConstraintViolationError") {
            if symbol.symbol_type == crate::type_system::symbol_table::SymbolType::Type {
                return Ok(symbol.core_type.clone());
            }
        }
        Err(TypeError::UndeclaredErrorType {
            name: "ConstraintViolationError".to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Extract a non-generic nominal type name from a constrain target.
    fn constrained_type_name(target_core_type: &CoreType) -> Option<&str> {
        match target_core_type {
            CoreType::Generic { name, type_args } if type_args.is_empty() => Some(name.as_str()),
            _ => None,
        }
    }

    /// Build the diagnostic used for non-fallible propagate/guard subjects.
    fn non_error_expression_type_error(
        context: FallibleExpressionContext,
        span: Span,
    ) -> TypeError {
        match context {
            FallibleExpressionContext::Propagate => TypeError::PropagateOnNonErrorExpression {
                span: TypeError::span_from_span(span),
            },
            FallibleExpressionContext::Guard => TypeError::GuardOnNonErrorExpression {
                span: TypeError::span_from_span(span),
            },
        }
    }

    /// Render a human-readable constructor name for diagnostics.
    fn constructor_display_name(callee: &Expr) -> String {
        match callee {
            Expr::Identifier { name, .. } => name.clone(),
            Expr::Member { object, member, .. } => match object.as_ref() {
                Expr::Identifier { name, .. } => format!("{name}.{member}"),
                _ => member.clone(),
            },
            _ => "<constructor>".to_owned(),
        }
    }

    /// Resolve the constructor target to a nominal core type.
    fn resolve_constructor_target_type(
        &mut self,
        callee: &Expr,
        callee_span: Span,
    ) -> Result<CoreType, TypeError> {
        match callee {
            Expr::Identifier {
                name,
                span: name_span,
                ..
            } => {
                if let Ok(core_type) = self.environment().lookup_type(name, *name_span) {
                    return Ok(core_type.clone());
                }
                if let Some(symbol) = self.symbol_table().lookup(name) {
                    return Ok(symbol.core_type.clone());
                }
                Err(TypeError::SymbolNotFound {
                    name: name.clone(),
                    suggestion: self.suggest_visible_identifier(name),
                    span: TypeError::span_from_span(*name_span),
                })
            }
            Expr::Member {
                object,
                member,
                span: member_span,
                ..
            } => {
                if let Expr::Identifier { name, .. } = object.as_ref() {
                    let qualified_variant = format!("{name}.{member}");
                    if let Some(symbol) = self.symbol_table().lookup(&qualified_variant) {
                        return Ok(symbol.core_type.clone());
                    }
                    return Err(TypeError::UnknownVariant {
                        type_name: name.clone(),
                        variant_name: member.clone(),
                        span: TypeError::span_from_span(*member_span),
                    });
                }
                let callee_type = self.type_check_expr(callee)?;
                Err(TypeError::InvalidOperation {
                    operation: "constructor target".to_owned(),
                    type_name: callee_type.to_string(),
                    span: TypeError::span_from_span(callee_span),
                })
            }
            _ => {
                let callee_type = self.type_check_expr(callee)?;
                Err(TypeError::InvalidOperation {
                    operation: "constructor target".to_owned(),
                    type_name: callee_type.to_string(),
                    span: TypeError::span_from_span(callee_span),
                })
            }
        }
    }
}
