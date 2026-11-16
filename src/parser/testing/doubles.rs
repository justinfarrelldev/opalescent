//! Import-edge declarations for typed test doubles.
use crate::ast::Documentation;
use crate::ast::testing::{DoubleDeclaration, SourceReference};
use crate::parser::{ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a static edge declaration before resolution and optimization.
    pub(super) fn parse_double_declaration(
        &mut self,
        documentation: Option<Documentation>,
    ) -> ParseResult<DoubleDeclaration> {
        let start = self.advance().span.start;
        let name = self.consume_test_word()?;
        if !self.test_word("replaces") {
            return Err(self.test_syntax_error("Expected 'replaces' in double declaration"));
        }
        self.advance();
        let function_start = self.current_token().span.start;
        let path = vec![self.consume_test_word()?];
        let function = SourceReference {
            path,
            span: Span::new(function_start, self.previous_token().span.end),
        };
        self.consume(&TokenType::From, "Expected 'from' in double declaration")?;
        let from = self.parse_import_path()?;
        self.skip_test_trivia();
        self.consume(
            &TokenType::Cast,
            "Expected 'as used by' in double declaration",
        )?;
        for word in ["used", "by"] {
            if !self.test_word(word) {
                return Err(self.test_syntax_error("Expected 'as used by' in double declaration"));
            }
            self.advance();
        }
        let as_used_by = self.parse_import_path()?;
        Ok(DoubleDeclaration {
            name,
            function,
            from,
            as_used_by,
            documentation,
            span: Span::new(start, self.previous_token().span.end),
        })
    }
}
