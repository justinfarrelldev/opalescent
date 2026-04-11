//! Function-call type resolution helpers.
//!
//! This module isolates call-site generic instantiation to keep the expression
//! checker module within line-count limits and preserve focused responsibilities.

extern crate alloc;

use crate::ast::{AstNode, Expr, Type};
use crate::token::Span;
use crate::type_system::checker::helpers::{coerce_literal_to_expected, type_mismatch_error};
use crate::type_system::checker::TypeChecker;
use crate::type_system::constraints::TypeConstraint;
use crate::type_system::errors::TypeError;
use crate::type_system::substitution::Substitution;
use crate::type_system::types::{CoreType, GenericTypeParameter};
use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

impl TypeChecker {
    /// Instantiate polymorphic call-site types so each function call receives fresh type variables.
    pub(super) fn instantiate_call_type(
        &mut self,
        core_type: &CoreType,
        instantiations: &mut BTreeMap<usize, CoreType>,
        span: Span,
    ) -> Result<CoreType, TypeError> {
        match *core_type {
            CoreType::Variable(ref type_var) => {
                if let Some(instantiated) = instantiations.get(&type_var.id) {
                    Ok(instantiated.clone())
                } else {
                    let fresh = self.fresh_type_var_auto(span)?;
                    instantiations.insert(type_var.id, fresh.clone());
                    Ok(fresh)
                }
            }
            CoreType::Array(ref element_type) => Ok(CoreType::Array(Box::new(
                self.instantiate_call_type(element_type, instantiations, span)?,
            ))),
            CoreType::Function {
                ref parameters,
                ref return_types,
                ref error_types,
                ..
            } => {
                let instantiated_parameters = parameters
                    .iter()
                    .map(|param| self.instantiate_call_type(param, instantiations, span))
                    .collect::<Result<Vec<_>, _>>()?;
                let instantiated_returns = return_types
                    .iter()
                    .map(|ret| self.instantiate_call_type(ret, instantiations, span))
                    .collect::<Result<Vec<_>, _>>()?;
                let instantiated_errors = error_types
                    .iter()
                    .map(|err| self.instantiate_call_type(err, instantiations, span))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: instantiated_parameters,
                    return_types: instantiated_returns,
                    error_types: instantiated_errors,
                })
            }
            CoreType::Generic {
                ref name,
                ref type_args,
            } => {
                let instantiated_args = type_args
                    .iter()
                    .map(|arg| self.instantiate_call_type(arg, instantiations, span))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: instantiated_args,
                })
            }
            _ => Ok(core_type.clone()),
        }
    }

    /// Type check a function call, including optional explicit generic arguments.
    #[expect(
        clippy::too_many_lines,
        reason = "Call typing centralizes arity, generic constraints, and argument reconciliation"
    )]
    pub(super) fn type_check_call_expr_impl(
        &mut self,
        callee: &Expr,
        generic_args: Option<&[Type]>,
        args: &[Expr],
        span: Span,
    ) -> Result<CoreType, TypeError> {
        let callee_type = self.type_check_expr(callee)?;
        match callee_type {
            CoreType::Function {
                generic_params,
                parameters,
                return_types,
                error_types: _error_types,
            } => {
                let mut type_var_instantiations: BTreeMap<usize, CoreType> = BTreeMap::new();
                let mut local_inference = Substitution::empty();
                let instantiated_parameters = parameters
                    .iter()
                    .map(|parameter_type| {
                        self.instantiate_call_type(
                            parameter_type,
                            &mut type_var_instantiations,
                            span,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let instantiated_return_types = return_types
                    .iter()
                    .map(|return_type| {
                        self.instantiate_call_type(return_type, &mut type_var_instantiations, span)
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                let instantiated_generic_variables = Self::instantiate_generic_variables(
                    generic_params.as_slice(),
                    &type_var_instantiations,
                );

                if generic_args.is_none() {
                    for declared_generic in &generic_params {
                        if !type_var_instantiations.contains_key(&declared_generic.type_var.id) {
                            return Err(TypeError::CannotInferGenericType {
                                param_name: declared_generic.name.clone(),
                                span: TypeError::span_from_span(span),
                            });
                        }
                    }
                }

                if let Some(explicit_generic_args) = generic_args {
                    if explicit_generic_args.len() != generic_params.len() {
                        return Err(TypeError::ArityMismatch {
                            expected: generic_params.len(),
                            found: explicit_generic_args.len(),
                            span: TypeError::span_from_span(span),
                        });
                    }

                    self.add_explicit_generic_constraints(
                        generic_params.as_slice(),
                        instantiated_generic_variables.as_slice(),
                        explicit_generic_args,
                    )?;
                }

                self.add_declared_generic_constraints(
                    generic_params.as_slice(),
                    instantiated_generic_variables.as_slice(),
                    span,
                );

                if instantiated_parameters.len() != args.len() {
                    return Err(TypeError::InvalidOperation {
                        operation: alloc::format!(
                            "function call expected {} arguments but received {}",
                            instantiated_parameters.len(),
                            args.len()
                        ),
                        type_name: "function".to_owned(),
                        span: TypeError::span_from_span(span),
                    });
                }

                for (index, arg_expr) in args.iter().enumerate() {
                    let param_type = instantiated_parameters[index].clone();
                    let arg_type = self.type_check_expr(arg_expr)?;
                    let constrained_target = Self::resolve_constrained_target(
                        &param_type,
                        generic_params.as_slice(),
                        instantiated_generic_variables.as_slice(),
                    );

                    let reconciled_type = if let Some(target_type) = constrained_target {
                        if self.types_compatible(&target_type, &arg_type) {
                            arg_type
                        } else if let Some(adjusted) =
                            coerce_literal_to_expected(&target_type, arg_expr, &arg_type)
                        {
                            adjusted
                        } else {
                            return Err(type_mismatch_error(
                                &target_type,
                                None,
                                &arg_type,
                                arg_expr.span(),
                            ));
                        }
                    } else if matches!(param_type, CoreType::Variable(_))
                        || self.types_compatible(&param_type, &arg_type)
                        || Self::core_type_contains_variable(&param_type)
                        || Self::core_type_contains_variable(&arg_type)
                    {
                        arg_type
                    } else if let Some(adjusted) =
                        coerce_literal_to_expected(&param_type, arg_expr, &arg_type)
                    {
                        adjusted
                    } else {
                        return Err(type_mismatch_error(
                            &param_type,
                            None,
                            &arg_type,
                            arg_expr.span(),
                        ));
                    };

                    let parameter_applied = local_inference.apply(&param_type);
                    let argument_applied = local_inference.apply(&reconciled_type);
                    let argument_substitution = self.unify(
                        &parameter_applied,
                        &argument_applied,
                        None,
                        Some(arg_expr.span()),
                    )?;
                    local_inference = local_inference.compose(&argument_substitution);

                    self.add_constraint(TypeConstraint::equality(
                        param_type,
                        reconciled_type,
                        None,
                        Some(arg_expr.span()),
                    ));
                }

                let raw_return_type =
                    instantiated_return_types.first().cloned().ok_or_else(|| {
                        TypeError::ConstraintSolvingFailed {
                            reason: "function call has no declared return type".to_owned(),
                            span: TypeError::span_from_span(span),
                        }
                    })?;

                let inferred_return_type = local_inference.apply(&raw_return_type);

                if let CoreType::Variable(ref return_var) = inferred_return_type {
                    Ok(Self::resolve_return_constraint_type(
                        return_var.id,
                        generic_params.as_slice(),
                        instantiated_generic_variables.as_slice(),
                    )
                    .unwrap_or(inferred_return_type))
                } else {
                    Ok(inferred_return_type)
                }
            }
            other => Err(
                crate::type_system::checker::helpers::invalid_operation_error(
                    "function call",
                    &other,
                    span,
                ),
            ),
        }
    }

    /// Resolve concrete instantiated types for each declared generic parameter.
    fn instantiate_generic_variables(
        generic_params: &[GenericTypeParameter],
        type_var_instantiations: &BTreeMap<usize, CoreType>,
    ) -> Vec<(usize, CoreType)> {
        let mut instantiated = Vec::new();
        for declared_generic in generic_params {
            let instantiated_type = type_var_instantiations
                .get(&declared_generic.type_var.id)
                .cloned()
                .unwrap_or_else(|| CoreType::Variable(declared_generic.type_var.clone()));
            instantiated.push((declared_generic.type_var.id, instantiated_type));
        }
        instantiated
    }

    /// Retrieve the instantiated type for a declared generic, falling back when unresolved.
    fn instantiated_type_for_generic(
        generic_id: usize,
        instantiated_generic_variables: &[(usize, CoreType)],
        fallback: CoreType,
    ) -> CoreType {
        for entry in instantiated_generic_variables {
            if entry.0 == generic_id {
                return entry.1.clone();
            }
        }
        fallback
    }

    /// Add constraints introduced by explicit generic call arguments.
    fn add_explicit_generic_constraints(
        &mut self,
        generic_params: &[GenericTypeParameter],
        instantiated_generic_variables: &[(usize, CoreType)],
        explicit_generic_args: &[Type],
    ) -> Result<(), TypeError> {
        for (declared_generic, explicit_ast_type) in
            generic_params.iter().zip(explicit_generic_args.iter())
        {
            let explicit_core_type = Self::ast_type_to_core_type(explicit_ast_type)?;
            let target_generic_type = Self::instantiated_type_for_generic(
                declared_generic.type_var.id,
                instantiated_generic_variables,
                CoreType::Variable(declared_generic.type_var.clone()),
            );
            self.add_constraint(TypeConstraint::equality(
                target_generic_type,
                explicit_core_type,
                None,
                Some(explicit_ast_type.span()),
            ));
        }
        Ok(())
    }

    /// Add constraints declared on generic parameters at definition sites.
    fn add_declared_generic_constraints(
        &mut self,
        generic_params: &[GenericTypeParameter],
        instantiated_generic_variables: &[(usize, CoreType)],
        span: Span,
    ) {
        for declared_generic in generic_params {
            for required_constraint in &declared_generic.constraints {
                let target_generic_type = Self::instantiated_type_for_generic(
                    declared_generic.type_var.id,
                    instantiated_generic_variables,
                    CoreType::Variable(declared_generic.type_var.clone()),
                );
                self.add_constraint(TypeConstraint::equality(
                    target_generic_type,
                    required_constraint.clone(),
                    None,
                    Some(span),
                ));
            }
        }
    }

    /// Resolve the first declared constraint target for a variable parameter.
    fn resolve_constrained_target(
        param_type: &CoreType,
        generic_params: &[GenericTypeParameter],
        instantiated_generic_variables: &[(usize, CoreType)],
    ) -> Option<CoreType> {
        let CoreType::Variable(ref param_var) = *param_type else {
            return None;
        };

        for declared_generic in generic_params {
            let maybe_instantiated = Self::instantiated_type_for_generic(
                declared_generic.type_var.id,
                instantiated_generic_variables,
                CoreType::Variable(declared_generic.type_var.clone()),
            );
            if let CoreType::Variable(ref instantiated_var) = maybe_instantiated {
                if instantiated_var.id == param_var.id {
                    return declared_generic.constraints.first().cloned();
                }
            }
        }

        None
    }

    /// Resolve a call-site return variable to the first declared generic constraint.
    fn resolve_return_constraint_type(
        return_var_id: usize,
        generic_params: &[GenericTypeParameter],
        instantiated_generic_variables: &[(usize, CoreType)],
    ) -> Option<CoreType> {
        for declared_generic in generic_params {
            let maybe_instantiated = Self::instantiated_type_for_generic(
                declared_generic.type_var.id,
                instantiated_generic_variables,
                CoreType::Variable(declared_generic.type_var.clone()),
            );
            if let CoreType::Variable(ref instantiated_var) = maybe_instantiated {
                if instantiated_var.id == return_var_id {
                    return declared_generic.constraints.first().cloned();
                }
            }
        }

        None
    }

    /// Determine whether a core type includes one or more type variables.
    fn core_type_contains_variable(core_type: &CoreType) -> bool {
        match *core_type {
            CoreType::Variable(_) => true,
            CoreType::Array(ref element_type) => Self::core_type_contains_variable(element_type),
            CoreType::Function {
                ref parameters,
                ref return_types,
                ref error_types,
                ..
            } => {
                parameters.iter().any(Self::core_type_contains_variable)
                    || return_types.iter().any(Self::core_type_contains_variable)
                    || error_types.iter().any(Self::core_type_contains_variable)
            }
            CoreType::Generic { ref type_args, .. } => {
                type_args.iter().any(Self::core_type_contains_variable)
            }
            _ => false,
        }
    }
}
