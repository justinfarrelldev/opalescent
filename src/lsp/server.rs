//! Pure-Rust LSP server request dispatcher for Opalescent.

extern crate alloc;

use crate::lsp::completion::get_completions;
use crate::lsp::definition::get_definition;
use crate::lsp::diagnostics::get_diagnostics;
use crate::lsp::hover::get_hover;
use crate::lsp::protocol::{LspNotification, LspRequest, LspResponse};
use crate::lsp::rename::get_rename_edits;
use crate::lsp::semantic_tokens::get_semantic_tokens;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Stateless request handler with lifecycle flags.
#[derive(Debug, Default)]
pub struct LspServer {
    /// Whether `initialize` has completed.
    initialized: bool,
    /// Whether `shutdown` has been requested.
    shutdown: bool,
    /// Open document store keyed by URI.
    documents: BTreeMap<String, String>,
}

/// Find first position of `symbol` in source as a rename anchor.
fn find_symbol_position(source: &str, symbol: &str) -> Option<crate::lsp::protocol::Position> {
    for (line_index, line_text) in source.split('\n').enumerate() {
        if let Some(column_index) = line_text.find(symbol) {
            let start_ok = if column_index == 0 {
                true
            } else {
                let prev_char = line_text.chars().nth(column_index.saturating_sub(1));
                !prev_char.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            };
            let end_index = column_index.saturating_add(symbol.chars().count());
            let end_ok = if end_index >= line_text.chars().count() {
                true
            } else {
                let next_char = line_text.chars().nth(end_index);
                !next_char.is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
            };
            if start_ok && end_ok {
                return Some(crate::lsp::protocol::Position {
                    line: line_index,
                    character: column_index,
                });
            }
        }
    }
    None
}

impl LspServer {
    /// Apply one LSP notification to the in-memory document store.
    pub fn handle_notification(&mut self, notification: LspNotification) {
        match notification {
            LspNotification::DidOpen { uri, source }
            | LspNotification::DidChange { uri, source } => {
                self.documents.insert(uri, source);
            }
            LspNotification::DidClose { uri } => {
                self.documents.remove(&uri);
            }
        }
    }

    /// Return source text for an open document URI.
    #[must_use]
    pub fn document_source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(String::as_str)
    }

    /// Create a new LSP server instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: false,
            shutdown: false,
            documents: BTreeMap::new(),
        }
    }

    /// Handle one incoming request and return one response.
    #[must_use]
    pub fn handle_request(&mut self, request: LspRequest) -> LspResponse {
        match request {
            LspRequest::Initialize => {
                self.initialized = true;
                self.shutdown = false;

                let mut capabilities: BTreeMap<String, bool> = BTreeMap::new();
                capabilities.insert(String::from("diagnostics"), true);
                capabilities.insert(String::from("completion"), true);
                capabilities.insert(String::from("hover"), true);
                capabilities.insert(String::from("definition"), true);
                capabilities.insert(String::from("rename"), true);
                capabilities.insert(String::from("semanticTokens"), true);

                LspResponse::Initialized { capabilities }
            }
            LspRequest::Shutdown => {
                self.shutdown = true;
                LspResponse::Shutdown
            }
            _ if !self.initialized => LspResponse::Error(String::from("server not initialized")),
            _ if self.shutdown => LspResponse::Error(String::from("server is shut down")),
            LspRequest::Diagnostics { uri, source } => {
                self.handle_notification(LspNotification::DidOpen {
                    uri,
                    source: source.clone(),
                });
                LspResponse::Diagnostics(get_diagnostics(&source))
            }
            LspRequest::Completion {
                uri,
                source,
                position,
            } => {
                self.handle_notification(LspNotification::DidOpen {
                    uri,
                    source: source.clone(),
                });
                LspResponse::Completion(get_completions(&source, position))
            }
            LspRequest::Hover {
                uri,
                source,
                position,
            } => {
                self.handle_notification(LspNotification::DidOpen {
                    uri,
                    source: source.clone(),
                });
                LspResponse::Hover(get_hover(&source, position))
            }
            LspRequest::Definition {
                uri,
                source,
                position,
            } => {
                let definition = get_definition(&source, position, &uri);
                self.handle_notification(LspNotification::DidOpen { uri, source });
                LspResponse::Definition(definition)
            }
            LspRequest::Rename {
                uri,
                source,
                position,
                new_name,
            } => {
                let target = crate::lsp::definition::word_at_position(&source, position);
                self.handle_notification(LspNotification::DidOpen { uri, source });
                let mut edits_by_uri: BTreeMap<String, Vec<crate::lsp::protocol::TextEdit>> =
                    BTreeMap::new();
                if let Some(target_symbol) = target {
                    for (doc_uri, doc_source) in &self.documents {
                        let symbol_position = find_symbol_position(doc_source, &target_symbol);
                        if let Some(symbol_position) = symbol_position {
                            let edits = get_rename_edits(doc_source, symbol_position, &new_name);
                            if !edits.is_empty() {
                                edits_by_uri.insert(doc_uri.clone(), edits);
                            }
                        }
                    }
                }
                LspResponse::Rename(edits_by_uri)
            }
            LspRequest::SemanticTokens { uri, source } => {
                self.handle_notification(LspNotification::DidOpen {
                    uri,
                    source: source.clone(),
                });
                LspResponse::SemanticTokens(get_semantic_tokens(&source))
            }
        }
    }
}
