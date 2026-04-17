//! Modifier parsing for function declarations

use super::{ParseError, ParseResult, Parser};
use crate::ast::{FunctionModifier, Visibility};
use crate::token::TokenType;

impl Parser {
    /// Parse function modifiers (public, entry, pure, untested) and return visibility, entry flag, and modifiers list.
    pub(super) fn parse_modifiers(
        &mut self,
    ) -> ParseResult<(Visibility, bool, Vec<FunctionModifier>)> {
        let mut visibility = Visibility::Private;
        let mut is_entry = false;
        let mut modifiers = Vec::new();

        loop {
            match self.current_token().token_type {
                TokenType::Public => {
                    if visibility == Visibility::Public {
                        return Err(ParseError::UnexpectedToken {
                            expected: "declaration after modifiers".to_owned(),
                            found: "duplicate 'public' modifier".to_owned(),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    }
                    self.advance();
                    visibility = Visibility::Public;
                }
                TokenType::Entry => {
                    if is_entry {
                        return Err(ParseError::UnexpectedToken {
                            expected: "declaration after modifiers".to_owned(),
                            found: "duplicate 'entry' modifier".to_owned(),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    }
                    self.advance();
                    is_entry = true;
                }
                TokenType::Pure => {
                    if modifiers.contains(&FunctionModifier::Pure) {
                        return Err(ParseError::UnexpectedToken {
                            expected: "declaration after modifiers".to_owned(),
                            found: "duplicate 'pure' modifier".to_owned(),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    }
                    self.advance();
                    modifiers.push(FunctionModifier::Pure);
                }
                TokenType::Untested => {
                    if modifiers.contains(&FunctionModifier::Untested) {
                        return Err(ParseError::UnexpectedToken {
                            expected: "declaration after modifiers".to_owned(),
                            found: "duplicate 'untested' modifier".to_owned(),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    }
                    self.advance();
                    modifiers.push(FunctionModifier::Untested);
                }
                _ => break,
            }
        }

        Ok((visibility, is_entry, modifiers))
    }
}
