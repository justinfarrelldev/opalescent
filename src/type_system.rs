//! Type System Core for Opalescent Language
//!
//! This module provides the core type checking, type inference, and type safety
//! validation for the Opalescent programming language. It ensures static type safety
//! while providing helpful error messages and supporting advanced features like
//! generics and algebraic data types.
//!
//! ## Phase Integration
//!
//! This module is used by:
//! - **Phase 1**: Foundation for parser type annotations and AST type validation
//! - **Phase 2**: Function and variable type checking, type inference for lambdas and let bindings
//! - **Phase 3**: ADT validation, pattern matching, and generic type instantiation
//! - **Phase 4**: Cross-module type checking and import validation
//! - **Phase 5**: Type information for LLVM code generation
//! - **Phase 6**: ABI signature generation for hot reload compatibility checking
//!
//! ## Current Status & Future Enhancements
//!
//! ### Error Categories

//!
//! - [`TypeError::TypeNotFound`]: Type reference not in scope
//! - [`TypeError::TypeMismatch`]: Incompatible types in expression
//! - [`TypeError::InvalidOperation`]: Operation not supported for type
//! - [`TypeError::UnificationFailed`]: Type inference failure
//! - [`TypeError::OccursCheckFailed`]: Infinite type detected
//! - [`TypeError::ConstraintSolvingFailed`]: Constraint system failure
//!
//! ## Ownership Strategy
//!
//! - `lookup_type`: Returns reference (type environment owns the type)
//! - `ast_type_to_core_type`: Returns owned value (creates new `CoreType`)
//! - `unify`: Returns owned `Substitution` (creates new mapping)
//! - `fresh_type_var`: Returns owned `CoreType::Variable` (creates new type variable)
//!
//! ## Examples
//!
//! ### Basic Type Checking
//!
//! ```rust,ignore
//! use opalescent::type_system::{TypeChecker, CoreType};
//!
//! let checker = TypeChecker::new();
//! assert!(checker.environment().has_type("int32"));
//! assert!(checker.types_compatible(&CoreType::Int32, &CoreType::Int32));
//! ```
//!
//! ### Type Unification
//!
//! ```rust,ignore
//! use opalescent::token::{Position, Span};
//!
//! let mut checker = TypeChecker::new();
//! let span = Span::single(Position::start());
//! let var = checker.fresh_type_var("x".to_owned(), span)?;
//! let subst = checker.unify(&var, &CoreType::Int32, None, None)?;
//! ```
//!
//! ## Testing
//!
//! The module includes comprehensive unit tests covering:
//! - Type environment operations
//! - AST to `CoreType` conversion
//! - Type unification algorithm
//! - Occurs check validation
//! - Error message formatting
//! - ADT type validation
//! - Pattern matching type checking

#![expect(
    dead_code,
    reason = "Type system is foundational infrastructure being built incrementally"
)]

// Module declarations - order matters for dependencies
/// Arithmetic typing metadata and constant-folding helpers.
mod arithmetic;
mod checker;
mod constraints;
mod environment;
mod errors;
mod memory;
mod substitution;
mod symbol_table;
mod types;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_integration;
