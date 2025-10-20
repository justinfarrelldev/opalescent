//! Declaration parsing module for the Opalescent parser.
//!
//! This module contains all methods related to parsing top-level declarations,
//! including function declarations, type declarations, import declarations, and let declarations.
//! These methods are part of the Parser implementation but are organized here for modularity.
use super::{ParseError, ParseResult, Parser, next_node_id};
use crate::ast::{
    AstNode, Decl, Documentation, Field, HotReloadMetadata, ImportItem, LetBinding, Parameter,
    Stmt, Type, TypeDef, Variant, Visibility,
};
use crate::token::{Span, TokenType};

impl Parser {
    /// Parse a top-level declaration
    pub(super) fn parse_declaration(&mut self) -> ParseResult<Decl> {
        // Check for documentation comment
        let doc_comment = self.collect_documentation();
        self.skip_trivia_preserving_doc_comments();

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

    /// Collect consecutive documentation comment tokens, if present, and convert
    /// them into structured documentation metadata.
    fn collect_documentation(&mut self) -> Option<Documentation> {
        let mut raw_parts = Vec::new();
        let mut span: Option<Span> = None;

        while self.check(&TokenType::DocComment(String::new())) {
            let token = self.advance().clone();
            if let TokenType::DocComment(content) = token.token_type {
                span = Some(span.map_or(token.span, |existing| {
                    Span::new(existing.start, token.span.end)
                }));
                raw_parts.push(content);
            }

            while self.check(&TokenType::Newline) {
                self.advance();
            }
        }

        if raw_parts.is_empty() {
            return None;
        }

        let combined_raw = raw_parts.join("\n");
        span.map(|documentation_span| Documentation::from_raw(combined_raw, documentation_span))
    }

    /// Construct a `LetBinding` with consistent span calculation and node id assignment
    pub(super) fn create_let_binding(
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
        doc_comment: Option<Documentation>,
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
    pub(super) fn parse_parameter_list(&mut self) -> ParseResult<Vec<Parameter>> {
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
        doc_comment: Option<Documentation>,
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

    /// Parse a let declaration (variable declarations that can include lambda expressions)
    fn parse_let_declaration(
        &mut self,
        visibility: Visibility,
        doc_comment: Option<Documentation>,
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
}
