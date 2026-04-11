//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

#[doc = "ADT constructor, field-access, and match lowering support."]
#[path = "codegen/adts.rs"]
pub mod adts;
#[path = "codegen/context.rs"]
pub mod context;
#[doc = "Control-flow code generation support."]
#[path = "codegen/control_flow.rs"]
pub mod control_flow;
/// Expression lowering for LLVM backend.
#[path = "codegen/expressions.rs"]
pub mod expressions;
#[path = "codegen/expressions_string.rs"]
pub mod expressions_string;
#[doc = "Function-level code generation support."]
#[path = "codegen/functions.rs"]
pub mod functions;
#[doc = "Generic monomorphization naming and specialization cache wiring."]
#[path = "codegen/monomorphization.rs"]
pub mod monomorphization;
#[doc = "LLVM optimization pass pipeline configuration."]
#[path = "codegen/optimization.rs"]
pub mod optimization;
/// Statement lowering for LLVM backend.
#[path = "codegen/statements.rs"]
pub mod statements;
#[path = "codegen/types.rs"]
pub mod types;
#[path = "codegen/values.rs"]
pub mod values;

#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
#[cfg(test)]
#[path = "codegen/tests_optimization.rs"]
mod tests_optimization;
