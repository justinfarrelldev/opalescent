//! Array intrinsic registration and receiver binding.

extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::{CoreType, GenericTypeParameter, TypeVar};
use alloc::{boxed::Box, vec, vec::Vec};

impl TypeChecker {
    /// Register all method intrinsics for `Array<T>`.
    pub(super) fn register_array_intrinsics(&mut self) {
        let generic_t = GenericTypeParameter {
            name: "T".to_owned(),
            type_var: TypeVar::new(1_000, "T".to_owned()),
            constraints: Vec::new(),
        };
        let element_t = CoreType::Variable(generic_t.type_var.clone());

        self.register_builtin_method(
            "[t].length",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: Vec::new(),
                return_types: vec![CoreType::Int64],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].push",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: vec![element_t.clone()],
                return_types: vec![CoreType::Unit],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].pop",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: Vec::new(),
                return_types: vec![element_t.clone()],
                error_types: Vec::new(),
            },
        );
        self.register_array_transform_intrinsics(generic_t, element_t);
    }

    /// Register higher-order array methods (`map/filter/reduce/zip`) and iterable marker.
    fn register_array_transform_intrinsics(&mut self, generic_t: GenericTypeParameter, element_t: CoreType) {
        let u_map = CoreType::Variable(TypeVar::new(1_001, "U".to_owned()));
        let u_reduce = CoreType::Variable(TypeVar::new(1_002, "U".to_owned()));
        let u_zip = CoreType::Variable(TypeVar::new(1_003, "U".to_owned()));

        self.register_builtin_method(
            "[t].map",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: vec![CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![element_t.clone()],
                    return_types: vec![u_map.clone()],
                    error_types: Vec::new(),
                }],
                return_types: vec![CoreType::Array(Box::new(u_map))],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].filter",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: vec![CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![element_t.clone()],
                    return_types: vec![CoreType::Boolean],
                    error_types: Vec::new(),
                }],
                return_types: vec![CoreType::Array(Box::new(element_t.clone()))],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].reduce",
            CoreType::Function {
                generic_params: vec![generic_t.clone()],
                parameters: vec![
                    u_reduce.clone(),
                    CoreType::Function {
                        generic_params: Vec::new(),
                        parameters: vec![u_reduce.clone(), element_t.clone()],
                        return_types: vec![u_reduce.clone()],
                        error_types: Vec::new(),
                    },
                ],
                return_types: vec![u_reduce],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].zip",
            CoreType::Function {
                generic_params: vec![generic_t],
                parameters: vec![CoreType::Array(Box::new(u_zip.clone()))],
                return_types: vec![CoreType::Array(Box::new(CoreType::Generic {
                    name: "Pair".to_owned(),
                    type_args: vec![element_t.clone(), u_zip],
                }))],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "[t].__iter_element_type",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![element_t],
                error_types: Vec::new(),
            },
        );
    }

    /// Resolve array receiver members by binding intrinsic generic `T` to the receiver element type.
    pub(super) fn resolve_array_member_call(
        &self,
        receiver_type: &CoreType,
        member_name: &str,
    ) -> Option<CoreType> {
        let CoreType::Array(ref element_type) = *receiver_type else {
            return None;
        };

        let intrinsic = alloc::format!("[t].{member_name}");
        let symbol = self.symbol_table().lookup(&intrinsic)?;
        Some(Self::bind_array_signature(
            &symbol.core_type,
            element_type.as_ref(),
        ))
    }

    /// Replace synthetic array generic variables with the concrete receiver element type.
    fn bind_array_signature(signature: &CoreType, element_type: &CoreType) -> CoreType {
        match *signature {
            CoreType::Variable(ref type_var) => {
                if type_var.id == 1_000 {
                    return element_type.clone();
                }
                signature.clone()
            }
            CoreType::Array(ref inner) => {
                CoreType::Array(Box::new(Self::bind_array_signature(inner, element_type)))
            }
            CoreType::Function {
                ref parameters,
                ref return_types,
                ref error_types,
                ..
            } => CoreType::Function {
                generic_params: Vec::new(),
                parameters: parameters
                    .iter()
                    .map(|param| Self::bind_array_signature(param, element_type))
                    .collect(),
                return_types: return_types
                    .iter()
                    .map(|return_type| Self::bind_array_signature(return_type, element_type))
                    .collect(),
                error_types: error_types
                    .iter()
                    .map(|error_type| Self::bind_array_signature(error_type, element_type))
                    .collect(),
            },
            CoreType::Generic {
                ref name,
                ref type_args,
            } => CoreType::Generic {
                name: name.clone(),
                type_args: type_args
                    .iter()
                    .map(|type_arg| Self::bind_array_signature(type_arg, element_type))
                    .collect(),
            },
            _ => signature.clone(),
        }
    }
}
