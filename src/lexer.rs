//! Lexical analysis for the Opalescent programming language
//!
//! This module contains the lexer implementation that tokenizes Opalescent source code.

use crate::error::{LexError, LexErrors};
use crate::token::{Position, Span, Token, TokenType};
use std::collections::HashMap;
use unicode_xid::UnicodeXID;

/// The main lexer struct that tokenizes Opalescent source code
#[derive(Debug)]
pub struct Lexer<'input> {
    input: &'input str,
    chars: std::str::CharIndices<'input>,
    current: Option<(usize, char)>,
    position: Position,
    errors: LexErrors,
    keywords: HashMap<&'static str, TokenType>,
    whitespace_type: Option<WhitespaceType>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WhitespaceType {
    Spaces,
    Tabs,
}

impl<'input> Lexer<'input> {
    /// Create a new lexer for the given input
    pub fn new(input: &'input str) -> Self {
        let mut chars = input.char_indices();
        let current = chars.next();

        let mut keywords = HashMap::new();
        keywords.insert("let", TokenType::Let);
        keywords.insert("mutable", TokenType::Mutable);
        keywords.insert("f", TokenType::Function);
        keywords.insert("return", TokenType::Return);
        keywords.insert("void", TokenType::Void);
        keywords.insert("if", TokenType::If);
        keywords.insert("else", TokenType::Else);
        keywords.insert("for", TokenType::For);
        keywords.insert("while", TokenType::While);
        keywords.insert("in", TokenType::In);
        keywords.insert("break", TokenType::Break);
        keywords.insert("continue", TokenType::Continue);
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
        keywords.insert("as", TokenType::As);
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
        keywords.insert("type_of", TokenType::TypeOf);

        Self {
            input,
            chars,
            current,
            position: Position::start(),
            errors: LexErrors::new(),
            keywords,
            whitespace_type: None,
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
        self.skip_whitespace();

        if self.is_at_end() {
            return None;
        }

        let start_pos = self.position;

        match self.current_char() {
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
                    self.scan_multiline_comment(start_pos)
                } else {
                    self.scan_single_line_comment(start_pos)
                }
            }

            // String literals
            '\'' => self.scan_string_literal(start_pos),

            // Numbers
            c if c.is_ascii_digit() => self.scan_number(start_pos),

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.scan_identifier(start_pos),

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
            self.current.unwrap().0
        };

        let lexeme = self.input[start_offset..end_offset].to_string();
        let span = Span::new(start_pos, end_pos);

