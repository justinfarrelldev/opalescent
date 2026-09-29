//! Shared checked numeric conversion registry.
//!
//! The registry is crate-visible so the type system, codegen, and tests agree
//! on the exact public `standard.numeric` surface.

/// Shared checked numeric conversion registry entry.
///
/// The entries here are the single Rust-side source of truth for the public
/// `standard.numeric` surface, codegen declarations, and guard/propagate
/// semantic return-type mapping. Runtime C declarations in `runtime/` must stay
/// in sync with this list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NumericConversionSpec {
    /// Runtime and stdlib symbol name, e.g. `int64_to_int32`.
    pub name: &'static str,
    /// Source numeric type name.
    pub source: &'static str,
    /// Destination numeric type name.
    pub destination: &'static str,
}

/// Authoritative Rust-side checked numeric conversion list.
#[rustfmt::skip]
pub const NUMERIC_CONVERSION_SPECS: &[NumericConversionSpec] = &[
    // Signed integer narrowing.
    NumericConversionSpec { name: "int16_to_int8", source: "int16", destination: "int8" },
    NumericConversionSpec { name: "int32_to_int8", source: "int32", destination: "int8" },
    NumericConversionSpec { name: "int32_to_int16", source: "int32", destination: "int16" },
    NumericConversionSpec { name: "int64_to_int8", source: "int64", destination: "int8" },
    NumericConversionSpec { name: "int64_to_int16", source: "int64", destination: "int16" },
    NumericConversionSpec { name: "int64_to_int32", source: "int64", destination: "int32" },

    // Unsigned integer narrowing.
    NumericConversionSpec { name: "uint16_to_uint8", source: "uint16", destination: "uint8" },
    NumericConversionSpec { name: "uint32_to_uint8", source: "uint32", destination: "uint8" },
    NumericConversionSpec { name: "uint32_to_uint16", source: "uint32", destination: "uint16" },
    NumericConversionSpec { name: "uint64_to_uint8", source: "uint64", destination: "uint8" },
    NumericConversionSpec { name: "uint64_to_uint16", source: "uint64", destination: "uint16" },
    NumericConversionSpec { name: "uint64_to_uint32", source: "uint64", destination: "uint32" },

    // Signed to unsigned value conversions.
    NumericConversionSpec { name: "int8_to_uint8", source: "int8", destination: "uint8" },
    NumericConversionSpec { name: "int8_to_uint16", source: "int8", destination: "uint16" },
    NumericConversionSpec { name: "int8_to_uint32", source: "int8", destination: "uint32" },
    NumericConversionSpec { name: "int8_to_uint64", source: "int8", destination: "uint64" },
    NumericConversionSpec { name: "int16_to_uint8", source: "int16", destination: "uint8" },
    NumericConversionSpec { name: "int16_to_uint16", source: "int16", destination: "uint16" },
    NumericConversionSpec { name: "int16_to_uint32", source: "int16", destination: "uint32" },
    NumericConversionSpec { name: "int16_to_uint64", source: "int16", destination: "uint64" },
    NumericConversionSpec { name: "int32_to_uint8", source: "int32", destination: "uint8" },
    NumericConversionSpec { name: "int32_to_uint16", source: "int32", destination: "uint16" },
    NumericConversionSpec { name: "int32_to_uint32", source: "int32", destination: "uint32" },
    NumericConversionSpec { name: "int32_to_uint64", source: "int32", destination: "uint64" },
    NumericConversionSpec { name: "int64_to_uint8", source: "int64", destination: "uint8" },
    NumericConversionSpec { name: "int64_to_uint16", source: "int64", destination: "uint16" },
    NumericConversionSpec { name: "int64_to_uint32", source: "int64", destination: "uint32" },
    NumericConversionSpec { name: "int64_to_uint64", source: "int64", destination: "uint64" },

    // Unsigned to signed conversions that can exceed destination range.
    NumericConversionSpec { name: "uint8_to_int8", source: "uint8", destination: "int8" },
    NumericConversionSpec { name: "uint16_to_int8", source: "uint16", destination: "int8" },
    NumericConversionSpec { name: "uint16_to_int16", source: "uint16", destination: "int16" },
    NumericConversionSpec { name: "uint32_to_int8", source: "uint32", destination: "int8" },
    NumericConversionSpec { name: "uint32_to_int16", source: "uint32", destination: "int16" },
    NumericConversionSpec { name: "uint32_to_int32", source: "uint32", destination: "int32" },
    NumericConversionSpec { name: "uint64_to_int8", source: "uint64", destination: "int8" },
    NumericConversionSpec { name: "uint64_to_int16", source: "uint64", destination: "int16" },
    NumericConversionSpec { name: "uint64_to_int32", source: "uint64", destination: "int32" },
    NumericConversionSpec { name: "uint64_to_int64", source: "uint64", destination: "int64" },

    // Float to integer exact conversions.
    NumericConversionSpec { name: "float32_to_int8", source: "float32", destination: "int8" },
    NumericConversionSpec { name: "float32_to_int16", source: "float32", destination: "int16" },
    NumericConversionSpec { name: "float32_to_int32", source: "float32", destination: "int32" },
    NumericConversionSpec { name: "float32_to_int64", source: "float32", destination: "int64" },
    NumericConversionSpec { name: "float32_to_uint8", source: "float32", destination: "uint8" },
    NumericConversionSpec { name: "float32_to_uint16", source: "float32", destination: "uint16" },
    NumericConversionSpec { name: "float32_to_uint32", source: "float32", destination: "uint32" },
    NumericConversionSpec { name: "float32_to_uint64", source: "float32", destination: "uint64" },
    NumericConversionSpec { name: "float64_to_int8", source: "float64", destination: "int8" },
    NumericConversionSpec { name: "float64_to_int16", source: "float64", destination: "int16" },
    NumericConversionSpec { name: "float64_to_int32", source: "float64", destination: "int32" },
    NumericConversionSpec { name: "float64_to_int64", source: "float64", destination: "int64" },
    NumericConversionSpec { name: "float64_to_uint8", source: "float64", destination: "uint8" },
    NumericConversionSpec { name: "float64_to_uint16", source: "float64", destination: "uint16" },
    NumericConversionSpec { name: "float64_to_uint32", source: "float64", destination: "uint32" },
    NumericConversionSpec { name: "float64_to_uint64", source: "float64", destination: "uint64" },

    // Integer to float exact conversions that can lose precision.
    NumericConversionSpec { name: "int32_to_float32", source: "int32", destination: "float32" },
    NumericConversionSpec { name: "uint32_to_float32", source: "uint32", destination: "float32" },
    NumericConversionSpec { name: "int64_to_float32", source: "int64", destination: "float32" },
    NumericConversionSpec { name: "uint64_to_float32", source: "uint64", destination: "float32" },
    NumericConversionSpec { name: "int64_to_float64", source: "int64", destination: "float64" },
    NumericConversionSpec { name: "uint64_to_float64", source: "uint64", destination: "float64" },

    // Float narrowing exact conversion.
    NumericConversionSpec { name: "float64_to_float32", source: "float64", destination: "float32" },
];

/// Look up a checked numeric conversion by runtime/stdlib symbol name.
pub fn numeric_conversion_spec(name: &str) -> Option<&'static NumericConversionSpec> {
    NUMERIC_CONVERSION_SPECS
        .iter()
        .find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_entries_have_runtime_declarations_and_definitions() {
        let header = include_str!("../runtime/opal_runtime.h");
        let implementation = include_str!("../runtime/opal_parse.c");

        for spec in NUMERIC_CONVERSION_SPECS {
            assert!(
                header.contains(spec.name),
                "missing runtime header declaration for {}",
                spec.name
            );
            assert!(
                implementation.contains(spec.name),
                "missing runtime implementation for {}",
                spec.name
            );
        }
    }
}
