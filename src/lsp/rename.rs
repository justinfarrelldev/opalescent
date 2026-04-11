//! Rename refactoring provider for Opalescent LSP.

extern crate alloc;

use crate::lsp::definition::word_at_position;
use crate::lsp::protocol::{Position, Range, TextEdit};
use alloc::vec::Vec;

/// Compute in-document text edits for renaming symbol at `position`.
#[must_use]
pub fn get_rename_edits(source: &str, position: Position, new_name: &str) -> Vec<TextEdit> {
    let Some(target) = word_at_position(source, position) else {
        return Vec::new();
    };

    if target == new_name {
        return Vec::new();
    }

    let mut edits = Vec::new();
    for (line_index, line_text) in source.split('\n').enumerate() {
        let chars: Vec<char> = line_text.chars().collect();
        let mut cursor = 0_usize;
        while cursor < chars.len() {
            if is_word_boundary(&chars, cursor.saturating_sub(1_usize))
                && token_matches(&chars, cursor, &target)
            {
                let end = cursor.saturating_add(target.chars().count());
                if is_word_boundary(&chars, end) {
                    edits.push(TextEdit {
                        range: Range {
                            start: Position {
                                line: line_index,
                                character: cursor,
                            },
                            end: Position {
                                line: line_index,
                                character: end,
                            },
                        },
                        new_text: new_name.to_owned(),
                    });
                    cursor = end;
                    continue;
                }
            }

            cursor = cursor.saturating_add(1_usize);
        }
    }

    edits
}

/// Return true when index is outside a word token.
fn is_word_boundary(chars: &[char], index: usize) -> bool {
    if index >= chars.len() {
        return true;
    }

    let ch = chars[index];
    !(ch.is_ascii_alphanumeric() || ch == '_')
}

/// Return true if `target` starts at `offset` in `chars`.
fn token_matches(chars: &[char], offset: usize, target: &str) -> bool {
    let target_chars: Vec<char> = target.chars().collect();
    if offset.saturating_add(target_chars.len()) > chars.len() {
        return false;
    }

    for (index, target_ch) in target_chars.iter().enumerate() {
        if chars[offset.saturating_add(index)] != *target_ch {
            return false;
        }
    }

    true
}
