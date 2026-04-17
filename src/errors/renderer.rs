extern crate alloc;

use crate::errors::reporter::{CompilationErrorReport, CompilerError};
use alloc::format;
use alloc::string::String;
use core::fmt;
use miette::{
    Diagnostic, GraphicalReportHandler, GraphicalTheme, LabeledSpan, NamedSource, Severity,
};

#[derive(Debug)]
/// Runtime wrapper that supplies source text to an existing diagnostic.
struct DiagnosticWithSource<'diagnostic> {
    /// Full source text bound to the current filename.
    source_code: NamedSource<String>,
    /// Original diagnostic emitted by compiler phases.
    inner: &'diagnostic dyn Diagnostic,
}

impl fmt::Display for DiagnosticWithSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::error::Error for DiagnosticWithSource<'_> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        self.inner.source()
    }
}

impl Diagnostic for DiagnosticWithSource<'_> {
    fn code<'diagnostic>(&'diagnostic self) -> Option<Box<dyn fmt::Display + 'diagnostic>> {
        self.inner.code()
    }

    fn severity(&self) -> Option<Severity> {
        self.inner.severity()
    }

    fn help<'diagnostic>(&'diagnostic self) -> Option<Box<dyn fmt::Display + 'diagnostic>> {
        self.inner.help()
    }

    fn url<'diagnostic>(&'diagnostic self) -> Option<Box<dyn fmt::Display + 'diagnostic>> {
        self.inner.url()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        self.inner.labels()
    }

    fn related<'diagnostic>(
        &'diagnostic self,
    ) -> Option<Box<dyn Iterator<Item = &'diagnostic dyn Diagnostic> + 'diagnostic>> {
        self.inner.related()
    }

    fn diagnostic_source(&self) -> Option<&dyn Diagnostic> {
        self.inner.diagnostic_source()
    }
}

#[must_use]
pub fn render_diagnostic(filename: &str, source: &str, error: &dyn Diagnostic) -> String {
    let diagnostic = DiagnosticWithSource {
        source_code: NamedSource::new(filename, source.to_owned()),
        inner: error,
    };
    let handler = GraphicalReportHandler::new_themed(GraphicalTheme::unicode());
    let mut rendered = String::new();

    if handler.render_report(&mut rendered, &diagnostic).is_err() {
        return format!("error: {diagnostic}");
    }

    rendered
}

#[must_use]
pub fn render_report(filename: &str, source: &str, report: &CompilationErrorReport) -> String {
    let mut rendered_entries = Vec::new();

    for &(_phase, ref error) in report.entries() {
        match *error {
            CompilerError::Lexer(ref lex_error) => {
                rendered_entries.push(render_diagnostic(filename, source, lex_error));
            }
            CompilerError::Parser(ref parse_error) => {
                rendered_entries.push(render_diagnostic(filename, source, parse_error));
            }
            CompilerError::TypeChecker(ref type_error) => {
                rendered_entries.push(render_diagnostic(filename, source, type_error));
            }
            CompilerError::Codegen(ref codegen_error) => {
                rendered_entries.push(format!("error: {}", codegen_error.message));
            }
        }
    }

    let error_count = report.len();
    let summary = if error_count == 1 {
        String::from("error: aborting due to 1 previous error")
    } else {
        format!("error: aborting due to {error_count} previous errors")
    };

    if rendered_entries.is_empty() {
        summary
    } else {
        format!("{}\n\n{}", rendered_entries.join("\n\n"), summary)
    }
}

#[cfg(test)]
mod tests {
    use super::{render_diagnostic, render_report};
    use crate::error::LexError;
    use crate::errors::reporter::CompilationErrorReport;
    use crate::parser::errors::ParseError;
    use crate::token::Position;
    use crate::type_system::errors::TypeError;
    use miette::SourceSpan;

    #[test]
    fn test_render_diagnostic_lex_error() {
        let error = LexError::UnexpectedCharacter {
            character: '@',
            position: Position::new(1, 9, 8),
            span: SourceSpan::new(8.into(), 1),
        };

        let output = render_diagnostic("test.op", "let x = @;", &error);

        assert!(output.contains("let x = @;"));
        assert!(output.contains("opalescent::lexer::unexpected_character"));
    }

    #[test]
    fn test_render_diagnostic_unknown_span() {
        let error = TypeError::CannotInferGenericType {
            param_name: String::from("T"),
            span: TypeError::unknown_span(),
        };

        let output = render_diagnostic("test.op", "let x = value", &error);

        assert!(output.contains("Cannot infer generic type parameter 'T'"));
    }

    #[test]
    fn test_render_report_multiple_errors_with_summary() {
        let mut report = CompilationErrorReport::new();
        report.push_lex_error(LexError::UnexpectedCharacter {
            character: '@',
            position: Position::new(1, 9, 8),
            span: SourceSpan::new(8.into(), 1),
        });
        report.push_type_error(TypeError::TypeNotFound {
            type_name: String::from("MissingType"),
            span: SourceSpan::new(0.into(), 3),
        });

        let output = render_report("test.op", "let x = @;", &report);

        assert!(output.contains("error: aborting due to"));
    }

    #[test]
    fn test_renderer_produces_source_context_for_parse_error() {
        let source = "let x = ;";
        let error = ParseError::UnexpectedToken {
            expected: String::from("expression"),
            found: String::from(";"),
            span: SourceSpan::new(8.into(), 1),
        };

        let output = render_diagnostic("test.op", source, &error);

        assert!(output.contains("let x = ;"));
        assert!(output.contains("opalescent::parser::unexpected_token"));
    }

    #[test]
    fn test_renderer_produces_source_context_for_type_error() {
        let source = "let x: int32 = \"hello\"";
        let error = TypeError::TypeMismatch {
            expected: String::from("int32"),
            found: String::from("string"),
            found_span: SourceSpan::new(15.into(), 7),
            expected_span: None,
        };

        let output = render_diagnostic("test.op", source, &error);

        assert!(output.contains("let x: int32 = \"hello\""));
        assert!(output.contains("opalescent::type_system::type_mismatch"));
    }

    #[test]
    fn test_renderer_handles_codegen_error_without_span() {
        let mut report = CompilationErrorReport::new();
        report.push_codegen_error(String::from("invalid alloca placement"));

        let output = render_report("test.op", "let x = 1", &report);

        assert!(output.contains("invalid alloca placement"));
        assert!(output.contains("error: aborting due to 1 previous error"));
    }

    #[test]
    fn test_renderer_includes_suggestions_when_available() {
        let source = "x = 42";
        let error = TypeError::ImmutableAssignment {
            name: String::from("x"),
            assignment_span: SourceSpan::new(0.into(), 1),
            declaration_span: None,
        };

        let output = render_diagnostic("test.op", source, &error);

        assert!(output.contains("let mutable"));
    }
}
