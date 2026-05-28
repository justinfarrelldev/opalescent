//! String interpolation parsing helpers for the Opalescent parser.

use super::{ParseError, ParseResult, Parser};
use crate::ast::{Expr, StringPart};
use crate::error::LexError;
use crate::lexer::Lexer;
use crate::token::{Position, Span};

impl Parser {
    /// Parse string interpolation expressions ('Hello {world}')
    pub(super) fn parse_string_interpolation(
        &mut self,
        raw_lexeme: &str,
        span: Span,
    ) -> ParseResult<Expr> {
        let mut parts = Vec::new();
        let mut current_str = String::new();
        let raw_content = Self::interpolation_raw_content(raw_lexeme);
        let mut content_start_position = span.start;
        Self::advance_position_for_char(&mut content_start_position, '\'');
        let (decoded_content, decoded_positions) =
            Self::decode_string_content_with_positions(raw_content, content_start_position);

        let chars: Vec<char> = decoded_content.chars().collect();
        let mut index = 0;

        while index < chars.len() {
            let ch = chars[index];
            if ch == '{' {
                // Add the current string part if not empty
                if !current_str.is_empty() {
                    parts.push(StringPart::Literal(current_str.clone()));
                    current_str.clear();
                } else if parts.is_empty() {
                    // Add empty literal for strings starting with interpolation
                    parts.push(StringPart::Literal(String::new()));
                }

                // Parse the expression inside {}
                let mut expr_str = String::new();
                let mut brace_count = 1_i32;
                let mut found_closing_brace = false;
                let mut expr_index = index.saturating_add(1);

                while expr_index < chars.len() {
                    let expr_ch = chars[expr_index];
                    match expr_ch {
                        '{' => {
                            brace_count = brace_count.checked_add(1_i32).ok_or_else(|| {
                                ParseError::InvalidSyntax {
                                    message: "Too many nested braces in string interpolation"
                                        .to_owned(),
                                    span: LexError::span_from_span(span),
                                }
                            })?;
                            expr_str.push(expr_ch);
                        }
                        '}' => {
                            brace_count = brace_count.checked_sub(1_i32).ok_or_else(|| {
                                ParseError::InvalidSyntax {
                                    message: "Unmatched closing brace in string interpolation"
                                        .to_owned(),
                                    span: LexError::span_from_span(span),
                                }
                            })?;
                            if brace_count == 0_i32 {
                                found_closing_brace = true;
                                break;
                            }
                            expr_str.push(expr_ch);
                        }
                        _ => {
                            expr_str.push(expr_ch);
                        }
                    }

                    expr_index = expr_index.saturating_add(1);
                }

                // Check for unclosed braces
                if !found_closing_brace {
                    return Err(ParseError::InvalidSyntax {
                        message: "Unclosed interpolation brace in string".to_owned(),
                        span: LexError::span_from_span(span),
                    });
                }

                // Parse the expression string
                if expr_str.trim().is_empty() {
                    return Err(ParseError::InvalidSyntax {
                        message: "Empty interpolation expression in string".to_owned(),
                        span: LexError::span_from_span(span),
                    });
                }

                let expr_start_position = decoded_positions
                    .get(index.saturating_add(1))
                    .copied()
                    .unwrap_or(span.start);
                let expr_lexer = Lexer::new_with_start_position(&expr_str, expr_start_position);
                let (expr_tokens, _) = expr_lexer.tokenize();
                let mut expr_parser = Self::new(expr_tokens);

                match expr_parser.parse_expression() {
                    Ok(expr) => {
                        parts.push(StringPart::Expression(expr));
                    }
                    Err(_) => {
                        return Err(ParseError::InvalidSyntax {
                            message: format!(
                                "Invalid expression in string interpolation: {expr_str}"
                            ),
                            span: LexError::span_from_span(span),
                        });
                    }
                }

                index = expr_index.saturating_add(1);
            } else {
                current_str.push(ch);
                index = index.saturating_add(1);
            }
        }

        // Add any remaining string content
        parts.push(StringPart::Literal(current_str));

        Ok(Expr::StringInterpolation {
            parts,
            span,
            id: self.next_node_id(),
        })
    }

    /// Advance a source position by one consumed character.
    fn advance_position_for_char(position: &mut Position, ch: char) {
        position.offset = position.offset.saturating_add(ch.len_utf8());
        if ch == '\n' {
            position.line = position.line.saturating_add(1);
            position.column = 1;
        } else {
            position.column = position.column.saturating_add(1);
        }
    }

    /// Strip surrounding single quotes from a string literal lexeme.
    fn interpolation_raw_content(raw_lexeme: &str) -> &str {
        raw_lexeme
            .strip_prefix('\'')
            .and_then(|content| content.strip_suffix('\''))
            .unwrap_or(raw_lexeme)
    }

    /// Decode the raw string literal body while preserving each decoded char's source position.
    fn decode_string_content_with_positions(
        raw_content: &str,
        content_start_position: Position,
    ) -> (String, Vec<Position>) {
        let mut decoded = String::new();
        let mut positions = Vec::new();
        let mut position = content_start_position;
        let mut chars = raw_content.chars();

        while let Some(ch) = chars.next() {
            let char_start = position;

            if ch == '\\' {
                Self::advance_position_for_char(&mut position, ch);
                if let Some(escaped) = chars.next() {
                    let decoded_char = match escaped {
                        'n' => '\n',
                        't' => '\t',
                        'r' => '\r',
                        '\\' => '\\',
                        '\'' => '\'',
                        '{' => '{',
                        '}' => '}',
                        other => other,
                    };
                    decoded.push(decoded_char);
                    positions.push(char_start);
                    Self::advance_position_for_char(&mut position, escaped);
                } else {
                    decoded.push(ch);
                    positions.push(char_start);
                }
                continue;
            }

            decoded.push(ch);
            positions.push(char_start);
            Self::advance_position_for_char(&mut position, ch);
        }

        (decoded, positions)
    }
}
