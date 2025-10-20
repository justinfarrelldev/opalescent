//! Type unification for the Opalescent type system

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::substitution::Substitution;
use crate::type_system::types::{CoreType, TypeVar};
use alloc::vec;
use miette::SourceSpan;

impl TypeChecker {
    /// Validate that a type name is valid for the given core type
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the type to validate
    /// * `core_type` - The core type definition
    /// * `span` - Source location of the type definition (for error reporting)
    ///
    /// # Errors
    ///
    /// Returns `TypeError::TypeMismatch` if type already exists with different definition
    pub fn validate_type_name(
        &self,
        name: &str,
        core_type: &CoreType,
        span: Span,
    ) -> Result<(), TypeError> {
        if let Ok(existing_type) = self.environment.lookup_type(name, span) {
            if existing_type != core_type {
                return Err(TypeError::TypeMismatch {
                    expected: existing_type.to_string(),
                    found: core_type.to_string(),
                    found_span: TypeError::span_from_span(span),
                    expected_span: None,
                });
            }
        }
        Ok(())
    }

    /// Unify two types, returning a substitution that makes them equal
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when unification fails
    pub fn unify(
        &self,
        left: &CoreType,
        right: &CoreType,
        left_span: Option<Span>,
        right_span: Option<Span>,
    ) -> Result<Substitution, TypeError> {
        self.unify_impl(left, right, left_span, right_span)
    }

