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

use crate::ast::{
    AstNode, BinaryOp, Decl, Expr, Field, HotReloadMetadata, ImportItem, LambdaBody, LetBinding,
    LiteralValue, NodeId, Parameter, Program, Stmt, StringPart, Type, TypeDef, UnaryOp, Variant,
    Visibility,
};
use crate::error::LexError;
use crate::token::{Span, Token, TokenType};
use core::{
    mem,
    sync::atomic::{AtomicUsize, Ordering},
};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Errors that can occur during parsing
#[derive(Error, Debug, Diagnostic)]
pub enum ParseError {
    /// Found a token that doesn't match what was expected at this position
    #[error("Unexpected token: expected {expected}, found {found}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_token),
        help("Check the syntax around this location")
    )]
    UnexpectedToken {
        /// The token type that was expected at this position
        expected: String,
        /// The actual token that was found instead
        found: String,
        #[label("unexpected token")]
        /// Source span highlighting the unexpected token location
        span: SourceSpan,
    },

    /// Expected a specific token but it was not found
    #[error("Missing token: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::missing_token),
        help("Add the missing {expected}")
    )]
    MissingToken {
        /// The token type that was expected but not found
        expected: String,
        #[label("expected {expected} here")]
        /// Source span indicating where the missing token should be
        span: SourceSpan,
    },

    /// The syntax is invalid according to the language grammar
    #[error("Invalid syntax: {message}")]
    #[diagnostic(
        code(opalescent::parser::invalid_syntax),
        help("Check the language specification for correct syntax")
    )]
    InvalidSyntax {
        /// Description of what makes the syntax invalid
        message: String,
        #[label("invalid syntax")]
        /// Source span highlighting the location of invalid syntax
        span: SourceSpan,
    },

    /// Reached end of file while expecting more tokens
    #[error("Unexpected end of file: expected {expected}")]
    #[diagnostic(
        code(opalescent::parser::unexpected_eof),
        help("Complete the {expected}")
    )]
    UnexpectedEof {
        /// The token or construct that was expected before EOF
        expected: String,
        #[label("file ends here")]
        /// Source span indicating the end of file location
        span: SourceSpan,
    },
}

impl ParseError {
    /// Creates a source span from a token's span information
    /// Used for error reporting to highlight the token location in source code
    pub fn span_from_token(token: &Token) -> SourceSpan {
        LexError::span_from_span(token.span)
    }
}

/// Result type for parser operations
pub type ParseResult<T> = Result<T, ParseError>;

/// Collection of parse errors for multiple error reporting
#[derive(Debug)]
pub struct ParseErrors {
    /// Vector containing all parse errors encountered during parsing
    pub errors: Vec<ParseError>,
}

impl ParseErrors {
    /// Creates a new empty collection of parse errors
    pub const fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Add a parse error to the collection
    pub fn push(&mut self, error: ParseError) {
        self.errors.push(error);
    }

    /// Check if there are no errors in the collection
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Get the number of errors in the collection
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

/// Generates a unique node ID for AST nodes
/// Each call returns a monotonically increasing ID
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
    /// Determines the precedence level for a given token type
    /// Returns the appropriate precedence for binary operators
    pub const fn from_token(token_type: &TokenType) -> Self {
        match *token_type {
            // Remove assignment from expression precedence since it's a statement
            TokenType::Or => Self::Or,
            TokenType::Xor => Self::Xor,
            TokenType::And => Self::And,
            TokenType::BitOr => Self::BitOr,
            TokenType::BitXor => Self::BitXor,
            TokenType::BitAnd => Self::BitAnd,
            TokenType::Is | TokenType::IsNot => Self::Equality,
            TokenType::Less
            | TokenType::LessEqual
            | TokenType::Greater
            | TokenType::GreaterEqual => Self::Comparison,
            TokenType::BitShiftLeft
            | TokenType::BitShiftRight
            | TokenType::BitUnsignedShiftRight => Self::Shift,
            TokenType::Plus | TokenType::Minus => Self::Term,
            TokenType::Multiply | TokenType::Divide | TokenType::Modulo => Self::Factor,
            TokenType::Power => Self::Power,
            TokenType::LeftParen | TokenType::LeftBracket | TokenType::Dot => Self::Call,
            _ => Self::None,
        }
    }

    /// Get the next higher precedence level for left-associative operators
    /// Used in precedence climbing to determine when to stop parsing at current level
    pub const fn next(self) -> Self {
        match self {
            Self::Assignment => Self::Or,
            Self::Or => Self::Xor,
            Self::Xor => Self::And,
            Self::And => Self::BitOr,
            Self::BitOr => Self::BitXor,
            Self::BitXor => Self::BitAnd,
            Self::BitAnd => Self::Equality,
            Self::Equality => Self::Comparison,
            Self::Comparison => Self::Shift,
            Self::Shift => Self::Term,
            Self::Term => Self::Factor,
            Self::Factor => Self::Power,
            Self::Power => Self::Unary,
            _ => self,
        }
    }
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

    /// Parse a top-level declaration
    fn parse_declaration(&mut self) -> ParseResult<Decl> {
        // Check for documentation comment
        let doc_comment = if self.check(&TokenType::DocComment(String::new())) {
            if let &TokenType::DocComment(ref content) = &self.current_token().token_type {
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
        match self.current_token().token_type {
            TokenType::Function => {
                self.parse_function_declaration(visibility, is_entry, doc_comment)
            }
            TokenType::Type => self.parse_type_declaration(visibility, doc_comment),
            TokenType::Import => self.parse_import_declaration(),
            TokenType::Let => self.parse_let_declaration(visibility, doc_comment),
            TokenType::Identifier(_) if is_entry || visibility == Visibility::Public => {
                // This is a function declaration starting with identifier (entry main = f() or public foo = f())
                self.parse_function_declaration(visibility, is_entry, doc_comment)
            }
            _ => {
                let token = self.current_token();
                Err(ParseError::UnexpectedToken {
                    expected: "declaration (function, type, import, or let)".to_owned(),
                    found: format!("{}", token.token_type),
                    span: ParseError::span_from_token(token),
                })
            }
        }
    }

    /// Construct a `LetBinding` with consistent span calculation and node id assignment
    fn create_let_binding(
        name: String,
        name_span: Span,
        type_annotation: Option<Type>,
        is_mutable: bool,
    ) -> LetBinding {
        let binding_end = type_annotation
            .as_ref()
            .map_or(name_span.end, |ty| ty.span().end);

        LetBinding {
            name,
            type_annotation,
            is_mutable,
            span: Span::new(name_span.start, binding_end),
            id: next_node_id(),
        }
    }

    /// Parse a function declaration, supporting entry/public/visibility, parameter/return type parsing, and block/single-statement bodies.
    /// Integrates hot-reload metadata and ABI symbol info into the AST node.
    ///
    /// # Supported Syntaxes
    /// - `entry main = f(args: string[]): void => ...`
    /// - `public foo = f(x: int32, y: int32): int32 => ...`
    /// - `main = f(): void => ...`
    /// - `foo = f(x: int32): int32 => ...`
    ///
    /// # Errors
    /// Returns detailed parse errors for missing tokens, invalid syntax, and unsupported patterns.
    fn parse_function_declaration(
        &mut self,
        visibility: Visibility,
        is_entry: bool,
        doc_comment: Option<String>,
    ) -> ParseResult<Decl> {
        let start_span = self.current_token().span;

        // Parse function name (identifier)
        let name = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for function name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "function name (identifier)".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Expect '='
        self.consume(&TokenType::Assign, "Expected '=' after function name")?;

        // Expect 'f' keyword
        self.consume(&TokenType::Function, "Expected 'f' after '='")?;

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'f'")?;

        // Parse parameters (support zero or more)
        let parameters = self.parse_parameter_list()?;

        // Expect ')'
        self.consume(&TokenType::RightParen, "Expected ')' after parameters")?;

        // Parse optional return type
        let return_type = self
            .check(&TokenType::Colon)
            .then(|| {
                self.advance();
                self.parse_type()
            })
            .transpose()?;

        // Expect '=>'
        self.consume(&TokenType::Arrow, "Expected '=>' after function signature")?;

        // Parse function body (block or single statement)
        let body = if self.check(&TokenType::LeftBrace) {
            self.parse_block_statement()?
        } else {
            // Parse statements until end or new declaration
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
            let body_start = statements.first().map_or_else(
                || self.previous_token().span.start,
                |first_stmt| first_stmt.span().start,
            );
            let body_end = statements.last().map_or_else(
                || self.previous_token().span.end,
                |last_stmt| last_stmt.span().end,
            );
            let body_span = Span::new(body_start, body_end);
            Stmt::Block {
                statements,
                span: body_span,
                id: next_node_id(),
            }
        };

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        let mut metadata = HotReloadMetadata::for_function();
        if is_entry {
            metadata.is_hot_reloadable = false;
        }

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
            metadata,
        })
    }

