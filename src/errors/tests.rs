use crate::codegen::expressions::CodegenError;
use crate::error::{LexError, LexErrors};
use crate::errors::formatter::{
    error_doc_link, format_codegen_error, format_diagnostic, format_error_bundle, CompilerPhase,
};
use crate::errors::reporter::{CompilationErrorReport, CompilerError};
use crate::errors::suggestions::{
    closest_identifier_suggestion, did_you_mean_type_annotation, levenshtein_distance,
    SUGGESTION_DISTANCE_THRESHOLD,
};
use crate::parser::errors::ParseError;
use crate::token::{Position, Span};
use crate::type_system::errors::TypeError;
use miette::SourceSpan;

fn test_source_span(offset: usize, len: usize) -> SourceSpan {
    SourceSpan::new(offset.into(), len)
}

#[test]
fn test_levenshtein_distance_handles_common_cases() {
    assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
    assert_eq!(levenshtein_distance("symbol", "symbol"), 0);
    assert_eq!(levenshtein_distance("", "abc"), 3);
}

#[test]
fn test_identifier_suggestion_returns_best_candidate_within_threshold() {
    let known = vec![
        String::from("print"),
        String::from("println"),
        String::from("priority"),
    ];
    let suggestion = closest_identifier_suggestion("pritn", known.as_slice());
    assert!(suggestion.is_some(), "expected did-you-mean suggestion");
    if let Some(suggestion_value) = suggestion {
        assert_eq!(suggestion_value.suggestion, "print");
        assert!(suggestion_value.distance <= SUGGESTION_DISTANCE_THRESHOLD);
    }
}

#[test]
fn test_identifier_suggestion_returns_none_when_too_far() {
    let known = vec![String::from("print"), String::from("parse")];
    let suggestion = closest_identifier_suggestion("zzzzzz", known.as_slice());
    assert!(suggestion.is_none(), "no close candidate expected");
}

#[test]
fn test_type_annotation_hint_for_generic_inference_failure() {
    let error = TypeError::CannotInferGenericType {
        param_name: String::from("T"),
        span: test_source_span(3, 2),
    };
    let hint = did_you_mean_type_annotation(&error);
    assert!(hint.is_some(), "expected type-annotation hint");
    if let Some(hint_value) = hint {
        assert!(
            hint_value.contains("type annotation") || hint_value.contains("generic"),
            "hint should guide user toward explicit typing: {hint_value}"
        );
    }
}

#[test]
fn test_format_diagnostic_includes_phase_code_help_and_docs() {
    let error = ParseError::UnexpectedToken {
        expected: String::from("identifier"),
        found: String::from("}'"),
        span: test_source_span(10, 1),
    };
    let rendered = format_diagnostic(CompilerPhase::Parser, &CompilerError::Parser(error));
    assert!(rendered.contains("error[opalescent::parser::unexpected_token]"));
    assert!(rendered.contains("phase: parser"));
    assert!(rendered.contains("help:"));
    assert!(rendered.contains("docs: https://docs.opalescent.dev/errors/"));
}

#[test]
fn test_format_codegen_error_uses_standardized_code() {
    let rendered = format_codegen_error("failed to emit phi node");
    assert!(rendered.contains("opalescent::codegen::backend_failure"));
    assert!(rendered.contains("phase: codegen"));
    assert!(rendered.contains("failed to emit phi node"));
}

#[test]
fn test_error_bundle_joins_multiple_entries() {
    let entries = vec![
        (
            CompilerPhase::Lexer,
            CompilerError::Lexer(LexError::InvalidNumber {
                number: String::from("12x"),
                position: Position::new(1, 3, 2),
                span: test_source_span(2, 3),
            }),
        ),
        (
            CompilerPhase::Codegen,
            CompilerError::Codegen(String::from("broken ir")),
        ),
    ];
    let rendered = format_error_bundle(entries.as_slice());
    assert!(rendered.contains("phase: lexer"));
    assert!(rendered.contains("phase: codegen"));
    assert!(rendered.contains("\n\n"), "entries should be separated");
}

#[test]
fn test_compilation_error_report_collects_and_renders_multi_phase_errors() {
    let mut report = CompilationErrorReport::new();
    report.push_lex_error(LexError::UnexpectedCharacter {
        character: '@',
        position: Position::new(1, 1, 0),
        span: test_source_span(0, 1),
    });
    report.push_parse_error(ParseError::InvalidSyntax {
        message: String::from("invalid assignment"),
        span: test_source_span(4, 3),
    });
    report.push_type_error(TypeError::TypeMismatch {
        expected: String::from("int64"),
        found: String::from("string"),
        found_span: test_source_span(8, 6),
        expected_span: Some(test_source_span(2, 5)),
    });
    report.push_codegen_error(String::from("invalid alloca placement"));

    assert_eq!(report.len(), 4, "all phase errors should be collected");
    let rendered = report.render();
    assert!(rendered.contains("phase: lexer"));
    assert!(rendered.contains("phase: parser"));
    assert!(rendered.contains("phase: type checker"));
    assert!(rendered.contains("phase: codegen"));
}

