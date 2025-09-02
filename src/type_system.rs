//! Type System Core for Opalescent Language
//!
//! This module provides the core type checking, type inference, and type safety
//! validation for the Opalescent programming language. It ensures static type safety
//! while providing helpful error messages and supporting advanced features like
//! generics and algebraic data types.

#![expect(dead_code, reason = "Type system is foundational infrastructure being built incrementally")]

use crate::ast::Type;
use std::collections::HashMap;
use thiserror::Error;

/// Represents the core types supported by the Opalescent language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreType {
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer  
    Int64,
    /// 32-bit unsigned integer
    UInt32,
    /// 64-bit unsigned integer
    UInt64,
    /// 32-bit floating point
    Float32,
    /// 64-bit floating point
    Float64,
    /// Unicode string
    String,
    /// Boolean type
    Boolean,
    /// Unit type (empty value)
    Unit,
}

/// Type checking errors that can occur during type analysis
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    /// Type was not found in the current scope
    #[error("Type '{type_name}' not found")]
    TypeNotFound {
        /// Name of the type that was not found
        type_name: String,
    },
    
    /// Types do not match in an expression
    #[error("Type mismatch: expected '{expected}', found '{found}'")]
    TypeMismatch {
        /// Expected type name
        expected: String,
        /// Actually found type name
        found: String,
    },
    
    /// Invalid type operation
    #[error("Invalid operation '{operation}' for type '{type_name}'")]
    InvalidOperation {
        /// Operation that was attempted
        operation: String,
        /// Name of the type the operation was attempted on
        type_name: String,
    },
    
    /// Generic type parameter not found
    #[error("Generic type parameter '{param_name}' not found")]
    GenericParameterNotFound {
        /// Name of the generic parameter that was not found
        param_name: String,
    },
}

/// Environment for tracking types and their definitions
#[derive(Debug, Clone)]
pub struct TypeEnvironment {
    /// Map of type names to their definitions
    types: HashMap<String, CoreType>,
    /// Map of generic type parameters to their constraints
    generic_params: HashMap<String, Vec<String>>,
}

impl TypeEnvironment {
    /// Create a new type environment with built-in types
    pub fn new() -> Self {
        let mut env = Self {
            types: HashMap::new(),
            generic_params: HashMap::new(),
        };
        
        // Register built-in types
        env.register_builtin_types();
        env
    }
    
    /// Register all built-in core types
    fn register_builtin_types(&mut self) {
        self.types.insert("int32".to_owned(), CoreType::Int32);
        self.types.insert("int64".to_owned(), CoreType::Int64);
        self.types.insert("uint32".to_owned(), CoreType::UInt32);
        self.types.insert("uint64".to_owned(), CoreType::UInt64);
        self.types.insert("float32".to_owned(), CoreType::Float32);
        self.types.insert("float64".to_owned(), CoreType::Float64);
        self.types.insert("string".to_owned(), CoreType::String);
        self.types.insert("boolean".to_owned(), CoreType::Boolean);
        self.types.insert("unit".to_owned(), CoreType::Unit);
    }
    
