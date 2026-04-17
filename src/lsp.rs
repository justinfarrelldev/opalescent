#![expect(
    clippy::pub_use,
    reason = "Task 38 requires a crate-level LSP API surface exposed from src/lsp.rs"
)]

//! Opalescent Language Server Protocol implementation surface.

pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod hover;
pub mod protocol;
pub mod rename;
pub mod semantic_tokens;
pub mod server;
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
mod tests;
