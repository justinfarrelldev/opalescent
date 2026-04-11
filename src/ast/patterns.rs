//! Pattern matching AST structures.

extern crate alloc;

use crate::ast::{Expr, LiteralValue};
use crate::token::Span;
use alloc::string::String;
use alloc::vec::Vec;

/// Pattern for match arms.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard pattern `_` — matches anything.
    Wildcard {
        /// Source location.
        span: Span,
    },
    /// Literal pattern: integer, float, string, boolean.
    Literal {
        /// The literal value being matched.
        value: LiteralValue,
        /// Source location.
        span: Span,
    },
    /// Identifier binding: `x` in `match v { x => ... }`.
    Binding {
        /// Name of the binding variable.
        name: String,
        /// Source location.
        span: Span,
    },
    /// Variant pattern: `SomeType.Variant { field1: p1, ... }`.
    Variant {
        /// Optional type name prefix (`SomeType.`).
        type_name: Option<String>,
        /// Variant name.
        variant_name: String,
        /// Field patterns (named or positional).
        fields: Vec<(Option<String>, Pattern)>,
        /// Source location.
        span: Span,
    },
    /// Tuple/positional pattern: `(p1, p2, ...)`.
    Tuple {
        /// Constituent patterns.
        elements: Vec<Pattern>,
        /// Source location.
        span: Span,
    },
}

/// A single arm of a match expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    /// The pattern to match against.
    pub pattern: Pattern,
    /// Optional guard clause (`if condition`).
    pub guard: Option<Expr>,
    /// The body expression of this arm.
    pub body: Expr,
    /// Source location of this arm.
    pub span: Span,
}
