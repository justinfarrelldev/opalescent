//! Contextual test titles are scanned before string/comment tokenization.
//!
//! Headers and titles retain original source spans; no rewritten source is used.
use super::{Lexer, ModuleMode};
use crate::error::LexError;
use crate::token::{Span, Token, TokenType};

impl Lexer<'_> {
    /// Recognize prose only at the first non-whitespace position on a line.
    pub(super) fn scan_test_prose_header(&mut self) -> Option<Token> {
        if self.module_mode != ModuleMode::Test {
            return None;
        }
        let offset = self.position.offset.saturating_sub(self.span_offset_base);
        let prefix = self.input.get(..offset)?;
        if !prefix
            .rsplit('\n')
            .next()?
            .chars()
            .all(|c| c == ' ' || c == '\t')
        {
            return None;
        }
        let remaining = self.input.get(offset..)?;
        let header = ["scenario", "it should", "example"]
            .into_iter()
            .find(|header| {
                remaining.strip_prefix(header).is_some_and(|rest| {
                    rest.starts_with(':') || rest.starts_with(' ') || rest.starts_with('\t')
                })
            })?;
        // Assignments to these identifiers are ordinary code even in test mode.
        if remaining.get(header.len()..)?.trim_start().starts_with('=') {
            return None;
        }
        let start = self.position;
        for _ in header.chars() {
            self.advance();
        }
        let header_token = Token::new(
            TokenType::Identifier(header.to_owned()),
            Span::new(start, self.position),
            header.to_owned(),
        );
        self.skip_inline_whitespace();
        if self.current_char() == ':' {
            return Some(header_token);
        }
        let title_start = self.position;
        let title = if self.current_char() == '\'' {
            self.scan_string_literal_with_line_limit(title_start, true)
        } else {
            let mut value = String::new();
            while !self.is_at_end() && !matches!(self.current_char(), ':' | '\n' | '\r') {
                if self.current_char() == '\\' && self.peek() == Some(':') {
                    self.advance();
                }
                value.push(self.current_char());
                self.advance();
            }
            let trimmed = value.trim_end();
            let trailing_bytes = value.len().saturating_sub(trimmed.len());
            let mut end = self.position;
            end.offset = end.offset.saturating_sub(trailing_bytes);
            end.column = end.column.saturating_sub(
                value
                    .get(trimmed.len()..)
                    .unwrap_or_default()
                    .chars()
                    .count(),
            );
            let begin = title_start.offset.saturating_sub(self.span_offset_base);
            let finish = end.offset.saturating_sub(self.span_offset_base);
            Token::new(
                TokenType::StringLiteral(trimmed.to_owned()),
                Span::new(title_start, end),
                self.input.get(begin..finish).unwrap_or_default().to_owned(),
            )
        };
        self.skip_inline_whitespace();
        if self.current_char() != ':' || self.position.line != title_start.line {
            self.errors.push(LexError::UnterminatedTestTitle {
                span: LexError::span_from_position(title_start, 1),
            });
        }
        self.pending_tokens.push_back(title);
        Some(header_token)
    }
}
