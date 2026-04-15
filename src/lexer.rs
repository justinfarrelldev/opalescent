//! Lexical analysis for the Opalescent programming language
//!
//! This module contains the lexer implementation that tokenizes Opalescent source code.

extern crate alloc;

use crate::error::{LexError, LexErrors};
use crate::token::{self, Position, Span, Token, TokenType};
use alloc::collections::BTreeMap;
use alloc::collections::VecDeque;
use unicode_xid::UnicodeXID;

/// Reserved keywords in the Opalescent language
///
/// This constant provides a single source of truth for all language keywords,
/// used by both the lexer for tokenization and the parser tests for generating
/// valid identifiers in property-based tests.
pub const RESERVED_KEYWORDS: &[&str] = &[
    "and",
    "as",
    "band",
    "bnot",
    "bor",
    "boolean",
    "break",
    "bshl",
    "bshr",
    "bushr",
    "bxor",
    "continue",
    "else",
    "entry",
    "errors",
    "false",
    "float32",
    "float64",
    "for",
    "from",
    "f",
    "guard",
    "if",
    "import",
    "in",
    "int8",
    "int16",
    "int32",
    "int64",
    "into",
    "is",
    "let",
    "loop",
    "match",
    "mutable",
    "not",
    "or",
    "propagate",
    "public",
    "return",
    "string",
    "true",
    "type",
    "type_of",
    "uint8",
    "uint16",
    "uint32",
    "uint64",
    "void",
    "while",
    "xor",
];

/// The main lexer struct that tokenizes Opalescent source code
#[derive(Debug)]
pub struct Lexer<'input> {
    /// The input source code to tokenize
    input: &'input str,
    /// Character iterator with byte position information
    chars: core::str::CharIndices<'input>,
    /// Current character and its position, if not at end
    current: Option<(usize, char)>,
    /// Current position in the source code
    position: Position,
    /// Collection of lexical analysis errors
    errors: LexErrors,
    /// Map of keyword strings to their token types
    keywords: BTreeMap<&'static str, TokenType>,
    /// Type of whitespace detected for consistency checking
    whitespace_type: Option<WhitespaceType>,
    /// Pending virtual tokens waiting to be emitted.
    pending_tokens: VecDeque<Token>,
    /// Active indentation widths, from root to current nesting.
    indentation_stack: Vec<usize>,
    /// Whether the lexer is currently positioned at line start.
    at_line_start: bool,
    /// Whether a block-introducing token was seen on this line.
    line_has_block_starter: bool,
    /// Whether the next line is expected to increase indentation.
    awaiting_block_indent: bool,
}

/// Type of whitespace detected in the source code
///
/// Used to track consistent whitespace usage and detect mixing of spaces and tabs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WhitespaceType {
    /// Whitespace using space characters
    Spaces,
    /// Whitespace using tab characters
    Tabs,
}

