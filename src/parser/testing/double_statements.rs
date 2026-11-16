//! Typed rule configuration, verification and explicit restoration syntax.
use crate::ast::testing::{SourceReference, StubBehavior, TestingStatement};
use crate::ast::{Expr, Stmt};
use crate::parser::{ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse typed rule configuration, verification and explicit restoration.
    pub(in crate::parser) fn parse_double_statement(&mut self) -> ParseResult<Stmt> {
        let start = self.current_token().span.start;
        let keyword = self.consume_test_word()?;
        let expression = self.parse_expression()?;
        let statement = match keyword.as_str() {
            "restore" => TestingStatement::Restore(expression),
            "verify" => TestingStatement::Verify(expression),
            "spy" => {
                self.double_word("using")?;
                self.double_word("original")?;
                TestingStatement::Stub {
                    target: expression,
                    arguments: None,
                    behavior: StubBehavior::SpyOriginal,
                }
            }
            _ => self.parse_stub_configuration(expression)?,
        };
        Ok(Stmt::Testing {
            statement: Box::new(statement),
            span: Span::new(start, self.previous_token().span.end),
            id: self.next_node_id(),
        })
    }

    /// Separate exact argument rules from explicit fallback modes.
    fn parse_stub_configuration(&mut self, expression: Expr) -> ParseResult<TestingStatement> {
        if self.check(&TokenType::Colon) {
            self.start_test_block()?;
            for word in ["otherwise", "fail", "the", "test"] {
                self.double_word(word)?;
            }
            self.skip_test_trivia();
            self.consume(&TokenType::Dedent, "Expected end of stub fallback")?;
            return Ok(TestingStatement::Stub {
                target: expression,
                arguments: None,
                behavior: StubBehavior::FailTest,
            });
        }
        if self.test_word("using") {
            self.advance();
            return Ok(TestingStatement::Stub {
                target: expression,
                arguments: None,
                behavior: StubBehavior::Replacement(self.parse_expression()?),
            });
        }
        let Expr::Call {
            callee,
            args,
            generic_args: None,
            ..
        } = expression
        else {
            return Err(
                self.test_syntax_error("Exact stub rule requires typed argument expressions")
            );
        };
        self.skip_test_trivia();
        let behavior = if self.test_word("returns") {
            self.advance();
            StubBehavior::Return(self.parse_expression()?)
        } else {
            self.double_word("fails")?;
            self.double_word("with")?;
            let start = self.current_token().span.start;
            let mut path = vec![self.consume_test_word()?];
            while self.check(&TokenType::Dot) {
                self.advance();
                path.push(self.consume_test_word()?);
            }
            StubBehavior::ApplicationError(SourceReference {
                path,
                span: Span::new(start, self.previous_token().span.end),
            })
        };
        Ok(TestingStatement::Stub {
            target: *callee,
            arguments: Some(args),
            behavior,
        })
    }

    /// Consume one exact contextual double-configuration word.
    fn double_word(&mut self, word: &str) -> ParseResult<()> {
        if !self.test_word(word) {
            return Err(
                self.test_syntax_error(&format!("Expected '{word}' in double configuration"))
            );
        }
        self.advance();
        Ok(())
    }
}
