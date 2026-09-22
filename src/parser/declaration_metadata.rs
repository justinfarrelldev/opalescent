//! Parser helpers for proposal declaration metadata and extended type declaration forms.

extern crate alloc;

use super::{ParseError, ParseResult, Parser};
use crate::ast::{Decl, DeclarationAnnotation, TypeConstraint, TypeDeclarationForm};
use crate::token::{Span, TokenType};
use alloc::string::String;
use alloc::vec::Vec;

impl Parser {
    /// Check if the current token can start a type annotation.
    pub(super) fn is_type_start(&self) -> bool {
        if self.is_type_keyword() || self.check(&TokenType::Function) {
            return true;
        }
        if let TokenType::Identifier(ref name) = self.current_token().token_type {
            return name.chars().next().is_some_and(char::is_uppercase);
        }
        false
    }

    /// Check whether the current token ends a type declaration body.
    pub(super) fn is_type_body_terminator(&self) -> bool {
        if self.is_at_end() || self.check(&TokenType::Dedent) {
            return true;
        }

        let token = self.current_token();
        if token.span.start.column != 1 {
            return false;
        }

        match token.token_type {
            TokenType::At
            | TokenType::DocComment(_)
            | TokenType::Type
            | TokenType::Function
            | TokenType::Import
            | TokenType::Public
            | TokenType::Entry
            | TokenType::Let => true,
            TokenType::Identifier(ref name) => name == "namespace" || name == "error",
            _ => false,
        }
    }

    /// Check whether a token at an index is a specific contextual keyword.
    fn token_identifier_is(&self, index: usize, expected: &str) -> bool {
        self.tokens.get(index).is_some_and(
            |token| matches!(&token.token_type, &TokenType::Identifier(ref name) if name == expected),
        )
    }