impl<'input> Lexer<'input> {
    /// Create a new lexer for the given input
    pub fn new(input: &'input str) -> Self {
        let mut chars = input.char_indices();
        let current = chars.next();

        let mut keywords = BTreeMap::new();
        keywords.insert("let", TokenType::Let);
        keywords.insert("mutable", TokenType::Mutable);
        keywords.insert("f", TokenType::Function);
        keywords.insert("return", TokenType::Return);
        keywords.insert("void", TokenType::Void);
        keywords.insert("if", TokenType::If);
        keywords.insert("match", TokenType::Match);
        keywords.insert("else", TokenType::Else);
        keywords.insert("for", TokenType::For);
        keywords.insert("while", TokenType::While);
        keywords.insert("loop", TokenType::Loop);
        keywords.insert("in", TokenType::In);
        keywords.insert("break", TokenType::Break);
        keywords.insert("continue", TokenType::Continue);
        keywords.insert("errors", TokenType::Errors);
        keywords.insert("guard", TokenType::Guard);
        keywords.insert("into", TokenType::Into);
        keywords.insert("propagate", TokenType::Propagate);
        keywords.insert("int8", TokenType::Int8);
        keywords.insert("int16", TokenType::Int16);
        keywords.insert("int32", TokenType::Int32);
        keywords.insert("int64", TokenType::Int64);
        keywords.insert("uint8", TokenType::UInt8);
        keywords.insert("uint16", TokenType::UInt16);
        keywords.insert("uint32", TokenType::UInt32);
        keywords.insert("uint64", TokenType::UInt64);
        keywords.insert("float32", TokenType::Float32);
        keywords.insert("float64", TokenType::Float64);
        keywords.insert("string", TokenType::String);
        keywords.insert("boolean", TokenType::Boolean);
        keywords.insert("true", TokenType::BooleanLiteral(true));
        keywords.insert("false", TokenType::BooleanLiteral(false));
        keywords.insert("public", TokenType::Public);
        keywords.insert("entry", TokenType::Entry);
        keywords.insert("import", TokenType::Import);
        keywords.insert("from", TokenType::From);
        keywords.insert("as", TokenType::Cast);
        keywords.insert("type", TokenType::Type);
        keywords.insert("and", TokenType::And);
        keywords.insert("or", TokenType::Or);
        keywords.insert("not", TokenType::Not);
        keywords.insert("xor", TokenType::Xor);
        keywords.insert("band", TokenType::BitAnd);
        keywords.insert("bor", TokenType::BitOr);
        keywords.insert("bxor", TokenType::BitXor);
        keywords.insert("bnot", TokenType::BitNot);
        keywords.insert("bshl", TokenType::BitShiftLeft);
        keywords.insert("bshr", TokenType::BitShiftRight);
        keywords.insert("bushr", TokenType::BitUnsignedShiftRight);
        keywords.insert("is", TokenType::Is);
        keywords.insert("div_euclid", TokenType::DivEuclid);
        keywords.insert("mod_euclid", TokenType::ModEuclid);
        keywords.insert("type_of", TokenType::TypeOf);

        Self {
            input,
            chars,
            current,
            position: Position::start(),
            errors: LexErrors::new(),
            keywords,
            whitespace_type: None,
            pending_tokens: VecDeque::new(),
            indentation_stack: vec![0],
            at_line_start: true,
            line_has_block_starter: false,
            awaiting_block_indent: false,
        }
    }

    /// Get all tokens from the input
    pub fn tokenize(mut self) -> (Vec<Token>, LexErrors) {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            if let Some(token) = self.next_token() {
                tokens.push(token);
            }
        }

        while self.indentation_stack.len() > 1 {
            self.indentation_stack.pop();
            tokens.push(self.make_virtual_token(TokenType::Dedent));
        }

        // Add EOF token
        tokens.push(Token::new(
            TokenType::EndOfFile,
            Span::single(self.position),
            String::new(),
        ));

