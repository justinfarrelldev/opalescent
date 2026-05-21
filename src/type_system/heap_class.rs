//! Canonical heap/ownership classification for language and runtime surfaces.
//!
//! This module centralizes the ownership category used by the ongoing memory-model
//! migration so follow-on passes do not re-encode the same type-name checks.
//! The classifier is intentionally narrow for this phase: it covers the built-in
//! string/array surfaces plus the currently-known nominal runtime handles that
//! participate in heap ownership decisions.

use crate::type_system::types::CoreType;

/// Canonical heap/ownership category for a language or runtime-facing type surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HeapClass {
    /// Value is not a managed heap surface for the current migration work.
    InlineValue,
    /// Value is a heap allocation tracked through the reference-counted memory model.
    ReferenceCounted,
    /// Value is a heap allocation whose ownership is transferred directly to the caller.
    CallerOwned,
    /// Value is a runtime-managed handle with process-lifetime or atexit-style cleanup.
    RuntimeManaged,
}

impl HeapClass {
    /// Return whether this category represents a heap-allocated surface.
    #[must_use]
    pub const fn is_heap_allocated(self) -> bool {
        !matches!(self, Self::InlineValue)
    }
}

/// Classify a canonical nominal runtime type name when this phase recognizes it.
#[must_use]
pub fn classify_nominal_type(type_name: &str) -> Option<HeapClass> {
    match type_name {
        "Bytes" | "FilesystemPath" | "FileMetadata" | "FilePermissions" => {
            Some(HeapClass::CallerOwned)
        }
        "StringBuilder" | "FrameClock" => Some(HeapClass::RuntimeManaged),
        _ => None,
    }
}

/// Classify a resolved [`CoreType`] into the canonical heap/ownership category.
#[must_use]
pub fn classify_core_type(core_type: &CoreType) -> HeapClass {
    match core_type {
        &CoreType::String | &CoreType::Array(_) => HeapClass::ReferenceCounted,
        &CoreType::Generic {
            ref name,
            ref type_args,
        } if type_args.is_empty() => classify_nominal_type(name).unwrap_or(HeapClass::InlineValue),
        _ => HeapClass::InlineValue,
    }
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use super::{classify_core_type, classify_nominal_type, HeapClass};
    use crate::type_system::types::CoreType;
    use self::alloc::boxed::Box;
    use self::alloc::string::String;
    use self::alloc::vec::Vec;

    fn nominal(name: &str) -> CoreType {
        CoreType::Generic {
            name: String::from(name),
            type_args: Vec::new(),
        }
    }

    #[test]
    fn classifies_reference_counted_core_surfaces() {
        assert_eq!(classify_core_type(&CoreType::String), HeapClass::ReferenceCounted);
        assert_eq!(
            classify_core_type(&CoreType::Array(Box::new(CoreType::Int32))),
            HeapClass::ReferenceCounted
        );
    }

    #[test]
    fn test_heap_class_classifies_string_and_array_children() {
        assert_eq!(classify_core_type(&CoreType::String), HeapClass::ReferenceCounted);
        assert_eq!(
            classify_core_type(&CoreType::Array(Box::new(CoreType::String))),
            HeapClass::ReferenceCounted
        );
    }

    #[test]
    fn classifies_caller_owned_nominal_surfaces() {
        for type_name in ["Bytes", "FilesystemPath", "FileMetadata", "FilePermissions"] {
            assert_eq!(classify_nominal_type(type_name), Some(HeapClass::CallerOwned));
            assert_eq!(classify_core_type(&nominal(type_name)), HeapClass::CallerOwned);
        }
    }

    #[test]
    fn classifies_runtime_managed_nominal_surfaces() {
        for type_name in ["StringBuilder", "FrameClock"] {
            assert_eq!(
                classify_nominal_type(type_name),
                Some(HeapClass::RuntimeManaged)
            );
            assert_eq!(classify_core_type(&nominal(type_name)), HeapClass::RuntimeManaged);
        }
    }

    #[test]
    fn leaves_non_heap_values_inline_by_default() {
        assert_eq!(classify_core_type(&CoreType::Int32), HeapClass::InlineValue);
        assert_eq!(
            classify_core_type(&CoreType::Generic {
                name: String::from("UnclassifiedNominal"),
                type_args: Vec::new(),
            }),
            HeapClass::InlineValue
        );
        assert_eq!(classify_nominal_type("UnclassifiedNominal"), None);
    }
}
