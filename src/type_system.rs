//! Type System Core for Opalescent Language
//!
//! This module provides the core type checking, type inference, and type safety
//! validation for the Opalescent programming language. It ensures static type safety
//! while providing helpful error messages and supporting advanced features like
//! generics and algebraic data types.

#![expect(
    dead_code,
    reason = "Type system is foundational infrastructure being built incrementally"
)]

extern crate alloc;

use crate::ast::Type;
use alloc::{collections::BTreeMap, fmt, string::String, vec::Vec};
use thiserror::Error;

/// Represents type variables used in type inference
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar {
    /// Unique identifier for this type variable
    pub id: usize,
    /// Human-readable name for debugging
    pub name: String,
}

impl TypeVar {
    /// Create a new type variable with the given id and name
    pub const fn new(id: usize, name: String) -> Self {
        Self { id, name }
    }
}

/// Represents the core types supported by the Opalescent language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CoreType {
    /// 8-bit signed integer
    Int8,
    /// 16-bit signed integer
    Int16,
    /// 32-bit signed integer
    Int32,
    /// 64-bit signed integer  
    Int64,
    /// 8-bit unsigned integer
    UInt8,
    /// 16-bit unsigned integer
    UInt16,
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
    /// Type variable for inference
    Variable(TypeVar),
    /// Array type with element type
    Array(Box<CoreType>),
    /// Function type with parameter types and return type
    Function {
        /// Parameter types
        parameters: Vec<CoreType>,
        /// Return type
        return_type: Box<CoreType>,
    },
    /// Generic type with name and type arguments
    Generic {
        /// Name of the generic type
        name: String,
        /// Type arguments
        type_args: Vec<CoreType>,
    },
}

impl fmt::Display for CoreType {
    /// Format `CoreType` for user-friendly error messages
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::Int8 => write!(f, "int8"),
            Self::Int16 => write!(f, "int16"),
            Self::Int32 => write!(f, "int32"),
            Self::Int64 => write!(f, "int64"),
            Self::UInt8 => write!(f, "uint8"),
            Self::UInt16 => write!(f, "uint16"),
            Self::UInt32 => write!(f, "uint32"),
            Self::UInt64 => write!(f, "uint64"),
            Self::Float32 => write!(f, "float32"),
            Self::Float64 => write!(f, "float64"),
            Self::String => write!(f, "string"),
            Self::Boolean => write!(f, "boolean"),
            Self::Unit => write!(f, "unit"),
            Self::Variable(ref var) => write!(f, "{}", var.name),
            Self::Array(ref element_type) => write!(f, "[{element_type}]"),
            Self::Function {
                ref parameters,
                ref return_type,
            } => {
                write!(f, "(")?;
                for (i, param) in parameters.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{param}")?;
                }
                write!(f, ") -> {return_type}")
            }
            Self::Generic {
                ref name,
                ref type_args,
            } => {
                write!(f, "{name}")?;
                if !type_args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in type_args.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{arg}")?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
    }
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

    /// Unification failed between two types
    #[error("Cannot unify types '{left}' and '{right}'")]
    UnificationFailed {
        /// Left type in the unification
        left: String,
        /// Right type in the unification
        right: String,
    },

    /// Occurs check failed (infinite type)
    #[error("Occurs check failed: type variable '{var_name}' occurs in '{type_name}'")]
    OccursCheckFailed {
        /// Name of the type variable
        var_name: String,
        /// Name of the type it occurs in
        type_name: String,
    },

    /// Constraint solving failed
    #[error("Constraint solving failed: {reason}")]
    ConstraintSolvingFailed {
        /// Reason for the failure
        reason: String,
    },

    /// Type variable ID overflow occurred
    #[error("Type variable ID overflow - too many type variables generated")]
    TypeVariableOverflow,

    /// Feature not yet implemented
    #[error("Feature not yet implemented: {feature}")]
    NotImplementedYet {
        /// Description of the feature not yet implemented
        feature: String,
    },
}

/// Represents a substitution from type variables to types
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Substitution {
    /// Map from type variable IDs to their substituted types
    mappings: BTreeMap<usize, CoreType>,
}

impl Substitution {
    /// Create an empty substitution
    #[expect(clippy::missing_const_for_fn, reason = "BTreeMap::new() is not const")]
    pub fn empty() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    /// Create a substitution with a single mapping
    pub fn single(var_id: usize, type_value: CoreType) -> Self {
        let mut mappings = BTreeMap::new();
        mappings.insert(var_id, type_value);
        Self { mappings }
    }

