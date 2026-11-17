//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

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
/// Typed native assertion operations and source-aware failure sites.
pub(crate) mod native_assertions;
/// Explicit test-build function declarations and ordinary result emission.
pub mod native_functions;
/// Independent internal result channel for compiled native test artifacts.
pub mod native_outcome;
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
mod native_cleanup_call_tests;
#[cfg(test)]
mod native_function_tests;
#[cfg(test)]
mod native_guard_call_tests;
#[cfg(test)]
mod native_outcome_call_tests;
#[cfg(test)]
mod native_outcome_tests;
#[cfg(test)]
mod test_affine_aggregates_codegen;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_optimization;
