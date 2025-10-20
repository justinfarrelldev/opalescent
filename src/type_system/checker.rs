//! Type checker implementation for the Opalescent type system

#![allow(
    clippy::multiple_inherent_impl,
    reason = "TypeChecker impl blocks intentionally split across submodules (checker/*.rs) for code organization - each submodule handles a specific aspect of type checking"
)]

extern crate alloc;

use super::constraints::TypeConstraint;
use super::environment::TypeEnvironment;
use super::errors::TypeError;
use super::substitution::Substitution;
use super::symbol_table::{SymbolInfo, SymbolTable};
use super::types::{CoreType, TypeVar};
use crate::ast::Type;
use crate::token::Span;
use alloc::{boxed::Box, format, string::String, vec::Vec};

// Sub-modules
mod declarations;
mod expressions;
mod helpers;
mod statements;
mod unification;

/// Core type checker responsible for validating and inferring types
/// throughout the Opalescent type system
pub struct TypeChecker {
    /// Current type environment
    environment: TypeEnvironment,
    /// Counter for generating fresh type variables
    next_var_id: usize,
    /// Symbol table for tracking symbols in scope (Phase 2 and Phase 6)
    symbol_table: SymbolTable,
    /// Collected type constraints for inference (Phase 2)
    constraints: Vec<TypeConstraint>,
}

impl TypeChecker {
    /// Create a new type checker with a fresh environment
    pub fn new() -> Self {
        Self {
            environment: TypeEnvironment::new(),
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
        }
    }

