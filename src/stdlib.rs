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
//! - [`collections`] — generic collections: `OpalVec`, `OpalMap`, `OpalSet`, `OpalList`, `OpalIter`
//! - [`system`] — OS interfaces: platform detection, env vars, args, networking, threads, processes
//!
//! # `no_std` compatibility
//!
//! The `types`, `math`, `strings`, `io`, `fs`, and `collections` sub-modules use only
//! `core` and `alloc`.  The `system` module requires `std` for OS-level bindings.

#[path = "stdlib/collections.rs"]
pub mod collections;
#[path = "stdlib/fs.rs"]
pub mod fs;
#[path = "stdlib/io.rs"]
pub mod io;
#[path = "stdlib/math.rs"]
pub mod math;
#[path = "stdlib/strings.rs"]
pub mod strings;
#[path = "stdlib/system.rs"]
pub mod system;
#[path = "stdlib/types.rs"]
pub mod types;

#[cfg(test)]
#[path = "stdlib/tests.rs"]
mod tests;
