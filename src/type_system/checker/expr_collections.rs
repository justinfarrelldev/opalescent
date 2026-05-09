#![allow(clippy::missing_docs_in_private_items, clippy::pattern_type_mismatch, reason = "private checker helpers and pattern matches are internal implementation details")]
//! Collection and interpolation expression helpers.

extern crate alloc;

use super::helpers::{
    coerce_literal_to_expected, invalid_operation_error, is_boolean_type, is_numeric_type,
    is_string_type, type_mismatch_error,
};
use crate::ast::{AstNode, Expr, StringPart};
use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;

impl TypeChecker {
    /// Validate each interpolated expression, ensuring only display-safe primitives appear
    /// inside a string literal interpolation sequence.
    pub(super) fn type_check_string_interpolation(
        &mut self,
        parts: &[StringPart],
        _span: Span,
    ) -> Result<(), TypeError> {
        for part in parts {
            if let StringPart::Expression(ref expr) = *part {
                let expr_type = self.type_check_expr(expr)?;
                if !(is_numeric_type(&expr_type)
                    || is_boolean_type(&expr_type)
                    || is_string_type(&expr_type)
                    || Self::is_displayable_error_type(&expr_type)
                    || Self::is_guard_error_interpolation_type(&expr_type))
                {
                    return Err(invalid_operation_error(
                        "string interpolation",
                        &expr_type,
                        expr.span(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn is_guard_error_interpolation_type(expr_type: &CoreType) -> bool {
        match expr_type {
            CoreType::Generic { name, type_args } => {
                name == "GuardErrorContext" && type_args.iter().all(Self::is_displayable_error_type)
            }
            _ => false,
        }
    }

    fn is_displayable_error_type(expr_type: &CoreType) -> bool {
        match expr_type {
            CoreType::Generic { name, type_args } => {
                type_args.is_empty() && Self::looks_like_error_type_name(name)
            }
            _ => false,
        }
    }

    fn looks_like_error_type_name(name: &str) -> bool {
        name.ends_with("Error") && name.chars().next().is_some_and(char::is_uppercase)
    }

    /// Type check an array literal, deriving a unified element type and generating equality
    /// constraints between each element and the inferred element type.
    pub(super) fn type_check_array_expr(
        &mut self,
        elements: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let mut element_type: Option<(CoreType, Span)> = None;
        for element in elements {
            let element_span = element.span();
            let element_core_type = self.type_check_expr(element)?;
            if let Some(existing) = element_type.as_ref() {
                let existing_type = &existing.0;
                let existing_span = existing.1;
                let reconciled_type = if self.types_compatible(existing_type, &element_core_type) {
                    element_core_type
                } else if let Some(adjusted) =
                    coerce_literal_to_expected(existing_type, element, &element_core_type)
                {
                    adjusted
                } else if let Some(adjusted_existing) =
                    coerce_literal_to_expected(&element_core_type, &elements[0], existing_type)
                {
                    element_type = Some((adjusted_existing.clone(), existing_span));
                    adjusted_existing
                } else {
                    return Err(type_mismatch_error(
                        existing_type,
                        Some(existing_span),
                        &element_core_type,
                        element_span,
                    ));
                };
                let current_element_type = element_type
                    .as_ref()
                    .map_or_else(|| reconciled_type.clone(), |pair| pair.0.clone());
                self.add_constraint(TypeConstraint::equality(
                    current_element_type,
                    reconciled_type,
                    Some(existing_span),
                    Some(element_span),
                ));
            } else {
                element_type = Some((element_core_type, element_span));
            }
        }

        let resolved = match element_type {
            Some((core_type, _)) => core_type,
            None => self.fresh_type_var_auto(span)?,
        };

        Ok(CoreType::Array(alloc::boxed::Box::new(resolved)))
    }
}
