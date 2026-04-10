//! Type representations and related structures for the AST
//!
//! This module contains all type-related structures including type definitions,
//! parameters, variants, and fields.

extern crate alloc;
use crate::token::Span;
use alloc::string::String;

/// Type representations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// Basic types
    Basic {
        /// Name of the basic type
        name: String,
        /// Source code location of this type
        span: Span,
    },

    /// Generic types (Array<T>, Result<T, E>)
    Generic {
        /// Name of the generic type
        name: String,
        /// Type arguments for the generic
        type_args: alloc::vec::Vec<Type>,
        /// Source code location of this type
        span: Span,
    },

    /// Array types (T[])
    Array {
        /// Type of elements in the array
        element_type: Box<Type>,
        /// Source code location of this type
        span: Span,
    },

    /// Function types (f(int32, string): boolean)
    Function {
        /// Parameter types of the function
        parameters: alloc::vec::Vec<Type>,
        /// Return types of the function in declaration order.
        ///
        /// Backward compatibility: single-return functions use a vector with one element.
        return_types: alloc::vec::Vec<Type>,
        /// Error types that this function may produce
        ///
        /// Optional list of error type names (e.g., `Some(vec!["ParseError", "IoError"])`).
        /// Used for pretty-printing function signatures and documentation generation.
        /// Empty `Some(vec![])` means function declares no errors.
        /// `None` means errors clause not specified (defaults to no errors).
        errors: Option<alloc::vec::Vec<Type>>,
        /// Source code location of this type
        span: Span,
    },
}

impl Type {
    /// Convenience accessor for the span associated with this type node
    #[must_use]
    pub const fn span_const(&self) -> Span {
        match *self {
            Self::Basic { span, .. }
            | Self::Generic { span, .. }
            | Self::Array { span, .. }
            | Self::Function { span, .. } => span,
        }
    }

    /// Runtime-friendly span accessor delegating to the const variant.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span_const()
    }

    /// Render the type into its canonical signature representation used for documentation.
    ///
    /// This canonical form mirrors Opalescent source syntax so that generated documentation
    /// remains faithful to the user-written code. Complex types (generics, arrays, functions)
    /// are rendered recursively to ensure nested constructs are represented accurately.
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Match ergonomics bind enum fields by reference on &Type; explicit deref would clone the entire type unnecessarily."
    )]
    #[must_use]
    pub fn to_signature_string(&self) -> String {
        match self {
            Self::Basic { name: name_ref, .. } => name_ref.clone(),
            Self::Generic {
                name: name_ref,
                type_args: type_args_ref,
                ..
            } => {
                if type_args_ref.is_empty() {
                    return name_ref.clone();
                }

                let mut result = String::from(name_ref.as_str());
                result.push('<');
                for (index, arg) in type_args_ref.iter().enumerate() {
                    if index > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&arg.to_signature_string());
                }
                result.push('>');
                result
            }
            Self::Array {
                element_type: element_type_ref,
                ..
            } => {
                let mut result = element_type_ref.to_signature_string();
                result.push_str("[]");
                result
            }
            Self::Function {
                parameters: parameters_ref,
                return_types: return_types_ref,
                errors: errors_ref,
                ..
            } => {
                let mut result = String::from("f(");
                for (index, param_type) in parameters_ref.iter().enumerate() {
                    if index > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&param_type.to_signature_string());
                }
                result.push(')');
                result.push_str(": ");
                for (index, return_type) in return_types_ref.iter().enumerate() {
                    if index > 0 {
                        result.push_str(", ");
                    }
                    result.push_str(&return_type.to_signature_string());
                }

                if let Some(error_list) = errors_ref.as_ref() {
                    if !error_list.is_empty() {
                        result.push_str(" errors ");
                        for (index, error_type) in error_list.iter().enumerate() {
                            if index > 0 {
                                result.push_str(", ");
                            }
                            result.push_str(&error_type.to_signature_string());
                        }
                    }
                }

                result
            }
        }
    }
}

/// Function parameters
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// Name of the parameter
    pub name: String,
    /// Type of the parameter
    pub param_type: Type,
    /// Source code location of this parameter
    pub span: Span,
}

/// Type definitions for custom types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDef {
    /// Sum types (enums with variants)
    Sum {
        /// Variants of the sum type
        variants: alloc::vec::Vec<Variant>,
        /// Source code location of this type definition
        span: Span,
    },

    /// Product types (structs)
    Product {
        /// Fields of the product type
        fields: alloc::vec::Vec<Field>,
        /// Source code location of this type definition
        span: Span,
    },

    /// Type aliases
    Alias {
        /// Target type that this alias refers to
        target_type: Type,
        /// Source code location of this type definition
        span: Span,
    },
}

/// Variant for sum types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// Name of the variant
    pub name: String,
    /// Fields associated with this variant
    pub fields: alloc::vec::Vec<Field>,
    /// Source code location of this variant
    pub span: Span,
}

/// Field for product types and variants
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// Name of the field
    pub name: String,
    /// Type of the field
    pub type_annotation: Type,
    /// Source code location of this field
    pub span: Span,
}