    /// Look up a type by name
    pub fn lookup_type(&self, name: &str) -> Result<&CoreType, TypeError> {
        self.types.get(name).ok_or_else(|| TypeError::TypeNotFound {
            type_name: name.to_owned(),
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
    pub fn get_type_names(&self) -> Vec<&String> {
        self.types.keys().collect()
    }
}

impl Default for TypeEnvironment {
    fn default() -> Self {
        Self::new()
    }
}

/// Basic type checker for validating types
#[derive(Debug)]
pub struct TypeChecker {
    /// Current type environment
    environment: TypeEnvironment,
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        Self {
            environment: TypeEnvironment::new(),
        }
    }
    
    /// Create a type checker with a specific environment
    pub const fn with_environment(environment: TypeEnvironment) -> Self {
        Self { environment }
    }
    
    /// Get a reference to the current environment
    pub const fn environment(&self) -> &TypeEnvironment {
        &self.environment
    }
    
    /// Get a mutable reference to the current environment
    #[expect(clippy::missing_const_for_fn, reason = "Cannot have const fn with mutable reference")]
    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.environment
    }
    
    /// Convert an AST Type to a `CoreType` for basic validation
    pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, .. } => {
                match name.as_str() {
                    "int32" => Ok(CoreType::Int32),
                    "int64" => Ok(CoreType::Int64),
                    "uint32" => Ok(CoreType::UInt32),
                    "uint64" => Ok(CoreType::UInt64),
                    "float32" => Ok(CoreType::Float32),
                    "float64" => Ok(CoreType::Float64),
                    "string" => Ok(CoreType::String),
                    "boolean" => Ok(CoreType::Boolean),
                    "unit" => Ok(CoreType::Unit),
                    _ => Err(TypeError::TypeNotFound {
                        type_name: name.clone(),
                    }),
                }
            }
            Type::Array { .. } => {
                // Arrays are more complex and will be handled in later phases
                Err(TypeError::InvalidOperation {
                    operation: "array type resolution".to_owned(),
                    type_name: "array".to_owned(),
                })
            }
            Type::Function { .. } => {
                // Function types are more complex and will be handled in later phases
                Err(TypeError::InvalidOperation {
                    operation: "function type resolution".to_owned(),
                    type_name: "function".to_owned(),
                })
            }
            Type::Generic { .. } => {
                // Generic types will be handled in later phases
                Err(TypeError::InvalidOperation {
                    operation: "generic type resolution".to_owned(),
                    type_name: "generic".to_owned(),
                })
            }
        }
    }
    
    /// Check if two core types are compatible
    pub fn types_compatible(left: &CoreType, right: &CoreType) -> bool {
        left == right
    }
    
    /// Validate that a type name is valid for the given core type
    pub fn validate_type_name(&self, name: &str, core_type: &CoreType) -> Result<(), TypeError> {
        if let Ok(existing_type) = self.environment.lookup_type(name) {
            if existing_type != core_type {
                return Err(TypeError::TypeMismatch {
                    expected: format!("{existing_type:?}"),
                    found: format!("{core_type:?}"),
                });
            }
        }
        Ok(())
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_environment_creation() {
        let env = TypeEnvironment::new();
        assert!(env.has_type("int32"));
        assert!(env.has_type("string"));
        assert!(env.has_type("boolean"));
        assert!(!env.has_type("nonexistent"));
    }

    #[test]
    fn test_type_environment_lookup() {
        let env = TypeEnvironment::new();
        
        assert_eq!(env.lookup_type("int32").unwrap(), &CoreType::Int32);
        assert_eq!(env.lookup_type("string").unwrap(), &CoreType::String);
        assert_eq!(env.lookup_type("boolean").unwrap(), &CoreType::Boolean);
        
        assert!(env.lookup_type("nonexistent").is_err());
    }

    #[test]
    fn test_type_environment_register() {
        let mut env = TypeEnvironment::new();
        
        assert!(!env.has_type("custom"));
        env.register_type("custom".to_owned(), CoreType::Int32);
        assert!(env.has_type("custom"));
        assert_eq!(env.lookup_type("custom").unwrap(), &CoreType::Int32);
    }

    #[test]
    fn test_type_checker_creation() {
        let checker = TypeChecker::new();
        assert!(checker.environment().has_type("int32"));
        assert!(checker.environment().has_type("string"));
    }

    #[test]
    fn test_ast_type_to_core_type() {
        use crate::token::{Span, Position};
        
        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);
        
        let int32_type = Type::Basic { 
            name: "int32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int32_type).unwrap(),
            CoreType::Int32
        );
        
        let string_type = Type::Basic {
            name: "string".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&string_type).unwrap(),
            CoreType::String
        );
        
        let invalid_type = Type::Basic {
            name: "nonexistent".to_owned(),
            span,
        };
        assert!(TypeChecker::ast_type_to_core_type(&invalid_type).is_err());
    }

    #[test]
    fn test_types_compatible() {
        assert!(TypeChecker::types_compatible(&CoreType::Int32, &CoreType::Int32));
        assert!(TypeChecker::types_compatible(&CoreType::String, &CoreType::String));
        assert!(!TypeChecker::types_compatible(&CoreType::Int32, &CoreType::String));
        assert!(!TypeChecker::types_compatible(&CoreType::Boolean, &CoreType::Float32));
    }

    #[test]
    fn test_validate_type_name() {
        let checker = TypeChecker::new();
        
        // Valid type name for existing type
        assert!(checker.validate_type_name("int32", &CoreType::Int32).is_ok());
        
        // Invalid type name for different type
        assert!(checker.validate_type_name("int32", &CoreType::String).is_err());
        
        // New type name should be valid
        assert!(checker.validate_type_name("custom", &CoreType::Int32).is_ok());
    }

    #[test]
    fn test_core_type_equality() {
        assert_eq!(CoreType::Int32, CoreType::Int32);
        assert_ne!(CoreType::Int32, CoreType::Int64);
        assert_ne!(CoreType::String, CoreType::Boolean);
    }

    #[test]
    fn test_type_error_messages() {
        let not_found = TypeError::TypeNotFound {
            type_name: "test".to_owned(),
        };
        assert!(not_found.to_string().contains("Type 'test' not found"));
        
        let mismatch = TypeError::TypeMismatch {
            expected: "int32".to_owned(),
            found: "string".to_owned(),
        };
        assert!(mismatch.to_string().contains("Type mismatch"));
        assert!(mismatch.to_string().contains("expected 'int32'"));
        assert!(mismatch.to_string().contains("found 'string'"));
    }

    #[test]
    fn test_environment_get_type_names() {
        let env = TypeEnvironment::new();
        let type_names = env.get_type_names();
        
        assert!(type_names.iter().any(|&name| name == "int32"));
        assert!(type_names.iter().any(|&name| name == "string"));
        assert!(type_names.iter().any(|&name| name == "boolean"));
        assert_eq!(type_names.len(), 9); // All built-in types
    }
}
