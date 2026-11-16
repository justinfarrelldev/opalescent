//! Named example records and explicit test-only imports.
use crate::ast::ConstructorField;
use crate::ast::testing::{ExampleDeclaration, ExampleGroup, TestSupportImport, TestSupportKind};
use crate::parser::{ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Test-only imports have an explicit declaration-kind prefix.
    pub(super) fn starts_test_support_import(&self) -> bool {
        self.check(&TokenType::Import)
            && self.tokens.get(self.current.saturating_add(1)).is_some_and(|token| {
                matches!(token.token_type, TokenType::Identifier(ref name) if name == "world" || name == "double")
            })
    }

    /// Reuse ordinary named import, alias and module-path grammar.
    pub(super) fn parse_test_support_import(&mut self) -> ParseResult<TestSupportImport> {
        let start = self.advance().span.start;
        let kind = if self.consume_test_word()? == "world" {
            TestSupportKind::World
        } else {
            TestSupportKind::Double
        };
        let mut items = vec![self.parse_import_item(false)?];
        while self.check(&TokenType::Comma) {
            self.advance();
            items.push(self.parse_import_item(false)?);
        }
        self.consume(
            &TokenType::From,
            "Expected 'from' after test-support imports",
        )?;
        let source = self.parse_import_path()?;
        Ok(TestSupportImport {
            kind,
            items,
            source,
            span: Span::new(start, self.previous_token().span.end),
        })
    }

    /// Parse named records under one declared, ordinary record type.
    pub(super) fn parse_test_examples(&mut self) -> ParseResult<ExampleGroup> {
        let start = self.advance().span.start;
        if !self.test_word("of") {
            return Err(self.test_syntax_error("Expected 'examples of Type:'"));
        }
        self.advance();
        let record_type = self.parse_type()?;
        self.start_test_block()?;
        let mut examples = Vec::new();
        loop {
            self.skip_test_trivia();
            if self.check(&TokenType::Dedent) || self.is_at_end() {
                break;
            }
            let documentation = self.collect_documentation();
            self.skip_test_trivia();
            if !self.test_word("example") {
                return Err(self.test_syntax_error("Expected named example"));
            }
            let example_start = self.advance().span.start;
            let title = self.parse_test_title(false)?.unwrap_or_default();
            self.start_test_block()?;
            let mut fields = Vec::new();
            loop {
                self.skip_test_trivia();
                if self.check(&TokenType::Dedent) || self.is_at_end() {
                    break;
                }
                let field_start = self.current_token().span.start;
                let name = self.consume_test_word()?;
                self.consume(&TokenType::Colon, "Expected ':' after example field name")?;
                let value = self.parse_expression()?;
                fields.push(ConstructorField {
                    name,
                    value,
                    span: Span::new(field_start, self.previous_token().span.end),
                });
            }
            self.consume(&TokenType::Dedent, "Expected end of example fields")?;
            examples.push(ExampleDeclaration {
                title,
                fields,
                documentation,
                span: Span::new(example_start, self.previous_token().span.end),
            });
        }
        if examples.is_empty() {
            return Err(self.test_syntax_error("Expected at least one named example"));
        }
        self.consume(&TokenType::Dedent, "Expected end of examples")?;
        Ok(ExampleGroup {
            record_type,
            examples,
            span: Span::new(start, self.previous_token().span.end),
        })
    }
}
