//! Cross-reference indexing and linking for generated documentation.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;

/// Map of symbol name to anchor path used by rendered docs.
pub type CrossReferenceIndex = BTreeMap<String, String>;

/// Build a cross-reference index from symbol names.
#[must_use]
pub fn build_cross_reference_index(symbol_names: &[String]) -> CrossReferenceIndex {
    let mut index = BTreeMap::new();
    for name in symbol_names {
        index.insert(name.clone(), format!("#{name}"));
    }
    index
}

/// Replace known type names in text with Markdown links.
#[must_use]
pub fn link_text(text: &str, index: &CrossReferenceIndex) -> String {
    let mut output = String::new();
    let mut token = String::new();

    for character in text.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            token.push(character);
            continue;
        }

        flush_token(&mut output, &mut token, index);
        output.push(character);
    }

    flush_token(&mut output, &mut token, index);
    output
}

/// Flush buffered identifier token into output with optional link.
fn flush_token(output: &mut String, token: &mut String, index: &CrossReferenceIndex) {
    if token.is_empty() {
        return;
    }

    if let Some(anchor) = index.get(token) {
        output.push('[');
        output.push_str(token);
        output.push_str("](");
        output.push_str(anchor);
        output.push(')');
    } else {
        output.push_str(token);
    }

    token.clear();
}
