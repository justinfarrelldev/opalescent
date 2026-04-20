//! Library crate root for the Opalescent compiler toolchain.
//!
//! This crate exposes compiler pipeline modules so integration tests and
//! external binaries can orchestrate lexing, parsing, type checking, and
//! code generation through stable module paths.

#![expect(
    clippy::must_use_candidate,
    reason = "Legacy modules predate strict must_use adoption and are migrated incrementally"
)]
#![expect(
    clippy::missing_errors_doc,
    reason = "Legacy public APIs are being documented for errors in staged refactors"
)]
#![expect(
    clippy::return_self_not_must_use,
    reason = "Some fluent or constructor-style APIs are kept as-is for compatibility"
)]
#![expect(
    clippy::ref_patterns,
    reason = "Existing modules use ref-pattern matching style intentionally and are being migrated incrementally"
)]

pub mod app;
pub mod ast;
pub mod benchmarks;
pub mod build_system;
pub mod codegen;
pub mod compiler;
pub mod doc_gen;
pub mod error;
/// Compiler-wide error reporting infrastructure modules.
pub mod errors;
pub mod formatter;
pub mod hot_reload;
pub mod lexer;
pub mod lsp;
pub mod module_loader;
pub mod package_manager;
pub mod parser;
pub mod runtime;
pub mod stdlib;
pub mod testing;
pub mod token;
pub mod type_system;
