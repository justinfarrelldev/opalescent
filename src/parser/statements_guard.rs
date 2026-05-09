//! Guard statement parsing helpers split from `statements.rs` to keep file size manageable.

use crate::ast::{AstNode, Stmt, Type};
use crate::parser::{ParseError, ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a guard statement.
    ///
    /// Syntax:
    /// - `guard <expr> into <success_binding> [: Type] [mutable] else <error_binding> => <indent-body>`
    /// - `guard <expr> else <error_binding> => <indent-body>`
    pub(super) fn parse_guard_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance();

        if self.check(&TokenType::If) {
            let ambiguous_else_span = self
                .find_guard_ambiguous_if_else_span()
                .unwrap_or_else(|| ParseError::span_from_token(self.current_token()));
            let error = ParseError::GuardAmbiguousIfElse {
                span: ambiguous_else_span,
            };
            self.recover_guard_statement_clause();
            return Err(error);
        }

        let expression = self.parse_expression()?;

        let (success_binding, success_binding_type, success_binding_is_mutable) =
            if self.check(&TokenType::Into) {
                self.advance();
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

                let success_binding_type: Option<Type> = self
                    .check(&TokenType::Colon)
                    .then(|| {
                        self.advance();
                        self.parse_type()
                    })
                    .transpose()?;

                let success_binding_is_mutable = self.check(&TokenType::Mutable) && {
                    self.advance();
                    true
                };

                (
                    Some(success_binding),
                    success_binding_type,
                    success_binding_is_mutable,
                )
            } else {
                (None, None, false)
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
        self.active_guard_error_bindings.push(error_binding.clone());
        let else_body_result = self.parse_indented_body_with_leading_comments(
            "indentation block after '=>' in guard statement",
        );
        let popped_guard_error_binding = self.active_guard_error_bindings.pop();
        debug_assert_eq!(
            popped_guard_error_binding.as_deref(),
            Some(error_binding.as_str()),
            "guard error binding stack should unwind in LIFO order"
        );
        let else_body = else_body_result?;

        let span = Span::new(start_span.start, else_body.span().end);
        Ok(Stmt::Guard {
            expression: Box::new(expression),
            success_binding,
            success_binding_type,
            success_binding_is_mutable,
            error_binding,
            else_body,
            span,
            id: self.next_node_id(),
        })
    }

    /// Find the first `else` token in this guard statement header so diagnostics
    /// can point at the ambiguous token in `guard if ... else ...`.
    fn find_guard_ambiguous_if_else_span(&self) -> Option<miette::SourceSpan> {
        let mut index = self.current;

        while let Some(current_token) = self.tokens.get(index) {
            match current_token.token_type {
                TokenType::Else => return Some(ParseError::span_from_token(current_token)),
                TokenType::Newline | TokenType::EndOfFile => return None,
                _ => {
                    index = index.saturating_add(1);
                }
            }
        }

        None
    }

    /// Recover parser position after an ambiguous or malformed guard statement clause.
    fn recover_guard_statement_clause(&mut self) {
        let mut brace_depth = 0_usize;
        let mut saw_guard_arrow = false;

        while !self.is_at_end() {
            match self.current_token().token_type {
                TokenType::LeftBrace => {
                    brace_depth = brace_depth.saturating_add(1);
                    self.advance();
                }
                TokenType::RightBrace => {
                    if brace_depth == 0 {
                        break;
                    }
                    brace_depth = brace_depth.saturating_sub(1);
                    self.advance();
                }
                TokenType::Arrow if brace_depth == 0 => {
                    saw_guard_arrow = true;
                    self.advance();
                }
                TokenType::Newline => {
                    self.advance();

                    if saw_guard_arrow && self.check(&TokenType::Indent) {
                        let mut indent_depth = 0_usize;
                        while !self.is_at_end() {
                            match self.current_token().token_type {
                                TokenType::Indent => {
                                    indent_depth = indent_depth.saturating_add(1);
                                    self.advance();
                                }
                                TokenType::Dedent => {
                                    if indent_depth == 0 {
                                        break;
                                    }
                                    indent_depth = indent_depth.saturating_sub(1);
                                    self.advance();
                                    if indent_depth == 0 {
                                        break;
                                    }
                                }
                                _ => {
                                    self.advance();
                                }
                            }
                        }
                    }

                    break;
                }
                TokenType::EndOfFile => break,
                _ => {
                    self.advance();
                }
            }
        }
    }
}
