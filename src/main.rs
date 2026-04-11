//! Opalescent Programming Language Compiler binary entry point.
//!
//! The executable delegates all runtime workflow logic to the library crate.

/// Launch the Opalescent CLI application through the library crate.
fn main() {
    opalescent::app::run();
}
