extern crate alloc;

use alloc::collections::BTreeSet;

use crate::type_system::fallible_constructors::FallibleConstructorEntry;
use crate::type_system::propertyless_constructors::lookup_propertyless_constructor;
use crate::type_system::type_mapping::ast_type_to_core_type;
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
                if fields.is_empty() {
                    return Self::type_check_propertyless_constructor(name, span);
                }

                let owner_type = self.type_check_constructor_fields(name, fields, span)?;
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

                    if fields.is_empty() {
                        return Self::type_check_propertyless_constructor(&qualified_variant, span);
                    }

                    let owner_type =
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

    /// Type check propertyless constructor expressions.
    fn type_check_propertyless_constructor(
        owner_name: &str,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        if lookup_propertyless_constructor(owner_name).is_some() {
            return Ok(CoreType::Generic {
                name: owner_name.to_owned(),
                type_args: Vec::new(),
            });
        }

        Err(TypeError::InvalidOperation {
            operation: format!("propertyless constructor is not registered for {owner_name}"),
            type_name: owner_name.to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Validate constructor field set against the declared ADT field schema.
    fn type_check_constructor_fields(
        &mut self,
        owner_name: &str,
        fields: &[ConstructorField],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let Some(expected_fields) = self.adt_fields_for_owner(owner_name).cloned() else {
            return Err(TypeError::InvalidOperation {
                operation: "constructor field initialization".to_owned(),
                type_name: owner_name.to_owned(),
                span: TypeError::span_from_span(span),
            });
        };

        let type_owner_name = owner_name
            .split_once('.')
            .map_or_else(|| owner_name.to_owned(), |(owner, _)| owner.to_owned());
        let adt_generic_params = self
            .adt_generic_params_for(type_owner_name.as_str())
            .cloned()
            .unwrap_or_default();

        let mut fresh_instantiations: alloc::collections::BTreeMap<usize, CoreType> =
            alloc::collections::BTreeMap::new();
        let inference_substitution = self.type_check_named_constructor_fields(
            owner_name,
            fields,
            expected_fields
                .iter()
                .map(|(name, core_type)| (name.as_str(), core_type)),
            span,
            &mut fresh_instantiations,
        )?;

        self.finalize_generic_constructor_type(
            type_owner_name.as_str(),
            adt_generic_params.as_slice(),
            span,
            &mut fresh_instantiations,
            &inference_substitution,
        )
    }

    /// Validate constructor fields against a named field schema shared by ordinary and registered constructors.
    pub(super) fn type_check_named_constructor_fields<'field_iter, I>(
        &mut self,
        owner_name: &str,
        fields: &[ConstructorField],
        expected_fields: I,
        span: Span,
        fresh_instantiations: &mut alloc::collections::BTreeMap<usize, CoreType>,
    ) -> Result<crate::type_system::substitution::Substitution, TypeError>
    where
        I: IntoIterator<Item = (&'field_iter str, &'field_iter CoreType)>,
    {
        let expected_fields = expected_fields
            .into_iter()
            .map(|(field_name, core_type)| (field_name.to_owned(), core_type.clone()))
            .collect::<alloc::collections::BTreeMap<_, _>>();

        self.type_check_constructor_field_schema(
            owner_name,
            fields,
            &expected_fields,
            span,
            fresh_instantiations,
        )
    }

    /// Validate fields for a registered fallible constructor entry.
    pub(super) fn type_check_registered_constructor_fields(
        &mut self,
        entry: &FallibleConstructorEntry,
        fields: &[ConstructorField],
        span: Span,
    ) -> Result<crate::type_system::substitution::Substitution, TypeError> {
        let mut fresh_instantiations: alloc::collections::BTreeMap<usize, CoreType> =
            alloc::collections::BTreeMap::new();
        self.type_check_named_constructor_fields(
            entry.canonical_result_type_name,
            fields,
            entry
                .required_fields
                .iter()
                .map(|field| (field.name, &field.core_type)),
            span,
            &mut fresh_instantiations,
        )
    }

    /// Validate constructor fields against a named field schema and preserve inference behavior.
    fn type_check_constructor_field_schema(
        &mut self,
        owner_name: &str,
        fields: &[ConstructorField],
        expected_fields: &alloc::collections::BTreeMap<String, CoreType>,
        span: Span,
        fresh_instantiations: &mut alloc::collections::BTreeMap<usize, CoreType>,
    ) -> Result<crate::type_system::substitution::Substitution, TypeError> {
        let mut inference_substitution = crate::type_system::substitution::Substitution::empty();
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

            let expected_field_instantiated =
                self.instantiate_call_type(expected_type, fresh_instantiations, field.span)?;
            let field_value_type = self.type_check_expr(&field.value)?;
            let expected_field_applied = inference_substitution.apply(&expected_field_instantiated);
            let reconciled_value = if self
                .types_compatible(&expected_field_applied, &field_value_type)
                || matches!(expected_field_applied, CoreType::Variable(_))
            {
                field_value_type
            } else if let Some(adjusted) =
                coerce_literal_to_expected(&expected_field_applied, &field.value, &field_value_type)
            {
                adjusted
            } else {
                return Err(TypeError::FieldTypeMismatch {
                    type_name: owner_name.to_owned(),
                    field_name: field.name.clone(),
                    expected: expected_field_applied.to_string(),
                    found: field_value_type.to_string(),
                    span: TypeError::span_from_span(field.value.span()),
                });
            };

            let field_substitution = self.unify(
                &expected_field_applied,
                &reconciled_value,
                Some(field.span),
                Some(field.value.span()),
            )?;
            inference_substitution = inference_substitution.compose(&field_substitution);

            self.add_constraint(TypeConstraint::equality(
                expected_field_instantiated,
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

        Ok(inference_substitution)
    }

    /// Finalize inferred generic constructor arguments and emit constraints.
    fn finalize_generic_constructor_type(
        &mut self,
        owner_name: &str,
        generic_params: &[crate::type_system::types::GenericTypeParameter],
        span: Span,
        fresh_instantiations: &mut alloc::collections::BTreeMap<usize, CoreType>,
        inference_substitution: &crate::type_system::substitution::Substitution,
    ) -> Result<CoreType, TypeError> {
        let mut inferred_type_args = Vec::new();
        for generic_param in generic_params {
            let variable_type = self.instantiate_call_type(
                &CoreType::Variable(generic_param.type_var.clone()),
                fresh_instantiations,
                span,
            )?;
            let inferred_arg = inference_substitution.apply(&variable_type);
            if let CoreType::Variable(_) = inferred_arg {
                return Err(TypeError::CannotInferGenericType {
                    param_name: generic_param.name.clone(),
                    span: TypeError::span_from_span(span),
                });
            }

            for constraint in &generic_param.constraints {
                let resolved_constraint = inference_substitution.apply(constraint);
                self.add_constraint(TypeConstraint::equality(
                    inferred_arg.clone(),
                    resolved_constraint,
                    Some(span),
                    Some(span),
                ));
            }
            inferred_type_args.push(inferred_arg);
        }

        if !inferred_type_args.is_empty() {
            self.record_generic_instantiation(owner_name, inferred_type_args.as_slice());
        }

        Ok(CoreType::Generic {
            name: owner_name.to_owned(),
            type_args: inferred_type_args,
        })
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
                        ast_type_to_core_type(&field.type_annotation).map_err(TypeError::from)?;
                    }
                }
            }
            TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    ast_type_to_core_type(&field.type_annotation).map_err(TypeError::from)?;
                }
            }
            TypeDef::Alias {
                ref target_type, ..
            } => {
                ast_type_to_core_type(target_type).map_err(TypeError::from)?;
            }
        }
        Ok(())
    }
}
