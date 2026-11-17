//! Concrete application ABI contract for a precompiled test import edge.
use super::types::CoreType;
use crate::ast::BorrowKind;

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
