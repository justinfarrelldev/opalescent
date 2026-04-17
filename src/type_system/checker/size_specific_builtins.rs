//! Size-specific numeric I/O and conversion builtin registration.

extern crate alloc;

use crate::token::Span;
use crate::type_system::checker::TypeChecker;
use crate::type_system::symbol_table::{SymbolInfo, SymbolType, Visibility};
use crate::type_system::types::CoreType;
use alloc::{vec, vec::Vec};

impl TypeChecker {
    /// Register size-specific numeric I/O and conversion builtins for all integer and float types.
    pub(super) fn register_size_specific_builtins(&mut self) {
        // string_to_int* variants (int64 is import-only via `standard` module)
        self.register_string_to_int("string_to_int8", CoreType::Int8);
        self.register_string_to_int("string_to_int16", CoreType::Int16);
        self.register_string_to_int("string_to_uint8", CoreType::UInt8);
        self.register_string_to_int("string_to_uint16", CoreType::UInt16);
        self.register_string_to_int("string_to_uint32", CoreType::UInt32);
        self.register_string_to_int("string_to_uint64", CoreType::UInt64);

        self.register_string_to_int("string_to_float32", CoreType::Float32);
        self.register_string_to_int("string_to_float64", CoreType::Float64);

        // *_to_string infallible conversions
        self.register_to_string("int8_to_string", CoreType::Int8);
        self.register_to_string("int16_to_string", CoreType::Int16);
        self.register_to_string("int32_to_string", CoreType::Int32);
        self.register_to_string("int64_to_string", CoreType::Int64);
        self.register_to_string("uint8_to_string", CoreType::UInt8);
        self.register_to_string("uint16_to_string", CoreType::UInt16);
        self.register_to_string("uint32_to_string", CoreType::UInt32);
        self.register_to_string("uint64_to_string", CoreType::UInt64);
        self.register_to_string("float32_to_string", CoreType::Float32);
        self.register_to_string("float64_to_string", CoreType::Float64);
        self.register_to_string("bool_to_string", CoreType::Boolean);

        // random_int* variants (int64 is import-only via `math` module)
        self.register_random_int("random_int8", CoreType::Int8);
        self.register_random_int("random_int16", CoreType::Int16);
        self.register_random_int("random_uint8", CoreType::UInt8);
        self.register_random_int("random_uint16", CoreType::UInt16);
        self.register_random_int("random_uint32", CoreType::UInt32);
        self.register_random_int("random_uint64", CoreType::UInt64);

        // print_int* variants
        self.register_print_variant("print_int8", CoreType::Int8);
        self.register_print_variant("print_int16", CoreType::Int16);
        self.register_print_variant("print_int32", CoreType::Int32);
        self.register_print_variant("print_int64", CoreType::Int64);
        self.register_print_variant("print_uint8", CoreType::UInt8);
        self.register_print_variant("print_uint16", CoreType::UInt16);
        self.register_print_variant("print_uint32", CoreType::UInt32);
        self.register_print_variant("print_uint64", CoreType::UInt64);
        self.register_print_variant("print_float32", CoreType::Float32);
        self.register_print_variant("print_float64", CoreType::Float64);
        self.register_print_variant("print_string", CoreType::String);
    }

    /// Register a `string -> T` conversion builtin with the given name and return type.
    fn register_string_to_int(&mut self, name: &str, return_type: CoreType) {
        let sig = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![CoreType::String],
            return_types: vec![return_type],
            error_types: vec![CoreType::Generic {
                name: "ParseError".to_owned(),
                type_args: Vec::new(),
            }],
        };
        self.environment
            .register_builtin(name.to_owned(), sig.clone());
        self.symbol_table.register(SymbolInfo {
            name: name.to_owned(),
            symbol_type: SymbolType::Function,
            core_type: sig,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }

    /// Register an infallible `T -> string` conversion builtin.
    fn register_to_string(&mut self, name: &str, param_type: CoreType) {
        let sig = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![param_type],
            return_types: vec![CoreType::String],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin(name.to_owned(), sig.clone());
        self.symbol_table.register(SymbolInfo {
            name: name.to_owned(),
            symbol_type: SymbolType::Function,
            core_type: sig,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }
    /// Registers a random integer builtin function for the given numeric type.
    fn register_random_int(&mut self, name: &str, element_type: CoreType) {
        let sig = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![element_type.clone(), element_type.clone()],
            return_types: vec![element_type],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin(name.to_owned(), sig.clone());
        self.symbol_table.register(SymbolInfo {
            name: name.to_owned(),
            symbol_type: SymbolType::Function,
            core_type: sig,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }

    /// Register a single-argument print builtin that accepts `param_type` and returns `Unit`.
    fn register_print_variant(&mut self, name: &str, param_type: CoreType) {
        let sig = CoreType::Function {
            generic_params: Vec::new(),
            parameters: vec![param_type],
            return_types: vec![CoreType::Unit],
            error_types: Vec::new(),
        };
        self.environment
            .register_builtin(name.to_owned(), sig.clone());
        self.symbol_table.register(SymbolInfo {
            name: name.to_owned(),
            symbol_type: SymbolType::Function,
            core_type: sig,
            visibility: Visibility::Public,
            source_location: Span::single(crate::token::Position::start()),
            is_let_binding: false,
            is_mutable: false,
            read_count: 0,
        });
    }
}
