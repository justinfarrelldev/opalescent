//! Parser for the Opalescent programming language
//!
//! This module implements a recursive descent parser that converts tokens
//! into an Abstract Syntax Tree (AST).

#![allow(
    clippy::ref_patterns,
    clippy::needless_borrowed_reference,
    reason = "Using ref patterns for consistent pattern matching style throughout parser"
)]
#![allow(
    clippy::multiple_inherent_impl,
    reason = "Parser implementation is split across multiple submodules for maintainability - this is the intended design"
)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::missing_const_for_fn,
    reason = "Node ID counter must be mutable; arithmetic is intentional for ID generation"
)]

/// Parser submodule for closure capture analysis
mod captures;
/// Parser submodule for declaration parsing (functions, types, imports, let)
mod declarations;
/// Parser error types and error collection
pub mod errors;
/// Parser submodule for expression parsing (literals, operators, lambdas)
mod expressions;
/// Parser submodule for generic parameter declarations
mod generics;
/// Parser submodule for helper methods (token navigation, state management)
mod helpers;
/// Parser submodule for modifier parsing (public, entry, pure, untested)
mod modifiers;
/// Parser submodule for match expression and pattern parsing
mod patterns;
/// Operator precedence definitions
pub mod precedence;
/// Parser submodule for statement parsing (let, return, block, if, for, while, loop)
mod statements;
/// Parser submodule for type parsing (type annotations, function types)
mod types;

#[cfg(test)]
mod tests;

use crate::ast::{AstNode, Decl, NodeId, Program};
use crate::token::{Span, Token, TokenType};

use errors::{ParseError, ParseErrors, ParseResult};
use precedence::Precedence;

/// The main parser struct
#[derive(Debug)]
pub struct Parser {
    /// Vector of tokens to parse
    tokens: Vec<Token>,
    /// Current position in the token stream
    current: usize,
    /// Collection of parse errors encountered during parsing
    errors: ParseErrors,
    /// Comment declarations deferred while finishing indentation blocks.
    deferred_comment_declarations: Vec<Decl>,
    /// Documentation comments deferred while finishing indentation blocks.
    deferred_doc_comments: Vec<(String, Span)>,
    /// Node ID counter for unique AST node identification
    next_node_id: usize,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub const fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: ParseErrors::new(),
            deferred_comment_declarations: Vec::new(),
            deferred_doc_comments: Vec::new(),
            next_node_id: 1,
        }
    }

    /// Generates a unique node ID for AST nodes
    /// Each call returns a monotonically increasing ID
    fn next_node_id(&mut self) -> NodeId {
        let id = self.next_node_id;
        self.next_node_id += 1;
        NodeId(id)
    }

    /// Parse the tokens into a complete program AST
    pub fn parse(mut self) -> (Option<Program>, ParseErrors) {
        let start_span = self.current_token().span;
        let mut declarations = Vec::new();
        let mut parsed_non_import_declaration = false;

        while !self.is_at_end() {
            if !self.deferred_comment_declarations.is_empty() {
                declarations.append(&mut self.deferred_comment_declarations);
            }

            // Skip newlines between declarations and preserve comments as declarations.
            while !self.is_at_end() {
                if self.check(&TokenType::Newline) {
                    self.advance();
                } else if matches!(self.current_token().token_type, TokenType::Comment(_)) {
                    let comment_token = self.advance().clone();
                    declarations.push(Decl::Comment {
                        text: comment_token.lexeme,
                        span: comment_token.span,
                        id: self.next_node_id(),
                    });
                } else {
                    break;
                }
            }

            if self.is_at_end() {
                break;
            }

            match self.parse_declaration() {
                Ok(decl) => {
                    if matches!(&decl, &crate::ast::Decl::Import { .. }) {
                        if parsed_non_import_declaration {
                            self.errors.push(ParseError::InvalidSyntax {
                                message: String::from(
                                    "import declarations must appear before other declarations",
                                ),
                                span: crate::error::LexError::span_from_span(decl.span()),
                            });
                            self.synchronize();
                            continue;
                        }
                    } else {
                        parsed_non_import_declaration = true;
                    }
                    declarations.push(decl);
                }
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }
        }

        if !self.deferred_comment_declarations.is_empty() {
            declarations.append(&mut self.deferred_comment_declarations);
        }

        let end_span = self
            .tokens
            .last()
            .map_or(start_span, |last_token| last_token.span);

        let program_span = Span::new(start_span.start, end_span.end);

        let program = self.errors.is_empty().then(|| Program {
            declarations,
            span: program_span,
            id: self.next_node_id(),
        });

        (program, self.errors)
    }
}