    /// Create a type checker with a specific environment
    pub fn with_environment(environment: TypeEnvironment) -> Self {
        Self {
            environment,
            next_var_id: 0,
            symbol_table: SymbolTable::new(),
            constraints: Vec::new(),
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

    /// Get a reference to the symbol table
    pub const fn symbol_table(&self) -> &SymbolTable {
        &self.symbol_table
    }

    /// Get a mutable reference to the symbol table
    pub const fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        &mut self.symbol_table
    }

    /// Register a symbol for ABI signature generation (Phase 6)
    pub fn register_symbol(&mut self, symbol: SymbolInfo) {
        self.symbol_table.register(symbol);
    }

    /// Add a type constraint for inference (Phase 2)
    pub fn add_constraint(&mut self, constraint: TypeConstraint) {
        self.constraints.push(constraint);
    }

    /// Get all collected constraints
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Vec deref coercion to slice is not allowed in const fn"
    )]
    pub fn constraints(&self) -> &[TypeConstraint] {
        &self.constraints
    }

    /// Clear all collected constraints
    pub fn clear_constraints(&mut self) {
        self.constraints.clear();
    }

    /// Solve all collected constraints (Phase 2 - not yet implemented)
    ///
    /// This will be the main entry point for constraint-based type inference.
    /// It should unify all constraints and return a substitution that satisfies them all.
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if constraints cannot be satisfied.
    pub fn solve_constraints(&mut self) -> Result<Substitution, TypeError> {
        let pending_constraints = core::mem::take(&mut self.constraints);
        let mut substitution = Substitution::empty();

        for constraint in pending_constraints {
            match constraint {
                TypeConstraint::Equality(left, right) => {
                    let left_applied = substitution.apply(&left);
                    let right_applied = substitution.apply(&right);
                    let constraint_subst = self.unify(&left_applied, &right_applied)?;
                    substitution = substitution.compose(&constraint_subst);
                }
                TypeConstraint::HasField { .. } | TypeConstraint::Callable { .. } => {
                    return Err(TypeError::NotImplementedYet {
                        feature: format!("constraint type {constraint:?}"),
                        span: TypeError::unknown_span(),
                    });
                }
            }
        }

        Ok(substitution)
    }

    /// Generate a fresh type variable
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable name for the type variable
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if type variable ID overflows
    pub fn fresh_type_var(&mut self, name: String, span: Span) -> Result<CoreType, TypeError> {
        let var = TypeVar::new(self.next_var_id, name);
        self.next_var_id =
            self.next_var_id
                .checked_add(1)
                .ok_or_else(|| TypeError::ConstraintSolvingFailed {
                    reason: "type variable counter overflow".to_owned(),
                    span: TypeError::span_from_span(span),
                })?;
        Ok(CoreType::Variable(var))
    }

    /// Generate a fresh type variable with an auto-generated name
    ///
    /// # Arguments
    ///
    /// * `span` - Source location where the type variable is introduced (for error reporting)
    ///
    /// # Errors
    ///
    /// Returns `TypeError::ConstraintSolvingFailed` if type variable ID overflows
    pub fn fresh_type_var_auto(&mut self, span: Span) -> Result<CoreType, TypeError> {
        self.fresh_type_var(format!("t{}", self.next_var_id), span)
    }

    /// Convert an AST Type to a `CoreType` for validation and instantiation
    /// Supports generics, arrays, and function types.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when type conversion fails
    pub fn ast_type_to_core_type(ast_type: &Type) -> Result<CoreType, TypeError> {
        match *ast_type {
            Type::Basic { ref name, span } => match name.as_str() {
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
                    span: TypeError::span_from_span(span),
                }),
            },
            Type::Array {
                ref element_type, ..
            } => {
                let core_element = Self::ast_type_to_core_type(element_type.as_ref())?;
                Ok(CoreType::Array(Box::new(core_element)))
            }
            Type::Function {
                ref parameters,
                ref return_type,
                ..
            } => {
                let mut core_params = Vec::with_capacity(parameters.len());
                for param in parameters {
                    core_params.push(Self::ast_type_to_core_type(param)?);
                }
                let core_return = Self::ast_type_to_core_type(return_type.as_ref())?;
                Ok(CoreType::Function {
                    parameters: core_params,
                    return_type: Box::new(core_return),
                })
            }
            Type::Generic {
                ref name,
                ref type_args,
                ..
            } => {
                let mut core_args = Vec::with_capacity(type_args.len());
                for arg in type_args {
                    core_args.push(Self::ast_type_to_core_type(arg)?);
                }
                Ok(CoreType::Generic {
                    name: name.clone(),
                    type_args: core_args,
                })
            }
        }
    }

    /// Validate algebraic data type definitions against the known type environment to ensure all
    /// referenced field and variant types are resolvable.
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when ADT validation fails
    pub fn validate_adt_type(type_def: &crate::ast::TypeDef) -> Result<(), TypeError> {
        match *type_def {
            crate::ast::TypeDef::Sum { ref variants, .. } => {
                for variant in variants {
                    for field in &variant.fields {
                        Self::ast_type_to_core_type(&field.type_annotation)?;
                    }
                }
            }
            crate::ast::TypeDef::Product { ref fields, .. } => {
                for field in fields {
                    Self::ast_type_to_core_type(&field.type_annotation)?;
                }
            }
            crate::ast::TypeDef::Alias {
                ref target_type, ..
            } => {
                Self::ast_type_to_core_type(target_type)?;
            }
        }
        Ok(())
    }

    /// Type check a pattern match expression
    /// Ensures all patterns and arms are type compatible
    ///
    /// # Errors
    ///
    /// Returns `TypeError` variants when pattern matching validation fails
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
                    found_span: TypeError::unknown_span(),
                    expected_span: None,
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
                        found_span: TypeError::unknown_span(),
                        expected_span: None,
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
            | (&CoreType::Boolean, &CoreType::Boolean)
            | (&CoreType::String, &CoreType::String)
            | (&CoreType::Unit, &CoreType::Unit) => true,

            // Type variables are compatible with themselves
            (&CoreType::Variable(ref var1), &CoreType::Variable(ref var2)) => var1.id == var2.id,

            // Arrays are compatible if their element types are compatible
            (&CoreType::Array(ref left_elem), &CoreType::Array(ref right_elem)) => {
                self.types_compatible(left_elem.as_ref(), right_elem.as_ref())
            }

            // Functions are compatible if parameters and return types are compatible
            (
                &CoreType::Function {
                    parameters: ref left_params,
                    return_type: ref left_ret,
                },
                &CoreType::Function {
                    parameters: ref right_params,
                    return_type: ref right_ret,
                },
            ) => {
                if left_params.len() != right_params.len() {
                    return false;
                }
                for (left_param, right_param) in left_params.iter().zip(right_params.iter()) {
                    if !self.types_compatible(left_param, right_param) {
                        return false;
                    }
                }
                self.types_compatible(left_ret.as_ref(), right_ret.as_ref())
            }

            // Generic types are compatible if names and type arguments match
            (
                &CoreType::Generic {
                    name: ref left_name,
                    type_args: ref left_args,
                },
                &CoreType::Generic {
                    name: ref right_name,
                    type_args: ref right_args,
                },
            ) => {
                if left_name != right_name || left_args.len() != right_args.len() {
                    return false;
                }
                for (left_arg, right_arg) in left_args.iter().zip(right_args.iter()) {
                    if !self.types_compatible(left_arg, right_arg) {
                        return false;
                    }
                }
                true
            }

            // Different types are not compatible
            _ => false,
        }
    }

    /// Execute a closure within a fresh lexical scope, ensuring the scope is
    /// entered and exited even when the closure returns early.
    pub(super) fn within_new_scope<F, R>(&mut self, action: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.symbol_table.enter_scope();
        let result = action(self);
        self.symbol_table.exit_scope();
        result
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