    /// Apply this substitution to a type
    pub fn apply(&self, core_type: &CoreType) -> CoreType {
        match *core_type {
            CoreType::Variable(ref var) => self
                .mappings
                .get(&var.id)
                .map_or_else(|| core_type.clone(), |substituted| self.apply(substituted)),
            CoreType::Array(ref element_type) => {
                CoreType::Array(Box::new(self.apply(element_type)))
            }
            CoreType::Function {
                parameters: ref params,
                return_type: ref ret_type,
            } => CoreType::Function {
                parameters: params.iter().map(|p| self.apply(p)).collect(),
                return_type: Box::new(self.apply(ret_type)),
            },
            CoreType::Generic {
                name: ref type_name,
                type_args: ref args,
            } => CoreType::Generic {
                name: type_name.clone(),
                type_args: args.iter().map(|arg| self.apply(arg)).collect(),
            },
            // Primitive types don't contain type variables
            _ => core_type.clone(),
        }
    }

    /// Compose this substitution with another (self after other)
    pub fn compose(self, other: &Self) -> Self {
        let mut result_mappings = BTreeMap::new();

        // Apply self to all mappings in other
        for (var_id, type_value) in &other.mappings {
            result_mappings.insert(*var_id, self.apply(type_value));
        }

        // Add mappings from self that are not in other
        for (var_id, type_value) in self.mappings {
            result_mappings.entry(var_id).or_insert(type_value);
        }

        Self {
            mappings: result_mappings,
        }
    }

    /// Check if this substitution is empty
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }

    /// Get the mappings for testing
    pub const fn mappings(&self) -> &BTreeMap<usize, CoreType> {
        &self.mappings
    }
}

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

