//! Machine-readable diagnostics for editor integrations.

extern crate alloc;

use crate::errors::formatter::CompilerPhase;
use crate::errors::reporter::CompilerError;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use miette::Diagnostic;
use serde::Serialize;

/// Full diagnostic payload emitted for editor integrations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorDiagnosticReport {
    /// Whether the checked input had no error-severity diagnostics.
    pub success: bool,
    /// Compiler diagnostics and warnings mapped to source ranges.
    pub diagnostics: Vec<EditorDiagnostic>,
}

/// One editor diagnostic with exact zero-based source coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EditorDiagnostic {
    /// Source file path that owns this diagnostic.
    pub source_path: String,
    /// Diagnostic severity.
    pub severity: EditorDiagnosticSeverity,
    /// Compiler phase that emitted the diagnostic.
    pub phase: String,
    /// Stable diagnostic code when available.
    pub code: Option<String>,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Optional help text.
    pub help: Option<String>,
    /// Zero-based diagnostic range.
    pub range: EditorRange,
}

/// Editor-facing severity classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EditorDiagnosticSeverity {
    /// Compile-blocking error.
    Error,
    /// Non-fatal warning.
    Warning,
    /// Informational diagnostic.
    Information,
    /// Hint diagnostic.
    Hint,
}

/// A half-open zero-based source range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EditorRange {
    /// Inclusive start position.
    pub start: EditorPosition,
    /// Exclusive end position.
    pub end: EditorPosition,
}

/// A zero-based source position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EditorPosition {
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based character offset.
    pub character: usize,
}

/// Convert a compiler error report to editor diagnostics.
#[must_use]
pub fn report_to_editor_diagnostics(
    source_path: &str,
    source: &str,
    report: &crate::errors::reporter::CompilationErrorReport,
) -> Vec<EditorDiagnostic> {
    let mut diagnostics = Vec::new();
    for &(phase, ref compiler_error) in report.entries() {
        diagnostics.push(compiler_error_to_editor_diagnostic(
            source_path,
            source,
            phase,
            compiler_error,
        ));
    }
    diagnostics
}

/// Convert a type-checking warning to an editor diagnostic.
#[must_use]
pub fn warning_to_editor_diagnostic(
    source_path: &str,
    source: &str,
    warning: &crate::type_system::errors::Warning,
) -> EditorDiagnostic {
    diagnostic_to_editor_diagnostic(
        source_path,
        source,
        EditorDiagnosticSeverity::Warning,
        CompilerPhase::TypeChecker.display_name(),
        warning,
    )
}

/// Convert one compiler error into an editor diagnostic.
fn compiler_error_to_editor_diagnostic(
    source_path: &str,
    source: &str,
    phase: CompilerPhase,
    compiler_error: &CompilerError,
) -> EditorDiagnostic {
    match *compiler_error {
        CompilerError::Lexer(ref lex_error) => diagnostic_to_editor_diagnostic(
            source_path,
            source,
            EditorDiagnosticSeverity::Error,
            phase.display_name(),
            lex_error,
        ),
        CompilerError::Parser(ref parse_error) => diagnostic_to_editor_diagnostic(
            source_path,
            source,
            EditorDiagnosticSeverity::Error,
            phase.display_name(),
            parse_error,
        ),
        CompilerError::TypeChecker(ref type_error) => diagnostic_to_editor_diagnostic(
            source_path,
            source,
            EditorDiagnosticSeverity::Error,
            phase.display_name(),
            type_error,
        ),
        CompilerError::Codegen(ref codegen_error) => diagnostic_to_editor_diagnostic(
            source_path,
            source,
            EditorDiagnosticSeverity::Error,
            phase.display_name(),
            codegen_error,
        ),
    }
}

/// Convert any miette diagnostic into the editor data model.
fn diagnostic_to_editor_diagnostic<DiagnosticType>(
    source_path: &str,
    source: &str,
    severity: EditorDiagnosticSeverity,
    phase: &str,
    diagnostic: &DiagnosticType,
) -> EditorDiagnostic
where
    DiagnosticType: Diagnostic + fmt::Display + ?Sized,
{
    EditorDiagnostic {
        source_path: source_path.to_owned(),
        severity,
        phase: phase.to_owned(),
        code: diagnostic.code().map(|code| code.to_string()),
        message: diagnostic.to_string(),
        help: diagnostic.help().map(|help| help.to_string()),
        range: diagnostic_range(source, diagnostic),
    }
}

