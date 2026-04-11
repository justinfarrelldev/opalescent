//! ABI signature generation for hot-reload compatibility checks.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Narrow C ABI function table exported by hot modules.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleVTable {
    /// Pointer to module entry function exposed with C ABI.
    pub module_entry: extern "C" fn(),
}

/// Canonical function signature used for ABI hashing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    /// Function parameter types in declaration order.
    pub parameters: Vec<String>,
    /// Function return types in declaration order.
    pub return_types: Vec<String>,
}

/// Exported function descriptor used for ABI construction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedFunction {
    /// Public symbol name exported by the module.
    pub name: String,
    /// Canonical signature for the exported symbol.
    pub signature: FunctionSignature,
}

/// POD memory layout descriptor used for ABI compatibility hashing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PodLayout {
    /// Size of the POD type in bytes.
    pub size: usize,
    /// Alignment of the POD type in bytes.
    pub align: usize,
}

/// Machine-checkable ABI metadata for hot-module compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiSignature {
    /// Exported function signatures keyed by symbol name.
    pub exported_functions: BTreeMap<String, FunctionSignature>,
    /// POD type layout hash keyed by type name.
    pub exported_pod_types: BTreeMap<String, u64>,
    /// Deterministic ABI hash for quick compatibility checks.
    pub abi_hash: u64,
}

impl AbiSignature {
    /// Creates an empty ABI signature.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            exported_functions: BTreeMap::new(),
            exported_pod_types: BTreeMap::new(),
            abi_hash: 0,
        }
    }
}

impl Default for AbiSignature {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates an ABI signature from exported symbols and POD type layouts.
#[must_use]
pub fn generate_abi_signature(
    exported_functions: &[ExportedFunction],
    pod_layouts: &BTreeMap<String, PodLayout>,
) -> AbiSignature {
    let mut signature = AbiSignature::new();

    for exported_function in exported_functions {
        signature.exported_functions.insert(
            exported_function.name.clone(),
            exported_function.signature.clone(),
        );
    }

    for (type_name, memory_layout) in pod_layouts {
        signature
            .exported_pod_types
            .insert(type_name.clone(), pod_layout_hash(memory_layout));
    }

    signature.abi_hash = compute_abi_hash(&signature);
    signature
}

/// Returns true when two ABI signatures are fully compatible.
#[must_use]
pub fn signatures_compatible(left: &AbiSignature, right: &AbiSignature) -> bool {
    left.abi_hash == right.abi_hash
        && left.exported_functions == right.exported_functions
        && left.exported_pod_types == right.exported_pod_types
}

/// Hashes a POD layout into a stable 64-bit value.
fn pod_layout_hash(layout: &PodLayout) -> u64 {
    let mut bytes = String::new();
    bytes.push_str(&layout.size.to_string());
    bytes.push(':');
    bytes.push_str(&layout.align.to_string());
    deterministic_djb2_hash(bytes.as_bytes())
}

/// Computes the aggregate ABI hash for exported functions and POD layouts.
fn compute_abi_hash(signature: &AbiSignature) -> u64 {
    let mut canonical = String::new();

    for (function_name, function_signature) in &signature.exported_functions {
        canonical.push_str("fn:");
        canonical.push_str(function_name);
        canonical.push('(');
        let mut first_parameter = true;
        for parameter in &function_signature.parameters {
            if !first_parameter {
                canonical.push(',');
            }
            first_parameter = false;
            canonical.push_str(&parameter.to_string());
        }
        canonical.push(')');
        canonical.push_str("->");
        let mut first_return = true;
        for return_type in &function_signature.return_types {
            if !first_return {
                canonical.push(',');
            }
            first_return = false;
            canonical.push_str(&return_type.to_string());
        }
        canonical.push(';');
    }

    for (type_name, type_hash) in &signature.exported_pod_types {
        canonical.push_str("pod:");
        canonical.push_str(type_name);
        canonical.push('=');
        canonical.push_str(&type_hash.to_string());
        canonical.push(';');
    }

    deterministic_djb2_hash(canonical.as_bytes())
}

/// Computes a deterministic DJB2-style hash from byte content.
fn deterministic_djb2_hash(bytes: &[u8]) -> u64 {
    let mut hash = 5_381_u64;
    for byte in bytes {
        hash = hash
            .wrapping_shl(5)
            .wrapping_add(hash)
            .wrapping_add(u64::from(*byte));
    }
    hash
}
