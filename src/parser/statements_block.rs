//! Indentation block and leading-comment body parsing helpers split from `statements.rs`
//! to keep the statement parser below repository file-length limits.

use crate::ast::{AstNode, Stmt};
use crate::parser::{ParseError, ParseResult, Parser};
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
                    let id = self.next_node_id();
                    self.deferred_comment_declarations
                        .push(crate::ast::Decl::Comment {
                            text: comment_token.lexeme,
                            span: comment_token.span,
                            id,
                        });
                    self.skip_newlines();
                    continue;
                }

                let comment_token = self.advance().clone();
                let id = self.next_node_id();
                statements.push(Stmt::Comment {
                    text: comment_token.lexeme,
                    span: comment_token.span,
                    id,
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
            id: self.next_node_id(),
        })
    }

    /// Parse an indentation-based statement body while preserving leading comments.
    ///
    /// This helper consumes any `Comment`/`DocComment` tokens that appear after
    /// `skip_newlines()` and before the `Indent` token, then prepends them to the
    /// resulting block statement so control-flow statement bodies keep first-line
    /// comments.
    pub(super) fn parse_indented_body_with_leading_comments(
        &mut self,
        expected_message: &str,
    ) -> ParseResult<Box<Stmt>> {
        let mut leading_comments: Vec<Stmt> = Vec::new();
        while matches!(
            self.current_token().token_type,
            TokenType::Comment(_) | TokenType::DocComment(_)
        ) {
            let comment_token = self.advance().clone();
            leading_comments.push(Stmt::Comment {
                text: comment_token.lexeme,
                span: comment_token.span,
                id: self.next_node_id(),
            });
            self.skip_newlines();
        }

        if self.check(&TokenType::Indent) {
            let block_stmt = self.parse_indent_block()?;
            if let Stmt::Block {
                mut statements,
                span,
                id,
            } = block_stmt
            {
                let mut all_stmts = leading_comments;
                all_stmts.append(&mut statements);
                Ok(Box::new(Stmt::Block {
                    statements: all_stmts,
                    span,
                    id,
                }))
            } else {
                Ok(Box::new(block_stmt))
            }
        } else {
            Err(ParseError::UnexpectedToken {
                expected: expected_message.to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            })
        }
    }

    /// Parse an `if` branch body that preserves leading comments.
    ///
    /// Statement-form `if` bodies can tolerate an EOF terminator when the body
    /// contains only indented comments. Other control-flow forms keep the shared
    /// strict indented-body behavior.
    pub(super) fn parse_if_indented_body_with_leading_comments(
        &mut self,
    ) -> ParseResult<Box<Stmt>> {
        let mut leading_comments: Vec<Stmt> = Vec::new();
        while matches!(
            self.current_token().token_type,
            TokenType::Comment(_) | TokenType::DocComment(_)
        ) {
            let comment_token = self.advance().clone();
            leading_comments.push(Stmt::Comment {
                text: comment_token.lexeme,
                span: comment_token.span,
                id: self.next_node_id(),
            });
            self.skip_newlines();
        }

        if self.check(&TokenType::Indent) {
            let block_stmt = self.parse_indent_block()?;
            if let Stmt::Block {
                mut statements,
                span,
                id,
            } = block_stmt
            {
                let mut all_stmts = leading_comments;
                all_stmts.append(&mut statements);
                Ok(Box::new(Stmt::Block {
                    statements: all_stmts,
                    span,
                    id,
                }))
            } else {
                Ok(Box::new(block_stmt))
            }
        } else if let Some((first_comment, trailing_comments)) = leading_comments.split_first() {
            if self.is_at_end() && first_comment.span().start.column > 1 {
                let start = first_comment.span().start;
                let end = trailing_comments
                    .last()
                    .map_or(first_comment.span().end, |comment| comment.span().end);

                Ok(Box::new(Stmt::Block {
                    statements: leading_comments,
                    span: Span::new(start, end),
                    id: self.next_node_id(),
                }))
            } else {
                Err(ParseError::UnexpectedToken {
                    expected: "'{' or ':' after if condition".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                })
            }
        } else {
            Err(ParseError::UnexpectedToken {
                expected: "'{' or ':' after if condition".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            })
        }
    }
}
