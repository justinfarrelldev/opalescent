//! String stdlib built-in registration.
//!
//! Registers additive string helpers that are exposed as free functions in the
//! `standard` module, separate from the existing string method intrinsics.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::{vec, vec::Vec};

/// Opaque nominal type name for the shared `StringBuilder` handle.
const STRING_BUILDER_TYPE_NAME: &str = "StringBuilder";
/// Interned error name for using a finished string builder.
const BUILDER_FINISHED_ERROR: &str = "BuilderFinishedError";
/// Interned error name for string-builder allocation failures.
const ALLOCATION_FAILURE_ERROR: &str = "AllocationFailureError";

impl TypeChecker {
    /// Register additive string helpers and the nominal `StringBuilder` type.
    pub(super) fn register_string_builtins(&mut self) {
        self.register_string_nominal_types();

        self.register_string_builtin(
            "string_join",
            vec![
                CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
                CoreType::String,
            ],
            vec![CoreType::String],
            Vec::new(),
        );

        self.register_string_builtin(
            "string_builder_new",
            Vec::new(),
            vec![string_builder_core_type()],
            Vec::new(),
        );

        self.register_string_builtin(
            "string_builder_push",
            vec![string_builder_core_type(), CoreType::String],
            vec![CoreType::Unit],
            vec![
                error_core_type(BUILDER_FINISHED_ERROR),
                error_core_type(ALLOCATION_FAILURE_ERROR),
            ],
        );

        self.register_string_builtin(
            "string_builder_finish",
            vec![string_builder_core_type()],
            vec![CoreType::String],
            vec![
                error_core_type(BUILDER_FINISHED_ERROR),
                error_core_type(ALLOCATION_FAILURE_ERROR),
            ],
        );
    }

    /// Register the opaque `StringBuilder` and related error nominal types.
    fn register_string_nominal_types(&mut self) {
        self.environment.register_type(
            STRING_BUILDER_TYPE_NAME.to_owned(),
            string_builder_core_type(),
        );
        self.environment.register_type(
            BUILDER_FINISHED_ERROR.to_owned(),
            error_core_type(BUILDER_FINISHED_ERROR),
        );
        self.environment.register_type(
            ALLOCATION_FAILURE_ERROR.to_owned(),
            error_core_type(ALLOCATION_FAILURE_ERROR),
        );
    }

    /// Register a single string builtin in the environment and symbol table.
    fn register_string_builtin(
        &mut self,
        name: &str,
        parameters: Vec<CoreType>,
        return_types: Vec<CoreType>,
        error_types: Vec<CoreType>,
    ) {
        let owned_name: String = name.to_owned();
        let signature = CoreType::Function {
            generic_params: Vec::new(),
            parameters,
            return_types,
            error_types,
        };
        self.environment
            .register_builtin(owned_name.clone(), signature.clone());
        self.symbol_table.register(SymbolInfo {
            name: owned_name,
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

/// Build the nominal core type for the opaque `StringBuilder` handle.
fn string_builder_core_type() -> CoreType {
    CoreType::Generic {
        name: STRING_BUILDER_TYPE_NAME.to_owned(),
        type_args: Vec::new(),
    }
}

/// Build the nominal core type for a payload-free string builtin error.
fn error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
