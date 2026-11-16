//! Contextual metadata and hook syntax shared by scenarios and tests.
use crate::ast::Documentation;
use crate::ast::testing::{HookDeclaration, SourceReference, TestKind, TestMetadata};
use crate::parser::{ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Recognize only the supported metadata prefixes.
    pub(super) fn starts_test_metadata(&self) -> bool {
        ["unit", "units", "worlds", "activate", "kind", "requires"]
            .iter()
            .any(|word| self.test_word(word))
    }

    /// Consume a fixed sequence of contextual words.
    fn consume_test_phrase(&mut self, words: &[&str]) -> ParseResult<()> {
        for word in words {
            if !self.test_word(word) {
                return Err(self.test_syntax_error(&format!("Expected '{word}'")));
            }
            self.advance();
        }
        Ok(())
    }

    /// Parse replacement metadata, retaining original source spans.
    pub(super) fn parse_test_metadata(&mut self) -> ParseResult<TestMetadata> {
        let start = self.current_token().span.start;
        let clause = self.consume_test_word()?;
        match clause.as_str() {
            "requires" => {
                self.consume_test_phrase(&["exclusive", "access", "to"])?;
                self.consume(&TokenType::Colon, "Expected ':'")?;
                let bracketed = self.check(&TokenType::LeftBracket);
                if bracketed {
                    self.advance();
                }
                let mut keys = Vec::new();
                loop {
                    let token = self.current_token().clone();
                    let TokenType::StringLiteral(key) = token.token_type else {
                        return Err(
                            self.test_syntax_error("Exclusion keys must be constant strings")
                        );
                    };
                    if Self::test_title_interpolates(&token.lexeme) {
                        return Err(self.test_syntax_error("Exclusion keys cannot interpolate"));
                    }
                    self.advance();
                    keys.push(key);
                    if !self.check(&TokenType::Comma) {
                        break;
                    }
                    self.advance();
                }
                if bracketed {
                    self.consume(&TokenType::RightBracket, "Expected end of exclusion keys")?;
                }
                self.finish_test_metadata_line()?;
                Ok(TestMetadata::Exclusive {
                    keys,
                    span: Span::new(start, self.previous_token().span.end),
                })
            }
            "unit" | "units" => {
                self.consume_test_phrase(&["under", "test"])?;
                self.consume(&TokenType::Colon, "Expected ':'")?;
                let references = self.parse_test_references(false)?;
                if clause == "unit" && references.len() != 1 {
                    return Err(
                        self.test_syntax_error("Singular subject requires one function reference")
                    );
                }
                Ok(TestMetadata::Subjects {
                    plural: clause == "units",
                    references,
                    span: Span::new(start, self.previous_token().span.end),
                })
            }
            "worlds" | "activate" => {
                if clause == "activate" {
                    self.consume_test_phrase(&["worlds"])?;
                }
                self.consume(&TokenType::Colon, "Expected ':'")?;
                let references = self.parse_test_references(true)?;
                let span = Span::new(start, self.previous_token().span.end);
                Ok(if clause == "worlds" {
                    TestMetadata::Worlds { references, span }
                } else {
                    TestMetadata::ActivateWorlds { references, span }
                })
            }
            "kind" => {
                self.consume(&TokenType::Colon, "Expected ':'")?;
                let kind = match self.consume_test_word()?.as_str() {
                    "unit" => TestKind::Unit,
                    "integration" => TestKind::Integration,
                    "end_to_end" => TestKind::EndToEnd,
                    _ => {
                        return Err(
                            self.test_syntax_error("Expected unit, integration or end_to_end")
                        );
                    }
                };
                self.finish_test_metadata_line()?;
                Ok(TestMetadata::Kind {
                    kind,
                    span: Span::new(start, self.previous_token().span.end),
                })
            }
            _ => Err(self.test_syntax_error("Unsupported test metadata")),
        }
    }

    /// Parse genuine qualified references rather than call expressions or literals.
    pub(super) fn parse_test_references(
        &mut self,
        allow_none: bool,
    ) -> ParseResult<Vec<SourceReference>> {
        if allow_none && self.test_word("none") {
            self.advance();
            self.finish_test_metadata_line()?;
            return Ok(Vec::new());
        }
        let mut references = Vec::new();
        loop {
            let start = self.current_token().span.start;
            let mut path = vec![self.consume_test_word()?];
            while self.check(&TokenType::Dot) {
                self.advance();
                path.push(self.consume_test_word()?);
            }
            references.push(SourceReference {
                path,
                span: Span::new(start, self.previous_token().span.end),
            });
            if !self.check(&TokenType::Comma) {
                break;
            }
            self.advance();
        }
        self.finish_test_metadata_line()?;
        Ok(references)
    }

    /// Require metadata to end rather than silently ignoring a trailing call.
    fn finish_test_metadata_line(&self) -> ParseResult<()> {
        if self.is_at_end()
            || matches!(
                self.current_token().token_type,
                TokenType::Newline | TokenType::Comment(_) | TokenType::Dedent
            )
        {
            Ok(())
        } else {
            Err(self.test_syntax_error("Unexpected tokens after test metadata"))
        }
    }

    /// Parse a file/scenario hook with an ordinary typed callback.
    pub(super) fn parse_test_hook(
        &mut self,
        documentation: Option<Documentation>,
    ) -> ParseResult<HookDeclaration> {
        let start = self.current_token().span.start;
        let before = self.test_word("before");
        self.advance();
        self.consume_test_phrase(&["each", "test"])?;
        self.consume(&TokenType::Colon, "Expected ':' after hook")?;
        let callback = self.parse_managed_callback()?;
        Ok(HookDeclaration {
            before,
            callback,
            documentation,
            span: Span::new(start, self.previous_token().span.end),
        })
    }
}
