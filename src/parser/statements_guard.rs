#![allow(
    clippy::manual_let_else,
    reason = "guard destructure parsing favors explicit token handling over let-else churn during blocker cleanup"
)]
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
    #[expect(
        clippy::too_many_lines,
        reason = "guard parsing handles ambiguity recovery and both guard syntaxes in one path"
    )]
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

        let expression = match self.parse_expression() {
            Ok(expression) => expression,
            Err(parse_error) => {
                let previous_token = self.previous_token();
                if self.check(&TokenType::Else) || previous_token.token_type == TokenType::Else {
                    let span = if self.check(&TokenType::Else) {
                        ParseError::span_from_token(self.current_token())
                    } else {
                        ParseError::span_from_token(previous_token)
                    };
                    return Err(ParseError::GuardMissingElseClause { span });
                }
                return Err(parse_error);
            }
        };

        let (success_binding, success_binding_type, success_binding_is_mutable, success_bindings) =
            if self.check(&TokenType::Into) {
                self.advance();
                if !self.check_identifier() {
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier after 'into'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }

                let first_binding_token = self.advance().clone();
                let first_binding_name = match first_binding_token.token_type.clone() {
                    TokenType::Identifier(name) => name,
                    other => {
                        return Err(ParseError::UnexpectedToken {
                            expected: "identifier after 'into'".to_owned(),
                            found: format!("{other}"),
                            span: ParseError::span_from_token(&first_binding_token),
                        });
                    }
                };

                let starts_exact_name_multi_bind = self.check(&TokenType::Comma);
                let starts_explicit_label_multi_bind = self.check(&TokenType::Colon)
                    && self
                        .tokens
                        .get(self.current.saturating_add(1))
                        .is_some_and(|token| matches!(token.token_type, TokenType::Identifier(_)))
                    && self
                        .tokens
                        .get(self.current.saturating_add(2))
                        .is_some_and(|token| token.token_type == TokenType::Comma);

                if starts_exact_name_multi_bind || starts_explicit_label_multi_bind {
                    let mut success_bindings = Vec::new();
                    if starts_explicit_label_multi_bind {
                        self.advance();
                        if !self.check_identifier() {
                            return Err(ParseError::UnexpectedToken {
                                expected: "local binding name after guard destructure label"
                                    .to_owned(),
                                found: format!("{}", self.current_token().token_type),
                                span: ParseError::span_from_token(self.current_token()),
                            });
                        }
                        let local_name_token = self.advance().clone();
                        let local_name = match local_name_token.token_type {
                            TokenType::Identifier(value) => value,
                            _ => unreachable!(
                                "identifier check should guarantee a local binding name"
                            ),
                        };
                        success_bindings.push(self.create_let_binding(
                            local_name,
                            Some(first_binding_name),
                            local_name_token.span,
                            None,
                            false,
                        ));
                    } else {
                        success_bindings.push(self.create_let_binding(
                            first_binding_name,
                            None,
                            first_binding_token.span,
                            None,
                            false,
                        ));
                    }

                    while self.check(&TokenType::Comma) {
                        self.advance();
                        if !self.check_identifier() {
                            return Err(ParseError::UnexpectedToken {
                                expected: "identifier for guard destructured variable name"
                                    .to_owned(),
                                found: format!("{}", self.current_token().token_type),
                                span: ParseError::span_from_token(self.current_token()),
                            });
                        }
                        let next_label_or_name = self.advance().clone();
                        let (next_name, returned_label, next_span) = if self
                            .check(&TokenType::Colon)
                        {
                            self.advance();
                            if !self.check_identifier() {
                                return Err(ParseError::UnexpectedToken {
                                    expected: "local binding name after guard destructure label"
                                        .to_owned(),
                                    found: format!("{}", self.current_token().token_type),
                                    span: ParseError::span_from_token(self.current_token()),
                                });
                            }
                            let local_name_token = self.advance().clone();
                            let local_name = match local_name_token.token_type {
                                TokenType::Identifier(value) => value,
                                _ => unreachable!(
                                    "identifier check should guarantee a local binding name"
                                ),
                            };
                            let returned_label = match next_label_or_name.token_type {
                                TokenType::Identifier(value) => value,
                                _ => unreachable!(
                                    "identifier check should guarantee a guard destructure label"
                                ),
                            };
                            (local_name, Some(returned_label), local_name_token.span)
                        } else {
                            match next_label_or_name.token_type {
                                TokenType::Identifier(value) => {
                                    (value, None, next_label_or_name.span)
                                }
                                _ => {
                                    unreachable!("identifier check should guarantee a binding name")
                                }
                            }
                        };

                        success_bindings.push(self.create_let_binding(
                            next_name,
                            returned_label,
                            next_span,
                            None,
                            false,
                        ));
                    }

                    (None, None, false, success_bindings)
                } else {
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

                    let binding = self.create_let_binding(
                        first_binding_name.clone(),
                        None,
                        first_binding_token.span,
                        success_binding_type.clone(),
                        success_binding_is_mutable,
                    );

                    (
                        Some(first_binding_name),
                        success_binding_type,
                        success_binding_is_mutable,
                        vec![binding],
                    )
                }
            } else {
                (None, None, false, Vec::new())
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
            success_bindings,
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
