extern crate alloc;

use super::TypeChecker;
use crate::{
    ast::{AstNode, Expr},
    type_system::{errors::TypeError, types::CoreType},
};
use alloc::vec::Vec;

impl TypeChecker {
    /// Infer function core type from a lambda initializer when present.
    pub(super) fn lambda_signature_type(
        &self,
        initializer: &Expr,
    ) -> Result<Option<CoreType>, TypeError> {
        let Expr::Lambda {
            ref params,
            ref return_types,
            ref error_types,
            ..
        } = *initializer
        else {
            return Ok(None);
        };

        let parameter_types = params
            .iter()
            .map(|param| self.ast_type_to_core_type_with_generics(&param.param_type, &[]))
            .collect::<Result<Vec<_>, _>>()?;

        let return_core_types = return_types
            .iter()
            .map(|return_type| self.ast_type_to_core_type_with_generics(return_type, &[]))
            .collect::<Result<Vec<_>, _>>()?;

        let error_core_types = self.resolve_error_types(error_types, initializer.span())?;

        Ok(Some(CoreType::Function {
            generic_params: Vec::new(),
            parameters: parameter_types,
            return_types: return_core_types,
            error_types: error_core_types,
        }))
    }
}
