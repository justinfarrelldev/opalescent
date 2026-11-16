//! Parser entry point for explicitly selected test/test-support modules.
mod assertions;
mod double_statements;
mod doubles;
mod examples;
mod metadata;
use super::{ParseError, ParseErrors, ParseResult, Parser};
use crate::ast::testing::{
    ManagedCallback, ScenarioDeclaration, TestDeclaration, TestItem, TestModule, WorldDeclaration,
};
use crate::ast::{Documentation, Expr};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse framework declarations without enabling them in application code.
    pub fn parse_test_module(mut self) -> (Option<TestModule>, ParseErrors) {
        self.module_mode = crate::lexer::ModuleMode::Test;
        let start = self.current_token().span.start;
        match self.parse_test_items(false) {
            Ok(items) => {
                let span = Span::new(start, self.current_token().span.end);
                let module = self.errors.is_empty().then_some(TestModule { items, span });
                (module, self.errors)
            }
            Err(error) => {
                self.errors.push(error);
                (None, self.errors)
            }
        }
    }

    /// Parse a declaration list, stopping at its own indentation boundary.
    fn parse_test_items(&mut self, nested: bool) -> ParseResult<Vec<TestItem>> {
        let mut items = Vec::new();
        loop {
            self.skip_test_trivia();
            if self.is_at_end() || (nested && self.check(&TokenType::Dedent)) {
                break;
            }
            let documentation = self.collect_documentation();
            self.skip_test_trivia();
            let item = if self.starts_test_support_import() {
                if nested {
                    return Err(self.test_syntax_error("Imports must appear at module scope"));
                }
                TestItem::SupportImport(self.parse_test_support_import()?)
            } else if self.test_word("scenario") {
                TestItem::Scenario(self.parse_scenario(documentation)?)
            } else if self.test_word("double") {
                if nested {
                    return Err(self.test_syntax_error("Doubles must be declared at module scope"));
                }
                TestItem::Double(self.parse_double_declaration(documentation)?)
            } else if self.test_word("world") {
                if nested {
                    return Err(self.test_syntax_error("Worlds must be declared at module scope"));
                }
                TestItem::World(self.parse_world(documentation)?)
            } else if self.test_word("it should") {
                if !nested {
                    return Err(self.test_syntax_error("Tests must belong to a scenario"));
                }
                TestItem::Test(self.parse_behavior_test(documentation)?)
            } else if self.test_word("before") || self.test_word("after") {
                TestItem::Hook(self.parse_test_hook(documentation)?)
            } else if self.starts_test_metadata() {
                TestItem::Metadata(self.parse_test_metadata()?)
            } else {
                // Ordinary declarations retain the existing parser and file-role
                // validation. Put documentation back for that parser to consume.
                if let Some(doc) = documentation {
                    self.deferred_doc_comments.push((doc.raw, doc.span));
                }
                TestItem::Application(Box::new(self.parse_declaration()?))
            };
            items.push(item);
        }
        if nested {
            self.consume(&TokenType::Dedent, "Expected end of declaration block")?;
        }
        Ok(items)
    }

    /// Parse a prose scenario and its nested declaration list.
    fn parse_scenario(
        &mut self,
        documentation: Option<Documentation>,
    ) -> ParseResult<ScenarioDeclaration> {
        let start = self.advance().span.start;
        let title = self.parse_test_title(true)?;
        self.start_test_block()?;
        let items = self.parse_test_items(true)?;
        Ok(ScenarioDeclaration {
            title,
            items,
            documentation,
            span: Span::new(start, self.previous_token().span.end),
        })
    }

    /// Parse a World with exactly one setup and optional cleanup.
    fn parse_world(
        &mut self,
        documentation: Option<Documentation>,
    ) -> ParseResult<WorldDeclaration> {
        let start = self.advance().span.start;
        let name = self.consume_test_word()?;
        self.start_test_block()?;
        let mut dependencies = None;
        let mut setup = None;
        let mut cleanup = None;
        loop {
            self.skip_test_trivia();
            if self.check(&TokenType::Dedent) || self.is_at_end() {
                break;
            }
            if self.test_word("worlds") && dependencies.is_none() {
                self.advance();
                self.consume(&TokenType::Colon, "Expected ':'")?;
                dependencies = Some(self.parse_test_references(true)?);
            } else if self.test_word("setup") && setup.is_none() {
                self.advance();
                self.consume(&TokenType::Colon, "Expected ':'")?;
                setup = Some(self.parse_managed_callback()?);
            } else if self.test_word("cleanup") && cleanup.is_none() {
                self.advance();
                self.consume(&TokenType::Colon, "Expected ':'")?;
                cleanup = Some(self.parse_managed_callback()?);
            } else {
                return Err(self.test_syntax_error(
                    "Expected one setup, optional cleanup and optional worlds",
                ));
            }
        }
        self.consume(&TokenType::Dedent, "Expected end of World")?;
        let setup = setup
            .ok_or_else(|| self.test_syntax_error("World requires exactly one setup callback"))?;
        Ok(WorldDeclaration {
            name,
            dependencies: dependencies.unwrap_or_default(),
            setup,
            cleanup,
            documentation,
            span: Span::new(start, self.previous_token().span.end),
        })
    }

    /// Parse inline or metadata-bearing test declarations without a `run` wrapper.
    fn parse_behavior_test(
        &mut self,
        documentation: Option<Documentation>,
    ) -> ParseResult<TestDeclaration> {
        let start = self.advance().span.start;
        let title = self.parse_test_title(false)?.unwrap_or_default();
        self.consume(&TokenType::Colon, "Expected ':' after title")?;
        let mut metadata = Vec::new();
        let mut examples = None;
        let callback = if self.check(&TokenType::Function) {
            self.parse_managed_callback()?
        } else {
            self.skip_test_trivia();
            self.consume(&TokenType::Indent, "Expected test declaration body")?;
            self.skip_test_trivia();
            while self.starts_test_metadata() {
                metadata.push(self.parse_test_metadata()?);
                self.skip_test_trivia();
            }
            if self.test_word("examples") {
                examples = Some(self.parse_test_examples()?);
                self.skip_test_trivia();
            }
            let callback = self.parse_managed_callback()?;
            self.skip_test_trivia();
            self.consume(&TokenType::Dedent, "Expected exactly one test callback")?;
            callback
        };
        Ok(TestDeclaration {
            title,
            metadata,
            examples,
            callback,
            documentation,
            span: Span::new(start, self.previous_token().span.end),
        })
    }

    /// Parse a framework-owned ordinary function literal, optionally indented.
    fn parse_managed_callback(&mut self) -> ParseResult<ManagedCallback> {
        self.skip_test_trivia();
        let indented = self.check(&TokenType::Indent);
        if indented {
            self.advance();
            self.skip_test_trivia();
        }
        if !self.check(&TokenType::Function) {
            return Err(self.test_syntax_error("Expected an ordinary f(...) callback"));
        }
        let mut function = self.parse_expression()?;
        let Expr::Lambda {
            ref mut error_types,
            ..
        } = function
        else {
            return Err(self.test_syntax_error("Expected exactly one function literal"));
        };
        let infer_errors = error_types.as_slice() == ["inferred"];
        if infer_errors {
            error_types.clear();
        }
        if indented {
            self.skip_test_trivia();
            self.consume(&TokenType::Dedent, "Expected end of callback")?;
        }
        Ok(ManagedCallback {
            function,
            infer_errors,
        })
    }

    /// Static declaration text; interpolation is never a declaration identity.
    fn parse_test_title(&mut self, optional: bool) -> ParseResult<Option<String>> {
        if optional && self.check(&TokenType::Colon) {
            return Ok(None);
        }
        let token = self.advance().clone();
        match token.token_type {
            TokenType::StringLiteral(title) if !title.is_empty() => {
                // Quoted identity interpolation is checked by the title parser,
                // not lowered as an executable string expression.
                if token.lexeme.starts_with('\'') && Self::test_title_interpolates(&token.lexeme) {
                    return Err(self.test_syntax_error(
                        "Test titles must be static text, without interpolation",
                    ));
                }
                Ok(Some(title))
            }
            _ => Err(self.test_syntax_error("Expected a nonempty test title ending with ':'")),
        }
    }

    /// Identify unescaped interpolation markers in quoted titles.
    fn test_title_interpolates(text: &str) -> bool {
        let mut escaped = false;
        for character in text.chars() {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '{' {
                return true;
            }
        }
        false
    }

    /// Consume the colon/newline/indent that introduces a declaration block.
    fn start_test_block(&mut self) -> ParseResult<()> {
        self.consume(&TokenType::Colon, "Expected ':'")?;
        self.consume(
            &TokenType::Newline,
            "Expected declaration block on next line",
        )?;
        self.skip_test_trivia();
        self.consume(&TokenType::Indent, "Expected indented declaration block")?;
        Ok(())
    }

    /// Skip ordinary comments without consuming documentation for the next item.
    fn skip_test_trivia(&mut self) {
        while self.check(&TokenType::Newline) || self.check(&TokenType::Comment(String::new())) {
            self.advance();
        }
    }

    /// Contextual keyword matching without globally reserving conversational words.
    fn test_word(&self, expected: &str) -> bool {
        matches!(self.current_token().token_type, TokenType::Identifier(ref word) if word == expected)
    }

    /// Consume one contextual identifier.
    fn consume_test_word(&mut self) -> ParseResult<String> {
        if let TokenType::Identifier(word) = self.current_token().token_type.clone() {
            self.advance();
            Ok(word)
        } else {
            Err(self.test_syntax_error("Expected identifier"))
        }
    }

    /// Produce a localized diagnostic from the original token span.
    fn test_syntax_error(&self, message: &str) -> ParseError {
        ParseError::InvalidSyntax {
            message: message.to_owned(),
            span: ParseError::span_from_token(self.current_token()),
        }
    }
}
