//! Type constraint representation for constraint-based type inference

extern crate alloc;

use super::types::CoreType;
use crate::token::Span;
use alloc::{string::String, vec::Vec};

/// Type constraints used in constraint-based type inference
///
/// These constraints are collected during AST traversal and then solved
/// to determine concrete types for type variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeConstraint {
    /// Two types must be equal with optional source spans describing each operand
    Equality {
        /// Left-hand side type in the equality constraint
        left: CoreType,
        /// Right-hand side type in the equality constraint
        right: CoreType,
        /// Source span describing the origin of the left-hand side type
        left_span: Option<Span>,
        /// Source span describing the origin of the right-hand side type
        right_span: Option<Span>,
    },
    /// A type must have a specific field with a given type
    HasField {
        /// Owner type expected to contain the field
        owner: CoreType,
        /// Field name being validated
        field_name: String,
        /// Expected field type
        field_type: CoreType,
        /// Source span describing the owner type occurrence
        owner_span: Option<Span>,
        /// Source span describing the field lookup occurrence
        field_span: Option<Span>,
    },
    /// A type must be callable with specific argument and return types
    Callable {
        /// Callee type that must satisfy callable semantics
        callee: CoreType,
        /// Argument types expected for the call site
        arguments: Vec<CoreType>,
        /// Expected return type of the callable
        return_type: CoreType,
        /// Source span describing where the callee originates
        callee_span: Option<Span>,
        /// Source spans describing individual argument expressions
        argument_spans: Vec<Option<Span>>,
        /// Source span describing the expected return location
        return_span: Option<Span>,
    },
}

impl TypeConstraint {
    /// Create an equality constraint that retains optional span metadata for both operands.
    pub const fn equality(
        left: CoreType,
        right: CoreType,
        left_span: Option<Span>,
        right_span: Option<Span>,
    ) -> Self {
        Self::Equality {
            left,
            right,
            left_span,
            right_span,
        }
    }
}
