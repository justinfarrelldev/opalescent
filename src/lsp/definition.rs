//! Go-to-definition provider for Opalescent LSP.

extern crate alloc;

use crate::ast::{AstNode, Decl};
use crate::lexer::Lexer;
use crate::lsp::protocol::{Location, Position, Range};
use crate::parser::Parser;
use alloc::string::String;

/// Return declaration location for symbol at `position`.
#[must_use]
pub fn get_definition(source: &str, position: Position, uri: &str) -> Option<Location> {
    let symbol_name = word_at_position(source, position)?;

    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    if !lex_errors.is_empty() {
        return None;
    }

    let parser = Parser::new(tokens);
    let (program, _parse_errors) = parser.parse();

    if let Some(parsed_program) = program {
        for declaration in parsed_program.declarations {
            if let Some(location) =
                declaration_location_for_symbol(&declaration, &symbol_name, source, uri)
            {
                return Some(location);
            }
        }
    }

    textual_definition_fallback(source, &symbol_name, uri)
}

/// Extract identifier-like word at `position`.
#[must_use]
pub fn word_at_position(source: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = source.split('\n').collect();
    let line = lines.get(position.line)?;
    let characters: Vec<char> = line.chars().collect();
    if characters.is_empty() {
        return None;
    }

    let mut cursor = position
        .character
        .min(characters.len().saturating_sub(1_usize));

    while cursor > 0 {
        let current = characters.get(cursor)?;
        if current.is_ascii_alphanumeric() || *current == '_' {
            break;
        }
        cursor = cursor.saturating_sub(1_usize);
    }

    let current = characters.get(cursor)?;
    if !(current.is_ascii_alphanumeric() || *current == '_') {
        return None;
    }

    let mut start = cursor;
    while start > 0 {
        let prev_index = start.saturating_sub(1_usize);
        let prev = characters.get(prev_index)?;
        if prev.is_ascii_alphanumeric() || *prev == '_' {
            start = prev_index;
        } else {
            break;
        }
    }

    let mut end = cursor;
    while end < characters.len() {
        let current_char = characters.get(end)?;
        if current_char.is_ascii_alphanumeric() || *current_char == '_' {
            end = end.saturating_add(1_usize);
        } else {
            break;
        }
    }

    if end <= start {
        return None;
    }

    let mut out = String::new();
    for index in start..end {
        if let Some(ch) = characters.get(index) {
            out.push(*ch);
        }
    }
    Some(out)
}

/// Resolve declaration location for a symbol from one top-level declaration.
fn declaration_location_for_symbol(
    declaration: &Decl,
    symbol_name: &str,
    source: &str,
    uri: &str,
) -> Option<Location> {
    match *declaration {
        Decl::Function { ref name, .. } | Decl::Type { ref name, .. } => {
            if name == symbol_name {
                let range = span_to_range(source, declaration.span());
                return Some(Location {
                    uri: uri.to_owned(),
                    range,
                });
            }
        }
        Decl::Let { ref binding, .. } => {
            if binding.name == symbol_name {
                let range = span_to_range(source, declaration.span());
                return Some(Location {
                    uri: uri.to_owned(),
                    range,
                });
            }
        }
        Decl::Import { .. } => {}
    }

    None
}

/// Convert parser span to an LSP range.
fn span_to_range(source: &str, span: crate::token::Span) -> Range {
    Range {
        start: byte_offset_to_position(source, span.start.offset),
        end: byte_offset_to_position(source, span.end.offset),
    }
}

/// Convert byte offset into zero-based line/column position.
fn byte_offset_to_position(source: &str, offset: usize) -> Position {
    let mut line = 0_usize;
    let mut character = 0_usize;

    for (byte_index, ch) in source.char_indices() {
        if byte_index >= offset {
            return Position { line, character };
        }

        if ch == '\n' {
            line = line.saturating_add(1_usize);
            character = 0;
        } else {
            character = character.saturating_add(1_usize);
        }
    }

    Position { line, character }
}

/// Fallback declaration lookup based on textual `f <name>`/`let <name>` patterns.
fn textual_definition_fallback(source: &str, symbol_name: &str, uri: &str) -> Option<Location> {
    for (line_index, line_text) in source.split('\n').enumerate() {
        let function_pattern = format!("f {symbol_name}");
        if let Some(column_index) = line_text.find(&function_pattern) {
            let start_column = column_index.saturating_add(2_usize);
            let end_column = start_column.saturating_add(symbol_name.chars().count());
            return Some(Location {
                uri: uri.to_owned(),
                range: Range {
                    start: Position {
                        line: line_index,
                        character: start_column,
                    },
                    end: Position {
                        line: line_index,
                        character: end_column,
                    },
                },
            });
        }

        let let_pattern = format!("let {symbol_name}");
        if let Some(column_index) = line_text.find(&let_pattern) {
            let start_column = column_index.saturating_add(4_usize);
            let end_column = start_column.saturating_add(symbol_name.chars().count());
            return Some(Location {
                uri: uri.to_owned(),
                range: Range {
                    start: Position {
                        line: line_index,
                        character: start_column,
                    },
                    end: Position {
                        line: line_index,
                        character: end_column,
                    },
                },
            });
        }
    }

    None
}
