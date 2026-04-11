//! `opal fmt` command implementation.
//!
//! This module provides [`FormatCommand`] — the entry point used by the
//! `opal fmt` CLI subcommand to format Opalescent source files.

extern crate alloc;

use alloc::string::String;

use crate::formatter::config::FormatterConfig;
use crate::formatter::errors::FormatterResult;
use crate::formatter::printer::Formatter;

/// The `opal fmt` command.
///
/// # Examples
///
/// ```ignore
/// let cmd = FormatCommand::new("function main(): unit {}".to_owned(), false);
/// let output = cmd.execute()?;
/// ```
pub struct FormatCommand {
    /// The Opalescent source code to format.
    pub source: String,
    /// When `true`, the formatted output should be written back in-place.
    ///
    /// In the CLI this controls whether the file is overwritten; in tests this
    /// field is consulted but no actual file I/O is performed.
    pub in_place: bool,
}

impl FormatCommand {
    /// Construct a new [`FormatCommand`].
    #[must_use]
    pub const fn new(source: String, in_place: bool) -> Self {
        Self { source, in_place }
    }

    /// Execute the format command using default configuration.
    ///
    /// Returns the formatted source code as a [`String`].
    ///
    /// # Errors
    ///
    /// Returns a [`crate::formatter::errors::FormatterError`] if the source
    /// fails to parse.
    pub fn execute(&self) -> FormatterResult<String> {
        self.execute_with_config(FormatterConfig::default())
    }

    /// Execute the format command using the given [`FormatterConfig`].
    ///
    /// Returns the formatted source code as a [`String`].
    ///
    /// # Errors
    ///
    /// Returns a [`crate::formatter::errors::FormatterError`] if the source
    /// fails to parse.
    pub fn execute_with_config(&self, config: FormatterConfig) -> FormatterResult<String> {
        let formatter = Formatter::new(config);
        formatter.format_source(&self.source)
    }
}
