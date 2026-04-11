/// Opalescent core standard library — compatibility re-export module.
///
/// This file exists to satisfy Rust's module resolution when the parent file
/// (`src/stdlib.rs`) uses `#[path = "..."]` attributes to wire the submodules.
/// It mirrors the pattern established by `src/runtime/mod.rs` and similar modules.
///
/// All public API is re-exported from `src/stdlib.rs` directly; this file
/// provides the compatibility surface for `use crate::stdlib::...` import paths
/// inside the stdlib submodules themselves.

#[expect(
    clippy::pub_use,
    reason = "stdlib/mod.rs re-exports the public API surface for use crate::stdlib::... imports"
)]
pub use crate::stdlib::{collections, fs, io, math, strings, system, types};
