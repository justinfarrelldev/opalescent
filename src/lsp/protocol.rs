//! Core Language Server Protocol data model for Opalescent.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// Zero-based cursor position in a text document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Position {
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based character offset within `line`.
    pub character: usize,
}

/// A half-open text span from `start` (inclusive) to `end` (exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    /// Start position.
    pub start: Position,
    /// End position.
    pub end: Position,
}

/// A location in a text document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    /// Document URI.
    pub uri: String,
    /// Range in `uri`.
    pub range: Range,
}

/// Diagnostic severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Fatal or compile-blocking issue.
    Error,
    /// Non-fatal issue.
    Warning,
    /// Informational message.
    Information,
    /// Minor advisory hint.
    Hint,
}

/// A compiler diagnostic surfaced through LSP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Location of the issue.
    pub range: Range,
    /// Severity classification.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
}

/// Completion candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    /// Label shown in completion UI.
    pub label: String,
    /// Optional short detail, such as inferred type.
    pub detail: Option<String>,
}

/// Hover payload returned for symbol information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverResult {
    /// Markdown/plain-text-like content.
    pub contents: String,
    /// Optional range for highlighted symbol.
    pub range: Option<Range>,
}

/// Text edit used by rename refactoring.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEdit {
    /// Range to replace.
    pub range: Range,
    /// Replacement text.
    pub new_text: String,
}

/// Semantic token used by syntax highlighting clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    /// Zero-based line number.
    pub line: usize,
    /// Zero-based start character.
    pub start_character: usize,
    /// Token length in UTF-8 bytes.
    pub length: usize,
    /// Semantic token type.
    pub token_type: String,
}

/// Simplified LSP request messages handled by `LspServer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspRequest {
    /// Initialize server state.
    Initialize,
    /// Shutdown server state.
    Shutdown,
    /// Compute diagnostics for source text.
    Diagnostics { source: String },
    /// Compute completion items at `position`.
    Completion { source: String, position: Position },
    /// Compute hover contents at `position`.
    Hover { source: String, position: Position },
    /// Compute declaration location at `position`.
    Definition { source: String, position: Position },
    /// Compute rename edits for symbol at `position`.
    Rename {
        /// Source content.
        source: String,
        /// Symbol location.
        position: Position,
        /// New symbol name.
        new_name: String,
    },
    /// Compute semantic tokens for source text.
    SemanticTokens { source: String },
}

/// Simplified LSP notifications for transport-level integration tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspNotification {
    /// Document opened in editor.
    DidOpen {
        /// Document URI.
        uri: String,
        /// Current text.
        source: String,
    },
    /// Document content changed.
    DidChange {
        /// Document URI.
        uri: String,
        /// Updated text.
        source: String,
    },
    /// Document closed in editor.
    DidClose {
        /// Document URI.
        uri: String,
    },
}

/// Simplified LSP responses returned by `LspServer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LspResponse {
    /// Initialize acknowledgment with server capabilities.
    Initialized {
        /// Capability flags exposed by this server.
        capabilities: BTreeMap<String, bool>,
    },
    /// Shutdown acknowledgment.
    Shutdown,
    /// Diagnostics payload.
    Diagnostics(Vec<Diagnostic>),
    /// Completion payload.
    Completion(Vec<CompletionItem>),
    /// Hover payload.
    Hover(Option<HoverResult>),
    /// Definition payload.
    Definition(Option<Location>),
    /// Rename payload.
    Rename(Vec<TextEdit>),
    /// Semantic-token payload.
    SemanticTokens(Vec<SemanticToken>),
    /// Request failure.
    Error(String),
}
