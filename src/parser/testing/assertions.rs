//! Token/expression-boundary assertion parsing, never raw-text phrase splitting.
use crate::ast::Stmt;
use crate::ast::testing::{
    Comparison, Expectation, ExpectedCallCount, Matcher, SequenceRelation, SourceReference,
    TestingStatement,
};
use crate::parser::{ParseResult, Parser};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse the fixed typed expectation vocabulary in test-module mode.
    pub(in crate::parser) fn parse_test_expectation(&mut self) -> ParseResult<Stmt> {
        let start = self.advance().span.start;
        let eventually = self.test_word("eventually");
        if eventually {
            self.advance();
        }
        let actual = self.parse_expression()?;
        if eventually {
            self.skip_newlines();
        }
        let negated = self.check(&TokenType::Not);
        if negated {
            self.advance();
        }
        self.assertion_word("to")?;
        let matcher = if self.test_word("have") {
            if negated {
                return Err(self.test_syntax_error("Use 'to have never been called'"));
            }
            self.parse_call_matcher()?
        } else if self.test_word("be") {
            self.advance();
            self.parse_be_matcher(negated)?
        } else {
            self.parse_sequence_matcher(negated)?
        };
        let eventually_seconds = if eventually {
            self.skip_newlines();
            self.assertion_word("within")?;
            let seconds = self.parse_expression()?;
            self.assertion_word("seconds")?;
            Some(seconds)
        } else {
            None
        };
        let span = Span::new(start, self.previous_token().span.end);
        Ok(Stmt::Testing {
            statement: Box::new(TestingStatement::Expect(Expectation {
                actual,
                matcher,
                eventually_seconds,
                span,
            })),
            span,
            id: self.next_node_id(),
        })
    }

    /// Equality, ordering, emptiness and nominal-variant clauses.
    fn parse_be_matcher(&mut self, negated: bool) -> ParseResult<Matcher> {
        if self.test_word("empty") {
            self.advance();
            return Ok(Matcher::Empty { negated });
        }
        if self.test_word("the") {
            if negated {
                return Err(self.test_syntax_error("Negated variant matching is not supported"));
            }
            self.advance();
            let variant = self.consume_test_word()?;
            self.assertion_word("variant")?;
            self.assertion_word("of")?;
            let start = self.current_token().span.start;
            let mut path = vec![self.consume_test_word()?];
            while self.check(&TokenType::Dot) {
                self.advance();
                path.push(self.consume_test_word()?);
            }
            return Ok(Matcher::Variant {
                variant,
                nominal_type: SourceReference {
                    path,
                    span: Span::new(start, self.previous_token().span.end),
                },
            });
        }
        let relation = if self.test_word("greater") || self.test_word("less") {
            let relation = if self.test_word("greater") {
                Comparison::Greater
            } else {
                Comparison::Less
            };
            self.advance();
            self.assertion_word("than")?;
            Some(relation)
        } else if self.test_word("at") {
            self.advance();
            let relation = if self.test_word("least") {
                Comparison::AtLeast
            } else if self.test_word("most") {
                Comparison::AtMost
            } else {
                return Err(self.test_syntax_error("Expected 'at least' or 'at most'"));
            };
            self.advance();
            Some(relation)
        } else {
            None
        };
        if let Some(relation) = relation {
            if negated {
                return Err(self.test_syntax_error("Negated numeric comparisons are not supported"));
            }
            Ok(Matcher::Compare {
                expected: self.parse_expression()?,
                relation,
            })
        } else {
            Ok(Matcher::Equal {
                expected: self.parse_expression()?,
                negated,
            })
        }
    }

    /// String and exact ordered collection matching.
    fn parse_sequence_matcher(&mut self, negated: bool) -> ParseResult<Matcher> {
        let relation = if self.test_word("contain") {
            self.advance();
            if self.test_word("exactly") {
                if negated {
                    return Err(
                        self.test_syntax_error("Negated exact ordered matching is not supported")
                    );
                }
                self.advance();
                SequenceRelation::ExactlyInOrder
            } else {
                SequenceRelation::Contain
            }
        } else if self.test_word("start") || self.test_word("end") {
            let relation = if self.test_word("start") {
                if negated {
                    return Err(self.test_syntax_error("Negated prefix matching is not supported"));
                }
                SequenceRelation::StartWith
            } else {
                SequenceRelation::EndWith
            };
            self.advance();
            self.assertion_word("with")?;
            relation
        } else {
            return Err(self.test_syntax_error("Expected a supported assertion matcher"));
        };
        let expected = self.parse_expression()?;
        if relation == SequenceRelation::ExactlyInOrder {
            self.consume(&TokenType::In, "Expected 'in order'")?;
            self.assertion_word("order")?;
        }
        Ok(Matcher::Sequence {
            expected,
            relation,
            negated,
        })
    }

    /// Parse count and argument-snapshot verification clauses.
    fn parse_call_matcher(&mut self) -> ParseResult<Matcher> {
        self.assertion_word("have")?;
        let never = self.test_word("never");
        if never {
            self.advance();
        }
        self.assertion_word("been")?;
        self.assertion_word("called")?;
        if never {
            return Ok(Matcher::CallCount {
                count: ExpectedCallCount::Never,
            });
        }
        if self.test_word("once") {
            self.advance();
            return Ok(Matcher::CallCount {
                count: ExpectedCallCount::Once,
            });
        }
        if self.test_word("exactly") {
            self.advance();
            let count = self.parse_expression()?;
            self.assertion_word("times")?;
            return Ok(Matcher::CallCount {
                count: ExpectedCallCount::Exactly(count),
            });
        }
        self.assertion_word("with")?;
        self.consume(&TokenType::LeftParen, "Expected argument list")?;
        let mut arguments = Vec::new();
        self.skip_newlines();
        while !self.check(&TokenType::RightParen) {
            arguments.push(self.parse_expression()?);
            self.skip_newlines();
            if !self.check(&TokenType::Comma) {
                break;
            }
            self.advance();
            self.skip_newlines();
        }
        self.consume(&TokenType::RightParen, "Expected ')' after call arguments")?;
        Ok(Matcher::CalledWith { arguments })
    }

    /// Consume a contextual matcher word without reserving it in application code.
    fn assertion_word(&mut self, word: &str) -> ParseResult<()> {
        if !self.test_word(word) {
            return Err(self.test_syntax_error(&format!("Expected '{word}' in assertion")));
        }
        self.advance();
        Ok(())
    }
}
