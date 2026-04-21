//! Parsing for the `new <Type>:` indented-block constructor expression.
//!
//! This form replaces the legacy `Type { field: value }` brace syntax and is
//! the only way to construct a product type or sum-type variant in Opalescent.
//! Keeping it in its own module preserves the single-responsibility boundary
//! and keeps [`crate::parser::expressions`] within its line-count budget.
//!
//! Grammar:
//!
//! ```text
//! new_expr := 'new' callee ':' NEWLINE INDENT field (NEWLINE field)* DEDENT
//! callee   := IDENT ('.' IDENT)?
//! field    := IDENT ':' expression
//! ```

use super::{ParseError, ParseResult, Parser, Precedence};
use crate::ast::{AstNode, ConstructorField, Expr};
use crate::error::LexError;
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a `new` constructor expression with an indented field block.
    ///
    /// The `new` keyword is expected to be the current token when this is
    /// invoked (dispatched from `parse_primary`). The callee is parsed with
    /// `Precedence::Unary` so that `.` member access (for sum-type variants
    /// such as `Message.Text`) is consumed, but higher-precedence postfix
    /// operators like calls or array indexing are not, since those are not
    /// meaningful as constructor targets.
    ///
    /// The field list lives in a single `Indent`/`Dedent` block; a missing
    /// indent is a parse error so authors get an immediate diagnostic instead
    /// of a silently truncated constructor.
    pub(super) fn parse_new_expression(&mut self, start_span: Span) -> ParseResult<Expr> {
        // Consume the `new` keyword that triggered this dispatch.
        self.consume(&TokenType::New, "Expected 'new' keyword")?;

        // Parse the callee without letting the Pratt parser descend into
        // calls or other postfix forms that would be invalid here.
        let callee = self.parse_precedence(Precedence::Unary)?;
        match callee {
            Expr::Identifier { .. } | Expr::Member { .. } => {}
            _ => {
                return Err(ParseError::InvalidSyntax {
                    message:
                        "Expected a type name (e.g. `Person`) or variant (`Message.Text`) after `new`"
                            .to_owned(),
                    span: LexError::span_from_span(callee.span()),
                });
            }
        }

        self.consume(&TokenType::Colon, "Expected ':' after constructor callee")?;
        self.skip_newlines_and_comments();
        self.consume(
            &TokenType::Indent,
            "Expected indented field block after `new <Type>:` constructor",
        )?;

        let fields = self.parse_new_expression_field_block()?;

        self.consume(
            &TokenType::Dedent,
            "Expected dedent after constructor field block",
        )?;

        let span = Span::new(start_span.start, self.previous_token().span.end);
        Ok(Expr::Constructor {
            callee: Box::new(callee),
            fields,
            span,
            id: self.next_node_id(),
        })
    }

    /// Parse the `field: value` lines inside a `new` expression's indented
    /// block until a `Dedent` is reached. Blank lines and comments are
    /// skipped between fields so documentation pragmas and spacing do not
    /// terminate the block prematurely.
    fn parse_new_expression_field_block(&mut self) -> ParseResult<Vec<ConstructorField>> {
        let mut fields = Vec::new();
        self.skip_newlines_and_comments();

        while !self.is_at_end() && !self.check(&TokenType::Dedent) {
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

            self.skip_newlines_and_comments();
        }

        Ok(fields)
    }
}
