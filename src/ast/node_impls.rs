//! `AstNode` trait implementations for AST types
//!
//! This module implements the `AstNode` trait for all major AST node types,
//! providing span and node ID accessors, along with hot-reload metadata.

extern crate alloc;
use crate::ast::{AstNode, Decl, Expr, ModulePath, NodeId, Program, Stmt, SymbolInfo};
use crate::token::Span;

impl AstNode for Expr {
    fn span(&self) -> Span {
        match *self {
            Self::Literal { span, .. }
            | Self::Identifier { span, .. }
            | Self::Binary { span, .. }
            | Self::Unary { span, .. }
            | Self::Call { span, .. }
            | Self::Constructor { span, .. }
            | Self::Index { span, .. }
            | Self::Member { span, .. }
            | Self::Cast { span, .. }
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

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Literal { id, .. }
            | Self::Identifier { id, .. }
            | Self::Binary { id, .. }
            | Self::Unary { id, .. }
            | Self::Call { id, .. }
            | Self::Constructor { id, .. }
            | Self::Index { id, .. }
            | Self::Member { id, .. }
            | Self::Cast { id, .. }
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

    fn abi_symbols(&self) -> alloc::vec::Vec<SymbolInfo> {
        match *self {
            Self::Lambda { ref metadata, .. } => metadata.abi_symbol.iter().cloned().collect(),
            _ => alloc::vec::Vec::new(),
        }
    }

    fn dependencies(&self) -> alloc::vec::Vec<ModulePath> {
        match *self {
            Self::Lambda { ref metadata, .. } => metadata.dependencies.clone(),
            _ => alloc::vec::Vec::new(),
        }
    }

    fn is_hot_reloadable(&self) -> bool {
        matches!(*self, Self::Lambda { ref metadata, .. } if metadata.is_hot_reloadable)
    }
}

impl AstNode for Stmt {
    fn span(&self) -> Span {
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
            | Self::PropagateGuardError { span, .. }
            | Self::Loop { span, .. }
            | Self::Break { span, .. }
            | Self::Continue { span, .. }
            | Self::Comment { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
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
            | Self::PropagateGuardError { id, .. }
            | Self::Loop { id, .. }
            | Self::Break { id, .. }
            | Self::Continue { id, .. }
            | Self::Comment { id, .. } => id,
        }
    }
}

impl AstNode for Decl {
    fn span(&self) -> Span {
        match *self {
            Self::Function { span, .. }
            | Self::Type { span, .. }
            | Self::Import { span, .. }
            | Self::Let { span, .. }
            | Self::Comment { span, .. } => span,
        }
    }

    fn node_id(&self) -> NodeId {
        match *self {
            Self::Function { id, .. }
            | Self::Type { id, .. }
            | Self::Import { id, .. }
            | Self::Let { id, .. }
            | Self::Comment { id, .. } => id,
        }
    }

    fn abi_symbols(&self) -> alloc::vec::Vec<SymbolInfo> {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.abi_symbol.iter().cloned().collect(),
            Self::Comment { .. } => alloc::vec::Vec::new(),
        }
    }

    fn dependencies(&self) -> alloc::vec::Vec<ModulePath> {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.dependencies.clone(),
            Self::Comment { .. } => alloc::vec::Vec::new(),
        }
    }

    fn is_hot_reloadable(&self) -> bool {
        match *self {
            Self::Function { ref metadata, .. }
            | Self::Type { ref metadata, .. }
            | Self::Import { ref metadata, .. }
            | Self::Let { ref metadata, .. } => metadata.is_hot_reloadable,
            Self::Comment { .. } => false,
        }
    }
}

impl AstNode for Program {
    fn span(&self) -> Span {
        self.span
    }

    fn node_id(&self) -> NodeId {
        self.id
    }
}
