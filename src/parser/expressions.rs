//! Expression parsing functionality for the Opalescent parser
//!
//! This module handles parsing of all expression forms including:
//! - Literals (integers, floats, strings, booleans, void)
//! - Identifiers
//! - Binary and unary operations
//! - Function calls
//! - Array indexing and member access
//! - Type casts and `type_of` expressions
//! - Lambda expressions
//! - String interpolation

use super::{next_node_id, ParseError, ParseResult, Parser, Precedence};
use crate::ast::{
    AstNode, BinaryOp, Expr, HotReloadMetadata, LambdaBody, LiteralValue, Stmt, StringPart, UnaryOp,
};
use crate::error::LexError;
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse an expression using Pratt parsing
    pub(super) fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(Precedence::None)
    }

    /// Parse expression with given precedence (Pratt parser)
    fn parse_precedence(&mut self, precedence: Precedence) -> ParseResult<Expr> {
        // Parse prefix expression
        let mut expr = self.parse_primary()?;

        // Parse infix expressions
        while !self.is_at_end() {
            let token_precedence = Precedence::from_token(&self.current_token().token_type);

            // Break if the current token has lower precedence or is not an infix operator
            if precedence > token_precedence || token_precedence == Precedence::None {
                break;
            }

            expr = self.parse_infix(expr)?;
        }

        Ok(expr)
    }

    /// Parse primary expressions (literals, identifiers, parenthesized)
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let token = self.current_token();
        let span = token.span;

        match &token.token_type {
            &TokenType::IntegerLiteral(value) => Ok(self.parse_integer_literal(value, span)),
            &TokenType::FloatLiteral(value) => Ok(self.parse_float_literal(value, span)),
            &TokenType::StringLiteral(ref value) => self.parse_string_literal(value.clone(), span),
            &TokenType::BooleanLiteral(value) => Ok(self.parse_boolean_literal(value, span)),
            &TokenType::Void => Ok(self.parse_void_literal(span)),
            &TokenType::Identifier(ref name) => Ok(self.parse_identifier(name.clone(), span)),
            &TokenType::LeftParen => self.parse_parenthesized_expression(span),
            &TokenType::Minus | &TokenType::Plus | &TokenType::Not | &TokenType::BitNot => {
                let token_type = token.token_type.clone();
                self.parse_unary_expression(&token_type, span)
            }
            &TokenType::TypeOf => self.parse_type_of_expression(span),
            &TokenType::Function => self.parse_lambda_expression(span),
            &TokenType::Guard => self.parse_guard_expression(span),
            &TokenType::Propagate => self.parse_propagate_expression(span),
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression (literal, identifier, function call, lambda, or parenthesized expression)".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            }),
        }
    }

    /// Parse integer literal expressions
    fn parse_integer_literal(&mut self, value: i64, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Integer(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse a guard expression: `guard <expr> into <name> [: Type] [mutable] else <handler>`
    fn parse_guard_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        // consume 'guard'
        self.advance();

        // Parse the guarded expression with assignment precedence (safest general case)
        let guarded_expr = self.parse_precedence(Precedence::Assignment)?;

        // Expect 'into'
        if !self.check(&TokenType::Into) {
            let error = ParseError::GuardMissingIntoClause {
                span: ParseError::span_from_token(self.current_token()),
            };
            self.recover_guard_clause();
            return Err(error);
        }
        self.advance();

        // Parse binding name
        let (binding_name, _name_span) = if self.check_identifier() {
            let tok = self.advance().clone();
            if let TokenType::Identifier(n) = tok.token_type {
                (n, tok.span)
            } else {
                unreachable!("check_identifier ensured Identifier")
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Optional type annotation using bool::then + transpose to satisfy clippy pedantic
        let binding_type = self
            .check(&TokenType::Colon)
            .then(|| {
                self.advance();
                self.parse_type()
            })
            .transpose()?;

        // Optional 'mutable' keyword after binding (guard-specific syntax)
        // Use short-circuiting to only advance when the token is present
        let is_mutable = self.check(&TokenType::Mutable) && {
            self.advance();
            true
        };

        // Expect 'else'
        if !self.check(&TokenType::Else) {
            let error = ParseError::GuardMissingElseClause {
                span: ParseError::span_from_token(self.current_token()),
            };
            self.recover_guard_clause();
            return Err(error);
        }
        self.advance();

        // Handler can be a block or a single expression wrapped into a statement
        let else_stmt: Stmt = if self.check(&TokenType::LeftBrace) {
            self.parse_block_statement()?
        } else {
            let expr = self.parse_expression()?;
            let span = expr.span();
            Stmt::Expression {
                expr,
                span,
                id: next_node_id(),
            }
        };

        let end_span = match else_stmt {
            Stmt::Block { span, .. } => span,
            _ => else_stmt.span(),
        };
        let full_span = Span::new(start_span.start, end_span.end);

        Ok(Expr::Guard {
            expr: Box::new(guarded_expr),
            binding_name,
            binding_type,
            is_mutable,
            else_branch: Box::new(else_stmt),
            span: full_span,
            id: next_node_id(),
        })
    }

    /// Recover parser position after a malformed guard clause.
    ///
    /// This method advances the token stream until a likely statement boundary so
    /// that subsequent parsing can resume without cascading errors. We stop at
    /// semicolons, block terminators, or newline sentinels, matching the
    /// language's statement separators.
    fn recover_guard_clause(&mut self) {
        while !self.is_at_end() {
            match self.current_token().token_type {
                TokenType::RightBrace | TokenType::Newline | TokenType::EndOfFile => break,
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Parse a propagate expression: `propagate <call_expr>`
    fn parse_propagate_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        // consume 'propagate'
        self.advance();

        // Parse the inner expression and validate it's a call
        let inner = self.parse_expression()?;
        match inner {
            Expr::Call { .. } => {
                let end_span = inner.span();
                let span = Span::new(start_span.start, end_span.end);
                Ok(Expr::Propagate {
                    call: Box::new(inner),
                    span,
                    id: next_node_id(),
                })
            }
            _ => Err(ParseError::InvalidSyntax {
                message: "'propagate' must be followed by a function call expression".to_owned(),
                span: ParseError::span_from_token(self.previous_token()),
            }),
        }
    }

    /// Parse float literal expressions
    fn parse_float_literal(&mut self, value: f64, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Float(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse string literal expressions and string interpolation
    fn parse_string_literal(&mut self, value: String, span: Span) -> ParseResult<Expr> {
        self.advance();

        // Check if the string contains interpolation syntax
        if value.contains('{') {
            Self::parse_string_interpolation(&value, span)
        } else {
            Ok(Expr::Literal {
                value: LiteralValue::String(value),
                span,
                id: next_node_id(),
            })
        }
    }

    /// Parse string interpolation expressions ('Hello {world}')
    fn parse_string_interpolation(value: &str, span: Span) -> ParseResult<Expr> {
        let mut parts = Vec::new();
        let mut current_str = String::new();
        let mut chars = value.chars();

        while let Some(ch) = chars.next() {
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

                for expr_ch in chars.by_ref() {
                    match expr_ch {
                        '{' => {
                            brace_count = brace_count.checked_add(1_i32).ok_or_else(|| {
                                ParseError::InvalidSyntax {
                                    message: "Too many nested braces in string interpolation"
                                        .to_owned(),
                                    span: LexError::span_from_span(span),
                                }
                            })?;
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
                        }
                        _ => {}
                    }
                    expr_str.push(expr_ch);
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

                // Create a mini lexer/parser for the expression
                let expr_lexer = crate::lexer::Lexer::new(&expr_str);
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
            } else {
                current_str.push(ch);
            }
        }

        // Add any remaining string content
        parts.push(StringPart::Literal(current_str));

        Ok(Expr::StringInterpolation {
            parts,
            span,
            id: next_node_id(),
        })
    }

    /// Parse boolean literal expressions
    fn parse_boolean_literal(&mut self, value: bool, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Boolean(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse void literal expressions
    fn parse_void_literal(&mut self, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Void,
            span,
            id: next_node_id(),
        }
    }

    /// Parse identifier expressions
    fn parse_identifier(&mut self, name: String, span: Span) -> Expr {
        self.advance();
        Expr::Identifier {
            name,
            span,
            id: next_node_id(),
        }
    }

    /// Parse parenthesized expressions
    fn parse_parenthesized_expression(&mut self, span: Span) -> ParseResult<Expr> {
        self.advance();
        let expr = self.parse_precedence(Precedence::Assignment)?;
        self.consume(&TokenType::RightParen, "Expected ')' after expression")?;

        let end_span = self.previous_token().span;
        let paren_span = Span::new(span.start, end_span.end);

        Ok(Expr::Parenthesized {
            expr: Box::new(expr),
            span: paren_span,
            id: next_node_id(),
        })
    }

    /// Parse unary expressions
    fn parse_unary_expression(&mut self, token_type: &TokenType, span: Span) -> ParseResult<Expr> {
        let operator = UnaryOp::try_from(token_type.clone()).map_err(|_original_error| {
            ParseError::InvalidSyntax {
                message: format!("Invalid unary operator: {token_type}"),
                span: LexError::span_from_span(span),
            }
        })?;
        self.advance();
        let operand = self.parse_precedence(Precedence::Unary)?;

        let end_span = operand.span();
        let unary_span = Span::new(span.start, end_span.end);

        Ok(Expr::Unary {
            operator,
            operand: Box::new(operand),
            span: unary_span,
            id: next_node_id(),
        })
    }

    /// Parse `type_of` expressions
    fn parse_type_of_expression(&mut self, span: Span) -> ParseResult<Expr> {
        self.advance(); // consume 'type_of'

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'type_of'")?;

        // Parse the expression inside
        let expr = self.parse_expression()?;

        // Expect ')'
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after type_of expression",
        )?;

        let end_span = self.previous_token().span;
        let type_of_span = Span::new(span.start, end_span.end);

        Ok(Expr::TypeOf {
            expr: Box::new(expr),
            span: type_of_span,
            id: next_node_id(),
        })
    }

    /// Parse lambda expressions (f(x: T): U => expr, f<T, U>(x: T): U => block)
    fn parse_lambda_expression(&mut self, span: Span) -> ParseResult<Expr> {
        self.advance(); // consume 'f'

        // Parse optional generic parameters (<T, U>)
        #[expect(
            clippy::if_then_some_else_none,
            reason = "Result type makes bool::then inappropriate"
        )]
        let generic_params = if self.check(&TokenType::Less) {
            Some(self.parse_lambda_generic_parameters()?)
        } else {
            None
        };

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'f'")?;

        // Parse parameters (support zero or more)
        let params = self.parse_parameter_list()?;

        // Expect ')'
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after lambda parameters",
        )?;

        // Expect ':'
        self.consume(&TokenType::Colon, "Expected ':' after lambda parameters")?;

        let mut return_types = Vec::new();
        return_types.push(self.parse_type()?);

        while self.check(&TokenType::Comma) {
            self.advance();
            return_types.push(self.parse_type()?);
        }

        // Parse optional errors clause
        let error_types = self.parse_error_types_clause()?;

        // Expect '=>'
        self.consume(&TokenType::Arrow, "Expected '=>' after lambda return type")?;

        // Parse lambda body
        let body = self.parse_lambda_body()?;

        let end_span = self.previous_token().span;
        let lambda_span = Span::new(span.start, end_span.end);

        Ok(Expr::Lambda {
            generic_params,
            params,
            return_types,
            error_types,
            body,
            captured_variables: Vec::new(), // TODO: Implement closure capture analysis
            metadata: Box::new(HotReloadMetadata::for_expression()),
            span: lambda_span,
            id: next_node_id(),
        })
    }

    /// Parse generic parameters for lambda expressions (<T, U>)
    fn parse_lambda_generic_parameters(&mut self) -> ParseResult<Vec<String>> {
        self.advance(); // consume '<'

        let mut generic_params = Vec::new();

        // Handle empty generic arguments (error case)
        if self.check(&TokenType::Greater) {
            return Err(ParseError::InvalidSyntax {
                message: "Empty generic parameter list".to_owned(),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        // Parse comma-separated generic parameter names
        loop {
            if self.check_identifier() {
                let token = self.advance();
                if let &TokenType::Identifier(ref name) = &token.token_type {
                    generic_params.push(name.clone());
                } else {
                    return Err(ParseError::InvalidSyntax {
                        message: "Expected identifier for generic parameter".to_owned(),
                        span: ParseError::span_from_token(token),
                    });
                }
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "generic parameter name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

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

        // Expect '>'
        self.consume(&TokenType::Greater, "Expected '>' after generic parameters")?;

        Ok(generic_params)
    }

    /// Parse lambda body (expression or block)
    fn parse_lambda_body(&mut self) -> ParseResult<LambdaBody> {
        // Check if this is a block body (starts with '{') or a single expression
        if self.check(&TokenType::LeftBrace) {
            // Use existing block parsing for consistency with function bodies
            let block_stmt = self.parse_block_statement()?;
            if let Stmt::Block { statements, .. } = block_stmt {
                Ok(LambdaBody::Block(statements))
            } else {
                // This should never happen since parse_block_statement always returns Block
                Err(ParseError::InvalidSyntax {
                    message: "Expected block statement from parse_block_statement".to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                })
            }
        } else if self.check(&TokenType::Newline)
            || self.check(&TokenType::Return)
            || self.check(&TokenType::Let)
            || self.check(&TokenType::If)
            || self.check(&TokenType::For)
            || self.check(&TokenType::While)
            || self.check(&TokenType::Loop)
            || self.check(&TokenType::Break)
            || self.check(&TokenType::Continue)
            || self.check(&TokenType::Guard)
        {
            let statements = self.parse_blockless_body_statements();
            Ok(LambdaBody::Block(statements))
        } else {
            // Parse as single expression
            let expr = self.parse_expression()?;
            Ok(LambdaBody::Expression(Box::new(expr)))
        }
    }

    /// Parse infix expressions (binary operations, calls, etc.)
    fn parse_infix(&mut self, left: Expr) -> ParseResult<Expr> {
        let token = self.current_token();

        match &token.token_type {
            &TokenType::Plus
            | &TokenType::Minus
            | &TokenType::Multiply
            | &TokenType::Divide
            | &TokenType::Modulo
            | &TokenType::Power
            | &TokenType::Less
            | &TokenType::LessEqual
            | &TokenType::Greater
            | &TokenType::GreaterEqual
            | &TokenType::Is
            | &TokenType::IsNot
            | &TokenType::And
            | &TokenType::Or
            | &TokenType::Xor
            | &TokenType::BitAnd
            | &TokenType::BitOr
            | &TokenType::BitXor
            | &TokenType::BitShiftLeft
            | &TokenType::BitShiftRight
            | &TokenType::BitUnsignedShiftRight => {
                let operator =
                    BinaryOp::try_from(token.token_type.clone()).map_err(|_original_error| {
                        ParseError::InvalidSyntax {
                            message: format!("Invalid binary operator: {}", token.token_type),
                            span: ParseError::span_from_token(token),
                        }
                    })?;
                let precedence = Precedence::from_token(&token.token_type);
                self.advance();

                // For left-associative operators, use one level higher precedence for right side
                let right_precedence = if matches!(operator, BinaryOp::Power) {
                    // Right-associative: same precedence
                    precedence
                } else {
                    // Left-associative: one level higher precedence to ensure left-to-right grouping
                    precedence.next()
                };

                let right = self.parse_precedence(right_precedence)?;

                let start_span = left.span();
                let end_span = right.span();
                let binary_span = Span::new(start_span.start, end_span.end);

                Ok(Expr::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    span: binary_span,
                    id: next_node_id(),
                })
            }
            &TokenType::LeftParen => {
                // Function call
                self.advance();
                let mut args = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        // Parse arguments with Assignment precedence to avoid infinite recursion
                        args.push(self.parse_precedence(Precedence::Assignment)?);

                        if self.check(&TokenType::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;

                let start_span = left.span();
                let end_span = self.previous_token().span;
                let call_span = Span::new(start_span.start, end_span.end);

                Ok(Expr::Call {
                    callee: Box::new(left),
                    args,
                    span: call_span,
                    id: next_node_id(),
                })
            }
            _ => Ok(left),
        }
    }
}
