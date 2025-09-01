//! Parser for the Opalescent programming language
//!
//! This module implements a recursive descent parser that converts tokens
//! into an Abstract Syntax Tree (AST).

#![expect(
    dead_code,
    reason = "Parser features are being developed incrementally"
)]

use crate::ast::*;
use crate::error::LexError;
use crate::token::{Span, Token, TokenType};
use miette::{Diagnostic, SourceSpan};
use std::sync::atomic::{AtomicUsize, Ordering};
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_token),
        help("Check the syntax around this location")
    )]
    UnexpectedToken {
        expected: String,
        found: String,
        #[label("unexpected token")]
        span: SourceSpan,
    },

    #[error("Missing token: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::missing_token),
        help("Add the missing {expected}")
    )]
    MissingToken {
        expected: String,
        #[label("expected {expected} here")]
        span: SourceSpan,
    },

    #[error("Invalid syntax: {message}")]
    #[diagnostic(
        code(opalescent::parser::invalid_syntax),
        help("Check the language specification for correct syntax")
    )]
    InvalidSyntax {
        message: String,
        #[label("invalid syntax")]
        span: SourceSpan,
    },

    #[error("Unexpected end of file: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_eof),
        help("Complete the {expected}")
    )]
    UnexpectedEof {
        expected: String,
        #[label("file ends here")]
        span: SourceSpan,
    },
}

impl ParseError {
    pub fn span_from_token(token: &Token) -> SourceSpan {
        LexError::span_from_span(token.span)
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Collection of parse errors for multiple error reporting
#[derive(Debug)]
pub struct ParseErrors {
    pub errors: Vec<ParseError>,
}

impl ParseErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }
}

impl Default for ParseErrors {
    fn default() -> Self {
        Self::new()
    }
}

/// Node ID generator for unique AST node identification
static NEXT_NODE_ID: AtomicUsize = AtomicUsize::new(1);

fn next_node_id() -> NodeId {
    NodeId(NEXT_NODE_ID.fetch_add(1, Ordering::Relaxed))
}

/// Operator precedence levels (higher number = higher precedence)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    None = 0,
    Assignment = 1, // =
    Or = 2,         // or
    Xor = 3,        // xor
    And = 4,        // and
    BitOr = 5,      // bor
    BitXor = 6,     // bxor
    BitAnd = 7,     // band
    Equality = 8,   // is, is not
    Comparison = 9, // <, <=, >, >=
    Shift = 10,     // bshl, bshr, bushr
    Term = 11,      // +, -
    Factor = 12,    // *, /, %
    Power = 13,     // ^ (right-associative)
    Unary = 14,     // +x, -x, not x, bnot x
    Call = 15,      // function calls, array access
    Primary = 16,   // literals, identifiers, parentheses
}

impl Precedence {
    pub fn from_token(token_type: &TokenType) -> Self {
        match token_type {
            // Remove assignment from expression precedence since it's a statement
            TokenType::Or => Precedence::Or,
            TokenType::Xor => Precedence::Xor,
            TokenType::And => Precedence::And,
            TokenType::BitOr => Precedence::BitOr,
            TokenType::BitXor => Precedence::BitXor,
            TokenType::BitAnd => Precedence::BitAnd,
            TokenType::Is | TokenType::IsNot => Precedence::Equality,
            TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual => Precedence::Comparison,
            TokenType::BitShiftLeft
            | TokenType::BitShiftRight
            | TokenType::BitUnsignedShiftRight => Precedence::Shift,
            TokenType::Plus | TokenType::Minus => Precedence::Term,
            TokenType::Multiply | TokenType::Divide | TokenType::Modulo => Precedence::Factor,
            TokenType::Power => Precedence::Power,
            TokenType::LeftParen | TokenType::LeftBracket | TokenType::Dot => Precedence::Call,
            _ => Precedence::None,
        }
    }
}

/// The main parser struct
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    errors: ParseErrors,
}

