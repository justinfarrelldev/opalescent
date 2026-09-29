extern crate alloc;

use crate::numeric_conversions::NUMERIC_CONVERSION_SPECS;
use crate::type_system::symbol_table::SymbolType;
use crate::type_system::types::CoreType;
use alloc::{string::String, vec, vec::Vec};

/// Checked, value-preserving numeric conversion functions.
pub(super) fn standard_symbols_numeric_conversions() -> Vec<(String, CoreType, SymbolType)> {
    NUMERIC_CONVERSION_SPECS
        .iter()
        .map(|spec| {
            (
                String::from(spec.name),
                CoreType::Function {
                    generic_params: Vec::new(),
                    parameters: vec![numeric_core_type(spec.source)],
                    return_types: vec![numeric_core_type(spec.destination)],
                    error_types: vec![CoreType::Generic {
                        name: String::from("IntegerRangeError"),
                        type_args: Vec::new(),
                    }],
                },
                SymbolType::Function,
            )
        })
        .collect()
}

/// Convert a registry numeric type name into the type system's core type.
fn numeric_core_type(name: &str) -> CoreType {
    match name {
        "int8" => CoreType::Int8,
        "int16" => CoreType::Int16,
        "int32" => CoreType::Int32,
        "int64" => CoreType::Int64,
        "uint8" => CoreType::UInt8,
        "uint16" => CoreType::UInt16,
        "uint32" => CoreType::UInt32,
        "uint64" => CoreType::UInt64,
        "float32" => CoreType::Float32,
        "float64" => CoreType::Float64,
        _ => unreachable!("numeric conversion registry contains only supported numeric types"),
    }
}
