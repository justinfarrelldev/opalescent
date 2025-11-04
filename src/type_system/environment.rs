//! Type environment for managing type definitions

extern crate alloc;

use super::errors::TypeError;
use super::types::CoreType;
use crate::token::Span;
use alloc::collections::BTreeMap;
use alloc::{string::String, vec::Vec};

/// Environment for tracking types and their definitions
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Map of type names to their definitions
    types: BTreeMap<String, CoreType>,
    /// Map of generic type parameters to their constraints
    generic_params: BTreeMap<String, Vec<String>>,
}

impl TypeEnvironment {
    /// Create a new type environment with built-in types
    pub fn new() -> Self {
        let mut env = Self {
            types: BTreeMap::new(),
            generic_params: BTreeMap::new(),
        };

        // Register built-in types
        env.register_builtin_types();
        env
    }

    /// Register all built-in core types
    fn register_builtin_types(&mut self) {
        self.types.insert("int8".to_owned(), CoreType::Int8);
        self.types.insert("int16".to_owned(), CoreType::Int16);
        self.types.insert("int32".to_owned(), CoreType::Int32);
        self.types.insert("int64".to_owned(), CoreType::Int64);
        self.types.insert("uint8".to_owned(), CoreType::UInt8);
        self.types.insert("uint16".to_owned(), CoreType::UInt16);
        self.types.insert("uint32".to_owned(), CoreType::UInt32);
        self.types.insert("uint64".to_owned(), CoreType::UInt64);
        self.types.insert("float32".to_owned(), CoreType::Float32);
        self.types.insert("float64".to_owned(), CoreType::Float64);
        self.types.insert("string".to_owned(), CoreType::String);
        self.types.insert("boolean".to_owned(), CoreType::Boolean);
        self.types.insert("unit".to_owned(), CoreType::Unit);
        // The language spec presents `void` as the return type keyword, so alias it to `unit`.
        self.types.insert("void".to_owned(), CoreType::Unit);
    }

    /// Look up a type by name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the type to look up
    /// * `span` - Source location where the type was referenced (for error reporting)
    pub fn lookup_type(&self, name: &str, span: Span) -> Result<&CoreType, TypeError> {
        self.types.get(name).ok_or_else(|| TypeError::TypeNotFound {
            type_name: name.to_owned(),
            span: TypeError::span_from_span(span),
        })
    }

    /// Register a new type in the environment
    pub fn register_type(&mut self, name: String, core_type: CoreType) {
        self.types.insert(name, core_type);
    }

    /// Check if a type exists in the environment
    pub fn has_type(&self, name: &str) -> bool {
        self.types.contains_key(name)
    }

    /// Get all registered type names
    pub fn get_type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.types.keys().cloned().collect();
        names.sort();
        names
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}