/// Extract the first labeled source span from a diagnostic.
fn diagnostic_range<DiagnosticType>(source: &str, diagnostic: &DiagnosticType) -> EditorRange
where
    DiagnosticType: Diagnostic + ?Sized,
{
    if let Some(labels) = diagnostic.labels() {
        let collected: Vec<miette::LabeledSpan> = labels.collect();
        if let Some(first_label) = collected.first() {
            return byte_span_to_range(source, first_label.offset(), first_label.len());
        }
    }

    byte_span_to_range(source, 0, 0)
}

/// Convert a byte span into a zero-based editor range.
fn byte_span_to_range(source: &str, start_offset: usize, length: usize) -> EditorRange {
    let safe_start = start_offset.min(source.len());
    let safe_end = safe_start.saturating_add(length).min(source.len());

    EditorRange {
        start: byte_offset_to_position(source, safe_start),
        end: byte_offset_to_position(source, safe_end),
    }
}

/// Convert one byte offset into a zero-based line/character position.
fn byte_offset_to_position(source: &str, offset: usize) -> EditorPosition {
    let mut line = 0_usize;
    let mut character = 0_usize;

    for (byte_index, ch) in source.char_indices() {
        if byte_index >= offset {
            return EditorPosition { line, character };
        }

        if ch == '\n' {
            line = line.saturating_add(1_usize);
            character = 0;
        } else {
            character = character.saturating_add(1_usize);
        }
    }

    EditorPosition { line, character }
}

/// Serialize editor diagnostics to pretty JSON.
///
/// # Errors
/// Returns a serialization error if the report cannot be encoded.
pub fn to_json(report: &EditorDiagnosticReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

#[cfg(test)]
mod tests {
    use super::{
        EditorDiagnosticReport, EditorDiagnosticSeverity, EditorPosition,
        report_to_editor_diagnostics, to_json, warning_to_editor_diagnostic,
    };
    use crate::errors::reporter::CompilationErrorReport;
    use crate::type_system::errors::{TypeError, Warning};
    use miette::SourceSpan;

    #[test]
    fn type_error_report_maps_exact_source_range() {
        let source = "entry main = f(): void =>\n    let value: int32 = 'text'\n";
        let found_offset = source
            .find("'text'")
            .expect("fixture should contain literal");
        let expected_offset = source.find("int32").expect("fixture should contain type");
        let mut report = CompilationErrorReport::new();
        report.push_type_error(TypeError::TypeMismatch {
            expected: String::from("int32"),
            found: String::from("string"),
            expected_span: Some(SourceSpan::new(expected_offset.into(), 5)),
            found_span: SourceSpan::new(found_offset.into(), 6),
        });

        let diagnostics = report_to_editor_diagnostics("main.op", source, &report);

        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.source_path, "main.op");
        assert_eq!(diagnostic.severity, EditorDiagnosticSeverity::Error);
        assert_eq!(diagnostic.phase, "type checker");
        assert!(
            diagnostic
                .code
                .as_deref()
                .is_some_and(|code| code.contains("type_mismatch")),
            "expected stable diagnostic code, got {:?}",
            diagnostic.code
        );
        assert!(diagnostic.message.contains("Type mismatch"));
        assert_eq!(
            diagnostic.range.start,
            EditorPosition {
                line: 1,
                character: 23,
            }
        );
        assert_eq!(
            diagnostic.range.end,
            EditorPosition {
                line: 1,
                character: 29,
            }
        );
    }

    #[test]
    fn warning_maps_to_warning_severity_and_help() {
        let source =
            "entry main = f(): void errors HexDecodeError, SliceRangeError =>\n    return void\n";
        let warning = Warning::ReplaceableErrorList {
            family_name: String::from("BytesErrors"),
            replaceable_errors: String::from("HexDecodeError, SliceRangeError"),
            span: SourceSpan::new(0.into(), 64),
            suppression_annotation: None,
        };

        let diagnostic = warning_to_editor_diagnostic("main.op", source, &warning);

        assert_eq!(diagnostic.severity, EditorDiagnosticSeverity::Warning);
        assert_eq!(diagnostic.phase, "type checker");
        assert!(diagnostic.message.contains("BytesErrors"));
        assert!(
            diagnostic
                .help
                .as_deref()
                .is_some_and(|help| help.contains("errors BytesErrors")),
            "expected warning help, got {:?}",
            diagnostic.help
        );
        assert_eq!(diagnostic.range.start.line, 0);
        assert_eq!(diagnostic.range.end.line, 0);
    }

    #[test]
    fn editor_report_serializes_severity_as_lowercase_json() {
        let report = EditorDiagnosticReport {
            success: true,
            diagnostics: Vec::new(),
        };

        let json = to_json(&report).expect("report should serialize");

        assert!(json.contains("\"success\": true"));
        assert!(json.contains("\"diagnostics\": []"));
    }
}
