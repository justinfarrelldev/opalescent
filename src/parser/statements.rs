//! Statement parsing functionality for the Opalescent parser
//!
//! This module handles parsing of all statement forms including:
//! - Let statements (variable bindings within functions)
//! - Assignment statements
//! - Return statements
//! - Expression statements
//! - Block statements
//! - Control flow (if, for, while, loop, break, continue)

extern crate alloc;

use crate::ast::{AstNode, Expr, LabeledValue, Stmt};
use crate::error::LexError;
use crate::parser::{next_node_id, ParseError, ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a block delimited by `Indent` and `Dedent` tokens.
    pub(super) fn parse_indent_block(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.consume(&TokenType::Indent, "Expected indentation block start")?;

        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenType::Dedent) && !self.is_at_end() {
            if let &TokenType::DocComment(ref content) = &self.current_token().token_type {
                if self.current_token().span.start.column == 1 {
                    let doc_comment_span = self.current_token().span;
                    self.deferred_doc_comments
                        .push((content.clone(), doc_comment_span));
                    self.advance();
                    self.skip_newlines();
                    continue;
                }

                self.advance();
                self.skip_newlines();
                continue;
            }

            if self.consume_inline_doc_comment() {
                self.skip_newlines();
                continue;
            }

            if let &TokenType::Comment(_) = &self.current_token().token_type {
                if self.current_token().span.start.column == 1 {
                    let comment_token = self.advance().clone();
                    self.deferred_comment_declarations
                        .push(crate::ast::Decl::Comment {
                            text: comment_token.lexeme,
                            span: comment_token.span,
                            id: next_node_id(),
                        });
                    self.skip_newlines();
                    continue;
                }

                let comment_token = self.advance().clone();
                statements.push(Stmt::Comment {
                    text: comment_token.lexeme,
                    span: comment_token.span,
                    id: next_node_id(),
                });
                self.skip_newlines();
                continue;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }

            self.skip_newlines();
        }

        self.consume(
            &TokenType::Dedent,
            "Expected dedent after indentation block",
        )?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Block {
            statements,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a statement
    ///
    /// Dispatches to the appropriate statement parsing method based on
    /// the current token type. Handles all statement forms in the language.
    ///
    /// # Returns
    /// A parsed `Stmt` AST node, or a `ParseError` if the syntax is invalid.
    ///
    /// # Errors
    /// Returns a parse error if the statement syntax is invalid.
    pub(super) fn parse_statement(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines();

        match self.current_token().token_type {
            TokenType::Comment(_) => {
                let comment_token = self.advance().clone();
                Ok(Stmt::Comment {
                    text: comment_token.lexeme,
                    span: comment_token.span,
                    id: next_node_id(),
                })
            }
            TokenType::Let => self.parse_let_statement(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::LeftBrace => self.parse_block_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::For => self.parse_for_statement(),
            TokenType::While => self.parse_while_statement(),
            TokenType::Guard => {
                if self.is_guard_statement_form() {
                    self.parse_guard_statement()
                } else {
                    self.parse_expression_statement()
                }
            }
            TokenType::Loop => self.parse_loop_statement(),
            TokenType::Break => self.parse_break_statement(),
            TokenType::Continue => self.parse_continue_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    /// Return true when the current `guard` token uses statement-form syntax.
    ///
    /// Statement form: `guard <expr> into <ok> else <err> => <indent-body>`
    /// Expression form remains: `guard <expr> into <ok> [: T] [mutable] else <expr-or-block>`.
    fn is_guard_statement_form(&self) -> bool {
        let mut index = self.current.saturating_add(1);

        while let Some(current_token) = self.tokens.get(index) {
            match current_token.token_type {
                TokenType::Else => {
                    let next = self.tokens.get(index.saturating_add(1));
                    let after_next = self.tokens.get(index.saturating_add(2));
                    return next.is_some_and(|next_token| {
                        matches!(&next_token.token_type, &TokenType::Identifier(_))
                    }) && after_next
                        .is_some_and(|after_token| after_token.token_type == TokenType::Arrow);
                }
                TokenType::Newline | TokenType::EndOfFile => return false,
                _ => {
                    index = index.saturating_add(1);
                }
            }
        }

        false
    }

    /// Parse a let statement (variable binding within a function)
    ///
    /// Syntax: `let [mutable] name [: type] [= expr]`
    ///
    /// # Examples
    /// - `let x = 42`
    /// - `let mutable y: int32 = 0`
    /// - `let z: string`  (no initializer)
    ///
    /// # Errors
    /// Returns a parse error if the let statement syntax is invalid.
    pub(super) fn parse_let_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'let'

        // Check for mutable
        let is_mutable = if self.check(&TokenType::Mutable) {
            self.advance();
            true
        } else {
            false
        };

        // Parse variable name
        let (name, name_span) = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                (name.clone(), token.span)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for variable name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        if self.check(&TokenType::Comma) {
            let mut bindings = Vec::new();
            bindings.push(Self::create_let_binding(name, name_span, None, is_mutable));

            while self.check(&TokenType::Comma) {
                self.advance();
                let (next_name, next_span) = if self.check_identifier() {
                    let token = self.advance();
                    if let &TokenType::Identifier(ref value) = &token.token_type {
                        (value.clone(), token.span)
                    } else {
                        return Err(ParseError::InvalidSyntax {
                            message: "Expected identifier for destructured variable name"
                                .to_owned(),
                            span: ParseError::span_from_token(token),
                        });
                    }
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "variable name".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                };

                bindings.push(Self::create_let_binding(
                    next_name, next_span, None, is_mutable,
                ));
            }

            self.consume(
                &TokenType::Assign,
                "Expected '=' after destructured let bindings",
            )?;
            self.skip_newlines_and_comments();
            let initializer = self.parse_expression()?;
            let span = Span::new(start_span.start, initializer.span().end);

            return Ok(Stmt::LetDestructure {
                bindings,
                initializer,
                span,
                id: next_node_id(),
            });
        }

        // Parse optional type annotation
        let type_annotation = self
            .check(&TokenType::Colon)
            .then(|| {
                self.advance();
                self.parse_type()
            })
            .transpose()?;

        // Parse optional initializer
        let initializer = self
            .check(&TokenType::Assign)
            .then(|| {
                self.advance();
                self.skip_newlines_and_comments();
                self.parse_expression()
            })
            .transpose()?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        let binding = Self::create_let_binding(name, name_span, type_annotation, is_mutable);

        Ok(Stmt::Let {
            binding,
            initializer,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a return statement.
    ///
    /// Syntax:
    /// - `return` (void return)
    /// - `return expr`
    /// - `return label1: expr1, label2: expr2`
    ///
    /// # Errors
    ///
    /// Returns a parse error if the return payload syntax is invalid or labels repeat.
    pub(super) fn parse_return_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'return'

        let mut values = Vec::new();
        let mut seen_labels = alloc::collections::BTreeSet::new();

        if !self.check(&TokenType::Newline) && !self.is_at_end() {
            if self.check_identifier() {
                let has_label = self
                    .tokens
                    .get(self.current.saturating_add(1))
                    .is_some_and(|next_token| matches!(next_token.token_type, TokenType::Colon));

                if has_label {
                    loop {
                        let label_token = self.advance().clone();
                        let label = match label_token.token_type.clone() {
                            TokenType::Identifier(label_text) => label_text,
                            other => {
                                return Err(ParseError::UnexpectedToken {
                                    expected: "identifier".to_owned(),
                                    found: format!("{other}"),
                                    span: ParseError::span_from_token(&label_token),
                                });
                            }
                        };

                        if !seen_labels.insert(label.clone()) {
                            return Err(ParseError::DuplicateLabel {
                                label,
                                span: ParseError::span_from_token(&label_token),
                            });
                        }

                        self.consume(&TokenType::Colon, "Expected ':' after return label")?;
                        let expr_value = self.parse_expression()?;
                        let value_span = Span::new(label_token.span.start, expr_value.span().end);
                        values.push(LabeledValue {
                            label,
                            value: expr_value,
                            span: value_span,
                            id: next_node_id(),
                        });

                        if self.check(&TokenType::Comma) {
                            self.advance();
                            continue;
                        }

                        break;
                    }
                } else {
                    let expr_value = self.parse_expression()?;
                    values.push(LabeledValue {
                        label: String::new(),
                        span: expr_value.span(),
                        value: expr_value,
                        id: next_node_id(),
                    });
                }
            } else {
                let expr_value = self.parse_expression()?;
                values.push(LabeledValue {
                    label: String::new(),
                    span: expr_value.span(),
                    value: expr_value,
                    id: next_node_id(),
                });
            }
        }

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Return {
            values,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a block statement
    ///
    /// Syntax: `{ stmt1; stmt2; ... }`
    ///
    /// Block statements create a new scope and can contain zero or more statements.
    /// Statements are separated by newlines or semicolons.
    ///
    /// # Errors
    /// Returns a parse error if the block syntax is invalid or missing the closing brace.
    pub(super) fn parse_block_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.consume(&TokenType::LeftBrace, "Expected '{'")?;

        let mut statements = Vec::new();
        self.skip_newlines();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            if self.consume_inline_doc_comment() {
                self.skip_newlines();
                continue;
            }

            if let &TokenType::Comment(_) = &self.current_token().token_type {
                let comment_token = self.advance().clone();
                statements.push(Stmt::Comment {
                    text: comment_token.lexeme,
                    span: comment_token.span,
                    id: next_node_id(),
                });
                self.skip_newlines();
                continue;
            }

            if self.check(&TokenType::RightBrace) {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }

            self.skip_newlines();
        }

        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Block {
            statements,
            span,
            id: next_node_id(),
        })
    }

    /// Parse an if statement
    ///
    /// Syntax: `if condition { ... } [else { ... }]`
    ///
    /// # Examples
    /// - `if x > 0 { print("positive") }`
    /// - `if x > 0 { return x } else { return -x }`
    /// - `if x > 0 { ... } else if x < 0 { ... } else { ... }`
    ///
    /// # Errors
    /// Returns a parse error if the if statement syntax is invalid.
    pub(super) fn parse_if_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'if'

        // Parse condition
        let condition = self.parse_expression()?;

        let then_branch = if self.check(&TokenType::LeftBrace) {
            Box::new(self.parse_block_statement()?)
        } else if self.check(&TokenType::Colon) {
            self.advance();
            self.skip_newlines();
            Box::new(self.parse_indent_block()?)
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "'{' or ':' after if condition".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse optional else branch
        let else_branch = self
            .check(&TokenType::Else)
            .then(|| {
                self.advance(); // consume 'else'

                if self.check(&TokenType::If) {
                    self.parse_if_statement().map(Box::new)
                } else if self.check(&TokenType::LeftBrace) {
                    self.parse_block_statement().map(Box::new)
                } else if self.check(&TokenType::Colon) {
                    self.advance();
                    self.skip_newlines();
                    self.parse_indent_block().map(Box::new)
                } else {
                    Err(ParseError::UnexpectedToken {
                        expected: "'if', '{', or ':' after 'else'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    })
                }
            })
            .transpose()?;

        let end_span = else_branch
            .as_ref()
            .map_or_else(|| then_branch.span(), |else_stmt| else_stmt.span());

        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a for statement (for-in loop)
    ///
    /// Syntax: `for variable in iterable { ... }`
    ///
    /// # Examples
    /// - `for item in array { print(item) }`
    /// - `for i in range(0, 10) { sum = sum + i }`
    ///
    /// # Errors
    /// Returns a parse error if the for statement syntax is invalid.
    pub(super) fn parse_for_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'for'

        // Parse variable name
        let variable = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                // This should never happen since check_identifier validates the pattern
                debug_assert!(
                    matches!(self.current_token().token_type, TokenType::Identifier(_)),
                    "check_identifier should have validated this is an identifier token"
                );
                String::new() // fallback value
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse 'in' keyword
        self.consume(&TokenType::In, "Expected 'in' after for variable")?;

        // Parse iterable expression
        let iterable = self.parse_expression()?;

        let body = if self.check(&TokenType::LeftBrace) {
            Box::new(self.parse_block_statement()?)
        } else if self.check(&TokenType::Colon) {
            self.advance();
            self.skip_newlines();
            Box::new(self.parse_indent_block()?)
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "'{' or ':' after for loop header".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::For {
            variable,
            iterable,
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a while statement
    ///
    /// Syntax: `while condition { ... }`
    ///
    /// # Examples
    /// - `while x > 0 { x = x - 1 }`
    /// - `while not done { process_next() }`
    ///
    /// # Errors
    /// Returns a parse error if the while statement syntax is invalid.
    pub(super) fn parse_while_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'while'

        // Parse condition
        let condition = self.parse_expression()?;

        let body = if self.check(&TokenType::LeftBrace) {
            Box::new(self.parse_block_statement()?)
        } else if self.check(&TokenType::Colon) {
            self.advance();
            self.skip_newlines();
            Box::new(self.parse_indent_block()?)
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "'{' or ':' after while condition".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::While {
            condition,
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a guard statement.
    ///
    /// Syntax: `guard <expr> into <success_binding> else <error_binding> => <indent-body>`
    pub(super) fn parse_guard_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance();

        let expression = self.parse_expression()?;
        self.consume(&TokenType::Into, "Expected 'into' after guard expression")?;

        let success_binding = if self.check_identifier() {
            let token = self.advance().clone();
            if let TokenType::Identifier(name) = token.token_type {
                name
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier after 'into'".to_owned(),
                    found: format!("{}", token.token_type),
                    span: ParseError::span_from_token(&token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier after 'into'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        self.consume(&TokenType::Else, "Expected 'else' in guard statement")?;

        let error_binding = if self.check_identifier() {
            let token = self.advance().clone();
            if let TokenType::Identifier(name) = token.token_type {
                name
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier after 'else'".to_owned(),
                    found: format!("{}", token.token_type),
                    span: ParseError::span_from_token(&token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier after 'else'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        self.consume(&TokenType::Arrow, "Expected '=>' after guard else binding")?;
        self.skip_newlines();
        let else_body = Box::new(self.parse_indent_block()?);

        let span = Span::new(start_span.start, else_body.span().end);
        Ok(Stmt::Guard {
            expression: Box::new(expression),
            success_binding,
            error_binding,
            else_body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a loop statement (infinite loop)
    ///
    /// Syntax: `loop => { ... }`
    ///
    /// Infinite loops must use `break` to exit.
    ///
    /// # Examples
    /// - `loop => { if done { break } process() }`
    ///
    /// # Errors
    /// Returns a parse error if the loop statement syntax is invalid.
    pub(super) fn parse_loop_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'loop'

        // Expect '=>'
        if !self.check(&TokenType::Arrow) {
            return Err(ParseError::UnexpectedToken {
                expected: "'=>'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }
        self.advance(); // consume '=>'

        let body = if self.check(&TokenType::LeftBrace) {
            Box::new(self.parse_block_statement()?)
        } else {
            self.skip_newlines();
            if self.check(&TokenType::Indent) {
                Box::new(self.parse_indent_block()?)
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "'{' or indentation block after 'loop =>'".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        };

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Loop {
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse labeled value payloads used by `break` and `continue` statements.
    fn parse_labeled_control_flow_values(&mut self) -> ParseResult<Vec<LabeledValue>> {
        let mut values = Vec::new();
        let mut seen_labels = alloc::collections::BTreeSet::new();

        while self.check_identifier() {
            let label_token = self.advance().clone();
            let label_span = label_token.span;
            let label = match label_token.token_type.clone() {
                TokenType::Identifier(label_text) => label_text,
                other => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier".to_owned(),
                        found: format!("{other}"),
                        span: ParseError::span_from_token(&label_token),
                    });
                }
            };

            // Check for duplicate labels
            if !seen_labels.insert(label.clone()) {
                return Err(ParseError::DuplicateLabel {
                    label,
                    span: ParseError::span_from_token(&label_token),
                });
            }

            self.consume(
                &TokenType::Colon,
                "Expected ':' after label in break/continue payload",
            )?;

            let value_expr = self.parse_expression()?;
            let value_span = value_expr.span();
            let payload_span = Span::new(label_span.start, value_span.end);

            values.push(LabeledValue {
                label,
                value: value_expr,
                span: payload_span,
                id: next_node_id(),
            });

            if self.check(&TokenType::Comma) {
                self.advance();
                continue;
            }

            break;
        }

        Ok(values)
    }

    /// Parse a break statement with optional labeled payload values.
    fn parse_break_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'break'

        let values = self.parse_labeled_control_flow_values()?;
        let statement_end = values.last().map_or(start_span.end, |value| value.span.end);
        let span = Span::new(start_span.start, statement_end);

        Ok(Stmt::Break {
            values,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a continue statement
    ///
    /// Syntax: `continue`
    ///
    /// Skips to the next iteration of the nearest enclosing loop.
    fn parse_continue_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'continue'

        let values = self.parse_labeled_control_flow_values()?;
        let statement_end = values.last().map_or(start_span.end, |value| value.span.end);
        let span = Span::new(start_span.start, statement_end);

        Ok(Stmt::Continue {
            values,
            span,
            id: next_node_id(),
        })
    }

    /// Parse an expression statement or assignment statement
    ///
    /// An expression statement is any expression followed by a newline/semicolon.
    /// If the expression is followed by `=`, it's parsed as an assignment instead.
    ///
    /// # Syntax
    /// - Expression statement: `expr`
    /// - Assignment statement: `target = value`
    ///
    /// Valid assignment targets are:
    /// - Identifiers (variables): `x = 10`
    /// - Index expressions: `arr[0] = 5`
    /// - Member access: `obj.field = "value"`
    ///
    /// # Errors
    /// Returns a parse error if:
    /// - The expression syntax is invalid
    /// - An assignment target is invalid (e.g., `5 = x`)
    pub(super) fn parse_expression_statement(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expression()?;

        // Check if this is an assignment
        if self.check(&TokenType::Assign) {
            let start_span = expr.span();
            self.advance(); // consume '='

            let value = self.parse_expression()?;
            let end_span = value.span();
            let span = Span::new(start_span.start, end_span.end);

            // Validate that the target is assignable
            match expr {
                Expr::Identifier { .. } | Expr::Index { .. } | Expr::Member { .. } => {
                    // Valid assignment targets
                }
                _ => {
                    return Err(ParseError::InvalidSyntax {
                        message: "Invalid assignment target".to_owned(),
                        span: LexError::span_from_span(start_span),
                    });
                }
            }

            Ok(Stmt::Assignment {
                target: expr,
                value,
                span,
                id: next_node_id(),
            })
        } else {
            // Regular expression statement
            let span = expr.span();
            Ok(Stmt::Expression {
                expr,
                span,
                id: next_node_id(),
            })
        }
    }
}
