#![allow(
    clippy::missing_docs_in_private_items,
    clippy::pattern_type_mismatch,
    clippy::shadow_unrelated,
    reason = "fallible expression helpers intentionally match on borrowed AST nodes"
)]
//! Fallible-expression classification helpers extracted from `expressions.rs`.

extern crate alloc;

use super::{
    FallibleExpressionContext, FallibleExpressionInfo, FallibleExpressionKind, TypeChecker,
};
use crate::ast::{AstNode, Expr, Type};
use crate::token::Span;
use crate::type_system::errors::TypeError;
use crate::type_system::fallible_constructors::{
    CanonicalTypeIdentity, lookup_fallible_constructor,
};
use crate::type_system::types::CoreType;
use alloc::{format, string::String};

impl TypeChecker {
    /// Type-check a `propagate` expression.
    ///
    /// This function ensures that:
    /// 1. The `propagate` expression is used inside a function that declares error types.
    /// 2. The inner expression is a fallible call or registered fallible constructor.
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
        let current_fn_error_types = match self.symbol_table().current_function_error_types() {
            Some(&[]) | None => {
                return Err(TypeError::PropagateOutsideErrorFunction {
                    span: TypeError::span_from_span(span),
                });
            }
            Some(errors) => errors.to_vec(),
        };

        let fallible_info =
            self.classify_fallible_expression(call, FallibleExpressionContext::Propagate)?;

        if let Some(active_errors) = self.context.guard_error_stack.last() {
            if !Self::guard_error_type_sets_match(
                active_errors.as_slice(),
                &fallible_info.error_types,
            ) {
                return Err(TypeError::GuardChainedErrorMismatch {
                    expected: Self::format_error_type_list(active_errors.as_slice()),
                    found: Self::format_error_type_list(&fallible_info.error_types),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        let is_subset = fallible_info
            .error_types
            .iter()
            .all(|error_type| current_fn_error_types.contains(error_type));

        if !is_subset {
            return Err(TypeError::PropagateErrorMismatch {
                expected: Self::format_error_type_list(&current_fn_error_types),
                found: Self::format_error_type_list(&fallible_info.error_types),
                span: TypeError::span_from_span(
                    self.symbol_table.current_function_span().unwrap_or(span),
                ),
                callee_span: TypeError::span_from_span(call.span()),
            });
        }

        Ok(fallible_info.success_type)
    }

    /// Classify a fallible expression used by `propagate` and `guard`.
    pub(super) fn classify_fallible_expression(
        &mut self,
        expr: &Expr,
        context: FallibleExpressionContext,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        match expr {
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
            _ => Err(Self::non_error_expression_type_error(context, expr.span())),
        }
    }

    /// Classify a fallible function call.
    fn classify_call_fallible_expression(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        generic_args: Option<&[Type]>,
        args: &[Expr],
        span: Span,
        context: FallibleExpressionContext,
    ) -> Result<FallibleExpressionInfo, TypeError> {
        let callee_type = self.type_check_expr(callee)?;
        let CoreType::Function {
            return_types,
            error_types,
            ..
        } = callee_type
        else {
            return Err(Self::non_error_expression_type_error(context, expr.span()));
        };

        if error_types.is_empty() {
            return Err(Self::non_error_expression_type_error(context, expr.span()));
        }

        if return_types.len() != 1 {
            return Err(TypeError::ArityMismatch {
                expected: 1,
                found: return_types.len(),
                span: TypeError::span_from_span(expr.span()),
            });
        }

        let previous_propagate_context = self.context.in_propagate_context;
        let previous_guard_subject_context = self.context.in_guard_subject_context;
        match context {
            FallibleExpressionContext::Propagate => self.context.in_propagate_context = true,
            FallibleExpressionContext::Guard => self.context.in_guard_subject_context = true,
        }
        let call_result =
            self.type_check_call_expr(callee, generic_args, args, span, expr.node_id().0);
        self.context.in_propagate_context = previous_propagate_context;
        self.context.in_guard_subject_context = previous_guard_subject_context;
        call_result?;

        let success_type =
            return_types
                .first()
                .cloned()
                .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                    reason: "fallible call has no declared return type".to_owned(),
                    span: TypeError::span_from_span(expr.span()),
                })?;

        Ok(FallibleExpressionInfo {
            success_type,
            error_types,
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
