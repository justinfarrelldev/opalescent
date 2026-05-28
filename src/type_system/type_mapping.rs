extern crate alloc;

use crate::ast::Type;
use crate::token::Span;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstTypeMappingError {
    TypeNotFound { type_name: String, span: Span },
}

pub fn basic_type_name_to_core_type(name: &str) -> Option<CoreType> {
    match name {
        "int8" => Some(CoreType::Int8),
        "int16" => Some(CoreType::Int16),
        "int32" => Some(CoreType::Int32),
        "int64" => Some(CoreType::Int64),
        "uint8" => Some(CoreType::UInt8),
        "uint16" => Some(CoreType::UInt16),
        "uint32" => Some(CoreType::UInt32),
        "uint64" => Some(CoreType::UInt64),
        "float32" => Some(CoreType::Float32),
        "float64" => Some(CoreType::Float64),
        "string" => Some(CoreType::String),
        "boolean" => Some(CoreType::Boolean),
        "unit" | "void" => Some(CoreType::Unit),
        _ => None,
    }
}

pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, AstTypeMappingError> {
    match *ast_type {
        Type::Basic {
            ref name,
            span: _span,
        } => basic_type_name_to_core_type(name.as_str()).map_or_else(
            || {
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: Vec::new(),
                })
            },
            Ok,
        ),
        Type::Array {
            ref element_type, ..
        } => {
            let core_element = ast_type_to_core_type(element_type.as_ref())?;
            Ok(CoreType::Array(alloc::boxed::Box::new(core_element)))
        }
        Type::Function {
            ref parameters,
            ref return_types,
            ref errors,
            ..
        } => {
            let mut core_params = Vec::with_capacity(parameters.len());
            for param in parameters {
                core_params.push(ast_type_to_core_type(param)?);
            }
            let mut core_return_types = Vec::with_capacity(return_types.len());
            for return_type in return_types {
                core_return_types.push(ast_type_to_core_type(return_type)?);
            }

            let mut core_errors: Vec<CoreType> = Vec::new();
            if let Some(list) = errors.as_ref() {
                for err_ty in list {
                    match ast_type_to_core_type(err_ty) {
                        Ok(core) => core_errors.push(core),
                        Err(AstTypeMappingError::TypeNotFound { type_name, .. }) => {
                            core_errors.push(CoreType::Generic {
                                name: type_name,
                                type_args: Vec::new(),
                            });
                        }
                    }
                }
            }

            Ok(CoreType::Function {
                generic_params: Vec::new(),
                parameters: core_params,
                return_types: core_return_types,
                error_types: core_errors,
            })
        }
        Type::Generic {
            ref name,
            ref type_args,
            ..
        } => {
            let mut core_args = Vec::with_capacity(type_args.len());
            for arg in type_args {
                core_args.push(ast_type_to_core_type(arg)?);
            }
            Ok(CoreType::Generic {
                name: name.clone(),
                type_args: core_args,
            })
        }
    }
}
