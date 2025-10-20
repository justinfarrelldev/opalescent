//! Type constraint representation for constraint-based type inference

extern crate alloc;

use super::types::CoreType;
use alloc::{string::String, vec::Vec};

/// Type constraints used in constraint-based type inference
///
/// These constraints are collected during AST traversal and then solved
/// to determine concrete types for type variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    /// Two types must be equal
    Equality(CoreType, CoreType),
    /// A type must have a specific field with a given type
    HasField(CoreType, String, CoreType),
    /// A type must be callable with specific argument and return types
    Callable(CoreType, Vec<CoreType>, CoreType),
}
