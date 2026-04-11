//! Semantic token classification for Opalescent LSP.

extern crate alloc;

use crate::lexer::Lexer;
use crate::lsp::protocol::SemanticToken;
use crate::token::TokenType;
use alloc::vec::Vec;

/// Compute semantic tokens for syntax highlighting.
#[must_use]
pub fn get_semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let lexer = Lexer::new(source);
    let (tokens, _errors) = lexer.tokenize();

    let mut semantic_tokens = Vec::new();

    for token in tokens {
        let token_type_name = semantic_token_name(&token.token_type);
        if let Some(name) = token_type_name {
            semantic_tokens.push(SemanticToken {
                line: token.span.start.line.saturating_sub(1_usize),
                start_character: token.span.start.column.saturating_sub(1_usize),
                length: token.span.len(),
                token_type: name.to_owned(),
            });
        }
    }

    semantic_tokens
}

/// Map lexer token kinds to LSP semantic token type names.
const fn semantic_token_name(token_type: &TokenType) -> Option<&'static str> {
    match *token_type {
        TokenType::Identifier(_) => Some("variable"),
        TokenType::IntegerLiteral(_) | TokenType::FloatLiteral(_) => Some("number"),
        TokenType::StringLiteral(_) => Some("string"),
        TokenType::BooleanLiteral(_)
        | TokenType::Function
        | TokenType::Return
        | TokenType::If
        | TokenType::Else
        | TokenType::Match
        | TokenType::For
        | TokenType::While
        | TokenType::Loop
        | TokenType::Guard
        | TokenType::Into
        | TokenType::Propagate
        | TokenType::Public
        | TokenType::Entry
        | TokenType::Type
        | TokenType::Import => Some("keyword"),
        TokenType::Comment(_) | TokenType::DocComment(_) => Some("comment"),
        TokenType::Plus
        | TokenType::Minus
        | TokenType::Multiply
        | TokenType::Divide
        | TokenType::Power
        | TokenType::Modulo
        | TokenType::Assign
        | TokenType::Less
        | TokenType::LessEqual
        | TokenType::Greater
        | TokenType::GreaterEqual
        | TokenType::Is
        | TokenType::IsNot
        | TokenType::And
        | TokenType::Or
        | TokenType::Not
        | TokenType::Xor
        | TokenType::BitAnd
        | TokenType::BitOr
        | TokenType::BitXor
        | TokenType::BitNot
        | TokenType::BitShiftLeft
        | TokenType::BitShiftRight
        | TokenType::BitUnsignedShiftRight => Some("operator"),
        _ => None,
    }
}
