//! Function-call type resolution helpers.
//!
//! This module isolates call-site generic instantiation to keep the expression
//! checker module within line-count limits and preserve focused responsibilities.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::errors::TypeError;
use crate::type_system::types::CoreType;
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
}
