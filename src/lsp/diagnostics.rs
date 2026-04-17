//! Diagnostic extraction helpers for Opalescent LSP.

extern crate alloc;

use crate::errors::reporter::CompilationErrorReport;
use crate::lexer::Lexer;
use crate::lsp::protocol::{Diagnostic, DiagnosticSeverity, Position, Range};
use crate::parser::Parser;
use crate::type_system::checker::TypeChecker;
use alloc::vec::Vec;

/// Analyze `source` and return parser/type diagnostics mapped to LSP structures.
#[must_use]
pub fn get_diagnostics(source: &str) -> Vec<Diagnostic> {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();

    let mut report = CompilationErrorReport::new();
    report.extend_lex_errors(lex_errors.errors);

    let parser = Parser::new(tokens);
    let (program, parse_errors) = parser.parse();
    report.extend_parse_errors(parse_errors.errors);

    if let Some(parsed_program) = program {
        let mut type_checker = TypeChecker::new();
        if let Err(type_errors) = type_checker.type_check_program(&parsed_program) {
            report.extend_type_errors(type_errors);
        }
    }

    let mut diagnostics = Vec::new();
    for &(_phase, ref compiler_error) in report.entries() {
        diagnostics.push(map_compiler_error_to_diagnostic(source, compiler_error));
    }

    diagnostics
}

/// Convert byte-offset `SourceSpan` data from compiler errors into LSP diagnostics.
fn map_compiler_error_to_diagnostic(
    source: &str,
    compiler_error: &crate::errors::reporter::CompilerError,
) -> Diagnostic {
    match *compiler_error {
        crate::errors::reporter::CompilerError::Lexer(ref lex_error) => Diagnostic {
            range: source_span_to_range(source, lex_error),
            severity: DiagnosticSeverity::Error,
            message: format!("{lex_error}"),
        },
        crate::errors::reporter::CompilerError::Parser(ref parse_error) => Diagnostic {
            range: source_span_to_range(source, parse_error),
            severity: DiagnosticSeverity::Error,
            message: format!("{parse_error}"),
        },
        crate::errors::reporter::CompilerError::TypeChecker(ref type_error) => Diagnostic {
            range: source_span_to_range(source, type_error),
            severity: DiagnosticSeverity::Error,
            message: format!("{type_error}"),
        },
        crate::errors::reporter::CompilerError::Codegen(ref codegen_error) => Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            severity: DiagnosticSeverity::Warning,
            message: codegen_error.message.clone(),
        },
    }
}

/// Convert a miette diagnostic's first label span into an LSP `Range`.
fn source_span_to_range(source: &str, diagnostic: &dyn miette::Diagnostic) -> Range {
    if let Some(labels) = diagnostic.labels() {
        let collected: Vec<miette::LabeledSpan> = labels.collect();
        if let Some(first_label) = collected.first() {
            let offset = first_label.offset();
            let length = first_label.len();
            return byte_span_to_range(source, offset, length);
        }
    }

    Range {
        start: Position {
            line: 0,
            character: 0,
        },
        end: Position {
            line: 0,
            character: 0,
        },
    }
}

/// Convert byte offsets into zero-based line/column range.
fn byte_span_to_range(source: &str, start_offset: usize, length: usize) -> Range {
    let safe_start = start_offset.min(source.len());
    let safe_end = safe_start.saturating_add(length).min(source.len());

    Range {
        start: byte_offset_to_position(source, safe_start),
        end: byte_offset_to_position(source, safe_end),
    }
}

/// Convert a byte offset in `source` to LSP line and character coordinates.
fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0_usize;
    let mut character = 0_usize;

    for (byte_index, ch) in source.char_indices() {
        if byte_index >= offset {
            return Position { line, character };
        }

        if ch == '\n' {
            line = line.saturating_add(1_usize);
            character = 0;
        } else {
            character = character.saturating_add(1_usize);
        }
    }

    Position { line, character }
}
