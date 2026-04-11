//! Helper implementations for AST types
//!
//! This module contains convenience methods and helper implementations
//! for various AST node types.

extern crate alloc;
use crate::ast::{Field, HotReloadMetadata, ImportItem, LetBinding, Parameter, Pattern, Variant};
use crate::token::Span;
use alloc::string::String;

impl Parameter {
    /// Retrieve the source span associated with this parameter in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Runtime helper for retrieving the parameter span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }

    /// Render the parameter as `name: Type` for signature and documentation generation.
    #[must_use]
    pub fn to_signature_string(&self) -> String {
        let mut result = String::new();
        result.push_str(&self.name);
        result.push_str(": ");
        result.push_str(&self.param_type.to_signature_string());
        result
    }
}

impl HotReloadMetadata {
    /// Metadata with defaults for functions (not hot-reloadable until validated)
    pub const fn for_function() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata with defaults for top-level `let` declarations
    pub const fn for_let_declaration() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: true,
        }
    }

    /// Metadata with defaults for expressions (e.g., lambdas)
    pub const fn for_expression() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata defaults for type declarations (not hot-reloadable by default)
    pub const fn for_type_declaration() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }

    /// Metadata defaults for imports (never hot-reloadable)
    pub const fn for_import() -> Self {
        Self {
            abi_symbol: None,
            dependencies: alloc::vec::Vec::new(),
            is_hot_reloadable: false,
        }
    }
}

impl Variant {
    /// Retrieve the source span associated with this variant in const contexts.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Runtime helper for retrieving the variant span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }
}

impl Field {
    /// Retrieve the source span associated with this field definition.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Runtime helper returning the field span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }
}

impl LetBinding {
    /// Retrieve the source span associated with this binding.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        self.span
    }

    /// Runtime helper for retrieving the span outside of const contexts.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }
}

impl ImportItem {
    /// Retrieve the source span associated with this import item.
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Named { span, .. } | Self::Glob { span, .. } | Self::Type { span, .. } => span,
        }
    }

    /// Runtime helper for retrieving the import span.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }
}

impl Pattern {
    /// Retrieve the source span associated with this pattern.
    #[must_use]
    pub const fn span(&self) -> Span {
        match *self {
            Self::Wildcard { span }
            | Self::Literal { span, .. }
            | Self::Binding { span, .. }
            | Self::Variant { span, .. }
            | Self::Tuple { span, .. } => span,
        }
    }
}
