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
#[path = "benchmarks.rs"]
pub mod benchmarks;
#[path = "build_system.rs"]
pub mod build_system;
#[path = "codegen.rs"]
pub mod codegen;
pub mod compiler;
#[path = "doc_gen.rs"]
pub mod doc_gen;
pub mod error;
/// Compiler-wide error reporting infrastructure modules.
#[path = "errors.rs"]
pub mod errors;
#[path = "formatter.rs"]
pub mod formatter;
#[path = "hot_reload.rs"]
pub mod hot_reload;
pub mod lexer;
#[path = "lsp.rs"]
pub mod lsp;
#[path = "package_manager.rs"]
pub mod package_manager;
pub mod parser;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "stdlib.rs"]
pub mod stdlib;
#[path = "testing.rs"]
pub mod testing;
pub mod token;
pub mod type_system;
