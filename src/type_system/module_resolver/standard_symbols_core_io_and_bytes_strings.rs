#![allow(
    clippy::missing_docs_in_private_items,
    clippy::too_many_lines,
    reason = "symbol registration tables are intentionally explicit"
)]

extern crate alloc;

use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec::Vec};

pub(super) fn standard_symbols_string_helpers() -> Vec<(String, CoreType, SymbolType)> {
    vec![
        (
            String::from("string_join"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![
                    CoreType::Array(alloc::boxed::Box::new(CoreType::String)),
                    CoreType::String,
                ],
                return_types: vec![CoreType::String],
                error_types: vec![CoreType::Generic {
                    name: String::from("AllocationFailureError"),
                    type_args: Vec::new(),
                }],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_find_index_or"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String, CoreType::String, CoreType::Int64],
                return_types: vec![CoreType::Int64],
                error_types: Vec::new(),
            },
            SymbolType::Function,
        ),
        (
            String::from("string_find_last_index_of_text"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String, CoreType::String],
                return_types: vec![CoreType::Int64],
                error_types: vec![
                    CoreType::Generic {
                        name: String::from("StringEmptySearchTextError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("StringPatternNotFoundError"),
                        type_args: Vec::new(),
                    },
                ],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_split_lines"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String],
                return_types: vec![CoreType::Array(alloc::boxed::Box::new(CoreType::String))],
                error_types: vec![CoreType::Generic {
                    name: String::from("AllocationFailureError"),
                    type_args: Vec::new(),
                }],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_is_blank"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String],
                return_types: vec![CoreType::Boolean],
                error_types: Vec::new(),
            },
            SymbolType::Function,
        ),
        (
            String::from("string_trim_whitespace"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String],
                return_types: vec![CoreType::String],
                error_types: vec![CoreType::Generic {
                    name: String::from("AllocationFailureError"),
                    type_args: Vec::new(),
                }],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_take_prefix"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String, CoreType::Int64],
                return_types: vec![CoreType::String],
                error_types: vec![
                    CoreType::Generic {
                        name: String::from("StringNegativeCountError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("StringRangeOutOfBoundsError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("AllocationFailureError"),
                        type_args: Vec::new(),
                    },
                ],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_take_suffix"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String, CoreType::Int64],
                return_types: vec![CoreType::String],
                error_types: vec![
                    CoreType::Generic {
                        name: String::from("StringNegativeCountError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("StringRangeOutOfBoundsError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("AllocationFailureError"),
                        type_args: Vec::new(),
                    },
                ],
            },
            SymbolType::Function,
        ),
        (
            String::from("string_extract_range"),
            CoreType::Function {
                generic_params: Vec::new(),
                parameters: vec![CoreType::String, CoreType::Int64, CoreType::Int64],
                return_types: vec![CoreType::String],
                error_types: vec![
                    CoreType::Generic {
                        name: String::from("StringRangeOrderError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("StringRangeOutOfBoundsError"),
                        type_args: Vec::new(),
                    },
                    CoreType::Generic {
                        name: String::from("AllocationFailureError"),
                        type_args: Vec::new(),
                    },
                ],
            },
            SymbolType::Function,
        ),
        (
            String::from("AllocationFailureError"),
            CoreType::Generic {
                name: String::from("AllocationFailureError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
        (
            String::from("StringEmptySearchTextError"),
            CoreType::Generic {
                name: String::from("StringEmptySearchTextError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
        (
            String::from("StringPatternNotFoundError"),
            CoreType::Generic {
                name: String::from("StringPatternNotFoundError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
        (
            String::from("StringNegativeCountError"),
            CoreType::Generic {
                name: String::from("StringNegativeCountError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
        (
            String::from("StringRangeOutOfBoundsError"),
            CoreType::Generic {
                name: String::from("StringRangeOutOfBoundsError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
        (
            String::from("StringRangeOrderError"),
            CoreType::Generic {
                name: String::from("StringRangeOrderError"),
                type_args: Vec::new(),
            },
            SymbolType::Type,
        ),
    ]
}
