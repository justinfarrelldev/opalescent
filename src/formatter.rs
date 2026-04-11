//! Idempotent code formatter for Opalescent `.op` source files.
//!
//! This module provides a full formatting pipeline:
//!
//! 1. **Parsing** — the source is parsed into an AST via the existing
//!    [`crate::parser::Parser`].
//! 2. **Pretty-printing** — the AST is traversed and rendered as
//!    consistently-styled source code by [`printer::Formatter`].
//! 3. **Rules** — textual post-processing rules (indentation normalisation,
//!    trailing whitespace removal, etc.) are applied by [`rules::apply_all`].
//!
//! The formatter is **idempotent**: `format(format(x)) == format(x)` for all
//! valid Opalescent programs.
//!
//! # Quick Start
//!
//! ```ignore
//! use crate::formatter::{Formatter, FormatterConfig};
//!
//! let cfg = FormatterConfig::default();
//! let fmt = Formatter::new(cfg);
//! let output = fmt.format_source("function main(): unit {}")?;
//! ```

/// CLI command wrapper (`opal fmt`).
#[path = "formatter/command.rs"]
pub mod command;
/// Configuration for the formatter.
#[path = "formatter/config.rs"]
pub mod config;
/// Error and result types.
#[path = "formatter/errors.rs"]
pub mod errors;
/// Naming-convention checker.
#[path = "formatter/naming.rs"]
pub mod naming;
/// AST pretty-printer.
#[path = "formatter/printer.rs"]
pub mod printer;
/// Textual formatting rules applied after pretty-printing.
#[path = "formatter/rules.rs"]
pub mod rules;

#[cfg(test)]
#[path = "formatter/tests.rs"]
mod tests;