impl Parser {
    /// Create a new parser with the given tokens
    pub fn new(tokens: Vec<Token>) -> Self {
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
        self.skip_newlines_and_comments();

        while !self.is_at_end() {
            // Skip newlines between declarations
            self.skip_newlines_and_comments();

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

        let end_span = if let Some(last_token) = self.tokens.last() {
            last_token.span
        } else {
            start_span
        };

        let program_span = Span::new(start_span.start, end_span.end);

        let program = if self.errors.is_empty() {
            Some(Program {
                declarations,
                span: program_span,
                id: next_node_id(),
            })
        } else {
            None
        };

        (program, self.errors)
    }

    /// Parse a top-level declaration
    fn parse_declaration(&mut self) -> ParseResult<Decl> {
        // Check for documentation comment
        let doc_comment = if self.check(&TokenType::DocComment(String::new())) {
            if let TokenType::DocComment(content) = &self.current_token().token_type {
                let comment = content.clone();
                self.advance();
                self.skip_newlines_and_comments();
                Some(comment)
            } else {
                None
            }
        } else {
            None
        };

        // Check for visibility modifiers
        let visibility = if self.check(&TokenType::Public) {
            self.advance();
            Visibility::Public
        } else {
            Visibility::Private
        };

        // Check for entry keyword
        let is_entry = if self.check(&TokenType::Entry) {
            self.advance();
            true
        } else {
            false
        };

        // For entry and public functions, expect identifier next
        // For regular functions, expect 'f' keyword
        match &self.current_token().token_type {
            TokenType::Function => {
                self.parse_function_declaration(visibility, is_entry, doc_comment)
            }
            TokenType::Type => self.parse_type_declaration(visibility, doc_comment),
            TokenType::Import => self.parse_import_declaration(),
            TokenType::Identifier(_) if is_entry || visibility == Visibility::Public => {
                // This is a function declaration starting with identifier (entry main = f() or public foo = f())
                self.parse_function_declaration(visibility, is_entry, doc_comment)
            }
            _ => {
                let token = self.current_token();
                Err(ParseError::UnexpectedToken {
                    expected: "declaration (function, type, or import)".to_string(),
                    found: format!("{}", token.token_type),
                    span: ParseError::span_from_token(token),
                })
            }
        }
    }

    /// Parse a function declaration
    fn parse_function_declaration(
        &mut self,
        visibility: Visibility,
        is_entry: bool,
        doc_comment: Option<String>,
    ) -> ParseResult<Decl> {
        let start_span = self.current_token().span;

        // Parse function name - could start with 'f' keyword or identifier
        let name = if self.check(&TokenType::Function) {
            // This is a pattern like: f = f() => ... (anonymous function, we'll error for now)
            return Err(ParseError::InvalidSyntax {
                message: "Anonymous functions not supported at top level".to_string(),
                span: ParseError::span_from_token(self.current_token()),
            });
        } else if self.check_identifier() {
            // This is a pattern like: main = f() => ... or entry main = f() => ...
            let token = self.advance();
            if let TokenType::Identifier(name) = &token.token_type {
                name.clone()
            } else {
                unreachable!("check_identifier should guarantee this")
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "function name".to_string(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Expect '='
        self.consume(&TokenType::Assign, "Expected '=' after function name")?;

        // Expect 'f'
        self.consume(&TokenType::Function, "Expected 'f' after '='")?;

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'f'")?;

        // Parse parameters
        let mut parameters = Vec::new();
        if !self.check(&TokenType::RightParen) {
            loop {
                let param = self.parse_parameter()?;
                parameters.push(param);

                if self.check(&TokenType::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Expect ')'
        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;

        // Parse optional return type
        let return_type = if self.check(&TokenType::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Expect '=>'
        self.consume(&TokenType::Arrow, "Expected '=>' after function signature")?;

        // Parse function body - can be a single statement or block
        let body = if self.check(&TokenType::LeftBrace) {
            self.parse_block_statement()?
        } else {
            // Parse statements until we reach the end or a new declaration
            let mut statements = Vec::new();

            while !self.is_at_end() && !self.is_declaration_start() {
                self.skip_newlines_and_comments();

                if self.is_at_end() || self.is_declaration_start() {
                    break;
                }

                match self.parse_statement() {
                    Ok(stmt) => statements.push(stmt),
                    Err(error) => {
                        self.errors.push(error);
                        self.synchronize();
                        break;
                    }
                }
            }

            let body_start = if let Some(first_stmt) = statements.first() {
                first_stmt.span().start
            } else {
                self.previous_token().span.start
            };

            let body_end = if let Some(last_stmt) = statements.last() {
                last_stmt.span().end
            } else {
                self.previous_token().span.end
            };

            let body_span = Span::new(body_start, body_end);

            Stmt::Block {
                statements,
                span: body_span,
                id: next_node_id(),
            }
        };

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Decl::Function {
            name,
            parameters,
            return_type,
            body,
            visibility,
            is_entry,
            doc_comment,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a function parameter
    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let start_span = self.current_token().span;

        let name = if self.check_identifier() {
            let token = self.advance();
            if let TokenType::Identifier(name) = &token.token_type {
                name.clone()
            } else {
                unreachable!()
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "parameter name".to_string(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        self.consume(&TokenType::Colon, "Expected ':' after parameter name")?;

        let param_type = self.parse_type()?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Parameter {
            name,
            param_type,
            span,
        })
    }

    /// Parse a type declaration (placeholder)
    fn parse_type_declaration(
        &self,
        _visibility: Visibility,
        _doc_comment: Option<String>,
    ) -> ParseResult<Decl> {
        // TODO: Implement type declaration parsing
        Err(ParseError::InvalidSyntax {
            message: "Type declarations not yet implemented".to_string(),
            span: ParseError::span_from_token(self.current_token()),
        })
    }

    /// Parse an import declaration (placeholder)
    fn parse_import_declaration(&self) -> ParseResult<Decl> {
        // TODO: Implement import declaration parsing
        Err(ParseError::InvalidSyntax {
            message: "Import declarations not yet implemented".to_string(),
            span: ParseError::span_from_token(self.current_token()),
        })
    }

    /// Parse a type annotation
    fn parse_type(&mut self) -> ParseResult<Type> {
        let start_span = self.current_token().span;

        // For now, just parse basic types
        let name = match &self.current_token().token_type {
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            TokenType::Int8 => {
                self.advance();
                "int8".to_string()
            }
            TokenType::Int16 => {
                self.advance();
                "int16".to_string()
            }
            TokenType::Int32 => {
                self.advance();
                "int32".to_string()
            }
            TokenType::Int64 => {
                self.advance();
                "int64".to_string()
            }
            TokenType::UInt8 => {
                self.advance();
                "uint8".to_string()
            }
            TokenType::UInt16 => {
                self.advance();
                "uint16".to_string()
            }
            TokenType::UInt32 => {
                self.advance();
                "uint32".to_string()
            }
            TokenType::UInt64 => {
                self.advance();
                "uint64".to_string()
            }
            TokenType::Float32 => {
                self.advance();
                "float32".to_string()
            }
            TokenType::Float64 => {
                self.advance();
                "float64".to_string()
            }
            TokenType::String => {
                self.advance();
                "string".to_string()
            }
            TokenType::Boolean => {
                self.advance();
                "boolean".to_string()
            }
            TokenType::Void => {
                self.advance();
                "void".to_string()
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "type name".to_string(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        };

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        // Check for array type syntax (type[])
        if self.check(&TokenType::LeftBracket) {
            self.advance();
            self.consume(&TokenType::RightBracket, "Expected ']' after '['")?;

            let array_end_span = self.previous_token().span;
            let array_span = Span::new(start_span.start, array_end_span.end);

            Ok(Type::Array {
                element_type: Box::new(Type::Basic { name, span }),
                span: array_span,
            })
        } else {
            Ok(Type::Basic { name, span })
        }
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines_and_comments();

        match &self.current_token().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::LeftBrace => self.parse_block_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::For => self.parse_for_statement(),
            TokenType::While => self.parse_while_statement(),
            TokenType::Break => self.parse_break_statement(),
            TokenType::Continue => self.parse_continue_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    /// Parse a let statement
    fn parse_let_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'let'

        // Check for mutable
        let is_mutable = if self.check(&TokenType::Mutable) {
            self.advance();
            true
        } else {
            false
        };

        // Parse variable name
        let name = if self.check_identifier() {
            let token = self.advance();
            if let TokenType::Identifier(name) = &token.token_type {
                name.clone()
            } else {
                unreachable!()
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_string(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse optional type annotation
        let type_annotation = if self.check(&TokenType::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };

        // Parse optional initializer
        let initializer = if self.check(&TokenType::Assign) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        if is_mutable {
            Ok(Stmt::Mutable {
                name,
                type_annotation,
                initializer,
                span,
                id: next_node_id(),
            })
        } else {
            Ok(Stmt::Let {
                name,
                type_annotation,
                initializer,
                span,
                id: next_node_id(),
            })
        }
    }

    /// Parse a return statement
    fn parse_return_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'return'

        // Parse optional return value
        let value = if self.check(&TokenType::Newline) || self.is_at_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Return {
            value,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a block statement
    fn parse_block_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.consume(&TokenType::LeftBrace, "Expected '{'")?;

        let mut statements = Vec::new();

        while !self.check(&TokenType::RightBrace) && !self.is_at_end() {
            self.skip_newlines_and_comments();

            if self.check(&TokenType::RightBrace) {
                break;
            }

            match self.parse_statement() {
                Ok(stmt) => statements.push(stmt),
                Err(error) => {
                    self.errors.push(error);
                    self.synchronize();
                }
            }
        }

        self.consume(&TokenType::RightBrace, "Expected '}' after block")?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Block {
            statements,
            span,
            id: next_node_id(),
        })
    }

    /// Parse an if statement
    fn parse_if_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'if'

        // Parse condition
        let condition = self.parse_expression()?;

        // Parse then branch (must be a block)
        let then_branch = Box::new(self.parse_block_statement()?);

        // Parse optional else branch
        let else_branch = if self.check(&TokenType::Else) {
            self.advance(); // consume 'else'
            Some(Box::new(self.parse_statement()?))
        } else {
            None
        };

        let end_span = if let Some(ref else_stmt) = else_branch {
            else_stmt.span()
        } else {
            then_branch.span()
        };

        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a for statement
    fn parse_for_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'for'

        // Parse variable name
        let variable = if self.check_identifier() {
            let token = self.advance();
            if let TokenType::Identifier(name) = &token.token_type {
                name.clone()
            } else {
                unreachable!("check_identifier should guarantee this")
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_string(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse 'in' keyword
        self.consume(&TokenType::In, "Expected 'in' after for variable")?;

        // Parse iterable expression
        let iterable = self.parse_expression()?;

        // Parse body (must be a block)
        let body = Box::new(self.parse_block_statement()?);

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::For {
            variable,
            iterable,
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a while statement
    fn parse_while_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'while'

        // Parse condition
        let condition = self.parse_expression()?;

        // Parse body (must be a block)
        let body = Box::new(self.parse_block_statement()?);

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::While {
            condition,
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a break statement
    fn parse_break_statement(&mut self) -> ParseResult<Stmt> {
        let span = self.current_token().span;
        self.advance(); // consume 'break'

        Ok(Stmt::Break {
            span,
            id: next_node_id(),
        })
    }

    /// Parse a continue statement
    fn parse_continue_statement(&mut self) -> ParseResult<Stmt> {
        let span = self.current_token().span;
        self.advance(); // consume 'continue'

        Ok(Stmt::Continue {
            span,
            id: next_node_id(),
        })
    }

    /// Parse an expression statement or assignment statement
    fn parse_expression_statement(&mut self) -> ParseResult<Stmt> {
        let expr = self.parse_expression()?;

        // Check if this is an assignment
        if self.check(&TokenType::Assign) {
            let start_span = expr.span();
            self.advance(); // consume '='

            let value = self.parse_expression()?;
            let end_span = value.span();
            let span = Span::new(start_span.start, end_span.end);

            // Validate that the target is assignable
            match &expr {
                Expr::Identifier { .. } | Expr::Index { .. } | Expr::Member { .. } => {
                    // Valid assignment targets
                }
                _ => {
                    return Err(ParseError::InvalidSyntax {
                        message: "Invalid assignment target".to_string(),
                        span: LexError::span_from_span(start_span),
                    });
                }
            }

            Ok(Stmt::Assignment {
                target: expr,
                value,
                span,
                id: next_node_id(),
            })
        } else {
            // Regular expression statement
            let span = expr.span();
            Ok(Stmt::Expression {
                expr,
                span,
                id: next_node_id(),
            })
        }
    }

    /// Parse an expression using Pratt parsing
    fn parse_expression(&mut self) -> ParseResult<Expr> {
        self.parse_precedence(Precedence::None)
    }

    /// Parse expression with given precedence (Pratt parser)
    fn parse_precedence(&mut self, precedence: Precedence) -> ParseResult<Expr> {
        // Parse prefix expression
        let mut expr = self.parse_primary()?;

        // Parse infix expressions
        while !self.is_at_end() {
            let token_precedence = Precedence::from_token(&self.current_token().token_type);

            // Break if the current token has lower precedence or is not an infix operator
            if precedence > token_precedence || token_precedence == Precedence::None {
                break;
            }

            expr = self.parse_infix(expr)?;
        }

        Ok(expr)
    }

    /// Parse primary expressions (literals, identifiers, parenthesized)
    fn parse_primary(&mut self) -> ParseResult<Expr> {
        let token = self.current_token();
        let span = token.span;

        match &token.token_type {
            TokenType::IntegerLiteral(value) => {
                let value = *value;
                self.advance();
                Ok(Expr::Literal {
                    value: LiteralValue::Integer(value),
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::FloatLiteral(value) => {
                let value = *value;
                self.advance();
                Ok(Expr::Literal {
                    value: LiteralValue::Float(value),
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::StringLiteral(value) => {
                let value = value.clone();
                self.advance();
                Ok(Expr::Literal {
                    value: LiteralValue::String(value),
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::BooleanLiteral(value) => {
                let value = *value;
                self.advance();
                Ok(Expr::Literal {
                    value: LiteralValue::Boolean(value),
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::Void => {
                // Treat 'void' as a special literal value
                self.advance();
                Ok(Expr::Literal {
                    value: LiteralValue::Void,
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Identifier {
                    name,
                    span,
                    id: next_node_id(),
                })
            }
            TokenType::LeftParen => {
                self.advance();
                let expr = self.parse_precedence(Precedence::Assignment)?;
                self.consume(&TokenType::RightParen, "Expected ')' after expression")?;

                let end_span = self.previous_token().span;
                let paren_span = Span::new(span.start, end_span.end);

                Ok(Expr::Parenthesized {
                    expr: Box::new(expr),
                    span: paren_span,
                    id: next_node_id(),
                })
            }
            TokenType::Minus | TokenType::Plus | TokenType::Not | TokenType::BitNot => {
                let operator = UnaryOp::from(token.token_type.clone());
                self.advance();
                let operand = self.parse_precedence(Precedence::Unary)?;

                let end_span = operand.span();
                let unary_span = Span::new(span.start, end_span.end);

                Ok(Expr::Unary {
                    operator,
                    operand: Box::new(operand),
                    span: unary_span,
                    id: next_node_id(),
                })
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            }),
        }
    }

    /// Parse infix expressions (binary operations, calls, etc.)
    fn parse_infix(&mut self, left: Expr) -> ParseResult<Expr> {
        let token = self.current_token();

        match &token.token_type {
            TokenType::Plus
            | TokenType::Minus
            | TokenType::Multiply
            | TokenType::Divide
            | TokenType::Modulo
            | TokenType::Power
            | TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual
            | TokenType::Is
            | TokenType::IsNot
            | TokenType::And
            | TokenType::Or
            | TokenType::Xor
            | TokenType::BitAnd
            | TokenType::BitOr
            | TokenType::BitXor
            | TokenType::BitShiftLeft
            | TokenType::BitShiftRight
            | TokenType::BitUnsignedShiftRight => {
                let operator = BinaryOp::from(token.token_type.clone());
                let precedence = Precedence::from_token(&token.token_type);
                self.advance();

                // For left-associative operators, use one level higher precedence for right side
                let right_precedence = if matches!(operator, BinaryOp::Power) {
                    // Right-associative: same precedence
                    precedence
                } else {
                    // Left-associative: one level higher precedence to ensure left-to-right grouping
                    match precedence {
                        Precedence::Assignment => Precedence::Or,
                        Precedence::Or => Precedence::Xor,
                        Precedence::Xor => Precedence::And,
                        Precedence::And => Precedence::BitOr,
                        Precedence::BitOr => Precedence::BitXor,
                        Precedence::BitXor => Precedence::BitAnd,
                        Precedence::BitAnd => Precedence::Equality,
                        Precedence::Equality => Precedence::Comparison,
                        Precedence::Comparison => Precedence::Shift,
                        Precedence::Shift => Precedence::Term,
                        Precedence::Term => Precedence::Factor,
                        Precedence::Factor => Precedence::Power,
                        Precedence::Power => Precedence::Unary,
                        _ => precedence,
                    }
                };

                let right = self.parse_precedence(right_precedence)?;

                let start_span = left.span();
                let end_span = right.span();
                let binary_span = Span::new(start_span.start, end_span.end);

                Ok(Expr::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                    span: binary_span,
                    id: next_node_id(),
                })
            }
            TokenType::LeftParen => {
                // Function call
                self.advance();
                let mut args = Vec::new();

                if !self.check(&TokenType::RightParen) {
                    loop {
                        // Parse arguments with Assignment precedence to avoid infinite recursion
                        args.push(self.parse_precedence(Precedence::Assignment)?);

                        if self.check(&TokenType::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                self.consume(&TokenType::RightParen, "Expected ')' after arguments")?;

                let start_span = left.span();
                let end_span = self.previous_token().span;
                let call_span = Span::new(start_span.start, end_span.end);

                Ok(Expr::Call {
                    callee: Box::new(left),
                    args,
                    span: call_span,
                    id: next_node_id(),
                })
            }
            _ => Ok(left),
        }
    }

    /// Utility methods for parser state management
    fn current_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous_token(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous_token()
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
            || matches!(self.current_token().token_type, TokenType::EndOfFile)
    }

    fn is_declaration_start(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        match &self.current_token().token_type {
            TokenType::Public
            | TokenType::Entry
            | TokenType::Function
            | TokenType::Type
            | TokenType::Import => true,
            TokenType::DocComment(_) => true,
            _ => false,
        }
    }

    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.current_token().token_type)
                == std::mem::discriminant(token_type)
        }
    }

    fn check_identifier(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.current_token().token_type, TokenType::Identifier(_))
        }
    }

    fn consume(&mut self, token_type: &TokenType, _message: &str) -> ParseResult<&Token> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(ParseError::MissingToken {
                expected: format!("{token_type}"),
                span: ParseError::span_from_token(self.current_token()),
            })
        }
    }

    fn skip_newlines_and_comments(&mut self) {
        while !self.is_at_end() {
            match &self.current_token().token_type {
                TokenType::Newline | TokenType::Comment(_) | TokenType::DocComment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn synchronize(&mut self) {
        self.advance();

        while !self.is_at_end() {
            if matches!(self.previous_token().token_type, TokenType::Newline) {
                return;
            }

            match &self.current_token().token_type {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse_expression_from_string(input: &str) -> ParseResult<Expr> {
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_expression()
    }

    fn parse_statement_from_string(input: &str) -> ParseResult<Stmt> {
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_statement()
    }

    #[test]
    fn test_literal_expressions() {
        let integer_expr = parse_expression_from_string("42").unwrap();
        assert!(matches!(
            integer_expr,
            Expr::Literal {
                value: LiteralValue::Integer(42),
                ..
            }
        ));

        let float_expr = parse_expression_from_string("3.14").unwrap();
        assert!(
            matches!(float_expr, Expr::Literal { value: LiteralValue::Float(f), .. } if (f - 3.14).abs() < f64::EPSILON)
        );

        let string_expr = parse_expression_from_string("'hello'").unwrap();
        assert!(
            matches!(string_expr, Expr::Literal { value: LiteralValue::String(s), .. } if s == "hello")
        );

        let bool_expr = parse_expression_from_string("true").unwrap();
        assert!(matches!(
            bool_expr,
            Expr::Literal {
                value: LiteralValue::Boolean(true),
                ..
            }
        ));
    }

    #[test]
    fn test_identifier_expressions() {
        let identifier_expr = parse_expression_from_string("hello_world").unwrap();
        assert!(matches!(identifier_expr, Expr::Identifier { name, .. } if name == "hello_world"));
    }

    #[test]
    fn test_binary_expressions() {
        let add_expr = parse_expression_from_string("1 + 2").unwrap();
        assert!(matches!(
            add_expr,
            Expr::Binary {
                operator: BinaryOp::Add,
                ..
            }
        ));

        let less_than_expr = parse_expression_from_string("x < y").unwrap();
        assert!(matches!(
            less_than_expr,
            Expr::Binary {
                operator: BinaryOp::Less,
                ..
            }
        ));

        let logical_and_expr = parse_expression_from_string("a and b").unwrap();
        assert!(matches!(
            logical_and_expr,
            Expr::Binary {
                operator: BinaryOp::And,
                ..
            }
        ));
    }

    #[test]
    fn test_unary_expressions() {
        let negate_expr = parse_expression_from_string("-42").unwrap();
        assert!(matches!(
            negate_expr,
            Expr::Unary {
                operator: UnaryOp::Negate,
                ..
            }
        ));

        let not_expr = parse_expression_from_string("not true").unwrap();
        assert!(matches!(
            not_expr,
            Expr::Unary {
                operator: UnaryOp::Not,
                ..
            }
        ));
    }

    #[test]
    fn test_parenthesized_expressions() {
        let paren_expr = parse_expression_from_string("(1 + 2)").unwrap();
        assert!(matches!(paren_expr, Expr::Parenthesized { .. }));
    }

    #[test]
    fn test_function_calls() {
        let call_expr = parse_expression_from_string("print('hello')").unwrap();
        assert!(matches!(call_expr, Expr::Call { .. }));
    }

    #[test]
    fn test_operator_precedence() {
        // Test that multiplication has higher precedence than addition
        let precedence_expr = parse_expression_from_string("1 + 2 * 3").unwrap();
        if let Expr::Binary {
            left,
            operator: BinaryOp::Add,
            right,
            ..
        } = precedence_expr
        {
            assert!(matches!(
                *left,
                Expr::Literal {
                    value: LiteralValue::Integer(1),
                    ..
                }
            ));
            assert!(matches!(
                *right,
                Expr::Binary {
                    operator: BinaryOp::Multiply,
                    ..
                }
            ));
        } else {
            unreachable!("Expected addition with multiplication on right side");
        }
    }

    #[test]
    fn test_break_continue_statements() {
        // Test break statement
        let break_stmt = parse_statement_from_string("break").unwrap();
        if let Stmt::Break { .. } = break_stmt {
            // Good, break statement
        } else {
            unreachable!("Expected break statement, got {break_stmt:?}");
        }

        // Test continue statement
        let continue_stmt = parse_statement_from_string("continue").unwrap();
        if let Stmt::Continue { .. } = continue_stmt {
            // Good, continue statement
        } else {
            unreachable!("Expected continue statement, got {continue_stmt:?}");
        }
    }

    #[test]
    fn test_for_statements() {
        // Test simple for loop
        let simple_for =
            parse_statement_from_string("for item in collection { print(item) }").unwrap();
        if let Stmt::For {
            variable,
            iterable,
            body,
            ..
        } = simple_for
        {
            // Check variable
            assert_eq!(variable, "item");

            // Check iterable
            if let Expr::Identifier { name, .. } = iterable {
                assert_eq!(name, "collection");
            } else {
                unreachable!("Expected identifier in for iterable");
            }

            // Check body
            if let Stmt::Block { .. } = *body {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in for body");
            }
        } else {
            unreachable!("Expected for statement, got {simple_for:?}");
        }

        // TODO: Add test for array literal when array expressions are implemented
        /*
        // Test for loop with array literal
        let array_for = parse_statement_from_string("for i in [1, 2, 3] { sum = sum + i }").unwrap();
        if let Stmt::For { variable, iterable, body, .. } = array_for {
            assert_eq!(variable, "i");

            if let Expr::Array { .. } = iterable {
                // Good, array literal
            } else {
                unreachable!("Expected array in for iterable");
            }

            if let Stmt::Block { .. } = *body {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in for body");
            }
        } else {
            unreachable!("Expected for statement");
        }
        */
    }

    #[test]
    fn test_while_statements() {
        // Test simple while loop
        let simple_while = parse_statement_from_string("while x < 10 { x = x + 1 }").unwrap();
        if let Stmt::While {
            condition, body, ..
        } = simple_while
        {
            // Check condition
            if let Expr::Binary { .. } = condition {
                // Good, binary comparison
            } else {
                unreachable!("Expected binary expression in while condition");
            }

            // Check body
            if let Stmt::Block { .. } = *body {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in while body");
            }
        } else {
            unreachable!("Expected while statement, got {simple_while:?}");
        }

        // Test while with boolean variable
        let bool_while = parse_statement_from_string("while running { update() }").unwrap();
        if let Stmt::While {
            condition, body, ..
        } = bool_while
        {
            if let Expr::Identifier { name, .. } = condition {
                assert_eq!(name, "running");
            } else {
                unreachable!("Expected identifier in while condition");
            }

            if let Stmt::Block { .. } = *body {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in while body");
            }
        } else {
            unreachable!("Expected while statement");
        }
    }

    #[test]
    fn test_if_statements() {
        // Test simple if statement
        let simple_if = parse_statement_from_string("if x < 5 { return true }").unwrap();
        if let Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } = simple_if
        {
            // Check condition
            if let Expr::Binary { .. } = condition {
                // Good, binary comparison
            } else {
                unreachable!("Expected binary expression in if condition");
            }

            // Check then branch
            if let Stmt::Block { .. } = *then_branch {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in then branch");
            }

            // Check no else branch
            assert!(else_branch.is_none());
        } else {
            unreachable!("Expected if statement, got {simple_if:?}");
        }

        // Test if-else statement
        let if_else = parse_statement_from_string("if x { y = 1 } else { y = 2 }").unwrap();
        if let Stmt::If {
            condition,
            then_branch,
            else_branch,
            ..
        } = if_else
        {
            // Check condition
            if let Expr::Identifier { name, .. } = condition {
                assert_eq!(name, "x");
            } else {
                unreachable!("Expected identifier in if condition");
            }

            // Check then branch
            if let Stmt::Block { .. } = *then_branch {
                // Good, block statement
            } else {
                unreachable!("Expected block statement in then branch");
            }

            // Check else branch exists
            assert!(else_branch.is_some());
            if let Some(else_stmt) = else_branch {
                if let Stmt::Block { .. } = *else_stmt {
                    // Good, block statement
                } else {
                    unreachable!("Expected block statement in else branch");
                }
            }
        } else {
            unreachable!("Expected if statement");
        }
    }

    #[test]
    fn test_assignment_statements() {
        // Test simple assignment
        let input = "x = 5";
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let simple_expr = parser.parse_statement().unwrap();

        if let Stmt::Assignment { target, value, .. } = simple_expr {
            if let Expr::Identifier { name, .. } = target {
                assert_eq!(name, "x");
            } else {
                unreachable!("Expected identifier in assignment target, got {target:?}");
            }
            if let Expr::Literal {
                value: LiteralValue::Integer(n),
                ..
            } = value
            {
                assert_eq!(n, 5);
            } else {
                unreachable!("Expected integer literal in assignment value, got {value:?}");
            }
        } else {
            unreachable!("Expected assignment statement, got {simple_expr:?}");
        }

        // TODO: Add tests for array index and member access assignments
        // when those expression types are fully implemented
        /*
        // Test assignment to array index
        let array_assignment = parse_statement_from_string("arr[0] = 10").unwrap();
        if let Stmt::Assignment { target, value, .. } = array_assignment {
            if let Expr::Index { .. } = target {
                // Correct target type
            } else {
                unreachable!("Expected index expression in assignment target");
            }
            if let Expr::Literal {
                value: LiteralValue::Integer(n),
                ..
            } = value
            {
                assert_eq!(n, 10);
            } else {
                unreachable!("Expected integer literal in assignment value");
            }
        } else {
            unreachable!("Expected assignment statement");
        }

        // Test assignment to member access
        let member_assignment = parse_statement_from_string("obj.field = 'value'").unwrap();
        if let Stmt::Assignment { target, value, .. } = member_assignment {
            if let Expr::Member { .. } = target {
                // Correct target type
            } else {
                unreachable!("Expected member expression in assignment target");
            }
            if let Expr::Literal {
                value: LiteralValue::String(s),
                ..
            } = value
            {
                assert_eq!(s, "value");
            } else {
                unreachable!("Expected string literal in assignment value");
            }
        } else {
            unreachable!("Expected assignment statement");
        }
        */
    }

    #[test]
    fn test_simple_function_parsing() {
        let input = "entry main = f(args: string[]): void => return void";
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program, errors) = parser.parse();

        assert!(errors.is_empty(), "Parse errors: {errors:?}");
        assert!(program.is_some());

        let program = program.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Function {
            name,
            parameters,
            return_type,
            is_entry,
            ..
        } = program.declarations[0].clone()
        {
            assert_eq!(name, "main");
            assert_eq!(parameters.len(), 1);
            assert_eq!(parameters[0].name, "args");
            assert!(return_type.is_some());
            assert!(is_entry);
        } else {
            unreachable!("Expected function declaration");
        }
    }
}
