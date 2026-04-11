//! Pure-Rust LSP server request dispatcher for Opalescent.

extern crate alloc;

use crate::lsp::completion::get_completions;
use crate::lsp::definition::get_definition;
use crate::lsp::diagnostics::get_diagnostics;
use crate::lsp::hover::get_hover;
use crate::lsp::protocol::{LspRequest, LspResponse};
use crate::lsp::rename::get_rename_edits;
use crate::lsp::semantic_tokens::get_semantic_tokens;
use alloc::collections::BTreeMap;

/// Stateless request handler with lifecycle flags.
#[derive(Debug, Default)]
pub struct LspServer {
    /// Whether `initialize` has completed.
    initialized: bool,
    /// Whether `shutdown` has been requested.
    shutdown: bool,
}

impl LspServer {
    /// Create a new LSP server instance.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            initialized: false,
            shutdown: false,
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
            _ if !self.initialized => {
                LspResponse::Error(String::from("server not initialized"))
            }
            _ if self.shutdown => {
                LspResponse::Error(String::from("server is shut down"))
            }
            LspRequest::Diagnostics { source } => LspResponse::Diagnostics(get_diagnostics(&source)),
            LspRequest::Completion { source, position } => {
                LspResponse::Completion(get_completions(&source, position))
            }
            LspRequest::Hover { source, position } => {
                LspResponse::Hover(get_hover(&source, position))
            }
            LspRequest::Definition { source, position } => {
                LspResponse::Definition(get_definition(&source, position))
            }
            LspRequest::Rename {
                source,
                position,
                new_name,
            } => LspResponse::Rename(get_rename_edits(&source, position, &new_name)),
            LspRequest::SemanticTokens { source } => {
                LspResponse::SemanticTokens(get_semantic_tokens(&source))
            }
        }
    }
}