    /// Parse a function parameter
    fn parse_parameter(&mut self) -> ParseResult<Parameter> {
        let start_span = self.current_token().span;

        let name = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for parameter name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "parameter name (identifier)".to_owned(),
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

    /// Parse a comma-separated list of parameters within parentheses
    /// Used by both function declarations and lambda expressions for consistency
    fn parse_parameter_list(&mut self) -> ParseResult<Vec<Parameter>> {
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

        Ok(parameters)
    }

    /// Parse a type declaration (placeholder)
    fn parse_type_declaration(
        &mut self,
        visibility: Visibility,
        doc_comment: Option<String>,
    ) -> ParseResult<Decl> {
        let start_span = self.current_token().span;

        // Consume 'type' keyword
        self.consume(&TokenType::Type, "Expected 'type' keyword")?;

        // Parse type name (must be PascalCase identifier)
        let name = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for type name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "type name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // TODO: Parse generic parameters (<T, E>) when implemented

        // Consume colon
        self.consume(&TokenType::Colon, "Expected ':' after type name")?;

        // Skip newlines before type body
        self.skip_newlines_and_comments();

        // Parse type definition body
        let type_def = self.parse_type_definition_body(start_span)?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        Ok(Decl::Type {
            name,
            type_def,
            visibility,
            doc_comment,
            span,
            id: next_node_id(),
            metadata: HotReloadMetadata::for_type_declaration(),
        })
    }

    /// Parse the body of a type definition (variants for sum types, fields for product types)
    #[expect(
        clippy::too_many_lines,
        reason = "Complex parsing logic requires detailed handling of different type definition patterns"
    )]
    fn parse_type_definition_body(&mut self, start_span: Span) -> ParseResult<TypeDef> {
        if self.is_at_end() {
            return Err(ParseError::UnexpectedEof {
                expected: "type definition body".to_owned(),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        let mut variants_or_fields = Vec::new();
        let mut is_product_type = None; // None = unknown, Some(true) = product, Some(false) = sum

        while !self.is_at_end()
            && !self.check(&TokenType::Type)
            && !self.check(&TokenType::Function)
            && !self.check(&TokenType::Import)
            && !self.check(&TokenType::Public)
            && !self.check(&TokenType::Entry)
        {
            // Skip newlines
            self.skip_newlines_and_comments();

            if self.is_at_end()
                || self.check(&TokenType::Type)
                || self.check(&TokenType::Function)
                || self.check(&TokenType::Import)
                || self.check(&TokenType::Public)
                || self.check(&TokenType::Entry)
            {
                break;
            }

            // Parse a variant or field
            let field_or_variant_start = self.current_token().span;

            // Parse identifier name (variant name or field name)
            let name = if self.check_identifier() {
                let token = self.advance();
                if let &TokenType::Identifier(ref name) = &token.token_type {
                    name.clone()
                } else {
                    return Err(ParseError::InvalidSyntax {
                        message: "Expected identifier for variant or field name".to_owned(),
                        span: ParseError::span_from_token(token),
                    });
                }
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "variant or field name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            };

            if self.check(&TokenType::Colon) {
                // This is a field (product type) or variant with fields
                self.advance(); // consume ':'

                if is_product_type.is_none() {
                    // First field-like item - need to determine if this is a top-level field or variant with fields
                    if variants_or_fields.is_empty() {
                        // Could be either - need to look ahead
                        self.skip_newlines_and_comments();

                        // If next item after colon is a type keyword, this is a product type field
                        // If next item is an identifier (field name), this is likely a sum type variant
                        if self.is_type_keyword() {
                            // This is definitely a type annotation - product type
                            is_product_type = Some(true);
                        } else if self.check_identifier() {
                            // This looks like a field name in a variant - sum type
                            is_product_type = Some(false);
                        } else if self.check(&TokenType::Function) {
                            // Function type - product type
                            is_product_type = Some(true);
                        } else {
                            // Assume sum type variant with fields for now
                            is_product_type = Some(false);
                        }
                    }
                }

                if is_product_type == Some(true) {
                    // Parse as product type field
                    let field_type = self.parse_type()?;
                    let field_end_span = self.previous_token().span;
                    let field_span = Span::new(field_or_variant_start.start, field_end_span.end);

                    variants_or_fields.push((name, Some(field_type), field_span));
                } else {
                    // Parse as sum type variant with fields
                    self.skip_newlines_and_comments();

                    // Parse indented field list - keep parsing while we see identifiers
                    while !self.is_at_end() && self.check_identifier() {
                        // Parse field: name: type
                        let _field_start = self.current_token().span;

                        let _field_name = if self.check_identifier() {
                            let token = self.advance();
                            if let &TokenType::Identifier(ref field_name) = &token.token_type {
                                field_name.clone()
                            } else {
                                return Err(ParseError::InvalidSyntax {
                                    message: "Expected field name".to_owned(),
                                    span: ParseError::span_from_token(token),
                                });
                            }
                        } else {
                            break; // No more fields
                        };

                        self.consume(&TokenType::Colon, "Expected ':' after field name")?;
                        let _field_type = self.parse_type()?;

                        // TODO: Store variant fields properly when we implement them
                        // For now, we just parse and discard them to satisfy the syntax

                        // Skip newlines between fields
                        self.skip_newlines_and_comments();
                    }

                    let variant_end_span = self.previous_token().span;
                    let variant_span =
                        Span::new(field_or_variant_start.start, variant_end_span.end);

                    variants_or_fields.push((name, None, variant_span));
                }
            } else {
                // This is a simple variant without fields (enum-like)
                if is_product_type.is_none() {
                    is_product_type = Some(false); // Sum type
                } else if is_product_type == Some(true) {
                    return Err(ParseError::InvalidSyntax {
                        message: "Cannot mix fields and variants in type definition".to_owned(),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }

                let variant_end_span = self.previous_token().span;
                let variant_span = Span::new(field_or_variant_start.start, variant_end_span.end);

                variants_or_fields.push((name, None, variant_span));
            }
        }

        if variants_or_fields.is_empty() {
            return Err(ParseError::InvalidSyntax {
                message: "Type definition cannot be empty".to_owned(),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        let end_span = self.previous_token().span;
        let def_span = Span::new(start_span.start, end_span.end);

        // Build the appropriate TypeDef based on what we parsed
        if is_product_type == Some(true) {
            // Product type - convert to fields
            let mut fields = Vec::new();
            for (name, type_annotation, span) in variants_or_fields {
                let field_type = type_annotation.ok_or_else(|| ParseError::InvalidSyntax {
                    message: "Product type field missing type annotation".to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                })?;
                fields.push(Field {
                    name,
                    type_annotation: field_type,
                    span,
                });
            }

            Ok(TypeDef::Product {
                fields,
                span: def_span,
            })
        } else {
            // Sum type - convert to variants
            let variants = variants_or_fields
                .into_iter()
                .map(|(name, _type_annotation, span)| {
                    Variant {
                        name,
                        fields: Vec::new(), // TODO: Handle variant fields properly
                        span,
                    }
                })
                .collect();

            Ok(TypeDef::Sum {
                variants,
                span: def_span,
            })
        }
    }

    /// Check if current token is an identifier at the start of a line (for parsing type body structure)
    fn check_identifier_at_start_of_line(&self) -> bool {
        // This is a simplified version - in a real implementation, you'd track indentation
        self.check_identifier()
    }

    /// Check if current token is a type keyword
    fn is_type_keyword(&self) -> bool {
        matches!(
            self.current_token().token_type,
            TokenType::Int8
                | TokenType::Int16
                | TokenType::Int32
                | TokenType::Int64
                | TokenType::UInt8
                | TokenType::UInt16
                | TokenType::UInt32
                | TokenType::UInt64
                | TokenType::Float32
                | TokenType::Float64
                | TokenType::String
                | TokenType::Boolean
                | TokenType::Void
        )
    }

    /// Parse an import declaration
    /// Supports multiple syntax forms:
    /// - `import item from source`
    /// - `import item as alias from source`
    /// - `import item1, item2 from source`
    /// - `import type Item from source`
    /// - `import type Item1, Item2 from source`
    fn parse_import_declaration(&mut self) -> ParseResult<Decl> {
        let start_span = self.current_token().span;

        // Consume 'import' keyword
        self.advance();

        let mut items = Vec::new();
        let mut is_type_import = false;

        // Check for 'type' keyword
        if self.check(&TokenType::Type) {
            is_type_import = true;
            self.advance();
        }

        // Parse first import item
        if self.check_identifier() {
            let item = self.parse_import_item(is_type_import)?;
            items.push(item);

            // Parse additional items if there's a comma
            while self.check(&TokenType::Comma) {
                self.advance(); // consume ','

                // Check for trailing comma (not allowed)
                if self.check(&TokenType::From) {
                    return Err(ParseError::InvalidSyntax {
                        message: "Trailing comma in import list not allowed".to_owned(),
                        span: ParseError::span_from_token(self.previous_token()),
                    });
                }

                let additional_item = self.parse_import_item(is_type_import)?;
                items.push(additional_item);
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "identifier".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        // Expect 'from' keyword
        if !self.check(&TokenType::From) {
            return Err(ParseError::UnexpectedToken {
                expected: "'from'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }
        self.advance(); // consume 'from'

        // Parse source path (handles various import path formats)
        let source = self.parse_import_path()?;

        let end_span = self.previous_token().span;
        let import_span = Span::new(start_span.start, end_span.end);

        Ok(Decl::Import {
            items,
            source,
            span: import_span,
            id: next_node_id(),
            metadata: HotReloadMetadata::for_import(),
        })
    }

    /// Parse a single import item (either Named or Type)
    fn parse_import_item(&mut self, is_type: bool) -> ParseResult<ImportItem> {
        let start_span = self.current_token().span;

        // Parse item name
        let name = match self.current_token().token_type {
            TokenType::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "identifier".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        };

        // Check for 'as' alias
        let alias = if self.check(&TokenType::As) {
            self.advance(); // consume 'as'

            match self.current_token().token_type {
                TokenType::Identifier(ref alias_name) => {
                    let alias_name = alias_name.clone();
                    self.advance();
                    Some(alias_name)
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "identifier".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }
        } else {
            None
        };

        let end_span = self.previous_token().span;
        let item_span = Span::new(start_span.start, end_span.end);

        if is_type {
            Ok(ImportItem::Type {
                name,
                alias,
                span: item_span,
            })
        } else {
            Ok(ImportItem::Named {
                name,
                alias,
                span: item_span,
            })
        }
    }

    /// Parse import path supporting different formats:
    /// - String literals: "./path/to/module"
    /// - Relative paths: ./path/to/module  
    /// - Bare specifiers: math (stdlib only)
    fn parse_import_path(&mut self) -> ParseResult<String> {
        match self.current_token().token_type {
            // String literals are the simplest case
            TokenType::StringLiteral(ref path) => {
                let path = path.clone();
                self.advance();
                Ok(path)
            }

            // Bare identifiers for stdlib (math, etc.)
            TokenType::Identifier(ref name) => {
                let path = name.clone();
                self.advance();
                Ok(path)
            }

            // Relative paths starting with ./
            TokenType::Dot => {
                let mut path = String::from(".");
                self.advance(); // consume '.'

                if self.check(&TokenType::Divide) {
                    path.push('/');
                    self.advance(); // consume '/'

                    // Parse path components
                    path.push_str(&self.parse_path_components()?);
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "'/' after '.'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }

                Ok(path)
            }

            _ => Err(ParseError::UnexpectedToken {
                expected: "import path (string, identifier, or '.')".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            }),
        }
    }

    /// Parse path components separated by '/' (foo/bar/baz)
    /// Also handles file extensions like .types, .op
    fn parse_path_components(&mut self) -> ParseResult<String> {
        let mut components = Vec::new();

        // Parse first component
        match self.current_token().token_type {
            TokenType::Identifier(ref component) => {
                components.push(component.clone());
                self.advance();
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "path component".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        }

        // Parse additional components
        while self.check(&TokenType::Divide) {
            self.advance(); // consume '/'

            match self.current_token().token_type {
                TokenType::Identifier(ref component) => {
                    components.push(component.clone());
                    self.advance();
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "path component after '/'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }
        }

        // Handle file extensions (e.g., .types, .op)
        if self.check(&TokenType::Dot) {
            self.advance(); // consume '.'

            match self.current_token().token_type {
                TokenType::Identifier(ref extension) => {
                    let last_component = components.pop().unwrap_or_default();
                    components.push(format!("{last_component}.{extension}"));
                    self.advance();
                }
                _ => {
                    return Err(ParseError::UnexpectedToken {
                        expected: "file extension after '.'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }
        }

        Ok(components.join("/"))
    }

    /// Parse a type annotation
    fn parse_type(&mut self) -> ParseResult<Type> {
        let start_span = self.current_token().span;

        // Check for function type syntax: f(param1, param2): return_type
        if self.check(&TokenType::Function) {
            return self.parse_function_type(start_span);
        }

        // Parse the base type name
        let name = match &self.current_token().token_type {
            &TokenType::Identifier(ref name) => {
                let name = name.clone();
                self.advance();
                name
            }
            token if token.type_name().is_some() => {
                let name = token.type_name().unwrap_or("unknown").to_owned();
                self.advance();
                name
            }
            _ => {
                return Err(ParseError::UnexpectedToken {
                    expected: "type name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        };

        // Check for generic arguments and create appropriate type
        let current_type = if self.check(&TokenType::Less) {
            self.advance(); // consume '<'

            let mut type_args = Vec::new();

            // Handle empty generic arguments (error case)
            if self.check(&TokenType::Greater) {
                return Err(ParseError::InvalidSyntax {
                    message: "Empty generic argument list".to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

            // Parse comma-separated type arguments
            loop {
                type_args.push(self.parse_type()?);

                if self.check(&TokenType::Comma) {
                    self.advance(); // consume ','
                } else if self.check(&TokenType::Greater) {
                    break;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or '>'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }

            // Consume closing '>'
            self.consume(&TokenType::Greater, "Expected '>' after generic arguments")?;

            let generic_end_span = self.previous_token().span;
            let generic_span = Span::new(start_span.start, generic_end_span.end);

            Type::Generic {
                name,
                type_args,
                span: generic_span,
            }
        } else {
            Type::Basic {
                name,
                span: Span::new(start_span.start, self.previous_token().span.end),
            }
        };

        // Check for array type syntax (type[] or Generic<T>[] or nested like type[][])
        let mut current_type = current_type;
        while self.check(&TokenType::LeftBracket) {
            self.advance(); // consume '['
            self.consume(&TokenType::RightBracket, "Expected ']' after '['")?;

            let array_end_span = self.previous_token().span;
            let array_span = Span::new(start_span.start, array_end_span.end);

            current_type = Type::Array {
                element_type: Box::new(current_type),
                span: array_span,
            };
        }

        Ok(current_type)
    }

    /// Parse a function type: `f(param1, param2): return_type`
    fn parse_function_type(&mut self, start_span: Span) -> ParseResult<Type> {
        // Consume the 'f' keyword
        self.advance();

        // Expect opening parenthesis
        self.consume(
            &TokenType::LeftParen,
            "Expected '(' after 'f' in function type",
        )?;

        let mut parameters = Vec::new();

        // Handle empty parameter list: f(): return_type
        if !self.check(&TokenType::RightParen) {
            loop {
                // Parse parameter type
                parameters.push(self.parse_type()?);

                if self.check(&TokenType::Comma) {
                    self.advance(); // consume ','
                } else if self.check(&TokenType::RightParen) {
                    break;
                } else {
                    return Err(ParseError::UnexpectedToken {
                        expected: "',' or ')'".to_owned(),
                        found: format!("{}", self.current_token().token_type),
                        span: ParseError::span_from_token(self.current_token()),
                    });
                }
            }
        }

        // Consume closing parenthesis
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after function parameters",
        )?;

        // Expect colon for return type
        self.consume(&TokenType::Colon, "Expected ':' after function parameters")?;

        // Parse return type
        let return_type = Box::new(self.parse_type()?);

        let end_span = self.previous_token().span;
        let function_span = Span::new(start_span.start, end_span.end);

        Ok(Type::Function {
            parameters,
            return_type,
            span: function_span,
        })
    }

    /// Parse a let declaration (variable declarations that can include lambda expressions)
    fn parse_let_declaration(
        &mut self,
        visibility: Visibility,
        doc_comment: Option<String>,
    ) -> ParseResult<Decl> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'let'

        // Check for 'mutable' keyword
        let is_mutable = if self.check(&TokenType::Mutable) {
            self.advance();
            true
        } else {
            false
        };

        // Parse variable name
        let (name, name_span) = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                (name.clone(), token.span)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for variable name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name (identifier)".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse optional type annotation
        #[expect(
            clippy::if_then_some_else_none,
            reason = "Result type makes bool::then inappropriate"
        )]
        let type_annotation = if self.check(&TokenType::Colon) {
            self.advance(); // consume ':'
            Some(self.parse_type()?)
        } else {
            None
        };

        // Expect '='
        self.consume(
            &TokenType::Assign,
            "Expected '=' after variable name or type",
        )?;

        // Parse initializer expression
        let initializer = self.parse_expression()?;

        let end_span = self.previous_token().span;
        let let_span = Span::new(start_span.start, end_span.end);

        let binding = Self::create_let_binding(name, name_span, type_annotation, is_mutable);

        let mut metadata = HotReloadMetadata::for_let_declaration();
        if binding.is_mutable {
            metadata.is_hot_reloadable = false;
        }

        Ok(Decl::Let {
            binding,
            initializer,
            visibility,
            doc_comment,
            span: let_span,
            id: next_node_id(),
            metadata,
        })
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> ParseResult<Stmt> {
        self.skip_newlines_and_comments();

        match self.current_token().token_type {
            TokenType::Let => self.parse_let_statement(),
            TokenType::Return => self.parse_return_statement(),
            TokenType::LeftBrace => self.parse_block_statement(),
            TokenType::If => self.parse_if_statement(),
            TokenType::For => self.parse_for_statement(),
            TokenType::While => self.parse_while_statement(),
            TokenType::Loop => self.parse_loop_statement(),
            TokenType::Break => Ok(self.parse_break_statement()),
            TokenType::Continue => Ok(self.parse_continue_statement()),
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
        let (name, name_span) = if self.check_identifier() {
            let token = self.advance();
            if let &TokenType::Identifier(ref name) = &token.token_type {
                (name.clone(), token.span)
            } else {
                return Err(ParseError::InvalidSyntax {
                    message: "Expected identifier for variable name".to_owned(),
                    span: ParseError::span_from_token(token),
                });
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        };

        // Parse optional type annotation
        let type_annotation = self
            .check(&TokenType::Colon)
            .then(|| {
                self.advance();
                self.parse_type()
            })
            .transpose()?;

        // Parse optional initializer
        let initializer = self
            .check(&TokenType::Assign)
            .then(|| {
                self.advance();
                self.parse_expression()
            })
            .transpose()?;

        let end_span = self.previous_token().span;
        let span = Span::new(start_span.start, end_span.end);

        let binding = Self::create_let_binding(name, name_span, type_annotation, is_mutable);

        Ok(Stmt::Let {
            binding,
            initializer,
            span,
            id: next_node_id(),
        })
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
        let else_branch = self
            .check(&TokenType::Else)
            .then(|| {
                self.advance(); // consume 'else'
                self.parse_statement().map(Box::new)
            })
            .transpose()?;

        let end_span = else_branch
            .as_ref()
            .map_or_else(|| then_branch.span(), |else_stmt| else_stmt.span());

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
            if let &TokenType::Identifier(ref name) = &token.token_type {
                name.clone()
            } else {
                // This should never happen since check_identifier validates the pattern
                debug_assert!(
                    matches!(self.current_token().token_type, TokenType::Identifier(_)),
                    "check_identifier should have validated this is an identifier token"
                );
                String::new() // fallback value
            }
        } else {
            return Err(ParseError::UnexpectedToken {
                expected: "variable name".to_owned(),
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

    /// Parse a loop statement
    fn parse_loop_statement(&mut self) -> ParseResult<Stmt> {
        let start_span = self.current_token().span;
        self.advance(); // consume 'loop'

        // Expect '=>'
        if !self.check(&TokenType::Arrow) {
            return Err(ParseError::UnexpectedToken {
                expected: "'=>'".to_owned(),
                found: format!("{}", self.current_token().token_type),
                span: ParseError::span_from_token(self.current_token()),
            });
        }
        self.advance(); // consume '=>'

        // Parse body (must be a block)
        let body = Box::new(self.parse_block_statement()?);

        let end_span = body.span();
        let span = Span::new(start_span.start, end_span.end);

        Ok(Stmt::Loop {
            body,
            span,
            id: next_node_id(),
        })
    }

    /// Parse a break statement
    fn parse_break_statement(&mut self) -> Stmt {
        let span = self.current_token().span;
        self.advance(); // consume 'break'

        Stmt::Break {
            span,
            id: next_node_id(),
        }
    }

    /// Parse a continue statement
    fn parse_continue_statement(&mut self) -> Stmt {
        let span = self.current_token().span;
        self.advance(); // consume 'continue'

        Stmt::Continue {
            span,
            id: next_node_id(),
        }
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
            match expr {
                Expr::Identifier { .. } | Expr::Index { .. } | Expr::Member { .. } => {
                    // Valid assignment targets
                }
                _ => {
                    return Err(ParseError::InvalidSyntax {
                        message: "Invalid assignment target".to_owned(),
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
            &TokenType::IntegerLiteral(value) => Ok(self.parse_integer_literal(value, span)),
            &TokenType::FloatLiteral(value) => Ok(self.parse_float_literal(value, span)),
            &TokenType::StringLiteral(ref value) => self.parse_string_literal(value.clone(), span),
            &TokenType::BooleanLiteral(value) => Ok(self.parse_boolean_literal(value, span)),
            &TokenType::Void => Ok(self.parse_void_literal(span)),
            &TokenType::Identifier(ref name) => Ok(self.parse_identifier(name.clone(), span)),
            &TokenType::LeftParen => self.parse_parenthesized_expression(span),
            &TokenType::Minus | &TokenType::Plus | &TokenType::Not | &TokenType::BitNot => {
                let token_type = token.token_type.clone();
                self.parse_unary_expression(&token_type, span)
            }
            &TokenType::TypeOf => self.parse_type_of_expression(span),
            &TokenType::Function => self.parse_lambda_expression(span),
            _ => Err(ParseError::UnexpectedToken {
                expected: "expression (literal, identifier, function call, lambda, or parenthesized expression)".to_owned(),
                found: format!("{}", token.token_type),
                span: ParseError::span_from_token(token),
            }),
        }
    }

    /// Parse integer literal expressions
    fn parse_integer_literal(&mut self, value: i64, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Integer(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse float literal expressions
    fn parse_float_literal(&mut self, value: f64, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Float(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse string literal expressions and string interpolation
    fn parse_string_literal(&mut self, value: String, span: Span) -> ParseResult<Expr> {
        self.advance();

        // Check if the string contains interpolation syntax
        if value.contains('{') {
            Self::parse_string_interpolation(&value, span)
        } else {
            Ok(Expr::Literal {
                value: LiteralValue::String(value),
                span,
                id: next_node_id(),
            })
        }
    }

    /// Parse string interpolation expressions ('Hello {world}')
    fn parse_string_interpolation(value: &str, span: Span) -> ParseResult<Expr> {
        let mut parts = Vec::new();
        let mut current_str = String::new();
        let mut chars = value.chars();

        while let Some(ch) = chars.next() {
            if ch == '{' {
                // Add the current string part if not empty
                if !current_str.is_empty() {
                    parts.push(StringPart::Literal(current_str.clone()));
                    current_str.clear();
                } else if parts.is_empty() {
                    // Add empty literal for strings starting with interpolation
                    parts.push(StringPart::Literal(String::new()));
                }

                // Parse the expression inside {}
                let mut expr_str = String::new();
                let mut brace_count = 1_i32;
                let mut found_closing_brace = false;

                for expr_ch in chars.by_ref() {
                    match expr_ch {
                        '{' => {
                            brace_count = brace_count.checked_add(1_i32).ok_or_else(|| {
                                ParseError::InvalidSyntax {
                                    message: "Too many nested braces in string interpolation"
                                        .to_owned(),
                                    span: LexError::span_from_span(span),
                                }
                            })?;
                        }
                        '}' => {
                            brace_count = brace_count.checked_sub(1_i32).ok_or_else(|| {
                                ParseError::InvalidSyntax {
                                    message: "Unmatched closing brace in string interpolation"
                                        .to_owned(),
                                    span: LexError::span_from_span(span),
                                }
                            })?;
                            if brace_count == 0_i32 {
                                found_closing_brace = true;
                                break;
                            }
                        }
                        _ => {}
                    }
                    expr_str.push(expr_ch);
                }

                // Check for unclosed braces
                if !found_closing_brace {
                    return Err(ParseError::InvalidSyntax {
                        message: "Unclosed interpolation brace in string".to_owned(),
                        span: LexError::span_from_span(span),
                    });
                }

                // Parse the expression string
                if expr_str.trim().is_empty() {
                    return Err(ParseError::InvalidSyntax {
                        message: "Empty interpolation expression in string".to_owned(),
                        span: LexError::span_from_span(span),
                    });
                }

                // Create a mini lexer/parser for the expression
                let expr_lexer = crate::lexer::Lexer::new(&expr_str);
                let (expr_tokens, _) = expr_lexer.tokenize();
                let mut expr_parser = Self::new(expr_tokens);

                match expr_parser.parse_expression() {
                    Ok(expr) => {
                        parts.push(StringPart::Expression(expr));
                    }
                    Err(_) => {
                        return Err(ParseError::InvalidSyntax {
                            message: format!(
                                "Invalid expression in string interpolation: {expr_str}"
                            ),
                            span: LexError::span_from_span(span),
                        });
                    }
                }
            } else {
                current_str.push(ch);
            }
        }

        // Add any remaining string content
        parts.push(StringPart::Literal(current_str));

        Ok(Expr::StringInterpolation {
            parts,
            span,
            id: next_node_id(),
        })
    }

    /// Parse boolean literal expressions
    fn parse_boolean_literal(&mut self, value: bool, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Boolean(value),
            span,
            id: next_node_id(),
        }
    }

    /// Parse void literal expressions
    fn parse_void_literal(&mut self, span: Span) -> Expr {
        self.advance();
        Expr::Literal {
            value: LiteralValue::Void,
            span,
            id: next_node_id(),
        }
    }

    /// Parse identifier expressions
    fn parse_identifier(&mut self, name: String, span: Span) -> Expr {
        self.advance();
        Expr::Identifier {
            name,
            span,
            id: next_node_id(),
        }
    }

    /// Parse parenthesized expressions
    fn parse_parenthesized_expression(&mut self, span: Span) -> ParseResult<Expr> {
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

    /// Parse unary expressions
    fn parse_unary_expression(&mut self, token_type: &TokenType, span: Span) -> ParseResult<Expr> {
        let operator = UnaryOp::try_from(token_type.clone()).map_err(|_original_error| {
            ParseError::InvalidSyntax {
                message: format!("Invalid unary operator: {token_type}"),
                span: LexError::span_from_span(span),
            }
        })?;
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

    /// Parse `type_of` expressions
    fn parse_type_of_expression(&mut self, span: Span) -> ParseResult<Expr> {
        self.advance(); // consume 'type_of'

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'type_of'")?;

        // Parse the expression inside
        let expr = self.parse_expression()?;

        // Expect ')'
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after type_of expression",
        )?;

        let end_span = self.previous_token().span;
        let type_of_span = Span::new(span.start, end_span.end);

        Ok(Expr::TypeOf {
            expr: Box::new(expr),
            span: type_of_span,
            id: next_node_id(),
        })
    }

    /// Parse lambda expressions (f(x: T): U => expr, f<T, U>(x: T): U => block)
    fn parse_lambda_expression(&mut self, span: Span) -> ParseResult<Expr> {
        self.advance(); // consume 'f'

        // Parse optional generic parameters (<T, U>)
        #[expect(
            clippy::if_then_some_else_none,
            reason = "Result type makes bool::then inappropriate"
        )]
        let generic_params = if self.check(&TokenType::Less) {
            Some(self.parse_lambda_generic_parameters()?)
        } else {
            None
        };

        // Expect '('
        self.consume(&TokenType::LeftParen, "Expected '(' after 'f'")?;

        // Parse parameters (support zero or more)
        let params = self.parse_parameter_list()?;

        // Expect ')'
        self.consume(
            &TokenType::RightParen,
            "Expected ')' after lambda parameters",
        )?;

        // Expect ':'
        self.consume(&TokenType::Colon, "Expected ':' after lambda parameters")?;

        // Parse return type
        let return_type = self.parse_type()?;

        // Expect '=>'
        self.consume(&TokenType::Arrow, "Expected '=>' after lambda return type")?;

        // Parse lambda body
        let body = self.parse_lambda_body()?;

        let end_span = self.previous_token().span;
        let lambda_span = Span::new(span.start, end_span.end);

        Ok(Expr::Lambda {
            generic_params,
            params,
            return_type,
            body,
            captured_variables: Vec::new(), // TODO: Implement closure capture analysis
            metadata: HotReloadMetadata::for_expression(),
            span: lambda_span,
            id: next_node_id(),
        })
    }

    /// Parse generic parameters for lambda expressions (<T, U>)
    fn parse_lambda_generic_parameters(&mut self) -> ParseResult<Vec<String>> {
        self.advance(); // consume '<'

        let mut generic_params = Vec::new();

        // Handle empty generic arguments (error case)
        if self.check(&TokenType::Greater) {
            return Err(ParseError::InvalidSyntax {
                message: "Empty generic parameter list".to_owned(),
                span: ParseError::span_from_token(self.current_token()),
            });
        }

        // Parse comma-separated generic parameter names
        loop {
            if self.check_identifier() {
                let token = self.advance();
                if let &TokenType::Identifier(ref name) = &token.token_type {
                    generic_params.push(name.clone());
                } else {
                    return Err(ParseError::InvalidSyntax {
                        message: "Expected identifier for generic parameter".to_owned(),
                        span: ParseError::span_from_token(token),
                    });
                }
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "generic parameter name".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }

            if self.check(&TokenType::Comma) {
                self.advance(); // consume ','
            } else if self.check(&TokenType::Greater) {
                break;
            } else {
                return Err(ParseError::UnexpectedToken {
                    expected: "',' or '>'".to_owned(),
                    found: format!("{}", self.current_token().token_type),
                    span: ParseError::span_from_token(self.current_token()),
                });
            }
        }

        // Expect '>'
        self.consume(&TokenType::Greater, "Expected '>' after generic parameters")?;

        Ok(generic_params)
    }

    /// Parse lambda body (expression or block)
    fn parse_lambda_body(&mut self) -> ParseResult<LambdaBody> {
        // Check if this is a block body (starts with '{') or a single expression
        if self.check(&TokenType::LeftBrace) {
            // Use existing block parsing for consistency with function bodies
            let block_stmt = self.parse_block_statement()?;
            if let Stmt::Block { statements, .. } = block_stmt {
                Ok(LambdaBody::Block(statements))
            } else {
                // This should never happen since parse_block_statement always returns Block
                Err(ParseError::InvalidSyntax {
                    message: "Expected block statement from parse_block_statement".to_owned(),
                    span: ParseError::span_from_token(self.current_token()),
                })
            }
        } else {
            // Parse as single expression
            let expr = self.parse_expression()?;
            Ok(LambdaBody::Expression(Box::new(expr)))
        }
    }

    /// Parse infix expressions (binary operations, calls, etc.)
    fn parse_infix(&mut self, left: Expr) -> ParseResult<Expr> {
        let token = self.current_token();

        match &token.token_type {
            &TokenType::Plus
            | &TokenType::Minus
            | &TokenType::Multiply
            | &TokenType::Divide
            | &TokenType::Modulo
            | &TokenType::Power
            | &TokenType::Less
            | &TokenType::LessEqual
            | &TokenType::Greater
            | &TokenType::GreaterEqual
            | &TokenType::Is
            | &TokenType::IsNot
            | &TokenType::And
            | &TokenType::Or
            | &TokenType::Xor
            | &TokenType::BitAnd
            | &TokenType::BitOr
            | &TokenType::BitXor
            | &TokenType::BitShiftLeft
            | &TokenType::BitShiftRight
            | &TokenType::BitUnsignedShiftRight => {
                let operator =
                    BinaryOp::try_from(token.token_type.clone()).map_err(|_original_error| {
                        ParseError::InvalidSyntax {
                            message: format!("Invalid binary operator: {}", token.token_type),
                            span: ParseError::span_from_token(token),
                        }
                    })?;
                let precedence = Precedence::from_token(&token.token_type);
                self.advance();

                // For left-associative operators, use one level higher precedence for right side
                let right_precedence = if matches!(operator, BinaryOp::Power) {
                    // Right-associative: same precedence
                    precedence
                } else {
                    // Left-associative: one level higher precedence to ensure left-to-right grouping
                    precedence.next()
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
            &TokenType::LeftParen => {
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
    /// Get the current token without advancing the parser position
    fn current_token(&self) -> &Token {
        &self.tokens[self.current]
    }

    /// Get the previous token (the one before current position)
    /// Uses saturating subtraction to avoid underflow
    fn previous_token(&self) -> &Token {
        &self.tokens[self.current.saturating_sub(1)]
    }

    /// Advance to the next token and return the previous token
    /// Uses saturating addition to avoid overflow
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current = self.current.saturating_add(1);
        }
        self.previous_token()
    }

    /// Check if the parser has reached the end of the token stream
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
            || matches!(self.current_token().token_type, TokenType::EndOfFile)
    }

    /// Check if the current token starts a declaration
    fn is_declaration_start(&self) -> bool {
        if self.is_at_end() {
            return false;
        }

        matches!(
            self.current_token().token_type,
            TokenType::Public
                | TokenType::Entry
                | TokenType::Function
                | TokenType::Type
                | TokenType::Import
                | TokenType::DocComment(_)
        )
    }

    /// Check if the current token matches the expected token type
    fn check(&self, token_type: &TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            mem::discriminant(&self.current_token().token_type) == mem::discriminant(token_type)
        }
    }

    /// Check if the current token is an identifier
    fn check_identifier(&self) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(self.current_token().token_type, TokenType::Identifier(_))
        }
    }

    /// Consume a token of the expected type or return an error
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

    /// Skip newlines and comments in the token stream
    fn skip_newlines_and_comments(&mut self) {
        while !self.is_at_end() {
            match self.current_token().token_type {
                TokenType::Newline | TokenType::Comment(_) | TokenType::DocComment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    /// Synchronize the parser after an error by advancing to the next statement
    fn synchronize(&mut self) {
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
}

#[cfg(test)]
#[expect(
    clippy::panic,
    clippy::shadow_unrelated,
    clippy::pattern_type_mismatch,
    clippy::uninlined_format_args,
    reason = "Test code is allowed to use panic and have some relaxed linting rules for this module only"
)]
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

    fn parse_type_from_string(input: &str) -> ParseResult<Type> {
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_type()
    }

    fn parse_program_from_string(input: &str) -> Result<Program, Vec<ParseError>> {
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program_opt, errors) = parser.parse();

        if errors.is_empty() {
            Ok(program_opt.unwrap())
        } else {
            Err(errors.errors)
        }
    }

    // Error handling tests
    #[test]
    fn test_unexpected_token_errors() {
        // Test invalid expression syntax
        let result1 = parse_expression_from_string("5 + +");
        assert!(result1.is_err());
        if let Err(ParseError::UnexpectedToken {
            expected, found, ..
        }) = result1
        {
            assert!(expected.contains("expression") || expected.contains("operand"));
            assert_eq!(found, "end of file");
        }

        // Test invalid binary operation
        let result2 = parse_expression_from_string("5 + * 3");
        assert!(result2.is_err());
        assert!(matches!(result2, Err(ParseError::UnexpectedToken { .. })));

        // Test invalid parenthesized expression
        let result3 = parse_expression_from_string("(5 +)");
        assert!(result3.is_err());
    }

    #[test]
    fn test_missing_token_errors() {
        // Test missing closing parenthesis
        let result1 = parse_expression_from_string("(5 + 3");
        assert!(result1.is_err());

        // Test missing function call parentheses end
        let result2 = parse_expression_from_string("foo(5, 3");
        assert!(result2.is_err());

        // Test missing assignment value
        let result3 = parse_statement_from_string("let x =");
        assert!(result3.is_err());

        // Test missing block closing brace
        let result4 = parse_statement_from_string("{ let x = 5");
        assert!(result4.is_err());
    }

    #[test]
    fn test_invalid_syntax_errors() {
        // Test invalid variable name (not an identifier)
        let result1 = parse_statement_from_string("let 123");
        assert!(result1.is_err());

        // Test invalid assignment target
        let result2 = parse_statement_from_string("5 = 10");
        assert!(result2.is_err());

        // Test invalid function parameter syntax
        let result3 = parse_statement_from_string("let f = f(x y): int32 => x + y");
        assert!(result3.is_err());
    }

    #[test]
    fn test_unexpected_eof_errors() {
        // Test EOF in middle of expression
        let result1 = parse_expression_from_string("5 +");
        assert!(result1.is_err());

        // Test EOF in function parameters
        let result2 = parse_statement_from_string("let f = f(");
        assert!(result2.is_err());

        // Test EOF in block
        let result3 = parse_statement_from_string("{");
        assert!(result3.is_err());
    }

    #[test]
    fn test_type_annotation_errors() {
        // Test invalid type syntax
        let result1 = parse_type_from_string("int32[");
        assert!(result1.is_err());

        // Test invalid generic type syntax
        let result2 = parse_type_from_string("Map<string");
        assert!(result2.is_err());

        // Test invalid function type syntax
        let result3 = parse_type_from_string("f(int32");
        assert!(result3.is_err());
    }

    #[test]
    fn test_visibility_modifier_errors() {
        // Test invalid visibility placement
        let result1 = parse_statement_from_string("let public x = 5");
        assert!(result1.is_err());

        // Test duplicate visibility modifiers would be caught by lexer, but test parser response
        let result2 = parse_program_from_string("public public let x = 5");
        assert!(result2.is_err());
    }

    #[test]
    fn test_multiple_error_collection() {
        // Test program with multiple syntax errors
        let result = parse_program_from_string(
            "
            let x = 5 +
            let y =
            { missing_brace
        ",
        );

        // Should collect multiple errors
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors.len() >= 2,
            "Should collect multiple errors, got {}",
            errors.len()
        );
    }

    #[test]
    fn test_literal_expressions() {
        // Test value for floating point comparison - define at top to avoid items after statements
        #[expect(
            clippy::approx_constant,
            reason = "Test value intentionally matches pi approximation"
        )]
        const TEST_VALUE: f64 = 3.14;

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
            matches!(float_expr, Expr::Literal { value: LiteralValue::Float(f), .. } if (f - TEST_VALUE).abs() < f64::EPSILON)
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
    #[expect(
        clippy::cognitive_complexity,
        reason = "Complex test covering multiple loop scenarios"
    )]
    fn test_loop_statements() {
        // Test simple loop statement
        let simple_loop = parse_statement_from_string("loop => { break }").unwrap();
        if let Stmt::Loop { body, .. } = simple_loop {
            // Check body
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 1);
                if let Stmt::Break { .. } = statements[0] {
                    // Good, break statement
                } else {
                    unreachable!("Expected break statement in loop body");
                }
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement, got {simple_loop:?}");
        }

        // Test loop with multiple statements
        let complex_loop = parse_statement_from_string(
            "loop => { let x = 1; if x > 10 { break } else { continue } }",
        )
        .unwrap();
        if let Stmt::Loop { body, .. } = complex_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 2);
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
        }

        // Test loop with nested loops
        let nested_loop =
            parse_statement_from_string("loop => { loop => { break }; continue }").unwrap();
        if let Stmt::Loop { body, .. } = nested_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 2);
                // First statement should be a nested loop
                if let Stmt::Loop { .. } = statements[0] {
                    // Good, nested loop
                } else {
                    unreachable!("Expected nested loop statement");
                }
                // Second statement should be continue
                if let Stmt::Continue { .. } = statements[1] {
                    // Good, continue statement
                } else {
                    unreachable!("Expected continue statement");
                }
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
        }

        // Test loop with variable assignments and conditions
        let assignment_loop =
            parse_statement_from_string("loop => { let i = 0; i = i + 1; if i > 5 { break } }")
                .unwrap();
        if let Stmt::Loop { body, .. } = assignment_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 3);
                // Check that we have let, assignment, and if statements
                assert!(matches!(statements[0], Stmt::Let { .. }));
                assert!(matches!(statements[1], Stmt::Assignment { .. }));
                assert!(matches!(statements[2], Stmt::If { .. }));
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
        }

        // Test loop with function calls
        let function_call_loop =
            parse_statement_from_string("loop => { process_item(); if should_exit() { break } }")
                .unwrap();
        if let Stmt::Loop { body, .. } = function_call_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 2);
                // First should be an expression statement with function call
                if let &Stmt::Expression { ref expr, .. } = &statements[0] {
                    assert!(matches!(*expr, Expr::Call { .. }));
                } else {
                    unreachable!("Expected expression statement with function call");
                }
                // Second should be an if statement
                assert!(matches!(statements[1], Stmt::If { .. }));
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
        }

        // Test empty loop (just for syntax, though not practical)
        let empty_loop = parse_statement_from_string("loop => { }").unwrap();
        if let Stmt::Loop { body, .. } = empty_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 0);
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
        }
    }

    #[test]
    fn test_loop_error_cases() {
        // Test loop without arrow - should fail
        let missing_arrow = parse_statement_from_string("loop { break }");
        assert!(missing_arrow.is_err());

        // Test loop without body - should fail
        let missing_body = parse_statement_from_string("loop =>");
        assert!(missing_body.is_err());

        // Test loop with malformed arrow - should fail
        let bad_arrow = parse_statement_from_string("loop = { break }");
        assert!(bad_arrow.is_err());

        // Test loop with unclosed body - should fail
        let unclosed_body = parse_statement_from_string("loop => { break");
        assert!(unclosed_body.is_err());
    }

    #[test]
    fn test_loop_with_various_statements() {
        // Test loop containing all types of statements
        let comprehensive_loop = parse_statement_from_string(
            "loop => { 
                let x = 0;
                let mutable counter = 1;
                counter = counter + 1;
                for i in items { process(i) };
                while running { update() };
                if done { break };
                return void
            }",
        )
        .unwrap();

        if let Stmt::Loop { body, .. } = comprehensive_loop {
            if let Stmt::Block { statements, .. } = *body {
                assert_eq!(statements.len(), 7);
                assert!(matches!(statements[0], Stmt::Let { .. }));
                assert!(matches!(statements[1], Stmt::Let { .. }));
                if let Stmt::Let { binding, .. } = &statements[1] {
                    assert!(binding.is_mutable);
                } else {
                    unreachable!("Expected let statement with mutable binding");
                }
                assert!(matches!(statements[2], Stmt::Assignment { .. }));
                assert!(matches!(statements[3], Stmt::For { .. }));
                assert!(matches!(statements[4], Stmt::While { .. }));
                assert!(matches!(statements[5], Stmt::If { .. }));
                assert!(matches!(statements[6], Stmt::Return { .. }));
            } else {
                unreachable!("Expected block statement in loop body");
            }
        } else {
            unreachable!("Expected loop statement");
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

    #[test]
    fn test_type_of_expressions() {
        // Test type_of with literal
        let type_of_literal = parse_expression_from_string("type_of(42)").unwrap();
        if let Expr::TypeOf { expr, .. } = type_of_literal {
            if let Expr::Literal {
                value: LiteralValue::Integer(42),
                ..
            } = *expr
            {
                // Good, correct structure
            } else {
                unreachable!("Expected integer literal inside type_of");
            }
        } else {
            unreachable!("Expected type_of expression, got {type_of_literal:?}");
        }

        // Test type_of with variable
        let type_of_var = parse_expression_from_string("type_of(my_variable)").unwrap();
        if let Expr::TypeOf { expr, .. } = type_of_var {
            if let Expr::Identifier { name, .. } = *expr {
                assert_eq!(name, "my_variable");
            } else {
                unreachable!("Expected identifier inside type_of");
            }
        } else {
            unreachable!("Expected type_of expression");
        }

        // Test type_of with expression
        let type_of_expr = parse_expression_from_string("type_of(x + y)").unwrap();
        if let Expr::TypeOf { expr, .. } = type_of_expr {
            if let Expr::Binary {
                operator: BinaryOp::Add,
                ..
            } = *expr
            {
                // Good, binary expression inside type_of
            } else {
                unreachable!("Expected binary expression inside type_of");
            }
        } else {
            unreachable!("Expected type_of expression");
        }

        // Test nested type_of (though semantically questionable)
        let nested_type_of = parse_expression_from_string("type_of(type_of(x))").unwrap();
        if let Expr::TypeOf { expr, .. } = nested_type_of {
            if let Expr::TypeOf { .. } = *expr {
                // Good, nested type_of
            } else {
                unreachable!("Expected nested type_of inside outer type_of");
            }
        } else {
            unreachable!("Expected type_of expression");
        }
    }

    #[test]
    fn test_type_of_error_cases() {
        // Test type_of without parentheses - should fail
        let missing_parens = parse_expression_from_string("type_of x");
        assert!(missing_parens.is_err());

        // Test type_of without expression - should fail
        let missing_expr = parse_expression_from_string("type_of()");
        assert!(missing_expr.is_err());

        // Test type_of with unclosed parentheses - should fail
        let unclosed_parens = parse_expression_from_string("type_of(x");
        assert!(unclosed_parens.is_err());

        // Test empty type_of call - should fail
        let empty_call = parse_expression_from_string("type_of( )");
        assert!(empty_call.is_err());
    }

    #[test]
    fn test_type_of_in_complex_expressions() {
        // Test type_of in binary expressions
        let binary_with_type_of = parse_expression_from_string("type_of(x) is type_of(y)").unwrap();
        if let Expr::Binary {
            left,
            operator: BinaryOp::Is,
            right,
            ..
        } = binary_with_type_of
        {
            assert!(matches!(*left, Expr::TypeOf { .. }));
            assert!(matches!(*right, Expr::TypeOf { .. }));
        } else {
            unreachable!("Expected binary expression with type_of operands");
        }

        // Test type_of as function argument
        let type_of_as_arg = parse_expression_from_string("print(type_of(value))").unwrap();
        if let Expr::Call { args, .. } = type_of_as_arg {
            assert_eq!(args.len(), 1);
            assert!(matches!(args[0], Expr::TypeOf { .. }));
        } else {
            unreachable!("Expected function call with type_of argument");
        }

        // Test type_of with parenthesized expression
        let type_of_paren = parse_expression_from_string("type_of((x + y))").unwrap();
        if let Expr::TypeOf { expr, .. } = type_of_paren {
            if let Expr::Parenthesized { expr: inner, .. } = *expr {
                assert!(matches!(*inner, Expr::Binary { .. }));
            } else {
                unreachable!("Expected parenthesized expression inside type_of");
            }
        } else {
            unreachable!("Expected type_of expression");
        }
    }

    #[test]
    fn test_string_interpolation_simple() {
        // Test simple variable interpolation: 'Hello {world}'
        let simple = parse_expression_from_string("'Hello {world}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = simple {
            assert_eq!(parts.len(), 3);

            // First part should be literal "Hello "
            if let StringPart::Literal(ref text) = parts[0] {
                assert_eq!(text, "Hello ");
            } else {
                unreachable!("Expected literal part, got {:?}", parts[0]);
            }

            // Second part should be identifier expression
            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
                assert_eq!(name, "world");
            } else {
                unreachable!("Expected identifier expression, got {:?}", parts[1]);
            }

            // Third part should be empty literal (trailing string after last interpolation)
            if let StringPart::Literal(ref text) = parts[2] {
                assert_eq!(text, "");
            } else {
                unreachable!("Expected literal part, got {:?}", parts[2]);
            }
        } else {
            unreachable!("Expected string interpolation, got {:?}", simple);
        }
    }

    #[test]
    fn test_string_interpolation_multiple() {
        // Test multiple interpolations: 'fib({n}) = {result}'
        let multiple = parse_expression_from_string("'fib({n}) = {result}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = multiple {
            assert_eq!(parts.len(), 5);

            // Should be: literal("fib("), expr(n), literal(") = "), expr(result), literal("")
            if let StringPart::Literal(ref text) = parts[0] {
                assert_eq!(text, "fib(");
            } else {
                unreachable!("Expected literal part");
            }

            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
                assert_eq!(name, "n");
            } else {
                unreachable!("Expected identifier expression");
            }

            if let StringPart::Literal(ref text) = parts[2] {
                assert_eq!(text, ") = ");
            } else {
                unreachable!("Expected literal part");
            }

            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[3] {
                assert_eq!(name, "result");
            } else {
                unreachable!("Expected identifier expression");
            }

            if let StringPart::Literal(ref text) = parts[4] {
                assert_eq!(text, "");
            } else {
                unreachable!("Expected literal part");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_complex_expressions() {
        // Test complex expressions in interpolation: 'Result: {a + b * c}'
        let complex = parse_expression_from_string("'Result: {a + b * c}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = complex {
            assert_eq!(parts.len(), 3);

            if let StringPart::Literal(ref text) = parts[0] {
                assert_eq!(text, "Result: ");
            } else {
                unreachable!("Expected literal part");
            }

            if let StringPart::Expression(Expr::Binary { .. }) = parts[1] {
                // Good, binary expression
            } else {
                unreachable!("Expected binary expression");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_function_calls() {
        // Test function calls in interpolation: 'Value: {get_value()}'
        let func_call = parse_expression_from_string("'Value: {get_value()}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = func_call {
            assert_eq!(parts.len(), 3);

            if let StringPart::Expression(Expr::Call { .. }) = parts[1] {
                // Good, function call expression
            } else {
                unreachable!("Expected function call expression");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_type_of() {
        // Test type_of in interpolation: 'Type: {type_of(x)}'
        let type_of_interp = parse_expression_from_string("'Type: {type_of(x)}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = type_of_interp {
            assert_eq!(parts.len(), 3);

            if let StringPart::Expression(Expr::TypeOf { .. }) = parts[1] {
                // Good, type_of expression
            } else {
                unreachable!("Expected type_of expression");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_only_expression() {
        // Test string with only interpolation: '{value}'
        let only_expr = parse_expression_from_string("'{value}'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = only_expr {
            assert_eq!(parts.len(), 3);

            // Should be: literal(""), expr(value), literal("")
            if let StringPart::Literal(ref text) = parts[0] {
                assert_eq!(text, "");
            } else {
                unreachable!("Expected empty literal part");
            }

            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
                assert_eq!(name, "value");
            } else {
                unreachable!("Expected identifier expression");
            }

            if let StringPart::Literal(ref text) = parts[2] {
                assert_eq!(text, "");
            } else {
                unreachable!("Expected empty literal part");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_no_spaces() {
        // Test interpolation without spaces: 'a{b}c{d}e'
        let no_spaces = parse_expression_from_string("'a{b}c{d}e'").unwrap();
        if let Expr::StringInterpolation { parts, .. } = no_spaces {
            assert_eq!(parts.len(), 5);

            // Should be: literal("a"), expr(b), literal("c"), expr(d), literal("e")
            if let StringPart::Literal(ref text) = parts[0] {
                assert_eq!(text, "a");
            } else {
                unreachable!("Expected literal 'a'");
            }

            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[1] {
                assert_eq!(name, "b");
            } else {
                unreachable!("Expected identifier 'b'");
            }

            if let StringPart::Literal(ref text) = parts[2] {
                assert_eq!(text, "c");
            } else {
                unreachable!("Expected literal 'c'");
            }

            if let StringPart::Expression(Expr::Identifier { ref name, .. }) = parts[3] {
                assert_eq!(name, "d");
            } else {
                unreachable!("Expected identifier 'd'");
            }

            if let StringPart::Literal(ref text) = parts[4] {
                assert_eq!(text, "e");
            } else {
                unreachable!("Expected literal 'e'");
            }
        } else {
            unreachable!("Expected string interpolation");
        }
    }

    #[test]
    fn test_string_interpolation_error_cases() {
        // Test unclosed interpolation brace
        let result = parse_expression_from_string("'Hello {world'");
        assert!(result.is_err(), "Should fail on unclosed brace");

        // Test empty interpolation
        let empty_result = parse_expression_from_string("'Hello {}'");
        assert!(empty_result.is_err(), "Should fail on empty interpolation");

        // Test unmatched closing brace
        let _unmatched_result = parse_expression_from_string("'Hello world}'");
        // This should actually be a regular string literal with '}' in it
        // So it might not be an error, depending on implementation
    }

    // Basic type parsing tests
    #[test]
    fn test_basic_type_parsing() {
        // Test primitive types
        let int_type = parse_type_from_string("int32").unwrap();
        if let Type::Basic { name, .. } = int_type {
            assert_eq!(name, "int32");
        } else {
            panic!("Expected basic type");
        }

        let string_type = parse_type_from_string("string").unwrap();
        if let Type::Basic { name, .. } = string_type {
            assert_eq!(name, "string");
        } else {
            panic!("Expected basic type");
        }

        let bool_type = parse_type_from_string("boolean").unwrap();
        if let Type::Basic { name, .. } = bool_type {
            assert_eq!(name, "boolean");
        } else {
            panic!("Expected basic type");
        }

        let void_type = parse_type_from_string("void").unwrap();
        if let Type::Basic { name, .. } = void_type {
            assert_eq!(name, "void");
        } else {
            panic!("Expected basic type");
        }
    }

    #[test]
    fn test_array_type_parsing() {
        // Test simple array type
        let array_type = parse_type_from_string("int32[]").unwrap();
        if let Type::Array { element_type, .. } = array_type {
            if let Type::Basic { name, .. } = element_type.as_ref() {
                assert_eq!(name, "int32");
            } else {
                panic!("Expected basic element type");
            }
        } else {
            panic!("Expected array type");
        }

        // Test nested array type
        let nested_array = parse_type_from_string("string[][]").unwrap();
        if let Type::Array { element_type, .. } = nested_array {
            if let Type::Array {
                element_type: inner,
                ..
            } = element_type.as_ref()
            {
                if let Type::Basic { name, .. } = inner.as_ref() {
                    assert_eq!(name, "string");
                } else {
                    panic!("Expected basic inner element type");
                }
            } else {
                panic!("Expected nested array type");
            }
        } else {
            panic!("Expected array type");
        }
    }

    #[test]
    fn test_custom_type_parsing() {
        // Test custom type names (Pascal case)
        let custom_type = parse_type_from_string("MyCustomType").unwrap();
        if let Type::Basic { name, .. } = custom_type {
            assert_eq!(name, "MyCustomType");
        } else {
            panic!("Expected basic type for custom type");
        }

        // Test custom type with array
        let custom_array = parse_type_from_string("Person[]").unwrap();
        if let Type::Array { element_type, .. } = custom_array {
            if let Type::Basic { name, .. } = element_type.as_ref() {
                assert_eq!(name, "Person");
            } else {
                panic!("Expected basic element type");
            }
        } else {
            panic!("Expected array type");
        }
    }

    #[test]
    fn test_basic_type_parsing_error_cases() {
        // Test starting with a number token
        let result1 = parse_type_from_string("32");
        assert!(result1.is_err(), "Should fail on number token as type");

        // Test empty type
        let result2 = parse_type_from_string("");
        assert!(result2.is_err(), "Should fail on empty input");

        // Test invalid token as type name
        let result3 = parse_type_from_string("+");
        assert!(result3.is_err(), "Should fail on operator token as type");
    }

    #[test]
    fn test_generic_type_parsing_simple() {
        // Test simple generic type: Array<T>
        let simple_generic = parse_type_from_string("Array<T>").unwrap();
        if let Type::Generic {
            name, type_args, ..
        } = simple_generic
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);
            if let &Type::Basic {
                name: ref arg_name, ..
            } = &type_args[0]
            {
                assert_eq!(arg_name, "T");
            } else {
                unreachable!("Expected basic type T as argument");
            }
        } else {
            unreachable!("Expected generic type, got {simple_generic:?}");
        }
    }

    #[test]
    fn test_generic_type_parsing_multiple_params() {
        // Test multiple type parameters: Result<T, E>
        let multiple_params = parse_type_from_string("Result<T, E>").unwrap();
        if let Type::Generic {
            name, type_args, ..
        } = multiple_params
        {
            assert_eq!(name, "Result");
            assert_eq!(type_args.len(), 2);

            if let &Type::Basic {
                name: ref first_arg,
                ..
            } = &type_args[0]
            {
                assert_eq!(first_arg, "T");
            } else {
                unreachable!("Expected basic type T as first argument");
            }

            if let &Type::Basic {
                name: ref second_arg,
                ..
            } = &type_args[1]
            {
                assert_eq!(second_arg, "E");
            } else {
                unreachable!("Expected basic type E as second argument");
            }
        } else {
            unreachable!("Expected generic type, got {multiple_params:?}");
        }
    }

    #[test]
    fn test_generic_type_parsing_concrete_args() {
        // Test concrete type arguments: Array<int32>
        let concrete_args = parse_type_from_string("Array<int32>").unwrap();
        if let Type::Generic {
            name, type_args, ..
        } = concrete_args
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);
            if let &Type::Basic {
                name: ref arg_name, ..
            } = &type_args[0]
            {
                assert_eq!(arg_name, "int32");
            } else {
                unreachable!("Expected basic type int32 as argument");
            }
        } else {
            unreachable!("Expected generic type");
        }
    }

    #[test]
    fn test_generic_type_parsing_nested() {
        // Test nested generic types: Array<Result<T, E>>
        let nested_generic = parse_type_from_string("Array<Result<T, E>>").unwrap();
        if let Type::Generic {
            name, type_args, ..
        } = nested_generic
        {
            assert_eq!(name, "Array");
            assert_eq!(type_args.len(), 1);

            if let &Type::Generic {
                name: ref inner_name,
                type_args: ref inner_args,
                ..
            } = &type_args[0]
            {
                assert_eq!(inner_name, "Result");
                assert_eq!(inner_args.len(), 2);

                if let &Type::Basic {
                    name: ref t_name, ..
                } = &inner_args[0]
                {
                    assert_eq!(t_name, "T");
                } else {
                    unreachable!("Expected T in nested generic");
                }

                if let &Type::Basic {
                    name: ref e_name, ..
                } = &inner_args[1]
                {
                    assert_eq!(e_name, "E");
                } else {
                    unreachable!("Expected E in nested generic");
                }
            } else {
                unreachable!("Expected nested generic type as argument");
            }
        } else {
            unreachable!("Expected generic type");
        }
    }

    #[test]
    fn test_generic_type_parsing_with_array_suffix() {
        // Test generic type with array suffix: Array<T>[]
        let generic_array = parse_type_from_string("Array<T>[]").unwrap();
        if let Type::Array { element_type, .. } = generic_array {
            if let &Type::Generic {
                ref name,
                ref type_args,
                ..
            } = element_type.as_ref()
            {
                assert_eq!(name, "Array");
                assert_eq!(type_args.len(), 1);

                if let &Type::Basic {
                    name: ref arg_name, ..
                } = &type_args[0]
                {
                    assert_eq!(arg_name, "T");
                } else {
                    unreachable!("Expected T as type argument");
                }
            } else {
                unreachable!("Expected generic type as array element");
            }
        } else {
            unreachable!("Expected array type with generic element");
        }
    }

    #[test]
    fn test_generic_type_parsing_error_cases() {
        // Test unclosed angle bracket
        let unclosed_result = parse_type_from_string("Array<T");
        assert!(
            unclosed_result.is_err(),
            "Should fail on unclosed angle bracket"
        );

        // Test empty generic arguments
        let empty_result = parse_type_from_string("Array<>");
        assert!(
            empty_result.is_err(),
            "Should fail on empty generic arguments"
        );

        // Test missing comma between arguments
        let missing_comma_result = parse_type_from_string("Result<T E>");
        assert!(
            missing_comma_result.is_err(),
            "Should fail on missing comma"
        );
    }

    #[test]
    fn test_function_type_parsing_simple() {
        // Test simple function type: f(int32): string
        let simple_func = parse_type_from_string("f(int32): string").unwrap();
        if let Type::Function {
            parameters,
            return_type,
            ..
        } = simple_func
        {
            assert_eq!(parameters.len(), 1);
            if let &Type::Basic { ref name, .. } = &parameters[0] {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected basic type int32 as parameter");
            }

            if let &Type::Basic { ref name, .. } = return_type.as_ref() {
                assert_eq!(name, "string");
            } else {
                unreachable!("Expected basic type string as return type");
            }
        } else {
            unreachable!("Expected function type, got {simple_func:?}");
        }
    }

    #[test]
    fn test_function_type_parsing_multiple_params() {
        // Test multiple parameters: f(int32, string, boolean): void
        let multi_param = parse_type_from_string("f(int32, string, boolean): void").unwrap();
        if let Type::Function {
            parameters,
            return_type,
            ..
        } = multi_param
        {
            assert_eq!(parameters.len(), 3);

            if let &Type::Basic { ref name, .. } = &parameters[0] {
                assert_eq!(name, "int32");
            } else {
                unreachable!("Expected int32 as first parameter");
            }

            if let &Type::Basic { ref name, .. } = &parameters[1] {
                assert_eq!(name, "string");
            } else {
                unreachable!("Expected string as second parameter");
            }

            if let &Type::Basic { ref name, .. } = &parameters[2] {
                assert_eq!(name, "boolean");
            } else {
                unreachable!("Expected boolean as third parameter");
            }

            if let &Type::Basic { ref name, .. } = return_type.as_ref() {
                assert_eq!(name, "void");
            } else {
                unreachable!("Expected void as return type");
            }
        } else {
            unreachable!("Expected function type");
        }
    }

    #[test]
    fn test_function_type_parsing_no_params() {
        // Test function with no parameters: f(): void
        let no_params = parse_type_from_string("f(): void").unwrap();
        if let Type::Function {
            parameters,
            return_type,
            ..
        } = no_params
        {
            assert_eq!(parameters.len(), 0);

            if let &Type::Basic { ref name, .. } = return_type.as_ref() {
                assert_eq!(name, "void");
            } else {
                unreachable!("Expected void as return type");
            }
        } else {
            unreachable!("Expected function type");
        }
    }

    #[test]
    fn test_function_type_parsing_generic_params() {
        // Test function with generic parameters: f(Array<T>, Result<T, E>): boolean
        let generic_params = parse_type_from_string("f(Array<T>, Result<T, E>): boolean").unwrap();
        if let Type::Function {
            parameters,
            return_type,
            ..
        } = generic_params
        {
            assert_eq!(parameters.len(), 2);

            if let &Type::Generic {
                ref name,
                ref type_args,
                ..
            } = &parameters[0]
            {
                assert_eq!(name, "Array");
                assert_eq!(type_args.len(), 1);
            } else {
                unreachable!("Expected generic type Array<T> as first parameter");
            }

            if let &Type::Generic {
                ref name,
                ref type_args,
                ..
            } = &parameters[1]
            {
                assert_eq!(name, "Result");
                assert_eq!(type_args.len(), 2);
            } else {
                unreachable!("Expected generic type Result<T, E> as second parameter");
            }

            if let &Type::Basic { ref name, .. } = return_type.as_ref() {
                assert_eq!(name, "boolean");
            } else {
                unreachable!("Expected boolean as return type");
            }
        } else {
            unreachable!("Expected function type");
        }
    }

    #[test]
    fn test_function_type_parsing_array_suffix() {
        // Test function type with array suffix: f(int32): string[]
        let array_return = parse_type_from_string("f(int32): string[]").unwrap();
        if let Type::Function {
            parameters,
            return_type,
            ..
        } = array_return
        {
            assert_eq!(parameters.len(), 1);

            if let &Type::Array {
                ref element_type, ..
            } = return_type.as_ref()
            {
                if let &Type::Basic { ref name, .. } = element_type.as_ref() {
                    assert_eq!(name, "string");
                } else {
                    unreachable!("Expected string as array element type");
                }
            } else {
                unreachable!("Expected array type as return type");
            }
        } else {
            unreachable!("Expected function type");
        }
    }

    #[test]
    fn test_function_type_parsing_error_cases() {
        // Test function without parentheses - should fail
        let no_parens = parse_type_from_string("f int32: string");
        assert!(no_parens.is_err(), "Should fail on missing parentheses");

        // Test function without return type - should fail
        let no_return = parse_type_from_string("f(int32)");
        assert!(no_return.is_err(), "Should fail on missing return type");

        // Test function with unclosed parameters - should fail
        let unclosed_params = parse_type_from_string("f(int32: string");
        assert!(
            unclosed_params.is_err(),
            "Should fail on unclosed parameters"
        );

        // Test function with malformed parameters - should fail
        let bad_params = parse_type_from_string("f(int32 string): void");
        assert!(bad_params.is_err(), "Should fail on malformed parameters");
    }

    // Type declaration parsing tests
    #[test]
    fn test_simple_type_declaration_no_doc() {
        // Test a simple type without doc comments first
        let input = "type Direction:\n    North\n    East\n    South\n    West";

        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse simple type successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Type {
            name,
            type_def,
            doc_comment,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(name, "Direction");
            assert!(doc_comment.is_none());

            if let TypeDef::Sum { variants, .. } = type_def {
                assert_eq!(variants.len(), 4);
                assert_eq!(variants[0].name, "North");
                assert_eq!(variants[1].name, "East");
                assert_eq!(variants[2].name, "South");
                assert_eq!(variants[3].name, "West");
            } else {
                panic!("Expected sum type definition");
            }
        } else {
            panic!("Expected type declaration");
        }
    }

    #[test]
    fn test_simple_sum_type_parsing() {
        // Test a simple enum-like type without the complex doc comment for now
        let input = "type Direction:\n    North\n    East\n    South\n    West";

        let result = parse_program_from_string(input);
        assert!(result.is_ok(), "Should parse simple sum type successfully");

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Type {
            name,
            type_def,
            doc_comment,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(name, "Direction");
            assert!(doc_comment.is_none()); // Changed expectation since we're not providing a doc comment

            if let TypeDef::Sum { variants, .. } = type_def {
                assert_eq!(variants.len(), 4);
                assert_eq!(variants[0].name, "North");
                assert_eq!(variants[1].name, "East");
                assert_eq!(variants[2].name, "South");
                assert_eq!(variants[3].name, "West");

                // Simple enum variants should have no fields
                for variant in variants {
                    assert!(variant.fields.is_empty());
                }
            } else {
                panic!("Expected sum type definition");
            }
        } else {
            panic!("Expected type declaration");
        }
    }

    #[test]
    fn test_sum_type_with_fields_parsing() {
        // Test a sum type with variants that have fields - simplified for now
        let input = "type Message:\n    Text:\n        sender: string\n        body: string";

        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse sum type with fields successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Type { name, type_def, .. } = &program.declarations[0] {
            assert_eq!(name, "Message");

            if let TypeDef::Sum { variants, .. } = type_def {
                // For now, just check that we parse it as a sum type
                // We'll improve field parsing later
                assert!(!variants.is_empty());
            } else {
                panic!("Expected sum type definition, got: {:?}", type_def);
            }
        } else {
            panic!("Expected type declaration");
        }
    }

    #[test]
    fn test_product_type_parsing() {
        // Test a simple product type (struct-like)
        let input = "type Person:\n    name: string\n    age: int32";

        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse product type successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Type { name, type_def, .. } = &program.declarations[0] {
            assert_eq!(name, "Person");

            if let TypeDef::Product { fields, .. } = type_def {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].name, "name");
                assert_eq!(fields[1].name, "age");

                // Check field types
                if let Type::Basic { name, .. } = &fields[0].type_annotation {
                    assert_eq!(name, "string");
                } else {
                    panic!("Expected basic type for name field");
                }

                if let Type::Basic { name, .. } = &fields[1].type_annotation {
                    assert_eq!(name, "int32");
                } else {
                    panic!("Expected basic type for age field");
                }
            } else {
                panic!("Expected product type definition, got: {:?}", type_def);
            }
        } else {
            panic!("Expected type declaration");
        }
    }

    #[test]
    fn test_generic_type_declaration_parsing() {
        // Test a generic sum type - simplified for now
        let input = "type Result:\n    Ok\n    Error";

        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse generic type successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Type { name, .. } = &program.declarations[0] {
            assert_eq!(name, "Result");
            // TODO: Add tests for generic parameters once implemented
        } else {
            panic!("Expected type declaration");
        }
    }

    #[test]
    fn test_type_declaration_error_cases() {
        // Test missing colon after type name
        let result = parse_program_from_string("type Message\n    Text");
        assert!(result.is_err(), "Should fail on missing colon");

        // Test missing variants/fields
        let result = parse_program_from_string("type Empty:");
        assert!(result.is_err(), "Should fail on empty type body");

        // Test invalid variant syntax
        let result = parse_program_from_string("type Bad:\n    123Invalid");
        assert!(result.is_err(), "Should fail on invalid variant name");
    }

    #[test]
    fn test_import_single_item() {
        let input = "import is_prime from ./nums";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse single import successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./nums");
            assert_eq!(items.len(), 1);

            if let ImportItem::Named { name, alias, .. } = &items[0] {
                assert_eq!(name, "is_prime");
                assert!(alias.is_none());
            } else {
                panic!("Expected ImportItem::Named");
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_with_alias() {
        let input = "import is_prime as is_prime_new from ./nums";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse import with alias successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./nums");
            assert_eq!(items.len(), 1);

            if let ImportItem::Named { name, alias, .. } = &items[0] {
                assert_eq!(name, "is_prime");
                assert_eq!(alias.as_ref().unwrap(), "is_prime_new");
            } else {
                panic!("Expected ImportItem::Named with alias");
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_multiple_items() {
        let input = "import is_prime, gcd, pi from ./nums";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse multiple imports successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./nums");
            assert_eq!(items.len(), 3);

            let expected_names = ["is_prime", "gcd", "pi"];
            for (i, expected_name) in expected_names.iter().enumerate() {
                if let ImportItem::Named { name, alias, .. } = &items[i] {
                    assert_eq!(name, expected_name);
                    assert!(alias.is_none());
                } else {
                    panic!("Expected ImportItem::Named for {}", expected_name);
                }
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_type() {
        let input = "import type User from ./models.types";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse type import successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./models.types");
            assert_eq!(items.len(), 1);

            if let ImportItem::Type { name, alias, .. } = &items[0] {
                assert_eq!(name, "User");
                assert!(alias.is_none());
            } else {
                panic!("Expected ImportItem::Type");
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_mixed_with_aliases() {
        let input = "import is_prime as is_prime_new, gcd as greatest_cd from ./nums";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse mixed imports with aliases successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./nums");
            assert_eq!(items.len(), 2);

            if let ImportItem::Named { name, alias, .. } = &items[0] {
                assert_eq!(name, "is_prime");
                assert_eq!(alias.as_ref().unwrap(), "is_prime_new");
            } else {
                panic!("Expected first ImportItem::Named with alias");
            }

            if let ImportItem::Named { name, alias, .. } = &items[1] {
                assert_eq!(name, "gcd");
                assert_eq!(alias.as_ref().unwrap(), "greatest_cd");
            } else {
                panic!("Expected second ImportItem::Named with alias");
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_mixed_types_and_items() {
        let input = "import type User, Address from ./models.types";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Should parse multiple type imports successfully: {:?}",
            result.err()
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Import { items, source, .. } = &program.declarations[0] {
            assert_eq!(source, "./models.types");
            assert_eq!(items.len(), 2);

            let expected_names = ["User", "Address"];
            for (i, expected_name) in expected_names.iter().enumerate() {
                if let ImportItem::Type { name, alias, .. } = &items[i] {
                    assert_eq!(name, expected_name);
                    assert!(alias.is_none());
                } else {
                    panic!("Expected ImportItem::Type for {}", expected_name);
                }
            }
        } else {
            panic!("Expected import declaration");
        }
    }

    #[test]
    fn test_import_error_cases() {
        // Test missing 'from' keyword
        let result = parse_program_from_string("import is_prime ./nums");
        assert!(result.is_err(), "Should fail on missing 'from' keyword");

        // Test missing source path
        let result = parse_program_from_string("import is_prime from");
        assert!(result.is_err(), "Should fail on missing source path");

        // Test empty import list
        let result = parse_program_from_string("import from ./nums");
        assert!(result.is_err(), "Should fail on empty import list");

        // Test invalid alias syntax
        let result = parse_program_from_string("import is_prime as from ./nums");
        assert!(result.is_err(), "Should fail on invalid alias syntax");

        // Test missing item name
        let result = parse_program_from_string("import , gcd from ./nums");
        assert!(result.is_err(), "Should fail on missing item name");
    }

    #[test]
    fn test_function_parameter_edge_cases() {
        // Test function with generic parameter types
        let input = "entry main = f(items: Array<string>, result: Result<int32, string>): void => return void";
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program, errors) = parser.parse();

        assert!(errors.is_empty(), "Parse errors: {errors:?}");
        assert!(program.is_some());

        let program = program.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Function { parameters, .. } = &program.declarations[0] {
            assert_eq!(parameters.len(), 2);

            // Check first parameter (items: Array<string>)
            assert_eq!(parameters[0].name, "items");
            if let Type::Generic {
                name, type_args, ..
            } = &parameters[0].param_type
            {
                assert_eq!(name, "Array");
                assert_eq!(type_args.len(), 1);
                if let Type::Basic { name, .. } = &type_args[0] {
                    assert_eq!(name, "string");
                } else {
                    unreachable!("Expected string type argument");
                }
            } else {
                unreachable!("Expected generic type for first parameter");
            }

            // Check second parameter (result: Result<int32, string>)
            assert_eq!(parameters[1].name, "result");
            if let Type::Generic {
                name, type_args, ..
            } = &parameters[1].param_type
            {
                assert_eq!(name, "Result");
                assert_eq!(type_args.len(), 2);
            } else {
                unreachable!("Expected generic type for second parameter");
            }
        } else {
            unreachable!("Expected function declaration");
        }
    }

    #[test]
    fn test_function_array_parameter_types() {
        // Test function with array parameter types
        let input =
            "public process = f(numbers: int32[], names: string[][]): boolean[] => return void";
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program, errors) = parser.parse();

        assert!(errors.is_empty(), "Parse errors: {errors:?}");
        assert!(program.is_some());

        let program = program.unwrap();
        if let Decl::Function {
            parameters,
            return_type,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(parameters.len(), 2);

            // Check first parameter (numbers: int32[])
            assert_eq!(parameters[0].name, "numbers");
            if let Type::Array { element_type, .. } = &parameters[0].param_type {
                if let Type::Basic { name, .. } = element_type.as_ref() {
                    assert_eq!(name, "int32");
                } else {
                    unreachable!("Expected int32 element type");
                }
            } else {
                unreachable!("Expected array type for first parameter");
            }

            // Check second parameter (names: string[][])
            assert_eq!(parameters[1].name, "names");
            if let Type::Array { element_type, .. } = &parameters[1].param_type {
                if let Type::Array {
                    element_type: inner,
                    ..
                } = element_type.as_ref()
                {
                    if let Type::Basic { name, .. } = inner.as_ref() {
                        assert_eq!(name, "string");
                    } else {
                        unreachable!("Expected string inner element type");
                    }
                } else {
                    unreachable!("Expected nested array type");
                }
            } else {
                unreachable!("Expected array type for second parameter");
            }

            // Check return type (boolean[])
            assert!(return_type.is_some());
            if let Some(Type::Array { element_type, .. }) = return_type {
                if let Type::Basic { name, .. } = element_type.as_ref() {
                    assert_eq!(name, "boolean");
                } else {
                    unreachable!("Expected boolean element type in return");
                }
            } else {
                unreachable!("Expected array return type");
            }
        } else {
            unreachable!("Expected function declaration");
        }
    }

    #[test]
    fn test_function_complex_return_types() {
        // Test function with complex return types
        let input = "entry compute = f(x: int32): Result<Array<string>, string> => return void";
        let lexer = Lexer::new(input);
        let (tokens, _) = lexer.tokenize();
        let parser = Parser::new(tokens);
        let (program, errors) = parser.parse();

        assert!(errors.is_empty(), "Parse errors: {errors:?}");
        assert!(program.is_some());

        let program = program.unwrap();
        if let Decl::Function { return_type, .. } = &program.declarations[0] {
            assert!(return_type.is_some());
            if let Some(Type::Generic {
                name, type_args, ..
            }) = return_type
            {
                assert_eq!(name, "Result");
                assert_eq!(type_args.len(), 2);

                // First type arg should be Array<string>
                if let Type::Generic {
                    name,
                    type_args: inner_args,
                    ..
                } = &type_args[0]
                {
                    assert_eq!(name, "Array");
                    assert_eq!(inner_args.len(), 1);
                    if let Type::Basic { name, .. } = &inner_args[0] {
                        assert_eq!(name, "string");
                    } else {
                        unreachable!("Expected string type in Array");
                    }
                } else {
                    unreachable!("Expected Array<string> as first type arg");
                }

                // Second type arg should be string
                if let Type::Basic { name, .. } = &type_args[1] {
                    assert_eq!(name, "string");
                } else {
                    unreachable!("Expected string as second type arg");
                }
            } else {
                unreachable!("Expected generic return type");
            }
        } else {
            unreachable!("Expected function declaration");
        }
    }

    #[test]
    fn test_function_parameter_error_cases() {
        // Test invalid parameter syntax
        let result1 = parse_program_from_string("entry test = f(x y: int32): void => return void");
        assert!(
            result1.is_err(),
            "Should fail on missing colon in parameter"
        );

        // Test missing parameter type
        let result2 = parse_program_from_string("entry test = f(x:): void => return void");
        assert!(result2.is_err(), "Should fail on missing parameter type");

        // Test invalid parameter name
        let result3 = parse_program_from_string("entry test = f(123: int32): void => return void");
        assert!(result3.is_err(), "Should fail on numeric parameter name");

        // Test missing parameter name
        let result4 = parse_program_from_string("entry test = f(: int32): void => return void");
        assert!(result4.is_err(), "Should fail on missing parameter name");

        // Test malformed generic parameter type
        let result5 = parse_program_from_string("entry test = f(x: Array<>): void => return void");
        assert!(result5.is_err(), "Should fail on empty generic type args");
    }

    #[test]
    fn test_lambda_expression_basic() {
        // Test basic lambda expression as let declaration
        let input = "let add = f(x: int32, y: int32): int32 => x + y";
        let result = parse_program_from_string(input);
        assert!(result.is_ok(), "Failed to parse basic lambda: {result:?}");

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let {
            binding,
            initializer,
            visibility,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(binding.name, "add");
            assert!(!binding.is_mutable);
            assert!(binding.type_annotation.is_none());
            assert_eq!(*visibility, Visibility::Private);

            // Check that initializer is a lambda expression
            if let Expr::Lambda {
                generic_params,
                params,
                return_type,
                body,
                ..
            } = initializer
            {
                assert!(generic_params.is_none());
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "x");
                assert_eq!(params[1].name, "y");

                if let Type::Basic { name, .. } = &return_type {
                    assert_eq!(name, "int32");
                } else {
                    unreachable!("Expected basic return type");
                }

                if let LambdaBody::Expression(expr) = body {
                    if let Expr::Binary { .. } = expr.as_ref() {
                        // Binary expression is expected for x + y
                    } else {
                        unreachable!("Expected binary expression in lambda body");
                    }
                } else {
                    unreachable!("Expected expression body");
                }
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }

    #[test]
    fn test_lambda_expression_generic() {
        // Test generic lambda expression as let declaration
        let input = "let identity = f<T>(x: T): T => x";
        let result = parse_program_from_string(input);
        assert!(result.is_ok(), "Failed to parse generic lambda: {result:?}");

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let {
            binding,
            initializer,
            visibility,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(binding.name, "identity");
            assert!(!binding.is_mutable);
            assert!(binding.type_annotation.is_none());
            assert_eq!(*visibility, Visibility::Private);

            // Check that initializer is a lambda expression
            if let Expr::Lambda {
                generic_params,
                params,
                return_type,
                body,
                ..
            } = initializer
            {
                assert!(generic_params.is_some());
                let generics = generic_params.as_ref().unwrap();
                assert_eq!(generics.len(), 1);
                assert_eq!(generics[0], "T");

                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");

                // Check parameter type is generic
                if let Type::Basic { name, .. } = &params[0].param_type {
                    assert_eq!(name, "T");
                } else {
                    unreachable!("Expected generic parameter type");
                }

                if let Type::Basic { name, .. } = &return_type {
                    assert_eq!(name, "T");
                } else {
                    unreachable!("Expected generic return type");
                }

                if let LambdaBody::Expression(expr) = body {
                    if let Expr::Identifier { name, .. } = expr.as_ref() {
                        assert_eq!(name, "x");
                    } else {
                        unreachable!("Expected identifier in lambda body");
                    }
                } else {
                    unreachable!("Expected expression body");
                }
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }

    #[test]
    fn test_lambda_expression_no_params() {
        // Test lambda with no parameters as let declaration
        let input = "let get_42 = f(): int32 => 42";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse no-param lambda: {result:?}"
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let {
            binding,
            initializer,
            visibility,
            ..
        } = &program.declarations[0]
        {
            assert_eq!(binding.name, "get_42");
            assert!(!binding.is_mutable);
            assert!(binding.type_annotation.is_none());
            assert_eq!(*visibility, Visibility::Private);

            // Check that initializer is a lambda expression
            if let Expr::Lambda {
                generic_params,
                params,
                return_type,
                body,
                ..
            } = initializer
            {
                assert!(generic_params.is_none());
                assert_eq!(params.len(), 0);

                if let Type::Basic { name, .. } = &return_type {
                    assert_eq!(name, "int32");
                } else {
                    unreachable!("Expected basic return type");
                }

                if let LambdaBody::Expression(expr) = body {
                    if let Expr::Literal { value, .. } = expr.as_ref() {
                        if let LiteralValue::Integer(n) = value {
                            assert_eq!(*n, 42);
                        } else {
                            unreachable!("Expected integer literal");
                        }
                    } else {
                        unreachable!("Expected literal in lambda body");
                    }
                } else {
                    unreachable!("Expected expression body");
                }
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }

    #[test]
    fn test_lambda_expression_multiple_generics() {
        // Test lambda with multiple generic parameters as let declaration
        let input = "let map_fn = f<T, U>(transform: f(T): U, value: T): U => transform(value)";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse multi-generic lambda: {result:?}"
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        let let_decl = match &program.declarations[0] {
            Decl::Let {
                binding,
                initializer,
                visibility,
                ..
            } => {
                assert_eq!(binding.name, "map_fn");
                assert!(!binding.is_mutable);
                assert!(binding.type_annotation.is_none());
                assert_eq!(*visibility, Visibility::Private);
                initializer
            }
            _ => unreachable!("Expected let declaration"),
        };

        // Check that initializer is a lambda expression
        if let Expr::Lambda {
            generic_params,
            params,
            return_type,
            ..
        } = let_decl
        {
            validate_lambda_generics(generic_params.as_ref());
            validate_lambda_parameters(params);
            validate_lambda_return_type(return_type);
        } else {
            unreachable!("Expected lambda expression");
        }
    }

    /// Helper function to validate generic parameters in lambda expressions
    fn validate_lambda_generics(generic_params: Option<&Vec<String>>) {
        assert!(generic_params.is_some());
        let generics = generic_params.unwrap();
        assert_eq!(generics.len(), 2);
        assert_eq!(generics[0], "T");
        assert_eq!(generics[1], "U");
    }

    /// Helper function to validate lambda parameters in complex generic scenarios
    fn validate_lambda_parameters(params: &[Parameter]) {
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "transform");
        assert_eq!(params[1].name, "value");

        // Check first parameter is a function type
        if let Type::Function {
            parameters: fn_params,
            return_type: fn_return,
            ..
        } = &params[0].param_type
        {
            assert_eq!(fn_params.len(), 1);
            if let Type::Basic { name, .. } = &fn_params[0] {
                assert_eq!(name, "T");
            } else {
                unreachable!("Expected T parameter type");
            }

            if let Type::Basic { name, .. } = fn_return.as_ref() {
                assert_eq!(name, "U");
            } else {
                unreachable!("Expected U return type");
            }
        } else {
            unreachable!("Expected function type for transform parameter");
        }

        // Check second parameter type
        if let Type::Basic { name, .. } = &params[1].param_type {
            assert_eq!(name, "T");
        } else {
            unreachable!("Expected generic parameter type");
        }
    }

    /// Helper function to validate lambda return type
    fn validate_lambda_return_type(return_type: &Type) {
        if let Type::Basic { name, .. } = return_type {
            assert_eq!(name, "U");
        } else {
            unreachable!("Expected generic return type");
        }
    }

    #[test]
    fn test_lambda_expression_error_cases() {
        // Test missing parameters parentheses
        let result1 = parse_expression_from_string("f x: int32 => x");
        assert!(result1.is_err(), "Should fail on missing parentheses");

        // Test missing colon before return type
        let result2 = parse_expression_from_string("f() int32 => 42");
        assert!(result2.is_err(), "Should fail on missing colon");

        // Test missing arrow
        let result3 = parse_expression_from_string("f(): int32 42");
        assert!(result3.is_err(), "Should fail on missing arrow");

        // Test empty generic parameters
        let result4 = parse_expression_from_string("f<>(): void => void");
        assert!(result4.is_err(), "Should fail on empty generics");

        // Test malformed generic parameters
        let result5 = parse_expression_from_string("f<T,>(): void => void");
        assert!(
            result5.is_err(),
            "Should fail on trailing comma in generics"
        );
    }

    #[test]
    fn test_lambda_as_function_parameter() {
        // Test lambda as function parameter type
        let input = "entry test = f(callback: f(int32): boolean): void => return void";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse lambda as parameter: {result:?}"
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Function {
            parameters: params, ..
        } = &program.declarations[0]
        {
            assert_eq!(params.len(), 1);
            assert_eq!(params[0].name, "callback");

            if let Type::Function {
                parameters: fn_params,
                return_type: fn_return,
                ..
            } = &params[0].param_type
            {
                assert_eq!(fn_params.len(), 1);
                if let Type::Basic { name, .. } = &fn_params[0] {
                    assert_eq!(name, "int32");
                } else {
                    unreachable!("Expected int32 parameter type");
                }

                if let Type::Basic { name, .. } = fn_return.as_ref() {
                    assert_eq!(name, "boolean");
                } else {
                    unreachable!("Expected boolean return type");
                }
            } else {
                unreachable!("Expected function type");
            }
        } else {
            unreachable!("Expected function declaration");
        }
    }

    #[test]
    fn test_lambda_expression_nested() {
        // Test nested lambda expressions
        let input = "let curry_add = f(x: int32): f(int32): int32 => f(y: int32): int32 => x + y";
        let result = parse_program_from_string(input);
        assert!(result.is_ok(), "Failed to parse nested lambda: {result:?}");

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let { initializer, .. } = &program.declarations[0] {
            if let Expr::Lambda {
                params,
                return_type,
                body,
                ..
            } = initializer
            {
                assert_eq!(params.len(), 1);
                assert_eq!(params[0].name, "x");

                // Check return type is a function type
                if let Type::Function { .. } = return_type {
                    // Good
                } else {
                    unreachable!("Expected function return type for curried function");
                }

                // Check body contains another lambda
                if let LambdaBody::Expression(expr) = body {
                    if let Expr::Lambda { .. } = expr.as_ref() {
                        // Good, nested lambda found
                    } else {
                        unreachable!("Expected nested lambda in body");
                    }
                } else {
                    unreachable!("Expected expression body");
                }
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }

    #[test]
    fn test_lambda_expression_block_body() {
        // Test lambda with block body
        let input =
            "let complex_fn = f(x: int32): int32 => { let doubled = x * 2; return doubled + 1; }";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse block body lambda: {result:?}"
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let { initializer, .. } = &program.declarations[0] {
            if let Expr::Lambda { body, .. } = initializer {
                if let LambdaBody::Block(statements) = body {
                    assert_eq!(statements.len(), 2);
                    // First statement should be a let binding
                    if let Stmt::Let { binding, .. } = &statements[0] {
                        assert_eq!(binding.name, "doubled");
                    } else {
                        unreachable!("Expected let statement");
                    }
                    // Second statement should be a return
                    if let Stmt::Return { .. } = &statements[1] {
                        // Good
                    } else {
                        unreachable!("Expected return statement");
                    }
                } else {
                    unreachable!("Expected block body");
                }
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }

    #[test]
    fn test_lambda_expression_complex_generics() {
        // Test lambda with complex generic constraints
        let input = "let transform = f<T, U, V>(data: T[], mapper: f(T): U, reducer: f(U[]): V): V => reducer(map(data, mapper))";
        let result = parse_program_from_string(input);
        assert!(
            result.is_ok(),
            "Failed to parse complex generic lambda: {result:?}"
        );

        let program = result.unwrap();
        assert_eq!(program.declarations.len(), 1);

        if let Decl::Let { initializer, .. } = &program.declarations[0] {
            if let Expr::Lambda {
                generic_params,
                params,
                ..
            } = initializer
            {
                // Check generic parameters
                assert!(generic_params.is_some());
                let generics = generic_params.as_ref().unwrap();
                assert_eq!(generics.len(), 3);
                assert_eq!(generics[0], "T");
                assert_eq!(generics[1], "U");
                assert_eq!(generics[2], "V");

                // Check parameters
                assert_eq!(params.len(), 3);
                assert_eq!(params[0].name, "data");
                assert_eq!(params[1].name, "mapper");
                assert_eq!(params[2].name, "reducer");
            } else {
                unreachable!("Expected lambda expression");
            }
        } else {
            unreachable!("Expected let declaration");
        }
    }
}
