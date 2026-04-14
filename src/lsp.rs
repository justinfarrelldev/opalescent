#![expect(
    clippy::pub_use,
    reason = "Task 38 requires a crate-level LSP API surface exposed from src/lsp.rs"
)]

//! Opalescent Language Server Protocol implementation surface.

#[path = "lsp/completion.rs"]
pub mod completion;
#[path = "lsp/definition.rs"]
pub mod definition;
#[path = "lsp/diagnostics.rs"]
pub mod diagnostics;
#[path = "lsp/hover.rs"]
pub mod hover;
#[path = "lsp/protocol.rs"]
pub mod protocol;
#[path = "lsp/rename.rs"]
pub mod rename;
#[path = "lsp/semantic_tokens.rs"]
pub mod semantic_tokens;
#[path = "lsp/server.rs"]
pub mod server;
#[path = "lsp/transport.rs"]
pub mod transport;

pub use completion::get_completions;
pub use definition::get_definition;
pub use diagnostics::get_diagnostics;
pub use hover::get_hover;
pub use protocol::{
    CompletionItem, Diagnostic, DiagnosticSeverity, HoverResult, Location, LspNotification,
    LspRequest, LspResponse, Position, Range, SemanticToken, TextEdit,
};
pub use rename::get_rename_edits;
pub use semantic_tokens::get_semantic_tokens;
pub use server::LspServer;
pub use transport::{read_framed_message, write_framed_message};

#[cfg(test)]
#[path = "lsp/tests.rs"]
mod tests;
