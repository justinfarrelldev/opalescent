//! Compatibility re-export module for `src/formatter/`.
//!
//! This file exists so that `src/formatter/` is recognised by the module
//! system as a sub-directory module.  All public API items are re-exported
//! from the parent `src/formatter.rs` module via `crate::formatter`.

pub use crate::formatter::*;