        Token::new(token_type, span, lexeme)
    }

    /// Scan a single-line comment
    fn scan_single_line_comment(&mut self, start_pos: Position) -> Option<Token> {
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
            self.current.unwrap().0
        };

        let content = self.input[start_offset + 1..end_offset].trim().to_string();
        let span = Span::new(start_pos, self.position);

        Some(Token::new(
            TokenType::Comment(content),
            span,
            self.input[start_offset..end_offset].to_string(),
        ))
    }

    /// Scan a multi-line comment
    fn scan_multiline_comment(&mut self, start_pos: Position) -> Option<Token> {
        let start_offset = start_pos.offset;

        // Skip the second '#' (first was already skipped in next_token)
        self.advance();

        let mut content = String::new();
        let mut nesting_level = 1;

        while !self.is_at_end() && nesting_level > 0 {
            if self.current_char() == '#' && self.peek() == Some('#') {
                self.advance(); // Skip first #
                self.advance(); // Skip second #
                nesting_level -= 1;
            } else if self.current_char() == '\n' {
                content.push(self.current_char());
                self.advance_line();
            } else {
                content.push(self.current_char());
                self.advance();
            }
        }

        if nesting_level > 0 {
            let span = LexError::span_from_position(start_pos, 2);
            self.errors.push(LexError::UnterminatedComment {
                start: start_pos,
                span,
            });
        }

        let end_offset = if self.is_at_end() {
            self.input.len()
        } else {
            self.current
                .map(|(offset, _)| offset)
                .unwrap_or(self.input.len())
        };

        let span = Span::new(start_pos, self.position);
        let token_type = if content.trim_start().starts_with("Description:") {
            TokenType::DocComment(content.trim().to_string())
        } else {
            TokenType::Comment(content.trim().to_string())
        };

        Some(Token::new(
            token_type,
            span,
            self.input[start_offset..end_offset].to_string(),
        ))
    }

    /// Scan a string literal
    fn scan_string_literal(&mut self, start_pos: Position) -> Option<Token> {
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
            self.current.unwrap().0
        };

        let span = Span::new(start_pos, self.position);

        Some(Token::new(
            TokenType::StringLiteral(value),
            span,
            self.input[start_offset..end_offset].to_string(),
        ))
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
            self.current.unwrap().0
        };

        let number_str = &self.input[start_offset..end_offset];
        let span = Span::new(start_pos, self.position);

        if is_float {
            match number_str.parse::<f64>() {
                Ok(value) => Some(Token::new(
                    TokenType::FloatLiteral(value),
                    span,
                    number_str.to_string(),
                )),
                Err(_) => {
                    let err_span = LexError::span_from_position(start_pos, number_str.len());
                    self.errors.push(LexError::InvalidNumber {
                        number: number_str.to_string(),
                        position: start_pos,
                        span: err_span,
                    });
                    None
                }
            }
        } else {
            match number_str.parse::<i64>() {
                Ok(value) => Some(Token::new(
                    TokenType::IntegerLiteral(value),
                    span,
                    number_str.to_string(),
                )),
                Err(_) => {
                    let err_span = LexError::span_from_position(start_pos, number_str.len());
                    self.errors.push(LexError::InvalidNumber {
                        number: number_str.to_string(),
                        position: start_pos,
                        span: err_span,
                    });
                    None
                }
            }
        }
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self, start_pos: Position) -> Option<Token> {
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
            self.current.unwrap().0
        };

        let identifier = &self.input[start_offset..end_offset];
        let span = Span::new(start_pos, self.position);

        // Check if it's a keyword
        let token_type = if let Some(keyword_type) = self.keywords.get(identifier) {
            keyword_type.clone()
        } else {
            TokenType::Identifier(identifier.to_string())
        };

        Some(Token::new(token_type, span, identifier.to_string()))
    }

    /// Skip whitespace and track whitespace type
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.current_char() {
                ' ' => {
                    if let Some(WhitespaceType::Tabs) = self.whitespace_type {
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
                    if let Some(WhitespaceType::Spaces) = self.whitespace_type {
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
    fn current_char(&self) -> char {
        self.current.map(|(_, c)| c).unwrap_or('\0')
    }

    /// Peek at the next character
    fn peek(&self) -> Option<char> {
        let mut chars = self.chars.clone();
        chars.next().map(|(_, c)| c)
    }

    /// Check if we're at the end of input
    fn is_at_end(&self) -> bool {
        self.current.is_none()
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if self.current.is_some() {
            self.position.column += 1;
            self.position.offset += 1;
        }

        self.current = self.chars.next();
    }

    /// Advance to the next line
    fn advance_line(&mut self) {
        self.position.line += 1;
        self.position.column = 1;
        self.position.offset += 1;
        self.current = self.chars.next();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let lexer = Lexer::new("");
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 1);
        assert!(matches!(tokens[0].token_type, TokenType::EndOfFile));
    }

    #[test]
    fn test_single_tokens() {
        let input = "()[]{}:,";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 9); // 8 tokens + EOF

        let expected = [
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::LeftBracket,
            TokenType::RightBracket,
            TokenType::LeftBrace,
            TokenType::RightBrace,
            TokenType::Colon,
            TokenType::Comma,
        ];

        for (i, expected_type) in expected.iter().enumerate() {
            assert_eq!(tokens[i].token_type, *expected_type);
        }
    }

    #[test]
    fn test_operators() {
        let input = "+ - * / ^ % = < <= > >= =>";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());

        let expected = [
            TokenType::Plus,
            TokenType::Minus,
            TokenType::Multiply,
            TokenType::Divide,
            TokenType::Power,
            TokenType::Modulo,
            TokenType::Assign,
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Arrow,
        ];

        for (i, expected_type) in expected.iter().enumerate() {
            assert_eq!(tokens[i].token_type, *expected_type);
        }
    }

    #[test]
    fn test_keywords() {
        let input = "let mutable f return void if else";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());

        let expected = [
            TokenType::Let,
            TokenType::Mutable,
            TokenType::Function,
            TokenType::Return,
            TokenType::Void,
            TokenType::If,
            TokenType::Else,
        ];

        for (i, expected_type) in expected.iter().enumerate() {
            assert_eq!(tokens[i].token_type, *expected_type);
        }
    }

    #[test]
    fn test_identifiers() {
        let input = "hello_world _private snake_case";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 4); // 3 identifiers + EOF

        if let TokenType::Identifier(name) = tokens[0].token_type.clone() {
            assert_eq!(name, "hello_world");
        } else {
            unreachable!("Expected identifier");
        }
    }

    #[test]
    fn test_numbers() {
        let input = "42 3.14 0 999";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 5); // 4 numbers + EOF

        assert!(matches!(
            tokens[0].token_type,
            TokenType::IntegerLiteral(42)
        ));
        assert!(
            matches!(tokens[1].token_type, TokenType::FloatLiteral(f) if (f - 3.14).abs() < f64::EPSILON)
        );
        assert!(matches!(tokens[2].token_type, TokenType::IntegerLiteral(0)));
        assert!(matches!(
            tokens[3].token_type,
            TokenType::IntegerLiteral(999)
        ));
    }

    #[test]
    fn test_string_literals() {
        let input = r"'hello' 'world with spaces' 'with\nescapes'";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 4); // 3 strings + EOF

        if let TokenType::StringLiteral(s) = tokens[0].token_type.clone() {
            assert_eq!(s, "hello");
        } else {
            unreachable!("Expected string literal");
        }

        if let TokenType::StringLiteral(s) = tokens[2].token_type.clone() {
            assert_eq!(s, "with\nescapes");
        } else {
            unreachable!("Expected string literal with escape");
        }
    }

    #[test]
    fn test_multiline_comment_simple() {
        let input = "## hello world ##";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        if !errors.is_empty() {
            for error in &errors.errors {
                println!("Error: {error}");
            }
        }

        if !tokens.is_empty() {
            println!("Found {} tokens", tokens.len());
        }

        assert!(errors.is_empty());
        assert_eq!(tokens.len(), 2); // comment + EOF

        if let TokenType::Comment(content) = tokens[0].token_type.clone() {
            assert_eq!(content, "hello world");
        } else {
            unreachable!("Expected comment token");
        }
    }

    #[test]
    fn test_comments() {
        let input = "# single line comment\n##\nmulti-line\ncomment\n##";
        let lexer = Lexer::new(input);
        let (tokens, errors) = lexer.tokenize();

        if !errors.is_empty() {
            for error in &errors.errors {
                println!("Error: {error}");
            }
        }

        if !tokens.is_empty() {
            println!("Found {} tokens in test", tokens.len());
        }

        assert!(errors.is_empty());
        // single comment + newline + multiline comment + EOF
        assert_eq!(tokens.len(), 4);

        assert!(matches!(tokens[0].token_type, TokenType::Comment(_)));
        assert!(matches!(tokens[1].token_type, TokenType::Newline));
        assert!(matches!(tokens[2].token_type, TokenType::Comment(_)));
    }

    #[test]
    fn test_unterminated_string() {
        let input = "'unterminated string";
        let lexer = Lexer::new(input);
        let (_tokens, errors) = lexer.tokenize();

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors.errors[0],
            LexError::UnterminatedString { .. }
        ));
    }

    #[test]
    fn test_unexpected_character() {
        let input = "hello @ world";
        let lexer = Lexer::new(input);
        let (_tokens, errors) = lexer.tokenize();

        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors.errors[0],
            LexError::UnexpectedCharacter { character: '@', .. }
        ));
    }
}
