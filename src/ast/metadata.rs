//! Hot-reload metadata and ABI compatibility structures
//!
//! This module contains structures for tracking hot-reload metadata,
//! ABI signatures, and symbol information for dynamic reloading support.

extern crate alloc;
use crate::token::Span;

use super::Visibility;

/// Symbol information for ABI signature generation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    /// Name of the symbol (function/type/variable/constant)
    pub name: alloc::string::String,
    /// Type of the symbol
    pub symbol_type: SymbolType,
    /// Type signature for ABI compatibility
    pub signature: TypeSignature,
    /// Visibility of the symbol
    pub visibility: Visibility,
    /// Source location of the symbol
    pub source_location: Span,
}

/// Symbol type for ABI signature
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolType {
    /// Function symbol (represents a function declaration)
    Function,
    /// Type symbol (represents a type declaration)
    Type,
    /// Variable symbol (represents a variable declaration)
    Variable,
    /// Constant symbol (represents a constant declaration)
    Constant,
}

/// ABI type signature for hot-reload compatibility
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeSignature {
    /// String representation of the type signature
    pub signature: alloc::string::String,
}

/// Module path for dependency tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModulePath(pub alloc::string::String);

/// Reusable metadata for hot-reload aware AST nodes
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HotReloadMetadata {
    /// Optional ABI symbol information for this node
    pub abi_symbol: Option<SymbolInfo>,
    /// Other modules this node depends on for hot-reload safety
    pub dependencies: alloc::vec::Vec<ModulePath>,
    /// Whether this node is eligible for hot reload without restart
    pub is_hot_reloadable: bool,
}
