//! Concrete application ABI contract for a precompiled test import edge.
use super::types::CoreType;
use crate::ast::BorrowKind;

/// The builtin path has an immutable UTF-8 value representation; source nominals
/// with the same short spelling keep their distinct canonical identities.
#[must_use]
pub fn is_filesystem_path(core: &CoreType) -> bool {
    matches!(*core,CoreType::Generic {ref name,ref type_args} if name=="FilesystemPath" && type_args.is_empty())
}

/// Define the v1 argument-snapshot capability contract.
///
/// Supports numeric/boolean primitives, UTF-8 strings, recursively managed arrays
/// of those values and standalone builtin filesystem paths. Opaque/resources/
/// records and arrays of caller-owned paths remain count-only.
#[must_use]
pub fn supports_argument_snapshot(core: &CoreType) -> bool {
    is_filesystem_path(core) || managed_snapshot(core)
}

/// Existing RC/COW ownership is sufficient only for these recursively managed values.
fn managed_snapshot(core: &CoreType) -> bool {
    if let CoreType::Array(ref element) = *core {
        return managed_snapshot(element);
    }
    matches!(
        *core,
        CoreType::String
            | CoreType::Boolean
            | CoreType::Int8
            | CoreType::Int16
            | CoreType::Int32
            | CoreType::Int64
            | CoreType::UInt8
            | CoreType::UInt16
            | CoreType::UInt32
            | CoreType::UInt64
            | CoreType::Float32
            | CoreType::Float64
    )
}

/// A double's nominal application signature; harness failures never enter errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeDoubleSignature {
    /// Application parameter types in source declaration order.
    pub parameters: Vec<CoreType>,
    /// Existing reference modes in exactly the same order.
    pub parameter_modes: Vec<BorrowKind>,
    /// Application success result types, preserving existing multi-return order.
    pub return_types: Vec<CoreType>,
    /// Allowed nominal application errors, not harness assertion failures.
    pub error_types: Vec<CoreType>,
}
