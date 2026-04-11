//! Parser for the Opalescent programming language
//!
//! This module implements a recursive descent parser that converts tokens
//! into an Abstract Syntax Tree (AST).

#![expect(
    dead_code,
    reason = "Parser features are being developed incrementally"
)]
#![allow(
    clippy::ref_patterns,
    clippy::needless_borrowed_reference,
    reason = "Using ref patterns for consistent pattern matching style throughout parser"
)]
#![allow(
    clippy::multiple_inherent_impl,
    reason = "Parser implementation is split across multiple submodules for maintainability - this is the intended design"
)]

/// Parser submodule for declaration parsing (functions, types, imports, let)
mod declarations;
/// Parser error types and error collection
pub mod errors;
/// Parser submodule for expression parsing (literals, operators, lambdas)
mod expressions;
/// Parser submodule for helper methods (token navigation, state management)
mod helpers;
/// Parser submodule for generic parameter declarations
mod generics;
/// Operator precedence definitions
pub mod precedence;
/// Parser submodule for statement parsing (let, return, block, if, for, while, loop)
mod statements;
/// Parser submodule for type parsing (type annotations, function types)
mod types;

#[cfg(test)]
mod tests;

use crate::ast::{NodeId, Program};
use crate::token::{Span, Token};
use core::sync::atomic::{AtomicUsize, Ordering};

use errors::{ParseError, ParseErrors, ParseResult};
use precedence::Precedence;

/// Node ID generator for unique AST node identification
static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

/// Generates a unique node ID for AST nodes
/// Each call returns a monotonically increasing ID
fn next_node_id() -> NodeId {
    NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
}

/// The main parser struct
#[derive(Debug)]
pub struct Parser {
    /// Vector of tokens to parse
    tokens: Vec<Token>,
    /// Current position in the token stream
    current: usize,
    /// Collection of parse errors encountered during parsing
    errors: ParseErrors,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: ParseErrors::new(),
        }
    }

    /// Parse the tokens into a complete program AST
    pub fn parse(mut self) -> (Option<Program>, ParseErrors) {
        let start_span = self.current_token().span;
        let mut declarations = Vec::new();

        // Skip initial newlines and comments
        self.skip_trivia_preserving_doc_comments();

        while !self.is_at_end() {
            // Skip newlines between declarations while preserving doc comments
            self.skip_trivia_preserving_doc_comments();

            if self.is_at_end() {
                break;
            }

            match self.parse_declaration() {
                Ok(decl) => declarations.push(decl),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }
        }

        let end_span = self
            .tokens
            .last()
            .map_or(start_span, |last_token| last_token.span);

        let program_span = Span::new(start_span.start, end_span.end);

        let program = self.errors.is_empty().then(|| Program {
            declarations,
            span: program_span,
            id: next_node_id(),
        });

        (program, self.errors)
    }
}
