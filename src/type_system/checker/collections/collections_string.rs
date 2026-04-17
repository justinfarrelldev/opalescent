//! String intrinsic registration and member resolution.

extern crate alloc;

use crate::type_system::checker::TypeChecker;
use crate::type_system::types::CoreType;
use alloc::{boxed::Box, vec, vec::Vec};

impl TypeChecker {
    /// Register all method intrinsics for `string`.
    pub(super) fn register_string_intrinsics(&mut self) {
        self.register_builtin_method(
            "string.length",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::Int64],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "string.split",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String],
                return_types: vec![CoreType::Array(Box::new(CoreType::String))],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "string.join",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Array(Box::new(CoreType::String))],
                return_types: vec![CoreType::String],
                error_types: Vec::new(),
            },
        );
        self.register_string_predicate_and_case_intrinsics();
    }

    /// Register string predicate/case/slicing intrinsics and iterable marker.
    fn register_string_predicate_and_case_intrinsics(&mut self) {
        for name in ["contains", "starts_with", "ends_with"] {
            self.register_builtin_method(
                &alloc::format!("string.{name}"),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![CoreType::String],
                    return_types: vec![CoreType::Boolean],
                    error_types: Vec::new(),
                },
            );
        }

        self.register_builtin_method(
            "string.slice",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::Int64, CoreType::Int64],
                return_types: vec![CoreType::String],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "string.to_upper",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::String],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "string.to_lower",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::String],
                error_types: Vec::new(),
            },
        );
        self.register_builtin_method(
            "string.__iter_element_type",
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: Vec::new(),
                return_types: vec![CoreType::String],
                error_types: Vec::new(),
            },
        );
    }

    /// Resolve string receiver members using registered concrete intrinsic names.
    pub(super) fn resolve_string_member_call(
        &self,
        receiver_type: &CoreType,
        member_name: &str,
    ) -> Option<CoreType> {
        if !matches!(receiver_type, &CoreType::String) {
            return None;
        }

        let intrinsic = alloc::format!("string.{member_name}");
        self.symbol_table()
            .lookup(&intrinsic)
            .map(|symbol| symbol.core_type.clone())
    }
}