        (tokens, self.errors)
    }

    /// Get the next token from the input
    fn next_token(&mut self) -> Option<Token> {
        if let Some(token) = self.pending_tokens.pop_front() {
            return Some(token);
        }

        self.handle_line_start_indentation();

        if let Some(token) = self.pending_tokens.pop_front() {
            return Some(token);
        }

        self.skip_inline_whitespace();

        if self.is_at_end() {
            return None;
        }

        let start_pos = self.position;

        let token = match self.current_char() {
            // Single character tokens
            '(' => Some(self.make_token(TokenType::LeftParen, start_pos)),
            ')' => Some(self.make_token(TokenType::RightParen, start_pos)),
            '[' => Some(self.make_token(TokenType::LeftBracket, start_pos)),
            ']' => Some(self.make_token(TokenType::RightBracket, start_pos)),
            '{' => Some(self.make_token(TokenType::LeftBrace, start_pos)),
            '}' => Some(self.make_token(TokenType::RightBrace, start_pos)),
            ':' => Some(self.make_token(TokenType::Colon, start_pos)),
            ',' => Some(self.make_token(TokenType::Comma, start_pos)),
            '.' => Some(self.make_token(TokenType::Dot, start_pos)),
            '+' => Some(self.make_token(TokenType::Plus, start_pos)),
            '-' => Some(self.make_token(TokenType::Minus, start_pos)),
            '*' => Some(self.make_token(TokenType::Multiply, start_pos)),
            '/' => Some(self.make_token(TokenType::Divide, start_pos)),
            '^' => Some(self.make_token(TokenType::Power, start_pos)),
            '%' => Some(self.make_token(TokenType::Modulo, start_pos)),

            // Two character tokens
            '=' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Some(self.make_token(TokenType::Arrow, start_pos))
                } else {
                    Some(self.make_token(TokenType::Assign, start_pos))
                }
            }

            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Some(self.make_token(TokenType::LessEqual, start_pos))
                } else {
                    Some(self.make_token(TokenType::Less, start_pos))
                }
            }

            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Some(self.make_token(TokenType::GreaterEqual, start_pos))
                } else {
                    Some(self.make_token(TokenType::Greater, start_pos))
                }
            }

            // Comments
            '#' => {
                if self.peek() == Some('#') {
                    self.advance(); // Skip first #
                    Some(self.scan_multiline_comment(start_pos))
                } else {
                    Some(self.scan_single_line_comment(start_pos))
                }
            }

            // String literals
            '\'' => Some(self.scan_string_literal(start_pos)),

            // Numbers
            c if c.is_ascii_digit() => self.scan_number(start_pos),

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => Some(self.scan_identifier(start_pos)),

            // Newlines
            '\n' => {
                let token = self.make_token(TokenType::Newline, start_pos);
                // Note: make_token already calls advance_line for '\n'
                Some(token)
            }

            // Unknown character
            c => {
                let span = LexError::span_from_position(start_pos, 1);
                self.errors.push(LexError::UnexpectedCharacter {
                    character: c,
                    position: start_pos,
                    span,
                });
                self.advance();
                None
            }
        };

        if let Some(ref token) = token {
            self.update_block_indentation_state(token);
        }

        token
    }

    /// Build a virtual Indent or Dedent token at current position.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "Token::new is not const, but helper clarity is preferred"
    )]
    fn make_virtual_token(&self, token_type: TokenType) -> Token {
        Token::new(token_type, Span::single(self.position), String::new())
    }

    /// Process leading indentation and queue virtual indentation tokens.
    fn handle_line_start_indentation(&mut self) {
        if !self.at_line_start || self.is_at_end() {
            return;
        }

        let mut indentation = 0_usize;

        while !self.is_at_end() {
            match self.current_char() {
                ' ' => {
                    if self.whitespace_type == Some(WhitespaceType::Tabs) {
                        let tab_span = LexError::span_from_position(self.position, 1);
                        let space_span = LexError::span_from_position(self.position, 1);
                        self.errors.push(LexError::MixedWhitespace {
                            tab_span,
                            space_span,
                        });
                    } else {
                        self.whitespace_type = Some(WhitespaceType::Spaces);
                    }
                    indentation = indentation.saturating_add(1);
                    self.advance();
                }
                '\t' => {
                    if self.whitespace_type == Some(WhitespaceType::Spaces) {
                        let tab_span = LexError::span_from_position(self.position, 1);
                        let space_span = LexError::span_from_position(self.position, 1);
                        self.errors.push(LexError::MixedWhitespace {
                            tab_span,
                            space_span,
                        });
                    } else {
                        self.whitespace_type = Some(WhitespaceType::Tabs);
                    }
                    indentation = indentation.saturating_add(1);
                    self.advance();
                }
                '\r' => self.advance(),
                _ => break,
            }
        }

        if self.is_at_end() {
            return;
        }

        if matches!(self.current_char(), '\n' | '#') {
            return;
        }

        let current = self.indentation_stack.last().copied().unwrap_or(0);

        if indentation < current {
            while self
                .indentation_stack
                .last()
                .copied()
                .is_some_and(|level| level > indentation)
            {
                self.indentation_stack.pop();
                self.pending_tokens
                    .push_back(self.make_virtual_token(TokenType::Dedent));
            }
        } else if indentation > current && self.awaiting_block_indent {
            self.indentation_stack.push(indentation);
            self.pending_tokens
                .push_back(self.make_virtual_token(TokenType::Indent));
        }

        self.awaiting_block_indent = false;
        self.at_line_start = false;
    }

    /// Advance indentation state based on the emitted token.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "State transitions mutate lexer runtime state"
    )]
    fn update_block_indentation_state(&mut self, token: &Token) {
        match token.token_type {
            TokenType::Colon | TokenType::Arrow => {
                self.line_has_block_starter = true;
                self.at_line_start = false;
            }
            TokenType::Newline => {
                if self.line_has_block_starter {
                    self.awaiting_block_indent = true;
                }
                self.line_has_block_starter = false;
                self.at_line_start = true;
            }
            TokenType::Comment(_) | TokenType::DocComment(_) => {}
            _ => {
                self.line_has_block_starter = false;
                self.at_line_start = false;
            }
        }
    }

    /// Make a token advancing past the current character
    fn make_token(&mut self, token_type: TokenType, start_pos: Position) -> Token {
        let start_offset = start_pos.offset;
        let current_char = self.current_char();

        if current_char == '\n' {
            self.advance_line();
        } else {
            self.advance();
        }

        let end_pos = self.position;
        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let lexeme = self
            .input
            .get(start_offset..end_offset)
            .unwrap_or_default()
            .to_owned();
        let span = Span::new(start_pos, end_pos);

        Token::new(token_type, span, lexeme)
    }

    /// Scan a single-line comment
    fn scan_single_line_comment(&mut self, start_pos: Position) -> token::Token {
        let start_offset = start_pos.offset;

        // Skip the '#'
        self.advance();

        // Read until end of line
        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let content = self
            .input
            .get(start_offset.saturating_add(1)..end_offset)
            .unwrap_or_default()
            .trim()
            .to_owned();
        let span = Span::new(start_pos, self.position);

        Token::new(
            TokenType::Comment(content),
            span,
            self.input
                .get(start_offset..end_offset)
                .unwrap_or_default()
                .to_owned(),
        )
    }

    /// Scan a multi-line comment
    fn scan_multiline_comment(&mut self, start_pos: Position) -> token::Token {
        let start_offset = start_pos.offset;

        // Skip the second '#' (first was already skipped in next_token)
        self.advance();

        let mut content = String::new();
        let mut nesting_level = 1_i32;

        while !self.is_at_end() && nesting_level > 0_i32 {
            if self.current_char() == '#' && self.peek() == Some('#') {
                self.advance(); // Skip first #
                self.advance(); // Skip second #
                nesting_level = nesting_level.saturating_sub(1_i32);
            } else if self.current_char() == '\n' {
                content.push(self.current_char());
                self.advance_line();
            } else {
                content.push(self.current_char());
                self.advance();
            }
        }

        if nesting_level > 0_i32 {
            let span = LexError::span_from_position(start_pos, 2);
            self.errors.push(LexError::UnterminatedComment {
                start: start_pos,
                span,
            });
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let normalized_content = content.trim_matches('\n').to_owned();

        let span = Span::new(start_pos, self.position);
        let token_type = if normalized_content.trim_start().starts_with("Description:") {
            TokenType::DocComment(normalized_content)
        } else {
            TokenType::Comment(normalized_content.trim().to_owned())
        };

        Token::new(
            token_type,
            span,
            self.input
                .get(start_offset..end_offset)
                .unwrap_or_default()
                .to_owned(),
        )
    }

    /// Scan a string literal
    fn scan_string_literal(&mut self, start_pos: Position) -> token::Token {
        let start_offset = start_pos.offset;

        // Skip opening quote
        self.advance();

        let mut value = String::new();

        while !self.is_at_end() && self.current_char() != '\'' {
            if self.current_char() == '\\' {
                self.advance();
                if self.is_at_end() {
                    break;
                }

                match self.current_char() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '\'' => value.push('\''),
                    '{' => value.push('{'),
                    '}' => value.push('}'),
                    c => {
                        let span = LexError::span_from_position(self.position, 1);
                        self.errors.push(LexError::InvalidEscapeSequence {
                            sequence: c.to_string(),
                            position: self.position,
                            span,
                        });
                        value.push(c);
                    }
                }
                self.advance();
            } else if self.current_char() == '\n' {
                self.advance_line();
                value.push('\n');
            } else {
                value.push(self.current_char());
                self.advance();
            }
        }

        if self.is_at_end() {
            let span = LexError::span_from_position(start_pos, 1);
            self.errors.push(LexError::UnterminatedString {
                start: start_pos,
                span,
            });
        } else {
            // Skip closing quote
            self.advance();
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let span = Span::new(start_pos, self.position);

        Token::new(
            TokenType::StringLiteral(value),
            span,
            self.input
                .get(start_offset..end_offset)
                .unwrap_or_default()
                .to_owned(),
        )
    }

    /// Scan a number literal
    fn scan_number(&mut self, start_pos: Position) -> Option<Token> {
        let start_offset = start_pos.offset;

        // Scan integer part
        while !self.is_at_end() && self.current_char().is_ascii_digit() {
            self.advance();
        }

        let mut is_float = false;

        // Check for decimal point
        if !self.is_at_end()
            && self.current_char() == '.'
            && self.peek().is_some_and(|c| c.is_ascii_digit())
        {
            is_float = true;
            self.advance(); // consume '.'

            // Scan fractional part
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let number_str = self.input.get(start_offset..end_offset).unwrap_or_default();
        let span = Span::new(start_pos, self.position);

        if is_float {
            if let Ok(value) = number_str.parse::<f64>() {
                Some(Token::new(
                    TokenType::FloatLiteral(value),
                    span,
                    number_str.to_owned(),
                ))
            } else {
                let err_span = LexError::span_from_position(start_pos, number_str.len());
                self.errors.push(LexError::InvalidNumber {
                    number: number_str.to_owned(),
                    position: start_pos,
                    span: err_span,
                });
                None
            }
        } else if let Ok(value) = number_str.parse::<i64>() {
            Some(Token::new(
                TokenType::IntegerLiteral(value),
                span,
                number_str.to_owned(),
            ))
        } else {
            let err_span = LexError::span_from_position(start_pos, number_str.len());
            self.errors.push(LexError::InvalidNumber {
                number: number_str.to_owned(),
                position: start_pos,
                span: err_span,
            });
            None
        }
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self, start_pos: Position) -> token::Token {
        let start_offset = start_pos.offset;

        // First character is already validated
        self.advance();

        // Continue with identifier characters
        while !self.is_at_end() {
            let c = self.current_char();
            if c.is_xid_continue() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current.map_or(self.input.len(), |(offset, _)| offset)
        };

        let identifier = self.input.get(start_offset..end_offset).unwrap_or_default();
        let mut span = Span::new(start_pos, self.position);

        let mut token_type = self.keywords.get(identifier).map_or_else(
            || TokenType::Identifier(identifier.to_owned()),
            Clone::clone,
        );

        if identifier == "is"
            && matches!(token_type, TokenType::Is)
            && self.peek_keyword_after_whitespace().is_some()
        {
            self.skip_inline_whitespace();
            self.advance();
            while !self.is_at_end() {
                let c = self.current_char();
                if c.is_xid_continue() || c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }
            span = Span::new(start_pos, self.position);
            token_type = TokenType::IsNot;
        }

        Token::new(token_type, span, identifier.to_owned())
    }

    /// Peek ahead for the next keyword after skipping inline whitespace, returning the keyword name if found.
    /// Returns Some("not") if the next keyword after whitespace is "not", otherwise None.
    fn peek_keyword_after_whitespace(&self) -> Option<&'static str> {
        let start_offset = self.current.map_or(self.input.len(), |(off, _)| off);
        let mut chars = self.input.chars().skip(start_offset).collect::<Vec<_>>();

        // Skip inline whitespace (space, tab, carriage return)
        while let Some(&c) = chars.first() {
            match c {
                ' ' | '\t' | '\r' => {
                    chars.remove(0);
                }
                _ => break,
            }
        }

        // Collect the next identifier
        let mut ident = String::new();
        for c in &chars {
            if c.is_xid_continue() || *c == '_' {
                ident.push(*c);
            } else {
                break;
            }
        }

        if ident.is_empty() {
            return None;
        }

        self.keywords
            .get(ident.as_str())
            .and_then(|tt| (*tt == TokenType::Not).then_some("not"))
    }

    /// Skip inline whitespace while preserving newline token boundaries.
    fn skip_inline_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' => {
                    if self.whitespace_type == Some(WhitespaceType::Tabs) {
                        let tab_span = LexError::span_from_position(self.position, 1);
                        let space_span = LexError::span_from_position(self.position, 1);
                        self.errors.push(LexError::MixedWhitespace {
                            tab_span,
                            space_span,
                        });
                    } else {
                        self.whitespace_type = Some(WhitespaceType::Spaces);
                    }
                    self.advance();
                }
                '\t' => {
                    if self.whitespace_type == Some(WhitespaceType::Spaces) {
                        let tab_span = LexError::span_from_position(self.position, 1);
                        let space_span = LexError::span_from_position(self.position, 1);
                        self.errors.push(LexError::MixedWhitespace {
                            tab_span,
                            space_span,
                        });
                    } else {
                        self.whitespace_type = Some(WhitespaceType::Tabs);
                    }
                    self.advance();
                }
                '\r' => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Get the current character
    const fn current_char(&self) -> char {
        match self.current {
            Some((_, c)) => c,
            None => '\0',
        }
    }

    /// Peek at the next character
    fn peek(&self) -> Option<char> {
        let mut chars = self.chars.clone();
        chars.next().map(|(_, c)| c)
    }

    /// Check if we're at the end of input
    const fn is_at_end(&self) -> bool {
        self.current.is_none()
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if let Some((_, ch)) = self.current {
            self.position.column = self.position.column.saturating_add(1_usize);
            self.position.offset = self.position.offset.saturating_add(ch.len_utf8());
        }

        self.current = self.chars.next();
    }

    /// Advance to the next line
    fn advance_line(&mut self) {
        self.position.line = self.position.line.saturating_add(1_usize);
        self.position.column = 1_usize;
        if let Some((_, ch)) = self.current {
            self.position.offset = self.position.offset.saturating_add(ch.len_utf8());
        }
        self.current = self.chars.next();
    }
}

#[cfg(test)]
#[path = "lexer/tests.rs"]
mod tests;
