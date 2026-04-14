//! Unit tests for Opalescent LSP modules.

extern crate alloc;

use crate::lsp::completion::get_completions;
use crate::lsp::definition::get_definition;
use crate::lsp::diagnostics::get_diagnostics;
use crate::lsp::hover::get_hover;
use crate::lsp::protocol::{
    DiagnosticSeverity, LspNotification, LspRequest, LspResponse, Position,
};
use crate::lsp::rename::get_rename_edits;
use crate::lsp::semantic_tokens::get_semantic_tokens;
use crate::lsp::server::LspServer;
use crate::lsp::transport::{read_framed_message, write_framed_message};
use std::io::Cursor;

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
fn server_document_notifications_update_document_store() {
    let mut server = LspServer::new();
    let init = server.handle_request(LspRequest::Initialize);
    assert!(matches!(init, LspResponse::Initialized { .. }));

    server.handle_notification(LspNotification::DidOpen {
        uri: String::from("file:///doc.op"),
        source: String::from("let value = 1"),
    });
    assert_eq!(
        server.document_source("file:///doc.op"),
        Some("let value = 1"),
        "didOpen should store document"
    );

    server.handle_notification(LspNotification::DidChange {
        uri: String::from("file:///doc.op"),
        source: String::from("let value = 2"),
    });
    assert_eq!(
        server.document_source("file:///doc.op"),
        Some("let value = 2"),
        "didChange should update document"
    );

    server.handle_notification(LspNotification::DidClose {
        uri: String::from("file:///doc.op"),
    });
    assert!(
        server.document_source("file:///doc.op").is_none(),
        "didClose should remove document"
    );
}

#[test]
fn server_rename_returns_edits_across_open_documents() {
    let mut server = LspServer::new();
    let init = server.handle_request(LspRequest::Initialize);
    assert!(matches!(init, LspResponse::Initialized { .. }));

    server.handle_notification(LspNotification::DidOpen {
        uri: String::from("file:///a.op"),
        source: String::from(
            "entry f main(): int32 => {\n  let value: int32 = 1\n  return value\n}\n",
        ),
    });
    server.handle_notification(LspNotification::DidOpen {
        uri: String::from("file:///b.op"),
        source: String::from(
            "entry f helper(): int32 => {\n  let value: int32 = 2\n  return value\n}\n",
        ),
    });

    let response = server.handle_request(LspRequest::Rename {
        uri: String::from("file:///a.op"),
        source: String::from(
            "entry f main(): int32 => {\n  let value: int32 = 1\n  return value\n}\n",
        ),
        position: Position {
            line: 1,
            character: 6,
        },
        new_name: String::from("renamed_value"),
    });

    assert!(
        matches!(response, LspResponse::Rename(_)),
        "expected rename response with grouped edits"
    );
    let LspResponse::Rename(edits_by_uri) = response else {
        return;
    };

    assert!(
        edits_by_uri.contains_key("file:///a.op"),
        "rename should include triggering document"
    );
    assert!(
        edits_by_uri.contains_key("file:///b.op"),
        "rename should include other open documents"
    );
}

#[test]
fn server_definition_uses_request_uri() {
    let mut server = LspServer::new();
    let init = server.handle_request(LspRequest::Initialize);
    assert!(matches!(init, LspResponse::Initialized { .. }));

    let response = server.handle_request(LspRequest::Definition {
        uri: String::from("file:///defs.op"),
        source: String::from(
            "entry f main(): int32 => {\n  return helper()\n}\n\nf helper(): int32 => {\n  return 1\n}\n",
        ),
        position: Position {
            line: 1,
            character: 9,
        },
    });

    assert!(
        matches!(response, LspResponse::Definition(_)),
        "expected definition response"
    );
    let LspResponse::Definition(location_opt) = response else {
        return;
    };
    let location = location_opt.expect("expected definition location");
    assert_eq!(
        location.uri, "file:///defs.op",
        "definition location should use request URI"
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
        "file:///test.op",
    );

    assert!(definition.is_some(), "expected definition location");
    if let Some(location) = definition {
        assert_eq!(
            location.uri, "file:///test.op",
            "definition should use request URI"
        );
        assert!(
            location.range.start.line >= 4,
            "expected helper declaration line"
        );
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

    assert_eq!(
        edits.len(),
        3,
        "expected declaration and two uses to be renamed"
    );
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
        uri: String::from("file:///main.op"),
        source: String::from("entry f main(): int32 => { return 1 }"),
    });
    assert!(matches!(pre_init, LspResponse::Error(_)));

    let initialize = server.handle_request(LspRequest::Initialize);
    assert!(matches!(initialize, LspResponse::Initialized { .. }));

    let diagnostics = server.handle_request(LspRequest::Diagnostics {
        uri: String::from("file:///main.op"),
        source: String::from("entry f main(): int32 => { return 1 }"),
    });
    assert!(matches!(diagnostics, LspResponse::Diagnostics(_)));

    let shutdown = server.handle_request(LspRequest::Shutdown);
    assert!(matches!(shutdown, LspResponse::Shutdown));

    let post_shutdown = server.handle_request(LspRequest::Completion {
        uri: String::from("file:///main.op"),
        source: String::from("entry f main(): int32 => { return 1 }"),
        position: Position {
            line: 0,
            character: 0,
        },
    });
    assert!(matches!(post_shutdown, LspResponse::Error(_)));
}

#[test]
fn transport_reads_content_length_framed_payload() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
    let framed = format!("Content-Length: {}\r\n\r\n{}", payload.len(), payload);
    let mut cursor = Cursor::new(framed.into_bytes());

    let decoded = read_framed_message(&mut cursor).expect("expected framed message to decode");
    assert_eq!(decoded, payload);
}

#[test]
fn transport_writes_content_length_framed_payload() {
    let payload = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
    let mut buffer = Vec::new();

    write_framed_message(&mut buffer, payload).expect("expected framed payload write");

    let output = String::from_utf8(buffer).expect("framed payload must be utf8");
    let expected_prefix = format!("Content-Length: {}\r\n\r\n", payload.len());
    assert!(
        output.starts_with(&expected_prefix),
        "expected Content-Length header prefix"
    );
    assert!(
        output.ends_with(payload),
        "expected payload at end of framed message"
    );
}

#[test]
fn transport_rejects_missing_content_length_header() {
    let mut cursor = Cursor::new(b"\r\n{}".to_vec());
    let err = read_framed_message(&mut cursor).expect_err("expected missing header error");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}

#[test]
fn transport_rejects_invalid_content_length_value() {
    let mut cursor = Cursor::new(b"Content-Length: abc\r\n\r\n{}".to_vec());
    let err = read_framed_message(&mut cursor).expect_err("expected invalid length error");
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
}
