//! Formatter configuration for the Opalescent code formatter.
//!
//! This module provides the [`FormatterConfig`] struct which controls all
//! formatting behaviour. Configuration can be loaded from TOML strings
//! (e.g. from an `.opalfmt.toml` config file) or constructed directly via
//! `Default`.

extern crate alloc;

use alloc::string::String;

use crate::formatter::errors::{FormatterError, FormatterResult};

/// Configuration for the Opalescent code formatter.
///
/// All fields have sane defaults (see [`FormatterConfig::default`]).
///
/// # Examples
///
/// ```ignore
/// let cfg = FormatterConfig::default();
/// assert_eq!(cfg.indent_size, 4);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatterConfig {
    /// Number of spaces per indentation level.
    ///
    /// Ignored when [`FormatterConfig::use_tabs`] is `true`.
    pub indent_size: usize,

    /// Soft maximum line width used for line-length guidance.
    ///
    /// The formatter uses this as a hint for determining when to introduce
    /// line breaks; it does not hard-break every line that exceeds this limit.
    pub max_line_width: usize,

    /// When `true`, use a tab character instead of spaces for indentation.
    pub use_tabs: bool,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_width: 100,
            use_tabs: false,
        }
    }
}

impl FormatterConfig {
    /// Construct a new [`FormatterConfig`] with explicitly provided values.
    #[must_use]
    pub const fn new(indent_size: usize, max_line_width: usize, use_tabs: bool) -> Self {
        Self {
            indent_size,
            max_line_width,
            use_tabs,
        }
    }

    /// Return the indentation string for a single indent level.
    ///
    /// Returns `"\t"` when [`FormatterConfig::use_tabs`] is `true`, otherwise
    /// returns a string of [`FormatterConfig::indent_size`] space characters.
    #[must_use]
    pub fn indent_unit(&self) -> String {
        if self.use_tabs {
            String::from("\t")
        } else {
            " ".repeat(self.indent_size)
        }
    }

    /// Parse a [`FormatterConfig`] from a TOML-formatted string.
    ///
    /// Only the keys `indent_size`, `max_line_width`, and `use_tabs` are
    /// recognised. Unknown keys are silently ignored. Malformed TOML, or
    /// values that cannot be parsed into the expected types, produce
    /// [`FormatterError::InvalidConfig`].
    ///
    /// # Errors
    ///
    /// Returns [`FormatterError::InvalidConfig`] when the TOML is malformed or
    /// a recognised key contains an invalid value.
    pub fn from_toml_str(s: &str) -> FormatterResult<Self> {
        let mut cfg = Self::default();

        for raw_line in s.lines() {
            let line = raw_line.trim();
            // Skip comments and blank lines.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Skip TOML table headers (e.g. `[formatter]`).
            if line.starts_with('[') {
                continue;
            }
            let Some(eq_pos) = line.find('=') else {
                continue;
            };
            let key = line.get(..eq_pos).map_or("", str::trim);
            let after_eq = line.get(eq_pos.saturating_add(1)..).unwrap_or("");
            let value = after_eq.trim().trim_matches('"');
            match key {
                "indent_size" => {
                    cfg.indent_size = value.parse::<usize>().map_err(|_e| {
                        FormatterError::InvalidConfig(format!("invalid indent_size: {value:?}"))
                    })?;
                }
                "max_line_width" => {
                    cfg.max_line_width = value.parse::<usize>().map_err(|_e| {
                        FormatterError::InvalidConfig(format!("invalid max_line_width: {value:?}"))
                    })?;
                }
                "use_tabs" => match value {
                    "true" => cfg.use_tabs = true,
                    "false" => cfg.use_tabs = false,
                    other => {
                        return Err(FormatterError::InvalidConfig(format!(
                            "invalid use_tabs value: {other:?}"
                        )));
                    }
                },
                _ => {
                    // Unknown keys are silently ignored for forward-compatibility.
                }
            }
        }

        if cfg.indent_size == 0 {
            return Err(FormatterError::InvalidConfig(
                "indent_size must be at least 1".to_owned(),
            ));
        }

        Ok(cfg)
    }
}
