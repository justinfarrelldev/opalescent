//! Import declaration parsing module for the Opalescent parser.
//!
//! This module contains all methods related to parsing import declarations,
//! including import items, import paths, and path components.
extern crate alloc;
use super::{ParseError, ParseResult, Parser};
use crate::ast::{Decl, HotReloadMetadata, ImportItem, ImportStatement};
use crate::token::{Span, TokenType};
use alloc::string::String;
use alloc::vec::Vec;

impl Parser {
    /// Parse an import declaration
    pub(super) fn parse_import_declaration(&mut self) -> ParseResult<Decl> {
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
            statement: ImportStatement {
                names: items
                    .iter()
                    .map(|item| match item {
                        &ImportItem::Named { ref name, .. }
                        | &ImportItem::Type { ref name, .. } => name.clone(),
                        &ImportItem::Glob { .. } => String::from("*"),
                    })
                    .collect(),
                module: source.clone(),
            },
            items,
            source,
            span: import_span,
            id: self.next_node_id(),
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
        let alias = if self.check(&TokenType::Cast) {
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
}
