//! Opalescent core standard library — top-level module wiring.
//!
//! This module exposes the language-level standard library API. It is structured
//! around the following sub-modules:
//!
//! - [`types`] — checked and saturating arithmetic for all Opalescent numeric types
//! - [`math`] — mathematical constants and functions (per `language-spec/requirements/math.md`)
//! - [`strings`] — string operations operating on `&str` and `alloc::string::String`
//! - [`io`] — mockable I/O trait (`print`, `println`, `read_line`)
//! - [`fs`] — file system trait abstraction with in-memory mock for testing
//!
//! # `no_std` compatibility
//!
//! All sub-modules use only `core` and `alloc`; no `std`-exclusive APIs are used.
//! This ensures the stdlib can be linked into embedded or LLVM-generated targets.

#[path = "stdlib/fs.rs"]
pub mod fs;
#[path = "stdlib/io.rs"]
pub mod io;
#[path = "stdlib/math.rs"]
pub mod math;
#[path = "stdlib/strings.rs"]
pub mod strings;
#[path = "stdlib/types.rs"]
pub mod types;

#[cfg(test)]
#[path = "stdlib/tests.rs"]
mod tests;
