//! Type parsing functionality for the Opalescent parser
//!
//! This module handles parsing of all type expressions including:
//! - Basic types (int32, string, boolean, etc.)
//! - Generic types (Array<T>, Result<T, E>)
//! - Array types (T[], int32[][])
//! - Function types (f(int32, string): boolean)

use crate::ast::Type;
use crate::parser::{ParseError, ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a type annotation
    ///
    /// Supports multiple type forms:
    /// - Basic types: `int32`, `string`, `boolean`
    /// - Generic types: `Array<T>`, `Result<T, E>`
    /// - Array types: `int32[]`, `T[][]`
    /// - Function types: `f(int32, string): boolean`
    ///
    /// # Errors
    /// Returns a parse error if the type syntax is invalid or incomplete.
    pub(super) fn parse_type(&mut self) -> ParseResult<Type> {
        let start_span = self.current_token().span;

        // Check for function type syntax: f(param1, param2): return_type
        if self.check(&TokenType::Function) {
            return self.parse_function_type(start_span);
        }

        // Parse the base type name
        let name = match &self.current_token().token_type {
            &TokenType::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                name
            }
            token if token.type_name().is_some() => {
                let name = token.type_name().unwrap_or("unknown").to_owned();
                self.advance();
                name
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "type name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        };

        // Check for generic arguments and create appropriate type
        let current_type = if self.check(&TokenType::Less) {
            self.advance(); // consume '<'

            let mut type_args = Vec::new();

            // Handle empty generic arguments (error case)
            if self.check(&TokenType::Greater) {
                return Err(ParseError::InvalidSyntax {
                    message: "Empty generic argument list".to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

            // Parse comma-separated type arguments
            loop {
                type_args.push(self.parse_type()?);

                if self.check(&TokenType::Comma) {
                    self.advance(); // consume ','
                } else if self.check(&TokenType::Greater) {
                    break;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or '>'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }

            // Consume closing '>'
            self.consume(&TokenType::Greater, "Expected '>' after generic arguments")?;

            let generic_end_span = self.previous_token().span;
            let generic_span = Span::new(start_span.start, generic_end_span.end);

            Type::Generic {
                name,
                type_args,
                span: generic_span,
            }
        } else {
            Type::Basic {
                name,
                span: Span::new(start_span.start, self.previous_token().span.end),
            }
        };

        // Check for array type syntax (type[] or Generic<T>[] or nested like type[][])
        let mut current_type = current_type;
        while self.check(&TokenType::LeftBracket) {
            self.advance(); // consume '['
            self.consume(&TokenType::RightBracket, "Expected ']' after '['")?;

            let array_end_span = self.previous_token().span;
            let array_span = Span::new(start_span.start, array_end_span.end);

            current_type = Type::Array {
                element_type: Box::new(current_type),
                span: array_span,
            };
        }

        Ok(current_type)
    }

    /// Parse a function type: `f(param1, param2): return_type[, return_type...]`
    ///
    /// Function types represent the signature of a function, used for:
    /// - Higher-order function parameters
    /// - Function pointer types
    /// - Lambda expression type annotations
    ///
    /// # Examples
    /// - `f(): void` - No parameters, void return
    /// - `f(int32): boolean` - Single parameter
    /// - `f(int32, string): T` - Multiple parameters
    ///
    /// # Errors
    /// Returns a parse error if the function type syntax is invalid.
    pub(super) fn parse_function_type(&mut self, start_span: Span) -> ParseResult<Type> {
        // Consume the 'f' keyword
        self.advance();

        // Expect opening parenthesis
        self.consume(
            &TokenType::LeftParen,
            "Expected '(' after 'f' in function type",
        )?;

        let mut parameters = Vec::new();

        // Handle empty parameter list: f(): return_type
        if !self.check(&TokenType::RightParen) {
            loop {
                // Parse parameter type
                parameters.push(self.parse_type()?);

                if self.check(&TokenType::Comma) {
                    self.advance(); // consume ','
                } else if self.check(&TokenType::RightParen) {
                    break;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or ')'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }
        }

        // Consume closing parenthesis
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after function parameters",
        )?;

        // Expect colon for return type
        self.consume(&TokenType::Colon, "Expected ':' after function parameters")?;

        let mut return_types = Vec::new();
        return_types.push(self.parse_type()?);

        while self.check(&TokenType::Comma) {
            let next_token = self.tokens.get(self.current.saturating_add(1));
            let token_after_next = self.tokens.get(self.current.saturating_add(2));
            let reaches_parameter_boundary = matches!(
                (
                    next_token.map(|token| &token.token_type),
                    token_after_next.map(|token| &token.token_type)
                ),
                (Some(&TokenType::Identifier(_)), Some(&TokenType::Colon))
            );
            if reaches_parameter_boundary {
                break;
            }

            self.advance();
            return_types.push(self.parse_type()?);
        }

        // Parse optional errors clause for function types
        let errors = if self.check(&TokenType::Errors) {
            self.advance(); // consume 'errors' keyword
            let mut error_types = Vec::new();

            // Parse first error type (required after 'errors' keyword)
            let first_type = self.parse_type()?;
            error_types.push(first_type);

            // Parse additional error types separated by commas
            while self.check(&TokenType::Comma) {
                self.advance(); // consume comma
                let error_type = self.parse_type()?;
                error_types.push(error_type);
            }

            Some(error_types)
        } else {
            None
        };

        let end_span = self.previous_token().span;
        let function_span = Span::new(start_span.start, end_span.end);

        Ok(Type::Function {
            parameters,
            return_types,
            errors,
            span: function_span,
        })
    }
}
