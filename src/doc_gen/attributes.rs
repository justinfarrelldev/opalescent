//! Attribute parsing for documentation comments.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Parsed `@param` attribute entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocParam {
    /// Parameter name.
    pub name: String,
    /// Parameter description.
    pub description: String,
}

/// Parsed `@returns` attribute entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocReturn {
    /// Return value description.
    pub description: String,
}

/// Parsed `@example` attribute entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocExample {
    /// Example code snippet.
    pub code: String,
}

/// Parsed attribute and free-form documentation content.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedDocAttributes {
    /// Free-form description text that is not an attribute.
    pub description: Option<String>,
    /// Parsed parameter documentation entries.
    pub params: Vec<DocParam>,
    /// Parsed return documentation entry.
    pub returns: Option<DocReturn>,
    /// Parsed example blocks.
    pub examples: Vec<DocExample>,
}

/// Parse structured documentation attributes from raw comment text.
#[must_use]
pub fn parse_doc_attributes(raw_doc: &str) -> ParsedDocAttributes {
    let mut parsed = ParsedDocAttributes::default();
    let mut description_lines = Vec::new();

    for line in raw_doc.lines() {
        let trimmed_line = line.trim();
        if trimmed_line.is_empty() {
            continue;
        }

        if let Some(attribute_body) = trimmed_line.strip_prefix("@param") {
            let param_value = attribute_body.trim();
            if let Some((name_part, description_part)) = param_value.split_once(':') {
                let name = name_part.trim();
                if !name.is_empty() {
                    parsed.params.push(DocParam {
                        name: name.to_owned(),
                        description: description_part.trim().to_owned(),
                    });
                }
            }
            continue;
        }

        if let Some(attribute_body) = trimmed_line.strip_prefix("@returns") {
            let return_value = attribute_body
                .trim()
                .trim_start_matches(':')
                .trim()
                .to_owned();
            if !return_value.is_empty() {
                parsed.returns = Some(DocReturn {
                    description: return_value,
                });
            }
            continue;
        }

        if let Some(attribute_body) = trimmed_line.strip_prefix("@example") {
            let example_code = attribute_body
                .trim()
                .trim_start_matches(':')
                .trim()
                .to_owned();
            if !example_code.is_empty() {
                parsed.examples.push(DocExample { code: example_code });
            }
            continue;
        }

        if let Some((key, value)) = trimmed_line.split_once(':') {
            if key.trim().eq_ignore_ascii_case("description") {
                description_lines.push(value.trim().to_owned());
                continue;
            }
        }

        description_lines.push(trimmed_line.to_owned());
    }

    if !description_lines.is_empty() {
        parsed.description = Some(description_lines.join(" "));
    }

    parsed
}
