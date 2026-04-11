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
    AstNode, BinaryOp, ConstructorField, Expr, HotReloadMetadata, LambdaBody, LiteralValue, Stmt,
    StringPart, UnaryOp,
};
use crate::error::LexError;
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse an expression using Pratt parsing
    pub(super) fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(Precedence::None)
    }

    /// Parse expression with given precedence (Pratt parser)
    pub(super) fn parse_precedence(&mut self, precedence: Precedence) -> ParseResult<Expr> {
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

        if self.check(&TokenType::LeftBrace)
            && matches!(expr, Expr::Identifier { .. } | Expr::Member { .. })
            && self.constructor_field_list_starts_here()
        {
            expr = self.parse_constructor_suffix(expr)?;
        }

        Ok(expr)
    }

    /// Return true when the current `{` token starts a constructor field list.
    fn constructor_field_list_starts_here(&self) -> bool {
        if !self.check(&TokenType::LeftBrace) {
            return false;
        }

        let Some(next_token) = self.tokens.get(self.current.saturating_add(1)) else {
            return false;
        };
        let Some(next_after_identifier) = self.tokens.get(self.current.saturating_add(2)) else {
            return false;
        };

        matches!(
            (&next_token.token_type, &next_after_identifier.token_type),
            (&TokenType::Identifier(_), &TokenType::Colon)
        )
    }

    /// Parse the `{ field: value, ... }` constructor suffix following a callee expression.
    ///
    /// Called when a `{` is encountered in postfix position after a member expression,
    /// producing an `Expr::Constructor` node with the accumulated named fields.
    fn parse_constructor_suffix(&mut self, left: Expr) -> ParseResult<Expr> {
        self.advance();
        let mut fields = Vec::new();

        if !self.check(&TokenType::RightBrace) {
            loop {
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

                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        self.consume(
            &TokenType::RightBrace,
            "Expected '}' after constructor fields",
        )?;
        let span = Span::new(left.span().start, self.previous_token().span.end);

        Ok(Expr::Constructor {
            callee: Box::new(left),
            fields,
            span,
            id: next_node_id(),
        })
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
            &TokenType::Int8 => Ok(self.parse_identifier("int8".to_owned(), span)),
            &TokenType::Int16 => Ok(self.parse_identifier("int16".to_owned(), span)),
            &TokenType::Int32 => Ok(self.parse_identifier("int32".to_owned(), span)),
            &TokenType::Int64 => Ok(self.parse_identifier("int64".to_owned(), span)),
            &TokenType::UInt8 => Ok(self.parse_identifier("uint8".to_owned(), span)),
            &TokenType::UInt16 => Ok(self.parse_identifier("uint16".to_owned(), span)),
            &TokenType::UInt32 => Ok(self.parse_identifier("uint32".to_owned(), span)),
            &TokenType::UInt64 => Ok(self.parse_identifier("uint64".to_owned(), span)),
            &TokenType::Float32 => Ok(self.parse_identifier("float32".to_owned(), span)),
            &TokenType::Float64 => Ok(self.parse_identifier("float64".to_owned(), span)),
            &TokenType::String => Ok(self.parse_identifier("string".to_owned(), span)),
            &TokenType::Boolean => Ok(self.parse_identifier("boolean".to_owned(), span)),
            &TokenType::LeftParen => self.parse_parenthesized_expression(span),
            &TokenType::Minus | &TokenType::Plus | &TokenType::Not | &TokenType::BitNot => {
                let token_type = token.token_type.clone();
                self.parse_unary_expression(&token_type, span)
            }
            &TokenType::TypeOf => self.parse_type_of_expression(span),
            &TokenType::Function => self.parse_lambda_expression(span),
            &TokenType::If => self.parse_if_expression(span),
            &TokenType::Match => self.parse_match_expression(span),
            &TokenType::Guard => self.parse_guard_expression(span),
            &TokenType::Propagate => self.parse_propagate_expression(span),
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression (literal, identifier, function call, lambda, or parenthesized expression)".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            }),
        }
    }

    /// Parse an if expression: `if <condition> <then-branch> [else <else-branch>]`.
    fn parse_if_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        self.advance();

        let condition = self.parse_expression()?;
        let then_branch = Box::new(self.parse_block_statement()?);

        let else_branch = self
            .check(&TokenType::Else)
            .then(|| {
                self.advance();
                self.parse_statement().map(Box::new)
            })
            .transpose()?;

        let end_span = else_branch
            .as_ref()
            .map_or_else(|| then_branch.span(), |else_stmt| else_stmt.span());

        Ok(Expr::If {
            condition: Box::new(condition),
            then_branch,
            else_branch,
            span: Span::new(start_span.start, end_span.end),
            id: next_node_id(),
        })
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
        let generic_constraints = if self.check(&TokenType::Less) {
            Some(self.parse_type_parameter_declarations()?)
        } else {
            None
        };
        let generic_params = generic_constraints.as_ref().map(|declarations| {
            declarations
                .iter()
                .map(|declaration| declaration.name.clone())
                .collect::<Vec<String>>()
        });

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
            generic_constraints,
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
    #[expect(
        clippy::too_many_lines,
        reason = "Infix parsing keeps all precedence branches localized for maintainability"
    )]
    fn parse_infix(&mut self, left: Expr) -> ParseResult<Expr> {
        let token = self.current_token().clone();

        match token.token_type {
            TokenType::Less => {
                if let Some(generic_call_result) =
                    self.try_parse_explicit_generic_call(left.clone())
                {
                    return generic_call_result;
                }

                let operator = BinaryOp::try_from(TokenType::Less).map_err(|_original_error| {
                    ParseError::InvalidSyntax {
                        message: "Invalid binary operator: <".to_owned(),
                        span: ParseError::span_from_token(&token),
                    }
                })?;
                let precedence = Precedence::from_token(&TokenType::Less);
                self.advance();
                let right = self.parse_precedence(precedence.next())?;
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
            TokenType::Plus
            | TokenType::Minus
            | TokenType::Multiply
            | TokenType::Divide
            | TokenType::Modulo
            | TokenType::Power
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Is
            | TokenType::IsNot
            | TokenType::And
            | TokenType::Or
            | TokenType::Xor
            | TokenType::BitAnd
            | TokenType::BitOr
            | TokenType::BitXor
            | TokenType::BitShiftLeft
            | TokenType::BitShiftRight
            | TokenType::BitUnsignedShiftRight => {
                let operator =
                    BinaryOp::try_from(token.token_type.clone()).map_err(|_original_error| {
                        ParseError::InvalidSyntax {
                            message: format!("Invalid binary operator: {}", token.token_type),
                            span: ParseError::span_from_token(&token),
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
            TokenType::LeftParen => {
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
                    generic_args: None,
                    args,
                    span: call_span,
                    id: next_node_id(),
                })
            }
            TokenType::Dot => {
                self.advance();
                let member_token = self.advance().clone();
                let TokenType::Identifier(member) = member_token.token_type else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier after '.'".to_owned(),
                        found: format!("{}", member_token.token_type),
                        span: ParseError::span_from_token(&member_token),
                    });
                };

                let span = Span::new(left.span().start, member_token.span.end);
                Ok(Expr::Member {
                    object: Box::new(left),
                    member,
                    span,
                    id: next_node_id(),
                })
            }
            _ => Ok(left),
        }
    }

    /// Try to parse an explicit generic call of the form `callee<T1, T2>(...)`.
    ///
    /// Returns `None` when the current token stream does not represent a valid
    /// explicit generic call after `left`, allowing the caller to treat `<` as
    /// a normal binary operator instead.
    fn try_parse_explicit_generic_call(&mut self, left: Expr) -> Option<ParseResult<Expr>> {
        let checkpoint = self.current;
        if !self.check(&TokenType::Less) {
            return None;
        }

        self.advance();
        if self.check(&TokenType::Greater) {
            self.current = checkpoint;
            return None;
        }

        let mut generic_args = Vec::new();
        let Ok(first_type) = self.parse_type() else {
            self.current = checkpoint;
            return None;
        };
        generic_args.push(first_type);

        loop {
            if self.check(&TokenType::Comma) {
                self.advance();
                let Ok(next_type) = self.parse_type() else {
                    self.current = checkpoint;
                    return None;
                };
                generic_args.push(next_type);
                continue;
            }
            break;
        }

        if !self.check(&TokenType::Greater) {
            self.current = checkpoint;
            return None;
        }
        self.advance();

        if !self.check(&TokenType::LeftParen) {
            self.current = checkpoint;
            return None;
        }
        self.advance();

        let mut args = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                match self.parse_precedence(Precedence::Assignment) {
                    Ok(argument_expr) => args.push(argument_expr),
                    Err(parse_error) => return Some(Err(parse_error)),
                }

                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        if let Err(parse_error) =
            self.consume(&TokenType::RightParen, "Expected ')' after arguments")
        {
            return Some(Err(parse_error));
        }

        let start_span = left.span();
        let end_span = self.previous_token().span;
        let call_span = Span::new(start_span.start, end_span.end);
        Some(Ok(Expr::Call {
            callee: Box::new(left),
            generic_args: Some(generic_args),
            args,
            span: call_span,
            id: next_node_id(),
        }))
    }
}