    /// Check whether a token at an index is the built-in `type` keyword.
    fn token_is_type_keyword_at(&self, index: usize) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| matches!(token.token_type, TokenType::Type))
    }

    /// Check whether the current token is a contextual keyword.
    pub(super) fn check_contextual_keyword(&self, expected: &str) -> bool {
        self.token_identifier_is(self.current, expected)
    }

    /// Consume a contextual keyword and return its span.
    fn consume_contextual_keyword(&mut self, expected: &str) -> ParseResult<Span> {
        if self.check_contextual_keyword(expected) {
            let span = self.current_token().span;
            self.advance();
            Ok(span)
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("'{expected}'"),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            })
        }
    }

    /// Check whether the current tokens start any supported type declaration form.
    pub(super) fn starts_type_declaration_form(&self) -> bool {
        self.check(&TokenType::Type)
            || (self.token_identifier_is(self.current, "constrained")
                && self.token_is_type_keyword_at(self.current + 1))
            || (self.token_identifier_is(self.current, "non_exhaustive")
                && self.token_is_type_keyword_at(self.current + 1))
            || (self.token_identifier_is(self.current, "opaque")
                && self.token_identifier_is(self.current + 1, "immutable")
                && self.token_is_type_keyword_at(self.current + 2))
            || (self.token_identifier_is(self.current, "compiler_registered")
                && self.token_identifier_is(self.current + 1, "affine")
                && self.token_identifier_is(self.current + 2, "resource")
                && self.token_is_type_keyword_at(self.current + 3))
    }

    /// Check whether the current tokens start a named error-set declaration.
    pub(super) fn starts_error_set_declaration(&self) -> bool {
        self.token_identifier_is(self.current, "error")
            && self.token_identifier_is(self.current + 1, "set")
    }

    /// Parse the declaration-form prefix that appears before the `type` keyword.
    pub(super) fn parse_type_declaration_form(
        &mut self,
    ) -> ParseResult<(TypeDeclarationForm, Span)> {
        let start_span = self.current_token().span;
        if self.check(&TokenType::Type) {
            return Ok((TypeDeclarationForm::Nominal, start_span));
        }

        if self.check_contextual_keyword("constrained") {
            self.consume_contextual_keyword("constrained")?;
            return Ok((TypeDeclarationForm::Constrained, start_span));
        }

        if self.check_contextual_keyword("non_exhaustive") {
            self.consume_contextual_keyword("non_exhaustive")?;
            return Ok((TypeDeclarationForm::NonExhaustive, start_span));
        }

        if self.check_contextual_keyword("opaque") {
            self.consume_contextual_keyword("opaque")?;
            self.consume_contextual_keyword("immutable")?;
            return Ok((TypeDeclarationForm::OpaqueImmutable, start_span));
        }

        if self.check_contextual_keyword("compiler_registered") {
            self.consume_contextual_keyword("compiler_registered")?;
            self.consume_contextual_keyword("affine")?;
            self.consume_contextual_keyword("resource")?;
            return Ok((
                TypeDeclarationForm::CompilerRegisteredAffineResource,
                start_span,
            ));
        }

        Err(ParseError::UnexpectedToken {
            expected: "type declaration form".to_owned(),
            found: format!("{}", self.current_token().token_type),
            span: ParseError::span_from_token(self.current_token()),
        })
    }

    /// Check whether the current token starts a namespace declaration.
    pub(super) fn is_namespace_declaration_start(&self) -> bool {
        self.check_contextual_keyword("namespace")
    }

    /// Parse a `namespace a.b.c` declaration.
    pub(super) fn parse_namespace_declaration(&mut self) -> ParseResult<Decl> {
        let start_span = self.consume_contextual_keyword("namespace")?;
        let mut path = Vec::new();

        loop {
            let component = match self.current_token().token_type {
                TokenType::Identifier(ref name) => name.clone(),
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "namespace component".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            };
            path.push(component);
            self.advance();

            if self.check(&TokenType::Dot) {
                self.advance();
            } else {
                break;
            }
        }

        let end_span = self.previous_token().span;
        Ok(Decl::Namespace {
            path,
            span: Span::new(start_span.start, end_span.end),
            id: self.next_node_id(),
        })
    }

    /// Parse zero or more declaration annotations before a declaration.
    pub(super) fn parse_declaration_annotations(
        &mut self,
    ) -> ParseResult<Vec<DeclarationAnnotation>> {
        let mut annotations = Vec::new();
        let mut saw_abi_type_id = false;

        while self.check(&TokenType::At) {
            let annotation_start = self.advance().span;
            let name = match self.current_token().token_type {
                TokenType::Identifier(ref name) => name.clone(),
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "annotation name".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            };
            self.advance();
            self.consume(&TokenType::LeftParen, "Expected '(' after annotation name")?;

            let annotation = match name.as_str() {
                "availability" => {
                    let value = self.parse_annotation_identifier_value("availability value")?;
                    if value != "test_only" {
                        return Err(ParseError::InvalidSyntax {
                            message: "@availability only supports test_only".to_owned(),
                            span: ParseError::span_from_token(self.previous_token()),
                        });
                    }
                    self.consume(&TokenType::RightParen, "Expected ')' after annotation")?;
                    DeclarationAnnotation::Availability {
                        value,
                        span: Span::new(annotation_start.start, self.previous_token().span.end),
                    }
                }
                "constructor_visibility" => {
                    let value = self.parse_annotation_identifier_value("constructor visibility")?;
                    if !["runtime", "standard_library", "test_runner"].contains(&value.as_str()) {
                        return Err(ParseError::InvalidSyntax {
                            message: "unsupported @constructor_visibility value".to_owned(),
                            span: ParseError::span_from_token(self.previous_token()),
                        });
                    }
                    self.consume(&TokenType::RightParen, "Expected ')' after annotation")?;
                    DeclarationAnnotation::ConstructorVisibility {
                        value,
                        span: Span::new(annotation_start.start, self.previous_token().span.end),
                    }
                }
                "abi_type_id" => {
                    if saw_abi_type_id {
                        return Err(ParseError::InvalidSyntax {
                            message: "duplicate @abi_type_id annotation on one declaration"
                                .to_owned(),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    }
                    saw_abi_type_id = true;
                    let TokenType::IntegerLiteral(value) = self.current_token().token_type else {
                        return Err(ParseError::UnexpectedToken {
                            expected: "integer ABI type ID".to_owned(),
                            found: format!("{}", self.current_token().token_type),
                            span: ParseError::span_from_token(self.current_token()),
                        });
                    };
                    self.advance();
                    self.consume(&TokenType::RightParen, "Expected ')' after annotation")?;
                    DeclarationAnnotation::AbiTypeId {
                        value,
                        span: Span::new(annotation_start.start, self.previous_token().span.end),
                    }
                }
                "abi_evolution" => {
                    let value = self.parse_annotation_identifier_value("ABI evolution value")?;
                    if ![
                        "closed_major_only",
                        "additive_opaque",
                        "non_exhaustive_additive",
                    ]
                    .contains(&value.as_str())
                    {
                        return Err(ParseError::InvalidSyntax {
                            message: "unsupported @abi_evolution value".to_owned(),
                            span: ParseError::span_from_token(self.previous_token()),
                        });
                    }
                    self.consume(&TokenType::RightParen, "Expected ')' after annotation")?;
                    DeclarationAnnotation::AbiEvolution {
                        value,
                        span: Span::new(annotation_start.start, self.previous_token().span.end),
                    }
                }
                _ => {
                    return Err(ParseError::InvalidSyntax {
                        message: format!("unknown declaration annotation '@{name}'"),
                        span: ParseError::span_from_token(self.previous_token()),
                    });
                }
            };

            annotations.push(annotation);
            self.skip_trivia_preserving_doc_comments();
        }

        Ok(annotations)
    }

    /// Parse an identifier value inside an annotation argument list.
    fn parse_annotation_identifier_value(&mut self, expected: &str) -> ParseResult<String> {
        match self.current_token().token_type {
            TokenType::Identifier(ref value) => {
                let value = value.clone();
                self.advance();
                Ok(value)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: expected.to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            }),
        }
    }

    /// Parse the `where` predicate on a constrained type declaration.
    pub(super) fn parse_where_constraint(&mut self) -> ParseResult<TypeConstraint> {
        let where_span = self.consume_contextual_keyword("where")?;
        if self.is_at_end() || self.check(&TokenType::Newline) || self.check(&TokenType::Dedent) {
            return Err(ParseError::UnexpectedToken {
                expected: "constraint expression after where".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        let expression_start_index = self.current;
        self.parse_expression()?;
        let expression_end_index = self.current;
        let end_span = self.previous_token().span;

        Ok(TypeConstraint {
            expression: self.tokens_to_source(expression_start_index, expression_end_index),
            span: Span::new(where_span.start, end_span.end),
        })
    }

    /// Render a slice of consumed tokens back into a compact source-like string.
    fn tokens_to_source(&self, start: usize, end: usize) -> String {
        let mut rendered = String::new();
        for token in &self.tokens[start..end] {
            if matches!(
                token.token_type,
                TokenType::EndOfFile | TokenType::Newline | TokenType::Indent | TokenType::Dedent
            ) {
                continue;
            }
            match token.token_type {
                TokenType::Dot
                | TokenType::Comma
                | TokenType::RightParen
                | TokenType::RightBracket => {
                    rendered.push_str(&token.lexeme);
                }
                TokenType::LeftParen | TokenType::LeftBracket => {
                    if !rendered.is_empty() && !rendered.ends_with(' ') {
                        rendered.push(' ');
                    }
                    rendered.push_str(&token.lexeme);
                }
                _ => {
                    if !rendered.is_empty()
                        && !rendered.ends_with(' ')
                        && !rendered.ends_with('(')
                        && !rendered.ends_with('.')
                    {
                        rendered.push(' ');
                    }
                    rendered.push_str(&token.lexeme);
                }
            }
        }
        rendered
    }
}
