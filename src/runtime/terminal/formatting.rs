//! Terminal diagnostic safe-formatting and production-accounting helpers.

extern crate alloc;

use super::diagnostics::{TerminalDiagnostic, TerminalDiagnosticCollection};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
#[cfg(test)]
use core::mem::size_of;

/// Default trusted-diagnostic output bound used by the safe formatting lane.
pub const SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES: usize = 0x0001_0000;

#[derive(Clone, PartialEq, Eq)]
pub struct TrustedTerminalOutput(String);

impl fmt::Debug for TrustedTerminalOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrustedTerminalOutput")
            .field("utf8_bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}

impl TrustedTerminalOutput {
    /// Build trusted terminal output from explicitly reviewed application text.
    pub(crate) fn from_application_text(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct SafeTerminalDiagnosticOutput(String);

impl fmt::Debug for SafeTerminalDiagnosticOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SafeTerminalDiagnosticOutput")
            .field("utf8_bytes", &self.0.len())
            .finish_non_exhaustive()
    }
}

impl SafeTerminalDiagnosticOutput {
    /// Build safe diagnostic output from already-sanitized display text.
    pub(crate) fn new_sanitized(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

/// Format one diagnostic through the bounded escaped diagnostic lane.
#[must_use]
pub fn safe_terminal_diagnostic_format(
    diagnostic: &TerminalDiagnostic,
) -> SafeTerminalDiagnosticOutput {
    let rendered = format!(
        "backend={:?} operation={:?} stage={:?} coordinator={:?} session={:?} retryability={:?} os={:?} truncated={} detail={}",
        diagnostic.backend(),
        diagnostic.operation(),
        diagnostic.stage(),
        diagnostic.coordinator_state(),
        diagnostic.session_state(),
        diagnostic.retryability(),
        diagnostic.os_code(),
        diagnostic.was_truncated(),
        diagnostic.detail().as_str(),
    );
    SafeTerminalDiagnosticOutput::new_sanitized(sanitize_terminal_text(
        rendered.as_str(),
        SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES,
    ))
}

/// Format one collection through the bounded escaped diagnostic lane.
#[must_use]
pub fn safe_terminal_diagnostic_collection_format(
    diagnostics: &TerminalDiagnosticCollection,
) -> SafeTerminalDiagnosticOutput {
    let mut parts = Vec::new();
    for diagnostic in diagnostics.retained() {
        parts.push(
            safe_terminal_diagnostic_format(diagnostic)
                .as_str()
                .to_owned(),
        );
    }
    parts.push(format!(
        "retained_count={} omitted_count={} retained_bytes={} omitted_bytes={} collection_truncated={}",
        diagnostics.retained_count(),
        diagnostics.omitted_count(),
        diagnostics.retained_bytes(),
        diagnostics.omitted_bytes(),
        diagnostics.was_truncated(),
    ));
    SafeTerminalDiagnosticOutput::new_sanitized(sanitize_terminal_text(
        parts.join("\n").as_str(),
        SAFE_TERMINAL_DIAGNOSTIC_OUTPUT_MAX_BYTES,
    ))
}

/// Escape unsafe terminal text and enforce the bounded diagnostic output lane.
fn sanitize_terminal_text(input: &str, byte_limit: usize) -> String {
    const MARKER: &str = "...[truncated]";
    let mut output = String::new();
    for character in input.chars() {
        let replacement = escaped_terminal_character(character);
        if output.len().saturating_add(replacement.len()) > byte_limit {
            append_visible_truncation_marker(&mut output, byte_limit, MARKER);
            return output;
        }
        output.push_str(replacement.as_str());
    }
    output
}

/// Append a visible truncation marker while honoring the byte bound.
fn append_visible_truncation_marker(output: &mut String, byte_limit: usize, marker: &str) {
    while output.len().saturating_add(marker.len()) > byte_limit && !output.is_empty() {
        output.pop();
    }
    if output.len().saturating_add(marker.len()) <= byte_limit {
        output.push_str(marker);
    }
}

/// Convert one potentially unsafe terminal scalar into a visible escaped fragment.
fn escaped_terminal_character(character: char) -> String {
    if should_escape_terminal_character(character) {
        return format!("\\u{{{:X}}}", u32::from(character));
    }
    character.to_string()
}

/// Return whether a scalar must be escaped for safe diagnostic terminal display.
fn should_escape_terminal_character(character: char) -> bool {
    let codepoint = u32::from(character);
    matches!(codepoint, 0x00..=0x1F | 0x7F..=0x9F)
        || matches!(codepoint, 0x200E | 0x200F | 0x202A..=0x202E | 0x2066..=0x2069)
        || matches!(codepoint, 0xFDD0..=0xFDEF)
        || matches!(codepoint & 0xFFFF, 0xFFFE | 0xFFFF)
}

#[cfg(test)]
pub(crate) fn collection_metadata_bytes_for_tests() -> u64 {
    let total = size_of::<TerminalDiagnosticCollection>()
        .checked_add(64_usize)
        .expect("collection accounting constant should fit usize");
    u64::try_from(total).expect("usize fits u64")
}

/// Return production-accounting bytes reserved for collection metadata.
pub(crate) fn collection_metadata_bytes() -> u64 {
    #[cfg(test)]
    {
        collection_metadata_bytes_for_tests()
    }
    #[cfg(not(test))]
    {
        let total = size_of::<TerminalDiagnosticCollection>()
            .checked_add(64_usize)
            .expect("collection accounting constant should fit usize");
        u64::try_from(total).expect("usize fits u64")
    }
}

#[cfg(test)]
pub(crate) fn diagnostic_accounted_bytes_for_tests(detail: &str) -> u64 {
    let total = size_of::<TerminalDiagnostic>()
        .checked_add(32_usize)
        .expect("diagnostic accounting constant should fit usize");
    let base = u64::try_from(total).expect("usize fits u64");
    let detail_bytes = u64::try_from(detail.len()).expect("usize fits u64");
    base.saturating_add(detail_bytes)
}

/// Return production-accounting bytes for one complete diagnostic.
pub(crate) fn diagnostic_accounted_bytes(detail: &str) -> u64 {
    #[cfg(test)]
    {
        diagnostic_accounted_bytes_for_tests(detail)
    }
    #[cfg(not(test))]
    {
        let total = size_of::<TerminalDiagnostic>()
            .checked_add(32_usize)
            .expect("diagnostic accounting constant should fit usize");
        let base = u64::try_from(total).expect("usize fits u64");
        let detail_bytes = u64::try_from(detail.len()).expect("usize fits u64");
        base.saturating_add(detail_bytes)
    }
}
