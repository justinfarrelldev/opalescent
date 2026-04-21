//! Bytes stdlib built-in registration.
//!
//! Registers the language-level `Bytes` nominal type together with the
//! fixed set of `bytes_*` built-in functions that lift the Rust-side
//! [`crate::stdlib::bytes::Bytes`] API into Opalescent source code.
//!
//! The builtins are grouped here (rather than expanding `register_standard_builtins`
//! in [`super`]) so that the parent module stays under the project's 500-line
//! guideline while still exposing a single authoritative registration site.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::borrow::ToOwned;
use alloc::string::String;
use alloc::{vec, vec::Vec};

/// Opalescent-level name of the opaque byte-buffer nominal type.
const BYTES_TYPE_NAME: &str = "Bytes";

/// Name of the error nominal type reported by `bytes_from_hex`.
const HEX_DECODE_ERROR: &str = "HexDecodeError";

/// Name of the error nominal type reported by `bytes_slice`.
const SLICE_RANGE_ERROR: &str = "SliceRangeError";

impl TypeChecker {
    /// Register the `Bytes` nominal type and the six bytes built-in signatures.
    ///
    /// Call this exactly once from
    /// [`TypeChecker::register_standard_builtins`](super::TypeChecker::register_standard_builtins).
    pub(super) fn register_bytes_builtins(&mut self) {
        self.register_bytes_nominal_types();

        self.register_bytes_builtin("bytes_new", Vec::new(), vec![bytes_core_type()], Vec::new());

        self.register_bytes_builtin(
            "bytes_length",
            vec![bytes_core_type()],
            vec![CoreType::Int32],
            Vec::new(),
        );

        self.register_bytes_builtin(
            "bytes_to_hex",
            vec![bytes_core_type()],
            vec![CoreType::String],
            Vec::new(),
        );

        self.register_bytes_builtin(
            "bytes_concatenate",
            vec![bytes_core_type(), bytes_core_type()],
            vec![bytes_core_type()],
            Vec::new(),
        );

        self.register_bytes_builtin(
            "bytes_from_hex",
            vec![CoreType::String],
            vec![bytes_core_type()],
            vec![error_core_type(HEX_DECODE_ERROR)],
        );

        self.register_bytes_builtin(
            "bytes_slice",
            vec![bytes_core_type(), CoreType::Int32, CoreType::Int32],
            vec![bytes_core_type()],
            vec![error_core_type(SLICE_RANGE_ERROR)],
        );
    }

    /// Register the nominal `Bytes` type plus the bytes error type names.
    fn register_bytes_nominal_types(&mut self) {
        self.environment
            .register_type(BYTES_TYPE_NAME.to_owned(), bytes_core_type());
        self.environment.register_type(
            HEX_DECODE_ERROR.to_owned(),
            error_core_type(HEX_DECODE_ERROR),
        );
        self.environment.register_type(
            SLICE_RANGE_ERROR.to_owned(),
            error_core_type(SLICE_RANGE_ERROR),
        );
    }

    /// Register a single bytes built-in function signature in both the type
    /// environment and the symbol table.
    fn register_bytes_builtin(
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

/// Construct the opaque nominal [`CoreType`] representing `Bytes` values.
fn bytes_core_type() -> CoreType {
    CoreType::Generic {
        name: BYTES_TYPE_NAME.to_owned(),
        type_args: Vec::new(),
    }
}

/// Construct a nominal error [`CoreType`] with no parameters.
fn error_core_type(name: &str) -> CoreType {
    CoreType::Generic {
        name: name.to_owned(),
        type_args: Vec::new(),
    }
}