/// Basic type checker for validating types
#[derive(Debug)]
pub struct TypeChecker {
    /// Current type environment
    environment: TypeEnvironment,
    /// Counter for generating fresh type variables
    next_var_id: usize,
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        Self {
            environment: TypeEnvironment::new(),
            next_var_id: 0,
        }
    }

    /// Create a type checker with a specific environment
    pub const fn with_environment(environment: TypeEnvironment) -> Self {
        Self {
            environment,
            next_var_id: 0,
        }
    }

    /// Get a reference to the current environment
    pub const fn environment(&self) -> &TypeEnvironment {
        &self.environment
    }

    /// Get a mutable reference to the current environment
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Cannot have const fn with mutable reference"
    )]
    pub fn environment_mut(&mut self) -> &mut TypeEnvironment {
        &mut self.environment
    }

    /// Generate a fresh type variable
    pub fn fresh_type_var(&mut self, name: String) -> Result<CoreType, TypeError> {
        let var = TypeVar::new(self.next_var_id, name);
        self.next_var_id = self
            .next_var_id
            .checked_add(1)
            .ok_or(TypeError::TypeVariableOverflow)?;
        Ok(CoreType::Variable(var))
    }

    /// Generate a fresh type variable with an auto-generated name
    pub fn fresh_type_var_auto(&mut self) -> Result<CoreType, TypeError> {
        self.fresh_type_var(format!("t{}", self.next_var_id))
    }

    /// Convert an AST Type to a `CoreType` for validation and instantiation
    /// Supports generics, arrays, and function types.
    pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, .. } => match name.as_str() {
                "int8" => Ok(CoreType::Int8),
                "int16" => Ok(CoreType::Int16),
                "int32" => Ok(CoreType::Int32),
                "int64" => Ok(CoreType::Int64),
                "uint8" => Ok(CoreType::UInt8),
                "uint16" => Ok(CoreType::UInt16),
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
            },
            Type::Array {
                ref element_type, ..
            } => {
                let elem_core = Self::ast_type_to_core_type(element_type)?;
                Ok(CoreType::Array(Box::new(elem_core)))
            }
            Type::Function {
                ref parameters,
                ref return_type,
                ..
            } => {
                let param_types = parameters
                    .iter()
                    .map(Self::ast_type_to_core_type)
                    .collect::<Result<Vec<_>, _>>()?;
                let ret_type = Self::ast_type_to_core_type(return_type)?;
                Ok(CoreType::Function {
                    parameters: param_types,
                    return_type: Box::new(ret_type),
                })
            }
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let args = type_args
                    .iter()
                    .map(Self::ast_type_to_core_type)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: args,
                })
            }
        }
    }
    /// Validate ADT (sum/product) type definitions for correctness
    /// Returns Ok if all variants/fields are valid, or `TypeError` if not
    pub fn validate_adt_type(&self, type_def: &crate::ast::TypeDef) -> Result<(), TypeError> {
        match *type_def {
            crate::ast::TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                        self.validate_type_name(&field.name, &core_field_type)?;
                    }
                }
                Ok(())
            }
            crate::ast::TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    let core_field_type = Self::ast_type_to_core_type(&field.type_annotation)?;
                    self.validate_type_name(&field.name, &core_field_type)?;
                }
                Ok(())
            }
            crate::ast::TypeDef::Alias {
                ref target_type, ..
            } => {
                let _: CoreType = Self::ast_type_to_core_type(target_type)?;
                Ok(())
            }
        }
    }
    /// Type check a pattern match expression
    /// Ensures all patterns and arms are type compatible
    pub fn type_check_pattern_match(
        &self,
        matched_type: &CoreType,
        patterns: &[CoreType],
        arm_types: &[CoreType],
    ) -> Result<(), TypeError> {
        // Each pattern must be compatible with matched_type
        for pat in patterns {
            if !self.types_compatible(matched_type, pat) {
                return Err(TypeError::TypeMismatch {
                    expected: matched_type.to_string(),
                    found: pat.to_string(),
                });
            }
        }
        // All arm types must be compatible with each other
        if let Some(first) = arm_types.first() {
            for arm in arm_types {
                if !self.types_compatible(first, arm) {
                    return Err(TypeError::TypeMismatch {
                        expected: first.to_string(),
                        found: arm.to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    /// Check if two core types are structurally compatible (including nested types)
    ///
    /// This method performs deep structural comparison for complex types like
    /// arrays, functions, and generics, ensuring all nested components are compatible.
    /// For simple equality checking, use the `==` operator on `CoreType` directly.
    #[expect(
        clippy::only_used_in_recursion,
        reason = "self parameter needed for structural recursion"
    )]
    pub fn types_compatible(&self, left: &CoreType, right: &CoreType) -> bool {
        match (left, right) {
            // All primitive types
            (&CoreType::Int8, &CoreType::Int8)
            | (&CoreType::Int16, &CoreType::Int16)
            | (&CoreType::Int32, &CoreType::Int32)
            | (&CoreType::Int64, &CoreType::Int64)
            | (&CoreType::UInt8, &CoreType::UInt8)
            | (&CoreType::UInt16, &CoreType::UInt16)
            | (&CoreType::UInt32, &CoreType::UInt32)
            | (&CoreType::UInt64, &CoreType::UInt64)
            | (&CoreType::Float32, &CoreType::Float32)
            | (&CoreType::Float64, &CoreType::Float64)
            | (&CoreType::String, &CoreType::String)
            | (&CoreType::Boolean, &CoreType::Boolean)
            | (&CoreType::Unit, &CoreType::Unit) => true,

            // Variables are equal if they have the same ID
            (CoreType::Variable(left_var), CoreType::Variable(right_var)) => {
                left_var.id == right_var.id
            }

            // Arrays are compatible if element types are compatible
            (CoreType::Array(left_elem), CoreType::Array(right_elem)) => {
                self.types_compatible(left_elem, right_elem)
            }

            // Functions are compatible if parameters and return types are compatible
            (
                CoreType::Function {
                    parameters: left_params,
                    return_type: left_ret,
                },
                CoreType::Function {
                    parameters: right_params,
                    return_type: right_ret,
                },
            ) => {
                left_params.len() == right_params.len()
                    && left_params
                        .iter()
                        .zip(right_params.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
                    && self.types_compatible(left_ret, right_ret)
            }

            // Generic types are compatible if names and type arguments are compatible
            (
                CoreType::Generic {
                    name: left_name,
                    type_args: left_args,
                },
                CoreType::Generic {
                    name: right_name,
                    type_args: right_args,
                },
            ) => {
                left_name == right_name
                    && left_args.len() == right_args.len()
                    && left_args
                        .iter()
                        .zip(right_args.iter())
                        .all(|(l, r)| self.types_compatible(l, r))
            }

            // Different types are not compatible
            _ => false,
        }
    }

    /// Validate that a type name is valid for the given core type
    pub fn validate_type_name(&self, name: &str, core_type: &CoreType) -> Result<(), TypeError> {
        if let Ok(existing_type) = self.environment.lookup_type(name) {
            if existing_type != core_type {
                return Err(TypeError::TypeMismatch {
                    expected: existing_type.to_string(),
                    found: core_type.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Unify two types, returning a substitution that makes them equal
    pub fn unify(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        self.unify_impl(left, right)
    }

    /// Internal implementation of unification algorithm
    fn unify_impl(&self, left: &CoreType, right: &CoreType) -> Result<Substitution, TypeError> {
        match (left, right) {
            // Same primitive types unify with empty substitution
            (l, r) if self.types_compatible(l, r) => Ok(Substitution::empty()),

            // Variable unifies with any type (with occurs check)
            (&CoreType::Variable(ref var), other) | (other, &CoreType::Variable(ref var)) => {
                if Self::occurs_check(var.id, other) {
                    Err(TypeError::OccursCheckFailed {
                        var_name: var.name.clone(),
                        type_name: other.to_string(),
                    })
                } else {
                    Ok(Substitution::single(var.id, other.clone()))
                }
            }

            // Arrays unify if their element types unify
            (CoreType::Array(left_elem), CoreType::Array(right_elem)) => {
                self.unify_impl(left_elem, right_elem)
            }

            // Functions unify if parameters and return types unify
            (
                CoreType::Function {
                    parameters: left_params,
                    return_type: left_ret,
                },
                CoreType::Function {
                    parameters: right_params,
                    return_type: right_ret,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all parameters
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    let left_applied = combined_subst.apply(left_param);
                    let right_applied = combined_subst.apply(right_param);
                    let param_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&param_subst);
                }

                // Unify return types
                let left_ret_applied = combined_subst.apply(left_ret);
                let right_ret_applied = combined_subst.apply(right_ret);
                let ret_subst = self.unify_impl(&left_ret_applied, &right_ret_applied)?;
                combined_subst = combined_subst.compose(&ret_subst);

                Ok(combined_subst)
            }

            // Generic types unify if names match and type arguments unify
            (
                CoreType::Generic {
                    name: left_name,
                    type_args: left_args,
                },
                CoreType::Generic {
                    name: right_name,
                    type_args: right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return Err(TypeError::UnificationFailed {
                        left: left.to_string(),
                        right: right.to_string(),
                    });
                }

                let mut combined_subst = Substitution::empty();

                // Unify all type arguments
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    let left_applied = combined_subst.apply(left_arg);
                    let right_applied = combined_subst.apply(right_arg);
                    let arg_subst = self.unify_impl(&left_applied, &right_applied)?;
                    combined_subst = combined_subst.compose(&arg_subst);
                }

                Ok(combined_subst)
            }

            // Different types cannot be unified
            _ => Err(TypeError::UnificationFailed {
                left: left.to_string(),
                right: right.to_string(),
            }),
        }
    }

    /// Check if a type variable occurs in a type (prevents infinite types)
    /// Uses iterative approach to avoid stack overflow with deeply nested types
    fn occurs_check(var_id: usize, initial_type: &CoreType) -> bool {
        let mut work_queue = vec![initial_type];

        while let Some(current_type) = work_queue.pop() {
            match *current_type {
                CoreType::Variable(ref var) => {
                    if var.id == var_id {
                        return true;
                    }
                }
                CoreType::Array(ref element_type) => {
                    work_queue.push(element_type);
                }
                CoreType::Function {
                    parameters: ref params,
                    return_type: ref ret_type,
                } => {
                    work_queue.push(ret_type);
                    work_queue.extend(params.iter());
                }
                CoreType::Generic {
                    type_args: ref args,
                    ..
                } => {
                    work_queue.extend(args.iter());
                }
                // Primitive types don't contain variables - skip them
                _ => {}
            }
        }

        false
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
    use crate::ast::{Field, Type, TypeDef, Variant};
    use crate::token::{Position, Span};
    #[test]
    fn test_generic_type_instantiation() {
        let span = Span::single(Position::start());
        let ast_type = Type::Generic {
            name: "Result".to_owned(),
            type_args: vec![
                Type::Basic {
                    name: "int32".to_owned(),
                    span,
                },
                Type::Basic {
                    name: "string".to_owned(),
                    span,
                },
            ],
            span,
        };
        let core_type = TypeChecker::ast_type_to_core_type(&ast_type).unwrap();
        if let CoreType::Generic { name, type_args } = core_type {
            assert_eq!(name, "Result");
            assert_eq!(type_args.len(), 2);
            assert_eq!(type_args[0], CoreType::Int32);
            assert_eq!(type_args[1], CoreType::String);
        } else {
            unreachable!("Expected CoreType::Generic");
        }
    }

    #[test]
    fn test_adt_type_validation_sum() {
        let span = Span::single(Position::start());
        let variant = Variant {
            name: "Some".to_owned(),
            fields: vec![Field {
                name: "value".to_owned(),
                type_annotation: Type::Basic {
                    name: "int32".to_owned(),
                    span,
                },
                span,
            }],
            span,
        };
        let type_def = TypeDef::Sum {
            variants: vec![variant],
            span,
        };
        let checker = TypeChecker::new();
        assert!(checker.validate_adt_type(&type_def).is_ok());
    }

    #[test]
    fn test_adt_type_validation_product() {
        let span = Span::single(Position::start());
        let field = Field {
            name: "count".to_owned(),
            type_annotation: Type::Basic {
                name: "int32".to_owned(),
                span,
            },
            span,
        };
        let type_def = TypeDef::Product {
            fields: vec![field],
            span,
        };
        let checker = TypeChecker::new();
        assert!(checker.validate_adt_type(&type_def).is_ok());
    }

    #[test]
    fn test_pattern_match_type_check() {
        let checker = TypeChecker::new();
        let matched_type = CoreType::Int32;
        let patterns = vec![CoreType::Int32, CoreType::Int32];
        let arm_types = vec![CoreType::String, CoreType::String];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &patterns, &arm_types)
                .is_ok()
        );

        // Incompatible pattern
        let bad_patterns = vec![CoreType::String];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &bad_patterns, &arm_types)
                .is_err()
        );

        // Incompatible arm types
        let bad_arms = vec![CoreType::String, CoreType::Int32];
        assert!(
            checker
                .type_check_pattern_match(&matched_type, &patterns, &bad_arms)
                .is_err()
        );
    }

    #[test]
    fn test_type_environment_creation() {
        let env = TypeEnvironment::new();
        // Test basic types
        assert!(env.has_type("int32"));
        assert!(env.has_type("string"));
        assert!(env.has_type("boolean"));

        // Test extended integer types
        assert!(env.has_type("int8"));
        assert!(env.has_type("int16"));
        assert!(env.has_type("int64"));
        assert!(env.has_type("uint8"));
        assert!(env.has_type("uint16"));
        assert!(env.has_type("uint32"));
        assert!(env.has_type("uint64"));

        // Test floating point types
        assert!(env.has_type("float32"));
        assert!(env.has_type("float64"));

        // Test that non-existent types are not found
        assert!(!env.has_type("nonexistent"));
        assert!(!env.has_type("char"));
        assert!(!env.has_type("i32"));
    }

    #[test]
    fn test_type_environment_lookup() {
        let env = TypeEnvironment::new();

        // Test basic types
        assert_eq!(env.lookup_type("int32").unwrap(), &CoreType::Int32);
        assert_eq!(env.lookup_type("string").unwrap(), &CoreType::String);
        assert_eq!(env.lookup_type("boolean").unwrap(), &CoreType::Boolean);

        // Test extended integer types
        assert_eq!(env.lookup_type("int8").unwrap(), &CoreType::Int8);
        assert_eq!(env.lookup_type("int16").unwrap(), &CoreType::Int16);
        assert_eq!(env.lookup_type("int64").unwrap(), &CoreType::Int64);
        assert_eq!(env.lookup_type("uint8").unwrap(), &CoreType::UInt8);
        assert_eq!(env.lookup_type("uint16").unwrap(), &CoreType::UInt16);
        assert_eq!(env.lookup_type("uint32").unwrap(), &CoreType::UInt32);
        assert_eq!(env.lookup_type("uint64").unwrap(), &CoreType::UInt64);

        // Test floating point types
        assert_eq!(env.lookup_type("float32").unwrap(), &CoreType::Float32);
        assert_eq!(env.lookup_type("float64").unwrap(), &CoreType::Float64);

        // Test unit type
        assert_eq!(env.lookup_type("unit").unwrap(), &CoreType::Unit);

        // Test non-existent type
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
        use crate::token::{Position, Span};

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
    fn test_ast_type_to_core_type_extended_integers() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        // Test all integer types
        let int8_type = Type::Basic {
            name: "int8".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int8_type).unwrap(),
            CoreType::Int8
        );

        let int16_type = Type::Basic {
            name: "int16".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int16_type).unwrap(),
            CoreType::Int16
        );

        let uint8_type = Type::Basic {
            name: "uint8".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint8_type).unwrap(),
            CoreType::UInt8
        );

        let uint16_type = Type::Basic {
            name: "uint16".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint16_type).unwrap(),
            CoreType::UInt16
        );

        let uint32_type = Type::Basic {
            name: "uint32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint32_type).unwrap(),
            CoreType::UInt32
        );

        let uint64_type = Type::Basic {
            name: "uint64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&uint64_type).unwrap(),
            CoreType::UInt64
        );

        let int64_type = Type::Basic {
            name: "int64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&int64_type).unwrap(),
            CoreType::Int64
        );
    }

    #[test]
    fn test_ast_type_to_core_type_float_types() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        let float32_type = Type::Basic {
            name: "float32".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&float32_type).unwrap(),
            CoreType::Float32
        );

        let float64_type = Type::Basic {
            name: "float64".to_owned(),
            span,
        };
        assert_eq!(
            TypeChecker::ast_type_to_core_type(&float64_type).unwrap(),
            CoreType::Float64
        );
    }

    #[test]
    fn test_ast_type_to_core_type_complex_types() {
        use crate::token::{Position, Span};

        let start_pos = Position::new(1, 1, 0);
        let end_pos = Position::new(1, 6, 5);
        let span = Span::new(start_pos, end_pos);

        // Test that complex types now succeed
        let array_type = Type::Array {
            element_type: Box::new(Type::Basic {
                name: "int32".to_owned(),
                span,
            }),
            span,
        };
        let array_result = TypeChecker::ast_type_to_core_type(&array_type);
        assert!(array_result.is_ok());
        assert_eq!(
            array_result.unwrap(),
            CoreType::Array(Box::new(CoreType::Int32))
        );

        let function_type = Type::Function {
            parameters: vec![],
            return_type: Box::new(Type::Basic {
                name: "unit".to_owned(),
                span,
            }),
            span,
        };
        let function_result = TypeChecker::ast_type_to_core_type(&function_type);
        assert!(function_result.is_ok());
        assert_eq!(
            function_result.unwrap(),
            CoreType::Function {
                parameters: vec![],
                return_type: Box::new(CoreType::Unit),
            }
        );

        let generic_type = Type::Generic {
            name: "Array".to_owned(),
            type_args: vec![Type::Basic {
                name: "int32".to_owned(),
                span,
            }],
            span,
        };
        let generic_result = TypeChecker::ast_type_to_core_type(&generic_type);
        assert!(generic_result.is_ok());
        assert_eq!(
            generic_result.unwrap(),
            CoreType::Generic {
                name: "Array".to_owned(),
                type_args: vec![CoreType::Int32],
            }
        );
    }

    #[test]
    fn test_types_compatible() {
        let checker = TypeChecker::new();
        assert!(checker.types_compatible(&CoreType::Int32, &CoreType::Int32));
        assert!(checker.types_compatible(&CoreType::String, &CoreType::String));
        assert!(!checker.types_compatible(&CoreType::Int32, &CoreType::String));
        assert!(!checker.types_compatible(&CoreType::Boolean, &CoreType::Float32));
    }

    #[test]
    fn test_validate_type_name() {
        let checker = TypeChecker::new();

        // Valid type name for existing type
        assert!(
            checker
                .validate_type_name("int32", &CoreType::Int32)
                .is_ok()
        );

        // Invalid type name for different type
        assert!(
            checker
                .validate_type_name("int32", &CoreType::String)
                .is_err()
        );

        // New type name should be valid
        assert!(
            checker
                .validate_type_name("custom", &CoreType::Int32)
                .is_ok()
        );
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

        assert!(type_names.iter().any(|name| name == "int8"));
        assert!(type_names.iter().any(|name| name == "int16"));
        assert!(type_names.iter().any(|name| name == "int32"));
        assert!(type_names.iter().any(|name| name == "int64"));
        assert!(type_names.iter().any(|name| name == "uint8"));
        assert!(type_names.iter().any(|name| name == "uint16"));
        assert!(type_names.iter().any(|name| name == "uint32"));
        assert!(type_names.iter().any(|name| name == "uint64"));
        assert!(type_names.iter().any(|name| name == "float32"));
        assert!(type_names.iter().any(|name| name == "float64"));
        assert!(type_names.iter().any(|name| name == "string"));
        assert!(type_names.iter().any(|name| name == "boolean"));
        assert!(type_names.iter().any(|name| name == "unit"));

        // Ensure we have the minimum expected built-in types
        assert!(
            type_names.len() >= 13,
            "Expected at least 13 built-in types, found {}",
            type_names.len()
        );

        // Ensure names are sorted
        let mut sorted_names = type_names.clone();
        sorted_names.sort();
        assert_eq!(
            type_names, sorted_names,
            "Type names should be returned in sorted order"
        );
    }

    #[test]
    fn test_type_var_creation() {
        let var = TypeVar::new(42, "test_var".to_owned());
        assert_eq!(var.id, 42);
        assert_eq!(var.name, "test_var");
    }

    #[test]
    fn test_substitution_empty() {
        let subst = Substitution::empty();
        assert!(subst.is_empty());
        assert_eq!(subst.mappings().len(), 0);
    }

    #[test]
    fn test_substitution_single() {
        let var_id = 0;
        let core_type = CoreType::Int32;
        let subst = Substitution::single(var_id, core_type.clone());

        assert!(!subst.is_empty());
        assert_eq!(subst.mappings().len(), 1);
        assert_eq!(subst.mappings().get(&var_id), Some(&core_type));
    }

    #[test]
    fn test_substitution_apply_primitive() {
        let subst = Substitution::empty();
        let int_type = CoreType::Int32;

        // Applying substitution to primitive type should return the same type
        assert_eq!(subst.apply(&int_type), int_type);
    }

    #[test]
    fn test_substitution_apply_variable() {
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let int_type = CoreType::Int32;

        // Apply substitution that maps the variable to int32
        let subst = Substitution::single(var.id, int_type.clone());
        assert_eq!(subst.apply(&var_type), int_type);

        // Apply empty substitution should return the variable unchanged
        let empty_subst = Substitution::empty();
        assert_eq!(empty_subst.apply(&var_type), var_type);
    }

    #[test]
    fn test_substitution_apply_array() {
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var_type = CoreType::Array(Box::new(var_type));

        let subst = Substitution::single(var.id, CoreType::Int32);
        let expected = CoreType::Array(Box::new(CoreType::Int32));

        assert_eq!(subst.apply(&array_var_type), expected);
    }

    #[test]
    fn test_substitution_apply_function() {
        let var1 = TypeVar::new(0, "x".to_owned());
        let var2 = TypeVar::new(1, "y".to_owned());
        let var1_type = CoreType::Variable(var1.clone());
        let var2_type = CoreType::Variable(var2.clone());

        let function_type = CoreType::Function {
            parameters: vec![var1_type],
            return_type: Box::new(var2_type),
        };

        let mut mappings = BTreeMap::new();
        mappings.insert(var1.id, CoreType::Int32);
        mappings.insert(var2.id, CoreType::String);
        let subst = Substitution { mappings };

        let expected = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };

        assert_eq!(subst.apply(&function_type), expected);
    }

    #[test]
    fn test_substitution_compose() {
        // s1 maps x -> int32
        let s1 = Substitution::single(0, CoreType::Int32);

        // s2 maps y -> x (which should become int32 after composition)
        let var_x = TypeVar::new(0, "x".to_owned());
        let s2 = Substitution::single(1, CoreType::Variable(var_x));

        // Compose s1 after s2: s1(s2(...))
        let composed = s1.compose(&s2);

        // Should have mapping for y -> int32 and x -> int32
        assert_eq!(composed.mappings().len(), 2);
        assert_eq!(composed.mappings().get(&0), Some(&CoreType::Int32));
        assert_eq!(composed.mappings().get(&1), Some(&CoreType::Int32));
    }

    #[test]
    fn test_fresh_type_var_generation() {
        let mut checker = TypeChecker::new();

        let var1 = checker
            .fresh_type_var("test".to_owned())
            .expect("Should generate fresh type var");
        let var2 = checker
            .fresh_type_var_auto()
            .expect("Should generate fresh type var");

        // Should generate different variables
        assert_ne!(var1, var2);

        // Check they are variables
        assert!(matches!(var1, CoreType::Variable(_)));
        assert!(matches!(var2, CoreType::Variable(_)));
    }

    #[test]
    fn test_unify_identical_primitives() {
        let checker = TypeChecker::new();

        let int_result = checker.unify(&CoreType::Int32, &CoreType::Int32);
        assert!(int_result.is_ok());
        assert!(int_result.unwrap().is_empty());

        let string_result = checker.unify(&CoreType::String, &CoreType::String);
        assert!(string_result.is_ok());
        assert!(string_result.unwrap().is_empty());
    }

    #[test]
    fn test_unify_different_primitives() {
        let checker = TypeChecker::new();

        let mismatch_result = checker.unify(&CoreType::Int32, &CoreType::String);
        assert!(mismatch_result.is_err());

        if let Err(TypeError::UnificationFailed { left, right }) = mismatch_result {
            assert!(left.contains("int32"));
            assert!(right.contains("string"));
        } else {
            unreachable!("Expected UnificationFailed error");
        }
    }

    #[test]
    fn test_unify_variable_with_type() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let int_type = CoreType::Int32;

        let result = checker.unify(&var_type, &int_type);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert!(!subst.is_empty());
        assert_eq!(subst.mappings().get(&var.id), Some(&int_type));
    }

    #[test]
    fn test_unify_variable_with_variable() {
        let checker = TypeChecker::new();
        let var1 = TypeVar::new(0, "x".to_owned());
        let var2 = TypeVar::new(1, "y".to_owned());
        let var1_type = CoreType::Variable(var1.clone());
        let var2_type = CoreType::Variable(var2.clone());

        let result = checker.unify(&var1_type, &var2_type);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert!(!subst.is_empty());
        // One variable should be mapped to the other
        assert!(subst.mappings().contains_key(&var1.id) || subst.mappings().contains_key(&var2.id));
    }

    #[test]
    fn test_unify_arrays() {
        let checker = TypeChecker::new();
        let array_int = CoreType::Array(Box::new(CoreType::Int32));
        let array_string = CoreType::Array(Box::new(CoreType::String));

        // Arrays with same element type should unify
        let same_result = checker.unify(&array_int, &array_int);
        assert!(same_result.is_ok());
        assert!(same_result.unwrap().is_empty());

        // Arrays with different element types should not unify
        let different_result = checker.unify(&array_int, &array_string);
        assert!(different_result.is_err());
    }

    #[test]
    fn test_unify_arrays_with_variables() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var = CoreType::Array(Box::new(var_type));
        let array_int = CoreType::Array(Box::new(CoreType::Int32));

        let result = checker.unify(&array_var, &array_int);
        assert!(result.is_ok());

        let subst = result.unwrap();
        assert_eq!(subst.mappings().get(&var.id), Some(&CoreType::Int32));
    }

    #[test]
    fn test_unify_functions() {
        let checker = TypeChecker::new();
        let func1 = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };
        let func2 = CoreType::Function {
            parameters: vec![CoreType::Int32],
            return_type: Box::new(CoreType::String),
        };
        let func3 = CoreType::Function {
            parameters: vec![CoreType::String],
            return_type: Box::new(CoreType::Int32),
        };

        // Identical functions should unify
        let same_result = checker.unify(&func1, &func2);
        assert!(same_result.is_ok());
        assert!(same_result.unwrap().is_empty());

        // Different functions should not unify
        let different_result = checker.unify(&func1, &func3);
        assert!(different_result.is_err());
    }

    #[test]
    fn test_occurs_check() {
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());

        // Variable should occur in itself
        assert!(TypeChecker::occurs_check(var.id, &var_type));

        // Variable should occur in array containing it
        let array_var = CoreType::Array(Box::new(var_type));
        assert!(TypeChecker::occurs_check(var.id, &array_var));

        // Variable should not occur in different type
        assert!(!TypeChecker::occurs_check(var.id, &CoreType::Int32));

        // Variable should not occur in array of different type
        let array_int = CoreType::Array(Box::new(CoreType::Int32));
        assert!(!TypeChecker::occurs_check(var.id, &array_int));
    }

    #[test]
    fn test_occurs_check_prevents_infinite_types() {
        let checker = TypeChecker::new();
        let var = TypeVar::new(0, "x".to_owned());
        let var_type = CoreType::Variable(var.clone());
        let array_var = CoreType::Array(Box::new(var_type));

        // Trying to unify x with Array<x> should fail
        let infinite_result = checker.unify(&CoreType::Variable(var.clone()), &array_var);
        assert!(infinite_result.is_err());

        if let Err(TypeError::OccursCheckFailed {
            var_name,
            type_name,
        }) = infinite_result
        {
            assert_eq!(var_name, var.name);
            assert!(type_name.contains('[') && type_name.contains('x'));
        } else {
            unreachable!("Expected OccursCheckFailed error");
        }
    }
}
