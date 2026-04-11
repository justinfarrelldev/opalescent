extern crate alloc;

use super::{ParseError, ParseResult, Parser};
use crate::ast::TypeParameter;
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse generic type parameter declarations with optional constraints.
    pub(super) fn parse_type_parameter_declarations(&mut self) -> ParseResult<Vec<TypeParameter>> {
        self.consume(
            &TokenType::Less,
            "Expected '<' before generic type parameter declarations",
        )?;

        if self.check(&TokenType::Greater) {
            return Err(ParseError::InvalidSyntax {
                message: "Empty generic parameter list".to_owned(),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        let mut declarations = Vec::new();

        loop {
            if !self.check_identifier() {
                return Err(ParseError::UnexpectedToken {
                    expected: "generic parameter name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

            let token = self.advance().clone();
            let TokenType::Identifier(name) = token.token_type else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for generic parameter".to_owned(),
                    span: ParseError::span_from_token(self.previous_token()),
                });
            };

            let mut constraints = Vec::new();
            if self.check(&TokenType::Colon) {
                self.advance();
                constraints.push(self.parse_type()?);

                while self.check(&TokenType::Plus) {
                    self.advance();
                    constraints.push(self.parse_type()?);
                }
            }

            let end = if constraints.is_empty() {
                token.span.end
            } else {
                self.previous_token().span.end
            };
            declarations.push(TypeParameter {
                name,
                constraints,
                span: Span::new(token.span.start, end),
            });

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(
            &TokenType::Greater,
            "Expected '>' after generic type parameter declarations",
        )?;

        Ok(declarations)
    }
}
