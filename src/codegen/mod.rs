//! LLVM backend scaffolding for Opalescent Phase 5.
//!
//! This module introduces the initial code generation infrastructure used by
//! later tasks to lower typed AST nodes into LLVM IR.

#![expect(
    dead_code,
    reason = "LLVM backend scaffolding is introduced incrementally across upcoming codegen tasks"
)]
#![expect(
    clippy::mod_module_files,
    reason = "Task 21 explicitly requires src/codegen/mod.rs module root structure"
)]

/// LLVM context/module/builder ownership and target setup.
pub mod context;
pub mod expressions;
pub mod functions;
pub mod control_flow;
pub mod statements;
/// Core type to LLVM type conversion utilities.
pub mod types;
/// Value-construction helpers used by future expression lowering.
pub mod values;

#[cfg(test)]
mod tests;