#[test]
fn test_symbol_not_found_formatting_surfaces_did_you_mean_suggestion() {
    let type_error = TypeError::SymbolNotFound {
        name: String::from("pritn"),
        suggestion: Some(String::from("print")),
        span: test_source_span(12, 5),
    };
    let rendered = format_diagnostic(
        CompilerPhase::TypeChecker,
        &CompilerError::TypeChecker(type_error),
    );
    assert!(rendered.contains("did you mean 'print'?"));
}

#[test]
fn test_cannot_infer_generic_formatting_surfaces_type_annotation_suggestion() {
    let type_error = TypeError::CannotInferGenericType {
        param_name: String::from("T"),
        span: test_source_span(4, 1),
    };
    let rendered = format_diagnostic(
        CompilerPhase::TypeChecker,
        &CompilerError::TypeChecker(type_error),
    );
    assert!(rendered.contains("Consider adding type annotation"));
}

#[test]
fn test_error_doc_link_stable_generation() {
    let link = error_doc_link("opalescent::parser::unexpected_token");
    assert_eq!(
        link,
        "https://docs.opalescent.dev/errors/opalescent::parser::unexpected_token"
    );
}

#[test]
fn test_format_diagnostic_uses_codegen_variant_with_codegen_error_message() {
    let codegen_error = CodegenError::new(String::from("invalid gep index"));
    let rendered = format_diagnostic(
        CompilerPhase::Codegen,
        &CompilerError::Codegen(codegen_error.message),
    );
    assert!(rendered.contains("opalescent::codegen::backend_failure"));
    assert!(rendered.contains("invalid gep index"));
}

#[test]
fn test_reporter_extends_bulk_error_collections() {
    let mut report = CompilationErrorReport::new();
    let lex_errors = vec![LexError::InvalidIdentifier {
        identifier: String::from("BadName"),
        position: Position::new(1, 1, 0),
        span: test_source_span(0, 7),
    }];
    let parse_errors = vec![ParseError::UnexpectedEof {
        expected: String::from("expression"),
        span: test_source_span(7, 0),
    }];
    let type_errors = vec![TypeError::SymbolNotFound {
        name: String::from("unknown_name"),
        suggestion: None,
        span: test_source_span(9, 12),
    }];

    report.extend_lex_errors(lex_errors);
    report.extend_parse_errors(parse_errors);
    report.extend_type_errors(type_errors);

    assert_eq!(report.len(), 3);
    let rendered = report.render();
    assert!(rendered.contains("phase: lexer"));
    assert!(rendered.contains("phase: parser"));
    assert!(rendered.contains("phase: type checker"));
}

#[test]
fn test_lex_errors_can_be_promoted_to_report() {
    let mut lex_errors = LexErrors::new();
    lex_errors.push(LexError::MixedWhitespace {
        tab_span: test_source_span(2, 1),
        space_span: test_source_span(8, 1),
    });

    let mut report = CompilationErrorReport::new();
    report.extend_lex_errors(lex_errors.errors);

    assert_eq!(report.len(), 1);
    assert!(!report.is_empty());
}

#[test]
fn test_type_error_symbol_not_found_supports_suggestion_field() {
    let error = TypeError::SymbolNotFound {
        name: String::from("pritn"),
        suggestion: Some(String::from("print")),
        span: TypeError::span_from_span(Span::single(Position::new(1, 1, 0))),
    };

    assert!(
        matches!(error, TypeError::SymbolNotFound { .. }),
        "expected SymbolNotFound variant"
    );

    if let TypeError::SymbolNotFound {
        name, suggestion, ..
    } = error
    {
        assert_eq!(name, "pritn");
        assert_eq!(suggestion.as_deref(), Some("print"));
    }
}

#[test]
fn test_identifier_suggestion_prefers_lexicographically_smaller_on_tie() {
    let known = vec![String::from("crate"), String::from("trace")];
    let suggestion = closest_identifier_suggestion("grate", known.as_slice());
    assert!(suggestion.is_some(), "expected tie-break suggestion");
    if let Some(suggestion_value) = suggestion {
        assert_eq!(suggestion_value.distance, 1);
        assert_eq!(suggestion_value.suggestion, "crate");
    }
}
