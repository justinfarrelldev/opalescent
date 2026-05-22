//! Registry for runtime-backed fallible constructor syntax support.
//!
//! This module centralizes the single source of truth for constructor types that
//! lower through a runtime function returning the canonical error ABI.

extern crate alloc;

use crate::type_system::types::CoreType;
use alloc::{string::String, vec, vec::Vec};

/// Resolved canonical type identity used for fallible constructor lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanonicalTypeIdentity<'core_type> {
    /// Canonical nominal type name after alias resolution.
    pub name: &'core_type str,
}

impl<'core_type> CanonicalTypeIdentity<'core_type> {
    /// Create an identity from a canonical nominal type name.
    #[must_use]
    pub const fn new(name: &'core_type str) -> Self {
        Self { name }
    }

    /// Extract a lookup identity from a resolved core type.
    #[must_use]
    pub fn from_core_type(core_type: &'core_type CoreType) -> Option<Self> {
        match *core_type {
            CoreType::Generic { ref name, ref type_args } if type_args.is_empty() => {
                Some(Self::new(name.as_str()))
            }
            _ => None,
        }
    }
}

/// Metadata describing how a registered fallible constructor lowers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FallibleConstructorLowering {
    /// Runtime function symbol used during code generation.
    pub runtime_symbol: &'static str,
    /// Canonical result aggregate field index containing the error pointer.
    pub error_field_index: u32,
}

/// Ordered required field metadata for a registered fallible constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallibleConstructorField {
    /// Source-level field name accepted by constructor syntax.
    pub name: &'static str,
    /// Expected core type for the field expression.
    pub core_type: CoreType,
}

/// A fallible constructor registry entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallibleConstructorEntry {
    /// Canonical constructor result type identity used for lookup.
    pub canonical_result_type_name: &'static str,
    /// Runtime function symbol used during lowering.
    pub runtime_symbol: &'static str,
    /// Ordered required field schema in ABI call order.
    pub required_fields: Vec<FallibleConstructorField>,
    /// Canonical success type returned on the happy path.
    pub success_type: CoreType,
    /// Canonical error types returned on the error path.
    pub error_types: Vec<CoreType>,
    /// Lowering metadata shared with later codegen tasks.
    pub lowering: FallibleConstructorLowering,
}

fn frame_clock_entry() -> FallibleConstructorEntry {
    FallibleConstructorEntry {
        canonical_result_type_name: "FrameClock",
        runtime_symbol: "frame_clock_new",
        required_fields: vec![FallibleConstructorField {
            name: "frames_per_second",
            core_type: CoreType::Int32,
        }],
        success_type: CoreType::Generic {
            name: String::from("FrameClock"),
            type_args: Vec::new(),
        },
        error_types: vec![CoreType::Generic {
            name: String::from("InvalidFrameRateError"),
            type_args: Vec::new(),
        }],
        lowering: FallibleConstructorLowering {
            runtime_symbol: "frame_clock_new",
            error_field_index: 1,
        },
    }
}

#[cfg(test)]
fn test_second_entry() -> FallibleConstructorEntry {
    FallibleConstructorEntry {
        canonical_result_type_name: "TestFrameClock",
        runtime_symbol: "test_frame_clock_new",
        required_fields: vec![FallibleConstructorField {
            name: "seed",
            core_type: CoreType::Int32,
        }],
        success_type: CoreType::Generic {
            name: String::from("TestFrameClock"),
            type_args: Vec::new(),
        },
        error_types: vec![CoreType::Generic {
            name: String::from("TestFrameRateError"),
            type_args: Vec::new(),
        }],
        lowering: FallibleConstructorLowering {
            runtime_symbol: "test_frame_clock_new",
            error_field_index: 1,
        },
    }
}

/// Build the production fallible-constructor registry.
fn production_registry() -> Vec<FallibleConstructorEntry> {
    vec![frame_clock_entry()]
}

#[cfg(test)]
fn test_registry() -> Vec<FallibleConstructorEntry> {
    vec![frame_clock_entry(), test_second_entry()]
}

/// Look up a fallible constructor entry by resolved canonical type identity.
#[must_use]
pub fn lookup_fallible_constructor(
    identity: CanonicalTypeIdentity<'_>,
) -> Option<FallibleConstructorEntry> {
    registry_entries()
        .into_iter()
        .find(|entry| entry.canonical_result_type_name == identity.name)
}

#[cfg(test)]
/// Return the active fallible-constructor registry entries.
fn registry_entries() -> Vec<FallibleConstructorEntry> {
    test_registry()
}

#[cfg(not(test))]
fn registry_entries() -> Vec<FallibleConstructorEntry> {
    production_registry()
}
