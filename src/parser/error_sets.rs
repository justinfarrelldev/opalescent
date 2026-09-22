use super::{
    Parser,
    errors::{ParseError, ParseResult},
};
use crate::{
    ast::{Decl, Documentation, ErrorSetMember, HotReloadMetadata, Visibility},
    token::{Span, TokenType},
};

impl Parser {
    /// Parse a named error-set declaration.
    pub(super) fn parse_error_set_declaration(
        &mut self,
        visibility: Visibility,
        doc_comment: Option<Documentation>,
    ) -> ParseResult<Decl> {
        let start_span = self.current_token().span;

        if !self.check_contextual_keyword("error") {
            let token = self.current_token();
            return Err(ParseError::UnexpectedToken {
                expected: "'error'".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            });
        }
        self.advance();

        if !self.check_contextual_keyword("set") {
            let token = self.current_token();
            return Err(ParseError::UnexpectedToken {
                expected: "'set' after 'error'".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            });
        }
        self.advance();

        let name = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for error set name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "error set name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        self.consume(&TokenType::Assign, "Expected '=' after error set name")?;
        self.skip_trivia_preserving_doc_comments();

        let mut members = Vec::new();
        if !self.check_identifier() {
            return Err(ParseError::UnexpectedToken {
                expected: "error set member name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        loop {
            self.skip_trivia_preserving_doc_comments();
            if !self.check_identifier() {
                return Err(ParseError::UnexpectedToken {
                    expected: "error set member name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

            let token = self.advance();
            if let &TokenType::Identifier(ref member_name) = &token.token_type {
                members.push(ErrorSetMember {
                    name: member_name.clone(),
                    span: token.span,
                });
            }

            self.skip_trivia_preserving_doc_comments();
            if !self.check(&TokenType::Comma) {
                break;
            }
            self.advance();
            self.skip_trivia_preserving_doc_comments();
            if self.is_at_end() || self.check(&TokenType::Dedent) || self.is_declaration_start() {
                break;
            }
        }

        let end_span = members
            .last()
            .map_or_else(|| self.previous_token().span, |member| member.span);
        Ok(Decl::ErrorSet {
            name,
            members,
            visibility,
            doc_comment,
            span: Span::new(start_span.start, end_span.end),
            id: self.next_node_id(),
            metadata: HotReloadMetadata::for_type_declaration(),
        })
    }
}