    /// Internal implementation of unification algorithm
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed CoreType variants avoids unnecessary cloning during constraint solving"
    )]
    fn unify_impl(
        &self,
        left: &CoreType,
        right: &CoreType,
        left_span: Option<Span>,
        right_span: Option<Span>,
    ) -> Result<Substitution, TypeError> {
        if self.types_compatible(left, right) {
            return Ok(Substitution::empty());
        }

        if let CoreType::Variable(variable) = left {
            return Self::unify_type_variable(variable, right, left_span, right_span);
        }

        if let CoreType::Variable(variable) = right {
            return Self::unify_type_variable(variable, left, right_span, left_span);
        }

        match (left, right) {
            (CoreType::Array(left_elem), CoreType::Array(right_elem)) => self.unify_impl(
                left_elem.as_ref(),
                right_elem.as_ref(),
                left_span,
                right_span,
            ),
            (
                CoreType::Function {
                    parameters: left_params,
                    return_type: left_ret,
                },
                CoreType::Function {
                    parameters: right_params,
                    return_type: right_ret,
                },
            ) => self.unify_function_types(
                left_params,
                left_ret.as_ref(),
                right_params,
                right_ret.as_ref(),
                left_span,
                right_span,
            ),
            (
                CoreType::Generic {
                    name: left_name,
                    type_args: left_args,
                },
                CoreType::Generic {
                    name: right_name,
                    type_args: right_args,
                },
            ) => self.unify_generic_types(
                left_name, left_args, right_name, right_args, left_span, right_span,
            ),
            _ => Err(TypeError::UnificationFailed {
                left: left.to_string(),
                right: right.to_string(),
                left_span: Self::resolve_span(left_span, right_span),
                right_span: Self::resolve_span(right_span, left_span),
            }),
        }
    }

    /// Attempt to unify a type variable with another type, respecting occurs checks.
    fn unify_type_variable(
        variable: &TypeVar,
        other: &CoreType,
        variable_span: Option<Span>,
        other_span: Option<Span>,
    ) -> Result<Substitution, TypeError> {
        if Self::occurs_check(variable.id, other) {
            Err(TypeError::OccursCheckFailed {
                var_name: variable.name.clone(),
                type_name: other.to_string(),
                span: Self::resolve_span(variable_span, other_span),
            })
        } else {
            Ok(Substitution::single(variable.id, other.clone()))
        }
    }

    /// Unify two function types by unifying parameters and return types sequentially.
    fn unify_function_types(
        &self,
        left_params: &[CoreType],
        left_ret: &CoreType,
        right_params: &[CoreType],
        right_ret: &CoreType,
        left_span: Option<Span>,
        right_span: Option<Span>,
    ) -> Result<Substitution, TypeError> {
        if left_params.len() != right_params.len() {
            return Err(TypeError::UnificationFailed {
                left: CoreType::Function {
                    parameters: left_params.to_vec(),
                    return_type: Box::new(left_ret.clone()),
                }
                .to_string(),
                right: CoreType::Function {
                    parameters: right_params.to_vec(),
                    return_type: Box::new(right_ret.clone()),
                }
                .to_string(),
                left_span: Self::resolve_span(left_span, right_span),
                right_span: Self::resolve_span(right_span, left_span),
            });
        }

        let mut combined_subst = Substitution::empty();

        for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
            let left_applied = combined_subst.apply(left_param);
            let right_applied = combined_subst.apply(right_param);
            let param_subst =
                self.unify_impl(&left_applied, &right_applied, left_span, right_span)?;
            combined_subst = combined_subst.compose(&param_subst);
        }

        let left_ret_applied = combined_subst.apply(left_ret);
        let right_ret_applied = combined_subst.apply(right_ret);
        let ret_subst =
            self.unify_impl(&left_ret_applied, &right_ret_applied, left_span, right_span)?;

        Ok(combined_subst.compose(&ret_subst))
    }

    /// Unify two generic types by verifying names and recursively unifying type arguments.
    fn unify_generic_types(
        &self,
        left_name: &str,
        left_args: &[CoreType],
        right_name: &str,
        right_args: &[CoreType],
        left_span: Option<Span>,
        right_span: Option<Span>,
    ) -> Result<Substitution, TypeError> {
        if left_name != right_name || left_args.len() != right_args.len() {
            return Err(TypeError::UnificationFailed {
                left: CoreType::Generic {
                    name: left_name.to_owned(),
                    type_args: left_args.to_vec(),
                }
                .to_string(),
                right: CoreType::Generic {
                    name: right_name.to_owned(),
                    type_args: right_args.to_vec(),
                }
                .to_string(),
                left_span: Self::resolve_span(left_span, right_span),
                right_span: Self::resolve_span(right_span, left_span),
            });
        }

        let mut combined_subst = Substitution::empty();

        for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
            let left_applied = combined_subst.apply(left_arg);
            let right_applied = combined_subst.apply(right_arg);
            let arg_subst =
                self.unify_impl(&left_applied, &right_applied, left_span, right_span)?;
            combined_subst = combined_subst.compose(&arg_subst);
        }

        Ok(combined_subst)
    }

    /// Resolve the most appropriate span for diagnostics when unification fails.
    fn resolve_span(primary: Option<Span>, fallback: Option<Span>) -> SourceSpan {
        primary
            .or(fallback)
            .map_or_else(TypeError::unknown_span, TypeError::span_from_span)
    }

    /// Check if a type variable occurs in a type (prevents infinite types)
    ///
    /// # Arguments
    ///
    /// * `var_id` - ID of the type variable to search for
    /// * `initial_type` - Type to search within
    ///
    /// # Returns
    ///
    /// `true` if the variable occurs in the type, `false` otherwise
    pub(crate) fn occurs_check(var_id: usize, initial_type: &CoreType) -> bool {
        let mut work_queue = vec![initial_type];

        while let Some(current_type) = work_queue.pop() {
            match *current_type {
                CoreType::Variable(ref var) => {
                    if var.id == var_id {
                        return true;
                    }
                }
                CoreType::Array(ref element_type) => {
                    work_queue.push(element_type.as_ref());
                }
                CoreType::Function {
                    parameters: ref params,
                    return_type: ref ret_type,
                } => {
                    work_queue.push(ret_type.as_ref());
                    work_queue.extend(params.iter());
                }
                CoreType::Generic {
                    type_args: ref args,
                    ..
                } => {
                    work_queue.extend(args.iter());
                }
                // Primitive types don't contain variables - skip them
                _ => {}
            }
        }

        false
    }
}
