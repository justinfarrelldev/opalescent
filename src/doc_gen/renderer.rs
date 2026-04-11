//! Documentation renderer for Markdown and HTML output.

extern crate alloc;

use crate::doc_gen::cross_refs::{build_cross_reference_index, link_text};
use crate::doc_gen::extractor::{ApiDocSymbol, ApiSymbolKind};
use alloc::string::String;
use alloc::vec::Vec;

/// Output format for rendered documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    /// Render documentation as Markdown.
    Markdown,
    /// Render documentation as minimal HTML.
    Html,
}

/// Render documentation with the selected output format.
#[must_use]
pub fn render_documentation(symbols: &[ApiDocSymbol], format: RenderFormat) -> String {
    match format {
        RenderFormat::Markdown => render_markdown(symbols),
        RenderFormat::Html => render_html(symbols),
    }
}

/// Render extracted symbols into Markdown.
#[must_use]
pub fn render_markdown(symbols: &[ApiDocSymbol]) -> String {
    let symbol_names = symbols
        .iter()
        .map(|symbol| symbol.name.clone())
        .collect::<Vec<String>>();
    let cross_reference_index = build_cross_reference_index(symbol_names.as_slice());

    let mut output = String::new();
    output.push_str("# API Documentation\n\n");

    for symbol in symbols {
        output.push_str("## `");
        output.push_str(symbol.name.as_str());
        output.push_str("`\n\n");
        output.push_str("- Kind: ");
        output.push_str(symbol_kind_label(symbol.kind));
        output.push('\n');
        output.push_str("- Signature: `");
        output.push_str(symbol.signature.as_str());
        output.push_str("`\n\n");

        if let Some(description) = symbol.description.as_ref() {
            output.push_str(link_text(description, &cross_reference_index).as_str());
            output.push_str("\n\n");
        }

        if !symbol.attributes.params.is_empty() {
            output.push_str("**Parameters:**\n");
            for parameter in &symbol.attributes.params {
                output.push_str("- `");
                output.push_str(parameter.name.as_str());
                output.push_str("`: ");
                output.push_str(
                    link_text(parameter.description.as_str(), &cross_reference_index).as_str(),
                );
                output.push('\n');
            }
            output.push('\n');
        }

        if let Some(returns) = symbol.attributes.returns.as_ref() {
            output.push_str("**Returns:** ");
            output
                .push_str(link_text(returns.description.as_str(), &cross_reference_index).as_str());
            output.push_str("\n\n");
        }

        if !symbol.attributes.examples.is_empty() {
            output.push_str("**Example:**\n\n");
            for example in &symbol.attributes.examples {
                output.push_str("```opalescent\n");
                output.push_str(example.code.as_str());
                output.push_str("\n```\n\n");
            }
        }
    }

    output
}

/// Render extracted symbols into minimal HTML output.
#[must_use]
fn render_html(symbols: &[ApiDocSymbol]) -> String {
    let mut output = String::new();
    output.push_str("<h1>API Documentation</h1>\n");
    for symbol in symbols {
        output.push_str("<h2 id=\"");
        output.push_str(symbol.name.as_str());
        output.push_str("\"><code>");
        output.push_str(symbol.name.as_str());
        output.push_str("</code></h2>\n");
        output.push_str("<p><strong>Kind:</strong> ");
        output.push_str(symbol_kind_label(symbol.kind));
        output.push_str("</p>\n");
        output.push_str("<p><strong>Signature:</strong> <code>");
        output.push_str(symbol.signature.as_str());
        output.push_str("</code></p>\n");
    }
    output
}

/// Human-readable label for an API symbol kind.
#[must_use]
const fn symbol_kind_label(kind: ApiSymbolKind) -> &'static str {
    match kind {
        ApiSymbolKind::Function => "function",
        ApiSymbolKind::Type => "type",
        ApiSymbolKind::Let => "let",
    }
}
