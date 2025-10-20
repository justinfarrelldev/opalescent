//! Documentation structures for AST nodes
//!
//! This module contains structured documentation support for Opalescent,
//! including parsing of doc comments and metadata extraction.

extern crate alloc;
use crate::token::Span;
use alloc::collections::BTreeMap;
use alloc::string::String;

/// Structured documentation extracted from Opalescent doc comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Documentation {
    /// Raw documentation text exactly as written in source code (sans comment delimiters).
    pub raw: String,
    /// Named sections captured from `Key: Value` lines within the documentation block.
    pub sections: BTreeMap<String, String>,
    /// Attribute-style metadata parsed from lines starting with `@attribute`.
    pub attributes: BTreeMap<String, String>,
    /// Source span covering the entire documentation block for precise diagnostics.
    pub span: Span,
}

impl Documentation {
    /// Construct structured documentation from raw comment text while preserving metadata.
    #[must_use]
    pub fn from_raw(raw: String, span: Span) -> Self {
        fn flush_section(
            sections: &mut BTreeMap<String, String>,
            current_key: &mut Option<String>,
            buffer: &mut String,
        ) {
            if let Some(key) = current_key.take() {
                let value = buffer.trim();
                sections.insert(key, String::from(value));
                buffer.clear();
            }
        }

        let mut sections = BTreeMap::new();
        let mut attributes = BTreeMap::new();
        let mut current_key: Option<String> = None;
        let mut buffer = String::new();

        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            if trimmed.starts_with('@') {
                flush_section(&mut sections, &mut current_key, &mut buffer);
                let attribute_body = trimmed.trim_start_matches('@').trim();
                if attribute_body.is_empty() {
                    continue;
                }

                let mut parts = attribute_body.splitn(2, char::is_whitespace);
                let key_part = parts.next().unwrap_or_default().trim_matches(':');
                if key_part.is_empty() {
                    continue;
                }

                let value_part = parts
                    .next()
                    .map_or_else(String::new, |value| String::from(value.trim()));

                attributes.insert(key_part.to_lowercase(), value_part);
                continue;
            }

            if let Some((key, value_part)) = trimmed.split_once(':') {
                flush_section(&mut sections, &mut current_key, &mut buffer);
                current_key = Some(String::from(key.trim()));
                buffer.push_str(value_part.trim());
            } else if current_key.is_some() {
                if !buffer.is_empty() {
                    buffer.push(' ');
                }
                buffer.push_str(trimmed);
            }
        }

        flush_section(&mut sections, &mut current_key, &mut buffer);

        Self {
            raw,
            sections,
            attributes,
            span,
        }
    }
}
