//! Type unification for the Opalescent type system

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::substitution::Substitution;
use crate::type_system::types::CoreType;
use alloc::vec;

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
    pub fn unify(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        self.unify_impl(left, right)
    }

    /// Internal implementation of unification algorithm
    fn unify_impl(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        match (left, right) {
            // Same primitive types unify with empty substitution
            (l, r) if self.types_compatible(l, r) => Ok(Substitution::empty()),

            // Variable unifies with any type (with occurs check)
            (&CoreType::Variable(ref var), other) | (other, &CoreType::Variable(ref var)) => {
                if Self::occurs_check(var.id, other) {
                    Err(TypeError::OccursCheckFailed {
                        var_name: var.name.clone(),
                        type_name: other.to_string(),
                        span: TypeError::unknown_span(),
                    })
                } else {
                    Ok(Substitution::single(var.id, other.clone()))
                }
            }

            // Arrays unify if their element types unify
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.unify_impl(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions unify if parameters and return types unify
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all parameters
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    let left_applied = combined_subst.apply(left_param);
                    let right_applied = combined_subst.apply(right_param);
                    let param_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&param_subst);
                }

                // Unify return types
                let left_ret_applied = combined_subst.apply(left_ret.as_ref());
                let right_ret_applied = combined_subst.apply(right_ret.as_ref());
                let ret_subst = self.unify_impl(&left_ret_applied, &right_ret_applied)?;
                combined_subst = combined_subst.compose(&ret_subst);

                Ok(combined_subst)
            }

            // Generic types unify if names match and type arguments unify
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                        left_span: TypeError::unknown_span(),
                        right_span: TypeError::unknown_span(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all type arguments
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    let left_applied = combined_subst.apply(left_arg);
                    let right_applied = combined_subst.apply(right_arg);
                    let arg_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&arg_subst);
                }

                Ok(combined_subst)
            }

            // Different types cannot be unified
            _ => Err(TypeError::UnificationFailed {
                left: left.to_string(),
                right: right.to_string(),
                left_span: TypeError::unknown_span(),
                right_span: TypeError::unknown_span(),
            }),
        }
    }

    /// Check if a type variable occurs in a type (prevents infinite types)
    /// Uses iterative approach to avoid stack overflow with deeply nested types
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
