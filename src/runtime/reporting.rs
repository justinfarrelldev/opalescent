extern crate alloc;

use crate::runtime::errors::RuntimeError;
use alloc::format;
use alloc::string::String;
use miette::Diagnostic;

/// Render runtime errors in miette-style multi-line output.
#[must_use]
pub fn format_runtime_error(error: &RuntimeError) -> String {
    let code = error.code().map_or_else(
        || String::from("opalescent::runtime::unknown"),
        |value| value.to_string(),
    );
    let help = error.help().map_or_else(
        || String::from("No additional help available."),
        |value| value.to_string(),
    );
    let message = error.to_string();

    format!("error[{code}]\n  x {message}\n  help: {help}")
}
