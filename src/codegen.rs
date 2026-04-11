//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module is used for clippy builds where `mod.rs` module files are denied.

#![expect(
    dead_code,
    reason = "LLVM backend scaffolding is introduced incrementally across upcoming codegen tasks"
)]

#[path = "codegen/context.rs"]
pub mod context;
/// Expression lowering for LLVM backend.
#[path = "codegen/expressions.rs"]
pub mod expressions;
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
