use super::{next_node_id, ParseError, ParseResult, Parser};
use crate::ast::{AstNode, Expr, LiteralValue, MatchArm, Pattern};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a match expression: `match <scrutinee> { <arm>, ... }`
    ///
    /// The match keyword has already been consumed when this is called.
    /// Parses arms of the form `<pattern> [if <guard>] => <expr>`.
    pub(crate) fn parse_match_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        self.advance();
        let scrutinee = self.parse_expression()?;
        self.consume(
            &TokenType::LeftBrace,
            "Expected '{' to begin match expression arms",
        )?;

        let mut arms = Vec::new();
        while !self.check(&TokenType::RightBrace) {
            let pattern = self.parse_pattern()?;
            let guard = self
                .check(&TokenType::If)
                .then(|| {
                    self.advance();
                    self.parse_expression()
                })
                .transpose()?;

            self.consume(&TokenType::Arrow, "Expected '=>' after match pattern")?;
            let body = self.parse_expression()?;

            let arm_span_start = pattern.span().start;
            let arm_span_end = body.span().end;
            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: Span::new(arm_span_start, arm_span_end),
            });

            if self.check(&TokenType::Comma) {
                self.advance();
                if self.check(&TokenType::RightBrace) {
                    break;
                }
            } else {
                break;
            }
        }

        self.consume(
            &TokenType::RightBrace,
            "Expected '}' to close match expression",
        )?;
        let span = Span::new(start_span.start, self.previous_token().span.end);

        Ok(Expr::Match {
            scrutinee: Box::new(scrutinee),
            arms,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a single match pattern (wildcard, literal, binding, variant, or tuple).
    fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let token = self.current_token().clone();
        match token.token_type {
            TokenType::Identifier(name) => self.parse_identifier_pattern(name, token.span),
            TokenType::IntegerLiteral(value) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: LiteralValue::Integer(value),
                    span: token.span,
                })
            }
            TokenType::FloatLiteral(value) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: LiteralValue::Float(value),
                    span: token.span,
                })
            }
            TokenType::StringLiteral(value) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: LiteralValue::String(value),
                    span: token.span,
                })
            }
            TokenType::BooleanLiteral(value) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: LiteralValue::Boolean(value),
                    span: token.span,
                })
            }
            TokenType::LeftParen => self.parse_tuple_pattern(token.span),
            _ => Err(ParseError::UnexpectedToken {
                expected: "match pattern".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(&token),
            }),
        }
    }

    /// Parse a pattern that begins with an identifier: binding, variant, or wildcard `_`.
    fn parse_identifier_pattern(&mut self, name: String, start_span: Span) -> ParseResult<Pattern> {
        self.advance();

        if name == "_" {
            return Ok(Pattern::Wildcard { span: start_span });
        }

        if self.check(&TokenType::Dot) {
            self.advance();
            let variant_token = self.advance().clone();
            let TokenType::Identifier(variant_name) = variant_token.token_type else {
                return Err(ParseError::UnexpectedToken {
                    expected: "variant name after '.' in pattern".to_owned(),
                    found: format!("{}", variant_token.token_type),
                    span: ParseError::span_from_token(&variant_token),
                });
            };

            let fields = if self.check(&TokenType::LeftBrace) {
                self.parse_variant_pattern_fields()?
            } else {
                Vec::new()
            };

            return Ok(Pattern::Variant {
                type_name: Some(name),
                variant_name,
                fields,
                span: Span::new(start_span.start, self.previous_token().span.end),
            });
        }

        if self.check(&TokenType::LeftBrace) {
            let fields = self.parse_variant_pattern_fields()?;
            return Ok(Pattern::Variant {
                type_name: None,
                variant_name: name,
                fields,
                span: Span::new(start_span.start, self.previous_token().span.end),
            });
        }

        Ok(Pattern::Binding {
            name,
            span: start_span,
        })
    }

    /// Parse a tuple pattern: `(<pattern>, <pattern>, ...)`.
    fn parse_tuple_pattern(&mut self, start_span: Span) -> ParseResult<Pattern> {
        self.advance();
        let mut elements = Vec::new();

        if !self.check(&TokenType::RightParen) {
            loop {
                elements.push(self.parse_pattern()?);
                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.consume(
            &TokenType::RightParen,
            "Expected ')' to close tuple pattern",
        )?;
        Ok(Pattern::Tuple {
            elements,
            span: Span::new(start_span.start, self.previous_token().span.end),
        })
    }

    /// Parse the fields of a variant pattern: `{ field: pattern, ... }`.
    fn parse_variant_pattern_fields(&mut self) -> ParseResult<Vec<(Option<String>, Pattern)>> {
        self.consume(
            &TokenType::LeftBrace,
            "Expected '{' to start variant pattern fields",
        )?;

        let mut fields = Vec::new();
        while !self.check(&TokenType::RightBrace) {
            if self.check_identifier() {
                let checkpoint = self.current;
                let name_token = self.advance().clone();
                if self.check(&TokenType::Colon) {
                    self.advance();
                    let field_pattern = self.parse_pattern()?;
                    if let TokenType::Identifier(field_name) = name_token.token_type {
                        fields.push((Some(field_name), field_pattern));
                    }
                } else {
                    self.current = checkpoint;
                    fields.push((None, self.parse_pattern()?));
                }
            } else {
                fields.push((None, self.parse_pattern()?));
            }

            if self.check(&TokenType::Comma) {
                self.advance();
            } else {
                break;
            }
        }

        self.consume(
            &TokenType::RightBrace,
            "Expected '}' to close variant pattern fields",
        )?;

        Ok(fields)
    }
}
