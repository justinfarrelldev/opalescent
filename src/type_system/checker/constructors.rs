extern crate alloc;

use alloc::collections::BTreeSet;

use crate::{
    ast::{AstNode, ConstructorField, Expr, TypeDef},
    token::Span,
    type_system::{
        checker::TypeChecker, constraints::TypeConstraint, errors::TypeError, types::CoreType,
    },
};

use super::helpers::coerce_literal_to_expected;

impl TypeChecker {
    /// Type check constructor expressions for product and sum-variant forms.
    pub(super) fn type_check_constructor_expr(
        &mut self,
        callee: &Expr,
        fields: &[ConstructorField],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        match *callee {
            Expr::Identifier { ref name, .. } => {
                let owner_type = CoreType::Generic {
                    name: name.clone(),
                    type_args: Vec::new(),
                };
                self.type_check_constructor_fields(name, fields, span)?;
                Ok(owner_type)
            }
            Expr::Member {
                ref object,
                ref member,
                span: member_span,
                ..
            } => {
                if let Expr::Identifier {
                    ref name,
                    span: type_span,
                    ..
                } = **object
                {
                    let qualified_variant = format!("{name}.{member}");
                    if self.symbol_table().lookup(&qualified_variant).is_none() {
                        return Err(TypeError::UnknownVariant {
                            type_name: name.clone(),
                            variant_name: member.clone(),
                            span: TypeError::span_from_span(member_span),
                        });
                    }

                    let owner_type = CoreType::Generic {
                        name: name.clone(),
                        type_args: Vec::new(),
                    };
                    self.type_check_constructor_fields(&qualified_variant, fields, type_span)?;
                    Ok(owner_type)
                } else {
                    let callee_type = self.type_check_expr(callee)?;
                    Err(TypeError::InvalidOperation {
                        operation: "constructor target".to_owned(),
                        type_name: callee_type.to_string(),
                        span: TypeError::span_from_span(span),
                    })
                }
            }
            _ => {
                let callee_type = self.type_check_expr(callee)?;
                Err(TypeError::InvalidOperation {
                    operation: "constructor target".to_owned(),
                    type_name: callee_type.to_string(),
                    span: TypeError::span_from_span(span),
                })
            }
        }
    }

    /// Validate constructor field set against the declared ADT field schema.
    fn type_check_constructor_fields(
        &mut self,
        owner_name: &str,
        fields: &[ConstructorField],
        span: Span,
    ) -> Result<(), TypeError> {
        let Some(expected_fields) = self.adt_fields_for_owner(owner_name).cloned() else {
            return Err(TypeError::InvalidOperation {
                operation: "constructor field initialization".to_owned(),
                type_name: owner_name.to_owned(),
                span: TypeError::span_from_span(span),
            });
        };

        let mut seen_fields: BTreeSet<String> = BTreeSet::new();
        for field in fields {
            if seen_fields.contains(&field.name) {
                return Err(TypeError::DuplicateField {
                    field_name: field.name.clone(),
                    span: TypeError::span_from_span(field.span),
                });
            }
            seen_fields.insert(field.name.clone());

            let Some(expected_type) = expected_fields.get(&field.name) else {
                return Err(TypeError::MissingField {
                    type_name: owner_name.to_owned(),
                    field_name: field.name.clone(),
                    span: TypeError::span_from_span(field.span),
                });
            };

            let field_value_type = self.type_check_expr(&field.value)?;
            let reconciled_value = if self.types_compatible(expected_type, &field_value_type) {
                field_value_type
            } else if let Some(adjusted) =
                coerce_literal_to_expected(expected_type, &field.value, &field_value_type)
            {
                adjusted
            } else {
                return Err(TypeError::FieldTypeMismatch {
                    type_name: owner_name.to_owned(),
                    field_name: field.name.clone(),
                    expected: expected_type.to_string(),
                    found: field_value_type.to_string(),
                    span: TypeError::span_from_span(field.value.span()),
                });
            };

            self.add_constraint(TypeConstraint::equality(
                expected_type.clone(),
                reconciled_value,
                Some(field.span),
                Some(field.value.span()),
            ));
        }

        for required_name in expected_fields.keys() {
            if !seen_fields.contains(required_name) {
                return Err(TypeError::MissingField {
                    type_name: owner_name.to_owned(),
                    field_name: required_name.clone(),
                    span: TypeError::span_from_span(span),
                });
            }
        }

        Ok(())
    }

    /// Validate algebraic data type definitions against the known type environment to ensure all
    /// referenced field and variant types are resolvable.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when ADT validation fails
    pub fn validate_adt_type(type_def: &TypeDef) -> Result<(), TypeError> {
        match *type_def {
            TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        Self::ast_type_to_core_type(&field.type_annotation)?;
                    }
                }
            }
            TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    Self::ast_type_to_core_type(&field.type_annotation)?;
                }
            }
            TypeDef::Alias {
                ref target_type, ..
            } => {
                Self::ast_type_to_core_type(target_type)?;
            }
        }
        Ok(())
    }
}
