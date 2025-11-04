//! Helper methods for parser token navigation and state management
//!
//! This module contains utility functions used throughout the parser for:
//! - Token stream navigation (current, previous, advance)
//! - Token type checking (`check`, `check_identifier`)
//! - Token consumption (`consume`)
//! - Error recovery (`synchronize`, `skip_newlines_and_comments`)

extern crate alloc;
use crate::ast::{Parameter, Stmt, Type};
use crate::parser::{ParseError, ParseResult, Parser};
use crate::token::{Token, TokenType};
use alloc::string::String;
use core::mem;

impl Parser {
    /// Get the current token at the parser's position
    ///
    /// # Panics
    /// This method does not panic as it always returns a valid token reference,
    /// even when at the end of file (returns the EOF token).
    pub(super) fn current_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// Get the previous token (the one before current position)
    ///
    /// Uses saturating subtraction to avoid underflow when at the start.
    ///
    /// # Returns
    /// The token immediately before the current position, or the first token
    /// if the parser is at the start of the token stream.
    pub(super) fn previous_token(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    /// Advance to the next token and return the previous token
    ///
    /// Uses saturating addition to avoid overflow at the end of the stream.
    /// If already at the end, returns the current token without advancing.
    ///
    /// # Returns
    /// The token that was current before advancing.
    pub(super) fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current = self.current.saturating_add(1);
        }
        self.previous_token()
    }

    /// Check if the parser has reached the end of the token stream
    ///
    /// Returns true when either:
    /// - Current position is beyond the token vector
    /// - Current token is the `EndOfFile` token
    ///
    /// # Returns
    /// `true` if at the end of the token stream, `false` otherwise.
    pub(super) fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
            || matches!(self.current_token().token_type, TokenType::EndOfFile)
    }

    /// Check if the current token starts a declaration
    ///
    /// Used to determine when to stop parsing statement sequences
    /// and return control to the declaration parser.
    ///
    /// # Returns
    /// `true` if the current token can start a top-level declaration.
    /// Determine whether the parser is currently positioned at the start of a top-level
    /// declaration. This check treats any token that begins in column one as a new declaration,
    /// ensuring that indentation-based function bodies can include statements like `return` or
    /// `guard` without prematurely terminating the surrounding declaration.
    pub(super) fn is_declaration_start(&self) -> bool {
        if self.is_at_end() {
            return true;
        }

        let token = self.current_token();
        let column = token.span.start.column;

        match &token.token_type {
            &TokenType::DocComment(_)
            | &TokenType::Public
            | &TokenType::Entry
            | &TokenType::Function
            | &TokenType::Type
            | &TokenType::Import
            | &TokenType::Let => column == 1,
            &TokenType::EndOfFile => true,
            _ => false,
        }
    }

    /// Consume a documentation comment that appears within an indented scope rather than at the
    /// top level. This prevents inline documentation from being mistaken for the start of a new
    /// declaration while still preserving top-level documentation comments for subsequent
    /// declarations.
    pub(super) fn consume_inline_doc_comment(&mut self) -> bool {
        if self.is_at_end() {
            return false;
        }

        if matches!(&self.current_token().token_type, &TokenType::DocComment(_))
            && self.current_token().span.start.column > 1
        {
            self.advance();
            return true;
        }

        false
    }

    /// Determine whether the current token indicates that a blockless function or lambda body has
    /// reached the boundary where a new top-level declaration is about to begin.
    pub(super) fn is_blockless_body_terminated(&self) -> bool {
        self.is_at_end() || self.is_declaration_start()
    }

    /// Parse a sequence of statements that form the body of a function or lambda without explicit
    /// braces. This helper respects documentation comments that start at column one so that they
    /// remain attached to subsequent top-level declarations, while still skipping indented
    /// documentation within the body itself.
    pub(super) fn parse_blockless_body_statements(&mut self) -> Vec<Stmt> {
        let mut statements = Vec::new();

        while !self.is_blockless_body_terminated() {
            self.skip_trivia_preserving_doc_comments();

            if self.is_blockless_body_terminated() {
                break;
            }

            if self.consume_inline_doc_comment() {
                continue;
            }

            match self.parse_statement() {
                Ok(statement) => statements.push(statement),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }
        }

        statements
    }

    /// Check if the current token matches the expected token type
    ///
    /// Uses discriminant comparison to match token variants without
    /// comparing their associated data (e.g., all identifiers match,
    /// regardless of their actual name).
    ///
    /// # Arguments
    /// * `token_type` - The token type to check against
    ///
    /// # Returns
    /// `true` if the current token's type matches the expected type.
    pub(super) fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            mem::discriminant(&self.current_token().token_type) == mem::discriminant(token_type)
        }
    }

    /// Check if the current token is an identifier
    ///
    /// Convenience method for the common pattern of checking for any identifier,
    /// regardless of its actual name value.
    ///
    /// # Returns
    /// `true` if the current token is an `Identifier` variant.
    pub(super) fn check_identifier(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.current_token().token_type, TokenType::Identifier(_))
        }
    }

    /// Consume a token of the expected type or return an error
    ///
    /// If the current token matches the expected type, advances the parser
    /// and returns the consumed token. Otherwise, returns a `MissingToken` error.
    ///
    /// # Arguments
    /// * `token_type` - The token type expected at the current position
    /// * `_message` - Error message (currently unused, kept for API compatibility)
    ///
    /// # Returns
    /// The consumed token if successful, or a `ParseError` if the token doesn't match.
    ///
    /// # Errors
    /// Returns `ParseError::MissingToken` if the current token doesn't match the expected type.
    pub(super) fn consume(
        &mut self,
        token_type: &TokenType,
        _message: &str,
    ) -> ParseResult<&Token> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(ParseError::MissingToken {
                expected: format!("{token_type}"),
                span: ParseError::span_from_token(self.current_token()),
            })
        }
    }

    /// Skip newlines and comments in the token stream
    ///
    /// Advances the parser past any newline tokens, single-line comments,
    /// or documentation comments. Used for whitespace-insensitive parsing
    /// in most contexts.
    ///
    /// # Note
    /// Documentation comments are preserved during declaration parsing but
    /// skipped in other contexts where they're not syntactically significant.
    pub(super) fn skip_newlines_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.current_token().token_type {
                TokenType::Newline | TokenType::Comment(_) | TokenType::DocComment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Skip trivia tokens while preserving documentation comments for subsequent parsing.
    pub(super) fn skip_trivia_preserving_doc_comments(&mut self) {
        while !self.is_at_end() {
            match self.current_token().token_type {
                TokenType::Newline | TokenType::Comment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Synchronize the parser after an error by advancing to the next statement
    ///
    /// Implements panic-mode error recovery: after encountering a parse error,
    /// advances the parser to a safe synchronization point (typically the start
    /// of the next statement or declaration) to continue parsing and collect
    /// multiple errors in a single pass.
    ///
    /// # Synchronization Points
    /// - After a newline (end of previous statement)
    /// - Before keywords that start declarations or statements:
    ///   `function`, `let`, `for`, `if`, `while`, `return`, `type`, `import`
    ///
    /// # Error Recovery Strategy
    /// This method enables the parser to continue after errors, collecting
    /// multiple syntax errors in one compilation pass, improving developer
    /// experience by showing all issues at once rather than failing on the first error.
    pub(super) fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(self.previous_token().token_type, TokenType::Newline) {
                return;
            }

            match self.current_token().token_type {
                TokenType::Function
                | TokenType::Let
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Return
                | TokenType::Type
                | TokenType::Import => return,
                _ => {}
            }

            self.advance();
        }
    }

    /// Construct the canonical function signature string used in documentation metadata.
    ///
    /// The output mirrors Opalescent syntax, including parameter types, optional return type,
    /// and declared error clauses so that downstream tooling receives an exact contract view.
    #[must_use]
    pub(super) fn build_function_signature_section(
        name: &str,
        parameters: &[Parameter],
        return_type: Option<&Type>,
        error_types: &[String],
    ) -> String {
        let mut signature = String::from(name);
        signature.push_str(" = f(");

        for (index, parameter) in parameters.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            signature.push_str(&parameter.to_signature_string());
        }

        signature.push(')');

        if let Some(return_ty) = return_type {
            signature.push_str(": ");
            signature.push_str(&return_ty.to_signature_string());
        }

        if !error_types.is_empty() {
            signature.push_str(" errors ");
            for (index, error) in error_types.iter().enumerate() {
                if index > 0 {
                    signature.push_str(", ");
                }
                signature.push_str(error);
            }
        }

        signature
    }
}
