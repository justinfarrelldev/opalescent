//! Completion provider for Opalescent LSP.

extern crate alloc;

use crate::lexer::{Lexer, RESERVED_KEYWORDS};
use crate::lsp::protocol::{CompletionItem, Position};
use crate::token::TokenType;
use alloc::collections::BTreeSet;
use alloc::vec::Vec;

/// Return completion candidates at `position` for `source`.
#[must_use]
pub fn get_completions(source: &str, position: Position) -> Vec<CompletionItem> {
    let mut labels: BTreeSet<String> = BTreeSet::new();

    for keyword in RESERVED_KEYWORDS {
        labels.insert((*keyword).to_owned());
    }

    labels.insert(String::from("print"));
    labels.insert(String::from("take_input"));
    labels.insert(String::from("string_to_int32"));
    labels.insert(String::from("random_int32"));

    let line_prefix = line_prefix(source, position);

    let lexer = Lexer::new(source);
    let (tokens, _errors) = lexer.tokenize();
    for token in tokens {
        if let TokenType::Identifier(name) = token.token_type {
            labels.insert(name);
        }
    }

    let mut completions = Vec::new();
    for label in labels {
        if line_prefix.is_empty() || label.starts_with(&line_prefix) {
            completions.push(CompletionItem {
                detail: None,
                label,
            });
        }
    }

    completions
}

/// Extract the partial identifier prefix on the current line before `position`.
fn line_prefix(source: &str, position: Position) -> String {
    let lines: Vec<&str> = source.split('\n').collect();
    let maybe_line = lines.get(position.line);
    if let Some(line_text) = maybe_line {
        let prefix_text = take_chars_prefix(line_text, position.character);
        let mut collected = String::new();
        for ch in prefix_text.chars().rev() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                collected.insert(0, ch);
            } else {
                break;
            }
        }
        return collected;
    }

    String::new()
}

/// Return the first `limit` characters from `text`.
fn take_chars_prefix(text: &str, limit: usize) -> String {
    let mut output = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= limit {
            break;
        }
        output.push(ch);
    }
    output
}
