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
/// Transactional affine aggregate metadata shared by checker and codegen.
pub(crate) mod affine_aggregates;
/// Arithmetic typing metadata and constant-folding helpers.
mod arithmetic;
pub mod checker;
mod constraints;
mod environment;
mod error_families;
pub mod errors;
pub mod fallible_constructors;
pub mod heap_class;
mod memory;
/// Import/export resolver and module dependency graph implementation.
mod module_resolver;
pub mod propertyless_constructors;
pub(crate) use module_resolver::{AdtLayoutManifestKind, ModuleInterface};
mod substitution;
mod symbol_table;
/// Selected terminal public API prerequisite inventory and diagnostics.
pub(crate) mod terminal_public_api_prerequisites;
pub mod type_mapping;
pub mod types;

/// Return whether a module path belongs to the selected terminal proposal inventory.
#[must_use]
pub(crate) fn is_terminal_proposal_module_path(module_path: &str) -> bool {
    module_resolver::is_terminal_proposal_module_path(module_path)
}

/// Return whether a module/symbol pair belongs to the gated terminal proposal surface.
#[must_use]
pub(crate) fn is_terminal_proposal_codegen_gated_import(
    module_path: &str,
    symbol_name: &str,
) -> bool {
    module_resolver::is_terminal_proposal_codegen_gated_import(module_path, symbol_name)
}

/// Return whether a module/symbol pair belongs to the implemented Task 16 error surface.
#[must_use]
pub(crate) fn is_terminal_proposal_implemented_error_inspector_import(
    module_path: &str,
    symbol_name: &str,
) -> bool {
    module_resolver::is_terminal_proposal_implemented_error_inspector_import(
        module_path,
        symbol_name,
    )
}

/// Return whether a runtime symbol name belongs to any gated terminal proposal surface.
#[must_use]
pub(crate) fn is_terminal_proposal_codegen_gated_runtime_name(symbol_name: &str) -> bool {
    module_resolver::is_terminal_proposal_codegen_gated_runtime_name(symbol_name)
}

/// Resolve one stdlib terminal proposal function signature from the authoritative resolver.
#[must_use]
pub(crate) fn terminal_proposal_function_signature(
    module_path: &str,
    symbol_name: &str,
) -> Option<types::CoreType> {
    let resolver = module_resolver::ModuleResolver::new();
    let span = crate::token::Span::single(crate::token::Position::start());
    resolver
        .resolve_symbol(module_path, symbol_name, span)
        .ok()
        .map(|symbol| symbol.core_type)
}

/// Return whether a terminal proposal type name uses the selected product constructor surface.
#[must_use]
pub(crate) fn is_terminal_proposal_constructible_product_type(type_name: &str) -> bool {
    module_resolver::is_terminal_proposal_constructible_product_type(type_name)
}

/// Resolve an authoritative terminal proposal sum-variant ABI tag when one exists.
#[must_use]
pub(crate) fn terminal_proposal_variant_id(type_name: &str, variant_name: &str) -> Option<i64> {
    module_resolver::terminal_proposal_variant_id(type_name, variant_name)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod test_integration;

#[cfg(test)]
mod test_integration_adt;

#[cfg(test)]
mod test_integration_affine;

#[cfg(test)]
mod test_integration_collections;

#[cfg(test)]
mod test_integration_generics;

#[cfg(test)]
mod test_integration_modules;

#[cfg(test)]
mod test_integration_module_validation;

#[cfg(test)]
mod test_integration_ecosystem;
