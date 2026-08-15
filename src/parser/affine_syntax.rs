//! Contextual parser helpers for Task 7 affine and refinement proposal syntax.

use super::{ParseError, ParseResult, Parser, Precedence};
use crate::ast::{AstNode, BinaryOp, Expr};
use crate::error::LexError;
use crate::token::{Span, TokenType};

impl Parser {
    /// Return true when the token at `current + offset` is an identifier.
    pub(super) fn next_token_is_identifier(&self, offset: usize) -> bool {
        self.tokens
            .get(self.current.saturating_add(offset))
            .is_some_and(|token| matches!(token.token_type, TokenType::Identifier(_)))
    }

    /// Return true when the token at `current + offset` is a specific contextual keyword.
    pub(super) fn next_token_identifier_is(&self, offset: usize, expected: &str) -> bool {
        self.tokens
            .get(self.current.saturating_add(offset))
            .is_some_and(|token| {
                if let &TokenType::Identifier(ref name) = &token.token_type {
                    name == expected
                } else {
                    false
                }
            })
    }

    /// Return true when a token can start the type target in `constrain Type from value`.
    pub(super) fn token_can_start_constrain_target(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| match token.token_type {
                TokenType::Function
                | TokenType::Int8
                | TokenType::Int16
                | TokenType::Int32
                | TokenType::Int64
                | TokenType::UInt8
                | TokenType::UInt16
                | TokenType::UInt32
                | TokenType::UInt64
                | TokenType::Float32
                | TokenType::Float64
                | TokenType::String
                | TokenType::Boolean
                | TokenType::Void => true,
                TokenType::Identifier(ref name) => {
                    name.chars().next().is_some_and(char::is_uppercase)
                }
                _ => false,
            })
    }

    /// Parse canonical `ref value` and `mutable ref value` argument syntax.
    pub(super) fn parse_borrow_argument_expression(
        &mut self,
        is_mutable: bool,
        start_span: Span,
    ) -> ParseResult<Expr> {
        if is_mutable {
            self.advance();
            if !self.check_contextual_keyword("ref") {
                return Err(ParseError::UnexpectedToken {
                    expected: "'ref' after 'mutable'".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        }

        self.advance();
        if !self.check_identifier() {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier after 'ref'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        let target_token = self.advance().clone();
        let TokenType::Identifier(name) = target_token.token_type else {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier after 'ref'".to_owned(),
                found: format!("{}", target_token.token_type),
                span: ParseError::span_from_token(&target_token),
            });
        };

        Ok(Expr::Identifier {
            name,
            span: Span::new(start_span.start, target_token.span.end),
            id: self.next_node_id(),
        })
    }

    /// Parse `constrain Type from value` as a parser-only constrained value expression.
    pub(super) fn parse_constrain_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        self.advance();
        let target_type = self.parse_type()?;
        self.consume(&TokenType::From, "Expected 'from' after constrained type")?;
        let value = self.parse_precedence(Precedence::Assignment)?;
        let span = Span::new(start_span.start, value.span().end);

        Ok(Expr::Cast {
            expr: Box::new(value),
            target_type,
            span,
            id: self.next_node_id(),
        })
    }

    /// Finish parsing `is` and consume optional valid `into payload` refinement syntax.
    pub(super) fn finish_is_expression(
        &mut self,
        left: Expr,
        operator: BinaryOp,
        right: Expr,
    ) -> ParseResult<Expr> {
        let start_span = left.span();
        let end_span = if self.check(&TokenType::Into) {
            if !matches!(left, Expr::Identifier { .. }) {
                return Err(ParseError::InvalidSyntax {
                    message:
                        "refinement 'into' requires a direct identifier on the left side of 'is'"
                            .to_owned(),
                    span: LexError::span_from_span(left.span()),
                });
            }
            if !Self::is_nominal_variant_refinement_target(&right) {
                return Err(ParseError::InvalidSyntax {
                    message: "refinement 'into' requires a nominal Type.Variant on the right side of 'is'"
                        .to_owned(),
                    span: LexError::span_from_span(right.span()),
                });
            }

            self.advance();
            if !self.check_identifier() {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier after 'into'".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
            self.advance().span
        } else {
            right.span()
        };

        Ok(Expr::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
            span: Span::new(start_span.start, end_span.end),
            id: self.next_node_id(),
        })
    }

    /// Return true when an `is` right-hand expression has nominal `Type.Variant` shape.
    fn is_nominal_variant_refinement_target(expr: &Expr) -> bool {
        if let &Expr::Member {
            ref object,
            ref member,
            ..
        } = expr
        {
            !member.is_empty() && matches!(*object.as_ref(), Expr::Identifier { .. })
        } else {
            false
        }
    }
}
