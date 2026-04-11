//! Unit tests for Opalescent LSP modules.

extern crate alloc;

use crate::lsp::completion::get_completions;
use crate::lsp::definition::get_definition;
use crate::lsp::diagnostics::get_diagnostics;
use crate::lsp::hover::get_hover;
use crate::lsp::protocol::{DiagnosticSeverity, LspRequest, LspResponse, Position};
use crate::lsp::rename::get_rename_edits;
use crate::lsp::semantic_tokens::get_semantic_tokens;
use crate::lsp::server::LspServer;

#[test]
fn diagnostics_report_type_error_from_inline_source() {
    let source = "entry f main(): int32 => {\n  let value: int32 = true\n  return value\n}\n";
    let diagnostics = get_diagnostics(source);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error),
        "expected at least one error diagnostic"
    );
}

#[test]
fn completion_includes_keywords_and_locals() {
    let source = "entry f main(): int32 => {\n  let value: int32 = 1\n  val\n  return value\n}\n";
    let completions = get_completions(
        source,
        Position {
            line: 2,
            character: 3,
        },
    );

    assert!(
        completions.iter().any(|item| item.label == "value"),
        "expected local symbol completion"
    );
}

#[test]
fn hover_returns_symbol_type_info() {
    let source = "entry f main(): int32 => {\n  let value: int32 = 1\n  return value\n}\n";
    let hover = get_hover(
        source,
        Position {
            line: 1,
            character: 6,
        },
    );

    assert!(hover.is_some(), "expected hover result");
    if let Some(hover_result) = hover {
        assert!(
            hover_result.contents.contains("value"),
            "expected hover to mention symbol name"
        );
    }
}

#[test]
fn definition_returns_top_level_function_location() {
    let source = "entry f main(): int32 => {\n  return helper()\n}\n\nf helper(): int32 => {\n  return 1\n}\n";
    let definition = get_definition(
        source,
        Position {
            line: 4,
            character: 2,
        },
    );

    assert!(definition.is_some(), "expected definition location");
    if let Some(location) = definition {
        assert!(location.range.start.line >= 4, "expected helper declaration line");
    }
}

#[test]
fn rename_returns_all_occurrence_edits() {
    let source = "entry f main(): int32 => {\n  let value: int32 = 1\n  let second: int32 = value\n  return value\n}\n";
    let edits = get_rename_edits(
        source,
        Position {
            line: 1,
            character: 7,
        },
        "renamed_value",
    );

    assert_eq!(edits.len(), 3, "expected declaration and two uses to be renamed");
}

#[test]
fn semantic_tokens_classify_key_lexemes() {
    let source = "entry f main(): int32 => {\n  let value: int32 = 1\n  return value\n}\n";
    let tokens = get_semantic_tokens(source);

    assert!(
        tokens.iter().any(|token| token.token_type == "keyword"),
        "expected keyword token classification"
    );
    assert!(
        tokens.iter().any(|token| token.token_type == "variable"),
        "expected variable token classification"
    );
}

#[test]
fn server_lifecycle_and_dispatch_works_without_transport() {
    let mut server = LspServer::new();

    let pre_init = server.handle_request(LspRequest::Diagnostics {
        source: String::from("entry f main(): int32 => { return 1 }"),
    });
    assert!(matches!(pre_init, LspResponse::Error(_)));

    let initialize = server.handle_request(LspRequest::Initialize);
    assert!(matches!(initialize, LspResponse::Initialized { .. }));

    let diagnostics = server.handle_request(LspRequest::Diagnostics {
        source: String::from("entry f main(): int32 => { return 1 }"),
    });
    assert!(matches!(diagnostics, LspResponse::Diagnostics(_)));

    let shutdown = server.handle_request(LspRequest::Shutdown);
    assert!(matches!(shutdown, LspResponse::Shutdown));

    let post_shutdown = server.handle_request(LspRequest::Completion {
        source: String::from("entry f main(): int32 => { return 1 }"),
        position: Position {
            line: 0,
            character: 0,
        },
    });
    assert!(matches!(post_shutdown, LspResponse::Error(_)));
}
