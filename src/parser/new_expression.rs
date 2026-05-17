//! Parsing for `new` constructor expressions.
//!
//! This form replaces the legacy `Type { field: value }` brace syntax and
//! supports both bare constructors and canonical field-block constructors.
//! Keeping it in its own module preserves the single-responsibility boundary
//! and keeps [`crate::parser::expressions`] within its line-count budget.
//!
//! Grammar:
//!
//! ```text
//! new_expr := 'new' callee (':' NEWLINE INDENT field (NEWLINE field)* DEDENT)?
//! callee   := IDENT ('.' IDENT)?
//! field    := IDENT ':' expression
//! ```

use super::{ParseError, ParseResult, Parser, Precedence};
use crate::ast::{AstNode, ConstructorField, Expr};
use crate::error::LexError;
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a `new` constructor expression, accepting either a bare
    /// propertyless constructor or a field-block constructor.
    pub(super) fn parse_new_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        self.advance(); // consume `new`

        let token = self.current_token().clone();
        let mut callee = match token.token_type {
            TokenType::Identifier(name) => {
                self.advance();
                Expr::Identifier {
                    name,
                    span: token.span,
                    id: self.next_node_id(),
                }
            }
            _ => {
                return Err(ParseError::InvalidSyntax {
                    message:
                        "Expected a type name (e.g. `Person`) or variant (`Message.Text`) after `new`"
                            .to_owned(),
                    span: ParseError::span_from_token(&token),
                });
            }
        };

        if self.check(&TokenType::Dot) {
            self.advance();
            let member_token = self.advance().clone();
            let TokenType::Identifier(member) = member_token.token_type else {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier after '.'".to_owned(),
                    found: format!("{}", member_token.token_type),
                    span: ParseError::span_from_token(&member_token),
                });
            };

            let span = Span::new(callee.span().start, member_token.span.end);
            callee = Expr::Member {
                object: Box::new(callee),
                member,
                span,
                id: self.next_node_id(),
            };

            if self.check(&TokenType::Dot) {
                return Err(ParseError::InvalidSyntax {
                    message:
                        "Bare constructor callee allows at most one qualifier (`Type` or `Module.Type`)"
                            .to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        }

        if self.check(&TokenType::LeftParen) {
            return Err(ParseError::InvalidSyntax {
                message: "Bare constructor syntax must not be followed by `()`".to_owned(),
                span: LexError::span_from_span(callee.span()),
            });
        }

        if self.check(&TokenType::Colon) {
            self.consume(&TokenType::Colon, "Expected ':' after constructor callee")?;
            let fields = self.parse_new_expression_field_block()?;
            let span = Span::new(start_span.start, self.previous_token().span.end);
            Ok(Expr::Constructor {
                callee: Box::new(callee),
                fields,
                span,
                id: self.next_node_id(),
            })
        } else {
            let span = Span::new(start_span.start, callee.span().end);
            Ok(Expr::Constructor {
                callee: Box::new(callee),
                fields: Vec::new(),
                span,
                id: self.next_node_id(),
            })
        }
    }

    /// Parse the indented `field: value` block inside a constructor.
    fn parse_new_expression_field_block(&mut self) -> ParseResult<Vec<ConstructorField>> {
        let mut fields = Vec::new();
        self.skip_newlines_and_comments();
        self.consume(&TokenType::Indent, "Expected indentation block start")?;
        self.skip_newlines_and_comments();

        while !self.is_at_end() && !self.check(&TokenType::Dedent) {
            let field_token = self.advance().clone();
            let TokenType::Identifier(field_name) = field_token.token_type else {
                return Err(ParseError::UnexpectedToken {
                    expected: "field name".to_owned(),
                    found: format!("{}", field_token.token_type),
                    span: ParseError::span_from_token(&field_token),
                });
            };

            self.consume(&TokenType::Colon, "Expected ':' after field name")?;
            let field_value = self.parse_precedence(Precedence::Assignment)?;
            let field_span = Span::new(field_token.span.start, field_value.span().end);
            fields.push(ConstructorField {
                name: field_name,
                value: field_value,
                span: field_span,
            });

            self.skip_newlines_and_comments();
        }

        self.consume(&TokenType::Dedent, "Expected dedent after indentation block")?;
        Ok(fields)
    }
}

