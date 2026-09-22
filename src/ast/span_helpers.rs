//! Const span and node-id helpers for AST nodes.

use super::{Decl, Expr, NodeId, Program, Stmt};
use crate::token::Span;

impl Expr {
    /// Retrieve the source span associated with this expression in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Literal { span, .. }
            | Self::Identifier { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Call { span, .. }
            | Self::Constructor { span, .. }
            | Self::Index { span, .. }
            | Self::Member { span, .. }
            | Self::BorrowArgument { span, .. }
            | Self::Cast { span, .. }
            | Self::Constrain { span, .. }
            | Self::Refinement { span, .. }
            | Self::TypeOf { span, .. }
            | Self::StringInterpolation { span, .. }
            | Self::Parenthesized { span, .. }
            | Self::If { span, .. }
            | Self::Array { span, .. }
            | Self::Match { span, .. }
            | Self::Loop { span, .. }
            | Self::Lambda { span, .. }
            | Self::Guard { span, .. }
            | Self::Propagate { span, .. } => span,
        }
    }

    /// Retrieve the unique identifier associated with this expression in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        match *self {
            Self::Literal { id, .. }
            | Self::Identifier { id, .. }
            | Self::Binary { id, .. }
            | Self::Unary { id, .. }
            | Self::Call { id, .. }
            | Self::Constructor { id, .. }
            | Self::Index { id, .. }
            | Self::Member { id, .. }
            | Self::BorrowArgument { id, .. }
            | Self::Cast { id, .. }
            | Self::Constrain { id, .. }
            | Self::Refinement { id, .. }
            | Self::TypeOf { id, .. }
            | Self::StringInterpolation { id, .. }
            | Self::Parenthesized { id, .. }
            | Self::If { id, .. }
            | Self::Array { id, .. }
            | Self::Match { id, .. }
            | Self::Loop { id, .. }
            | Self::Lambda { id, .. }
            | Self::Guard { id, .. }
            | Self::Propagate { id, .. } => id,
        }
    }
}

impl Stmt {
    /// Retrieve the source span associated with this statement in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Let { span, .. }
            | Self::LetDestructure { span, .. }
            | Self::Assignment { span, .. }
            | Self::Return { span, .. }
            | Self::Expression { span, .. }
            | Self::Block { span, .. }
            | Self::If { span, .. }
            | Self::For { span, .. }
            | Self::While { span, .. }
            | Self::Guard { span, .. }
            | Self::Using { span, .. }
            | Self::PropagateGuardError { span, .. }
            | Self::Loop { span, .. }
            | Self::Break { span, .. }
            | Self::Continue { span, .. }
            | Self::Comment { span, .. } => span,
        }
    }

    /// Retrieve the unique identifier associated with this statement in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        match *self {
            Self::Let { id, .. }
            | Self::LetDestructure { id, .. }
            | Self::Assignment { id, .. }
            | Self::Return { id, .. }
            | Self::Expression { id, .. }
            | Self::Block { id, .. }
            | Self::If { id, .. }
            | Self::For { id, .. }
            | Self::While { id, .. }
            | Self::Guard { id, .. }
            | Self::Using { id, .. }
            | Self::PropagateGuardError { id, .. }
            | Self::Loop { id, .. }
            | Self::Break { id, .. }
            | Self::Continue { id, .. }
            | Self::Comment { id, .. } => id,
        }
    }
}

impl Decl {
    /// Retrieve the source span associated with this declaration in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Function { span, .. }
            | Self::Type { span, .. }
            | Self::ErrorSet { span, .. }
            | Self::Import { span, .. }
            | Self::Namespace { span, .. }
            | Self::Let { span, .. }
            | Self::Comment { span, .. } => span,
        }
    }

    /// Retrieve the unique identifier associated with this declaration in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        match *self {
            Self::Function { id, .. }
            | Self::Type { id, .. }
            | Self::ErrorSet { id, .. }
            | Self::Import { id, .. }
            | Self::Namespace { id, .. }
            | Self::Let { id, .. }
            | Self::Comment { id, .. } => id,
        }
    }
}

impl Program {
    /// Retrieve the source span associated with the entire program in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Retrieve the unique identifier associated with this program in const contexts.
    #[must_use]
    pub const fn node_id_const(&self) -> NodeId {
        self.id
    }
}
