//! Hover provider for Opalescent LSP.

extern crate alloc;

use crate::lexer::Lexer;
use crate::lsp::completion::get_completions;
use crate::lsp::definition::word_at_position;
use crate::lsp::protocol::{HoverResult, Position};
use crate::parser::Parser;
use crate::token::TokenType;
use crate::type_system::checker::TypeChecker;

/// Return hover content for symbol at `position`.
#[must_use]
pub fn get_hover(source: &str, position: Position) -> Option<HoverResult> {
    let hovered_word = word_at_position(source, position)?;

    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    if !lex_errors.is_empty() {
        return None;
    }

    let parser = Parser::new(tokens.clone());
    let (program, _parse_errors) = parser.parse();

    if let Some(parsed_program) = program {
        let mut checker = TypeChecker::new();
        if checker.type_check_program(&parsed_program).is_ok() {
            if let Some(symbol) = checker.symbol_table().lookup(&hovered_word) {
                return Some(HoverResult {
                    contents: format!("`{}`: {}", symbol.name, symbol.core_type),
                    range: None,
                });
            }
        }
    }

    for completion in get_completions(source, position) {
        if completion.label == hovered_word {
            return Some(HoverResult {
                contents: format!("`{}`", completion.label),
                range: None,
            });
        }
    }

    for token in tokens {
        if let TokenType::Identifier(name) = token.token_type {
            if name == hovered_word {
                return Some(HoverResult {
                    contents: format!("identifier `{name}`"),
                    range: None,
                });
            }
        }
    }

    Some(HoverResult {
        contents: format!("identifier `{hovered_word}`"),
        range: None,
    })
}
