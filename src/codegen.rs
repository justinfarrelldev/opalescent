//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

mod adt_metadata;
#[doc = "ADT constructor, field-access, and match lowering support."]
pub mod adts;
#[doc = "Transactional affine aggregate rollback and seal helpers."]
pub(crate) mod affine_aggregates;
pub mod binding_store;
pub mod context;
pub mod control_flow;
#[doc = "Return-statement helper routines for LLVM control-flow lowering."]
pub mod control_flow_return;
pub mod error;
pub mod error_abi;
pub mod expressions;
pub mod expressions_array;
#[doc = "String-aware array ownership helper routines for LLVM lowering."]
pub mod expressions_array_string;
pub mod expressions_cast;
pub mod expressions_numeric;
pub mod expressions_string;
#[doc = "Function-level code generation support."]
#[expect(
    clippy::pub_use,
    reason = "Re-exporting from functions_call for backward compatibility"
)]
pub mod functions;
#[doc = "Function call, propagate, and guard expression lowering."]
pub mod functions_call;
#[doc = "Standard library function declarations."]
pub mod functions_stdlib;
#[doc = "Generic monomorphization naming and specialization cache wiring."]
pub mod monomorphization;
/// Existing lexical scope exits shared by native abort sites.
pub(crate) mod native_abort;
mod native_assertion_storage;
/// Typed native assertion operations and source-aware failure sites.
pub(crate) mod native_assertions;
mod native_callback_boundary;
/// Typed native callback boundaries for the lifecycle runtime.
pub mod native_callbacks;
/// Generated runner-owned context layouts and callback metadata views.
pub mod native_context;
/// Checked native source function definitions and reference parameter bindings.
pub mod native_declarations;
/// Immutable target-aware lifecycle case descriptor emission.
pub mod native_descriptors;
pub(crate) mod native_double_configuration;
pub(crate) mod native_double_edges;
pub(crate) mod native_double_history;
pub(crate) mod native_double_owned;
pub(crate) mod native_double_replacements;
pub(crate) mod native_double_rules;
pub(crate) mod native_doubles;
/// Explicit test-build function declarations and ordinary result emission.
pub mod native_functions;
/// Independent internal result channel for compiled native test artifacts.
pub mod native_outcome;
/// Synchronous monotonic assertion polling in native artifacts.
pub mod native_polling;
/// Fully checked source graph to reusable native case descriptor artifact.
pub mod native_project;
pub(crate) mod native_standard_adapters;
/// Typed setup/example construction and ordinary value-drop boundaries.
pub mod native_value_callbacks;
#[doc = "LLVM optimization pass pipeline configuration."]
pub mod optimization;
#[doc = "RC runtime call emission for reference counting operations."]
pub mod rc_emitter;
pub mod scope_tracker;
/// Statement lowering for LLVM backend.
pub mod statements;
#[doc = "Compiler-owned string ownership helper routines for LLVM lowering."]
pub mod string_ownership;
pub mod types;
pub mod values;

#[cfg(test)]
mod native_assertion_tests;
#[cfg(test)]
mod native_callback_tests;
#[cfg(test)]
mod native_cleanup_call_tests;
#[cfg(test)]
mod native_declaration_tests;
#[cfg(test)]
mod native_descriptor_tests;
#[cfg(test)]
mod native_double_rule_tests;
#[cfg(test)]
mod native_function_tests;
#[cfg(test)]
mod native_guard_call_tests;
#[cfg(test)]
mod native_outcome_call_tests;
#[cfg(test)]
mod native_outcome_tests;
#[cfg(test)]
mod native_project_tests;
#[cfg(test)]
mod native_value_callback_tests;
#[cfg(test)]
mod native_world_callback_tests;
#[cfg(test)]
mod test_affine_aggregates_codegen;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_optimization;
