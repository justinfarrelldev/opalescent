//! Collection intrinsic registration and iterable helpers.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;

mod collections_array;
mod collections_string;

impl TypeChecker {
    /// Register array/string collection intrinsics as builtin method symbols.
    pub(super) fn register_collection_intrinsics(&mut self) {
        self.register_array_intrinsics();
        self.register_string_intrinsics();
    }

    /// Resolve the element type exposed by the iterable protocol for a core type.
    pub(super) fn iterable_element_type_for(&self, core_type: &CoreType) -> Option<CoreType> {
        if let CoreType::Array(ref element_type) = *core_type {
            return Some(element_type.as_ref().clone());
        }

        let marker_name = alloc::format!("{core_type}.__iter_element_type");
        let marker = self.symbol_table().lookup(&marker_name)?;
        if let CoreType::Function {
            ref return_types, ..
        } = marker.core_type
        {
            return return_types.first().cloned();
        }

        None
    }

    /// Resolve a collection member to a concrete receiver-specialized function type.
    pub(super) fn resolve_collection_member_call(
        &self,
        receiver_type: &CoreType,
        member_name: &str,
    ) -> Option<CoreType> {
        self.resolve_array_member_call(receiver_type, member_name)
            .or_else(|| self.resolve_string_member_call(receiver_type, member_name))
    }

    /// Register one intrinsic method symbol in environment and symbol table.
    pub(super) fn register_builtin_method(&mut self, name: &str, signature: CoreType) {
        let symbol_name = name.to_owned();
        self.environment
            .register_builtin(symbol_name.clone(), signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: symbol_name,
            symbol_type: SymbolType::Function,
            core_type: signature,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
            is_pure: false,
        });
    }
}
