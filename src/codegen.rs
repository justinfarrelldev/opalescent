//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

#![expect(
    dead_code,
    reason = "LLVM backend scaffolding is introduced incrementally across upcoming codegen tasks"
)]

#[path = "codegen/context.rs"]
pub mod context;
#[doc = "Control-flow code generation support."]
#[path = "codegen/control_flow.rs"]
pub mod control_flow;
/// Expression lowering for LLVM backend.
#[path = "codegen/expressions.rs"]
pub mod expressions;
#[doc = "Function-level code generation support."]
#[path = "codegen/functions.rs"]
pub mod functions;
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
