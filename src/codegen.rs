//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

#[doc = "ADT constructor, field-access, and match lowering support."]
pub mod adts;
pub mod binding_store;
pub mod context;
pub mod control_flow;
pub mod error;
pub mod error_abi;
pub mod expressions;
pub mod expressions_array;
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
#[doc = "LLVM optimization pass pipeline configuration."]
pub mod optimization;
#[doc = "RC runtime call emission for reference counting operations."]
pub mod rc_emitter;
pub mod scope_tracker;
/// Statement lowering for LLVM backend.
pub mod statements;
pub mod types;
pub mod values;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_optimization;
