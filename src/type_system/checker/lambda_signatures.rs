extern crate alloc;

use super::TypeChecker;
use crate::{
    ast::{AstNode, Expr, TypeParameter},
    token::Span,
    type_system::{
        errors::TypeError,
        types::{CoreType, GenericTypeParameter},
    },
};
use alloc::vec::Vec;

/// A signature and its body must bind generic variables by the same rules.
pub(super) struct LambdaGenericBindings {
    /// Quantified parameters retained in the callable type.
    pub(super) parameters: Vec<GenericTypeParameter>,
    /// Source names bound while resolving parameter/result annotations.
    pub(super) bindings: Vec<(String, CoreType)>,
}

impl TypeChecker {
    /// Allocate all parameters before resolving possibly interdependent bounds.
    pub(super) fn lambda_generic_bindings(
        &mut self,
        names: Option<&[String]>,
        declarations: Option<&[TypeParameter]>,
        span: Span,
    ) -> Result<LambdaGenericBindings, TypeError> {
        let mut result = LambdaGenericBindings {
            parameters: Vec::new(),
            bindings: Vec::new(),
        };
        let names_and_spans: Vec<_> = declarations.map_or_else(
            || {
                names
                    .unwrap_or_default()
                    .iter()
                    .map(|name| (name, span))
                    .collect()
            },
            |parameters| {
                parameters
                    .iter()
                    .map(|parameter| (&parameter.name, parameter.span))
                    .collect()
            },
        );
        for (name, parameter_span) in names_and_spans {
            let CoreType::Variable(type_var) = self.fresh_type_var(name.clone(), parameter_span)?
            else {
                return Err(TypeError::ConstraintSolvingFailed {
                    reason: "failed to allocate generic type variable".to_owned(),
                    span: TypeError::span_from_span(parameter_span),
                });
            };
            result
                .bindings
                .push((name.clone(), CoreType::Variable(type_var.clone())));
            result.parameters.push(GenericTypeParameter {
                name: name.clone(),
                type_var,
                constraints: Vec::new(),
            });
        }
        if let Some(parameters) = declarations {
            for (parameter, declaration) in result.parameters.iter_mut().zip(parameters) {
                parameter.constraints = declaration
                    .constraints
                    .iter()
                    .map(|constraint| {
                        self.ast_type_to_core_type_with_generics(constraint, &result.bindings)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
            }
        }
        Ok(result)
    }

    /// Infer function core type from a lambda initializer when present.
    pub(super) fn lambda_signature_type(
        &mut self,
        initializer: &Expr,
    ) -> Result<Option<CoreType>, TypeError> {
        let Expr::Lambda {
            ref params,
            ref return_types,
            ref error_types,
            ref generic_params,
            ref generic_constraints,
            ..
        } = *initializer
        else {
            return Ok(None);
        };
        let generics = self.lambda_generic_bindings(
            generic_params.as_deref(),
            generic_constraints.as_deref(),
            initializer.span(),
        )?;
        let parameter_types = params
            .iter()
            .map(|param| {
                self.ast_type_to_core_type_with_generics(&param.param_type, &generics.bindings)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let return_core_types = return_types
            .iter()
            .map(|return_type| {
                self.ast_type_to_core_type_with_generics(return_type, &generics.bindings)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let error_core_types = self.resolve_error_types(error_types, initializer.span())?;
        Ok(Some(CoreType::Function {
            generic_params: generics.parameters,
            parameters: parameter_types,
            return_types: return_core_types,
            error_types: error_core_types,
        }))
    }
}
