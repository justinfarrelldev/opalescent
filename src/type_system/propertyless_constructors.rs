//! Registry for propertyless constructor syntax support.
//!
//! This module centralizes the single source of truth for constructor names
//! that may omit a field block in source code and the runtime function each
//! constructor lowers to.

/// A propertyless constructor registry entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertylessConstructorEntry {
    /// Constructor type name accepted by the parser and type checker.
    pub type_name: &'static str,
    /// Runtime function name used during code generation.
    pub runtime_function: &'static str,
}

/// Canonical propertyless constructor entries.
pub const PROPERTYLESS_CONSTRUCTORS: [PropertylessConstructorEntry; 1] =
    [PropertylessConstructorEntry {
        type_name: "Bytes",
        runtime_function: "bytes_new",
    }];

// FUTURE: Additional opaque handles or propertyless built-ins (e.g., StringBuilder)
// can be added to this registry. Currently, only `Bytes` is supported to match
// the Opalescent standard library's dedicated binary data type.

/// Look up a propertyless constructor entry by type name.
#[must_use]
pub fn lookup_propertyless_constructor(
    type_name: &str,
) -> Option<&'static PropertylessConstructorEntry> {
    PROPERTYLESS_CONSTRUCTORS
        .iter()
        .find(|entry| entry.type_name == type_name)
}

/// Return whether a type name supports propertyless constructor syntax.
#[must_use]
pub fn is_propertyless_constructor_type(type_name: &str) -> bool {
    lookup_propertyless_constructor(type_name).is_some()
}

#[cfg(test)]
mod tests {
    use super::{is_propertyless_constructor_type, lookup_propertyless_constructor};

    /// `Bytes` must resolve to the `bytes_new` runtime constructor.
    #[test]
    fn lookup_bytes_propertyless_constructor() {
        let entry = lookup_propertyless_constructor("Bytes");
        assert_eq!(entry.map(|entry| entry.runtime_function), Some("bytes_new"));
    }

    /// `StringBuilder` must not be registered as propertyless.
    #[test]
    fn lookup_string_builder_propertyless_constructor() {
        let entry = lookup_propertyless_constructor("StringBuilder");
        assert!(entry.is_none(), "StringBuilder should not have a propertyless constructor entry");
    }

    /// The registry helper must report support only for `Bytes`.
    #[test]
    fn propertyless_constructor_type_predicate() {
        assert!(
            is_propertyless_constructor_type("Bytes"),
            "Bytes should be reported as propertyless"
        );
        assert!(
            !is_propertyless_constructor_type("StringBuilder"),
            "StringBuilder should not be reported as propertyless"
        );
    }
}
