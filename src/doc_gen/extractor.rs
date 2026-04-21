//! AST extraction for documentation generation.

extern crate alloc;

use crate::ast::{Decl, Documentation, Program, TypeDef, Visibility};
use crate::doc_gen::attributes::{
    DocExample, DocParam, DocReturn, ParsedDocAttributes, parse_doc_attributes,
};
use alloc::string::String;
use alloc::vec::Vec;

/// Public API symbol kind used in generated docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiSymbolKind {
    /// Function declaration.
    Function,
    /// Type declaration.
    Type,
    /// Top-level let declaration.
    Let,
}

/// Extracted documentation-ready representation of a public symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiDocSymbol {
    /// Symbol name.
    pub name: String,
    /// Symbol category.
    pub kind: ApiSymbolKind,
    /// Canonical signature string.
    pub signature: String,
    /// Raw documentation text.
    pub raw_doc: String,
    /// Optional free-form description.
    pub description: Option<String>,
    /// Parsed structured attributes.
    pub attributes: ParsedDocAttributes,
}

/// Extract public API documentation symbols from the AST.
#[must_use]
pub fn extract_public_api_docs(program: &Program) -> Vec<ApiDocSymbol> {
    let mut symbols = Vec::new();

    for declaration in &program.declarations {
        match *declaration {
            Decl::Function {
                ref name,
                ref parameters,
                ref return_types,
                ref error_types,
                ref visibility,
                ref doc_comment,
                ..
            } if *visibility == Visibility::Public => {
                let signature =
                    function_signature(name, parameters, return_types.as_deref(), error_types);
                symbols.push(symbol_from_docs(
                    name,
                    ApiSymbolKind::Function,
                    signature,
                    doc_comment.as_ref(),
                ));
            }
            Decl::Type {
                ref name,
                ref type_def,
                ref visibility,
                ref doc_comment,
                ..
            } if *visibility == Visibility::Public => {
                let signature = type_signature(name, type_def);
                symbols.push(symbol_from_docs(
                    name,
                    ApiSymbolKind::Type,
                    signature,
                    doc_comment.as_ref(),
                ));
            }
            Decl::Let {
                ref binding,
                ref initializer,
                ref visibility,
                ref doc_comment,
                ..
            } if *visibility == Visibility::Public => {
                let signature = binding.type_annotation.as_ref().map_or_else(
                    || format!("{} = {:?}", binding.name, initializer),
                    |type_annotation| {
                        format!(
                            "{}: {}",
                            binding.name,
                            type_annotation.to_signature_string()
                        )
                    },
                );
                symbols.push(symbol_from_docs(
                    &binding.name,
                    ApiSymbolKind::Let,
                    signature,
                    doc_comment.as_ref(),
                ));
            }
            _ => {}
        }
    }

    symbols.sort_by(|left_symbol, right_symbol| left_symbol.name.cmp(&right_symbol.name));
    symbols
}

/// Build a documentation symbol from declaration-level metadata.
fn symbol_from_docs(
    name: &str,
    kind: ApiSymbolKind,
    signature: String,
    documentation: Option<&Documentation>,
) -> ApiDocSymbol {
    let raw_doc = documentation.map_or_else(String::new, |docs| docs.raw.clone());
    let mut attributes = parse_doc_attributes(raw_doc.as_str());

    if let Some(docs) = documentation {
        if attributes.description.is_none() {
            attributes.description = docs.sections.get("Description").cloned();
        }

        for (key, value) in &docs.attributes {
            if key == "param" {
                if let Some((name_part, description_part)) = value.split_once(':') {
                    let param_name = name_part.trim();
                    if !param_name.is_empty() {
                        attributes.params.push(DocParam {
                            name: param_name.to_owned(),
                            description: description_part.trim().to_owned(),
                        });
                    }
                }
                continue;
            }

            if key == "returns" {
                let return_description = value.trim();
                if !return_description.is_empty() {
                    attributes.returns = Some(DocReturn {
                        description: return_description.to_owned(),
                    });
                }
                continue;
            }

            if key == "example" {
                let example_code = value.trim();
                if !example_code.is_empty() {
                    attributes.examples.push(DocExample {
                        code: example_code.to_owned(),
                    });
                }
            }
        }
    }

    let description = attributes.description.clone();

    ApiDocSymbol {
        name: name.to_owned(),
        kind,
        signature,
        raw_doc,
        description,
        attributes,
    }
}

/// Render function signature text used by generated docs.
fn function_signature(
    name: &str,
    parameters: &[crate::ast::Parameter],
    return_types: Option<&[crate::ast::Type]>,
    error_types: &[String],
) -> String {
    let mut signature = format!("{name} = f(");
    for (index, parameter) in parameters.iter().enumerate() {
        if index > 0 {
            signature.push_str(", ");
        }
        signature.push_str(parameter.name.as_str());
        signature.push_str(": ");
        signature.push_str(&parameter.param_type.to_signature_string());
    }
    signature.push(')');

    if let Some(return_type_list) = return_types {
        signature.push_str(": ");
        for (index, return_type) in return_type_list.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            signature.push_str(&return_type.to_signature_string());
        }
    }

    if !error_types.is_empty() {
        signature.push_str(" errors ");
        for (index, error_type) in error_types.iter().enumerate() {
            if index > 0 {
                signature.push_str(", ");
            }
            signature.push_str(error_type);
        }
    }

    signature
}

/// Render type declaration signature text used by generated docs.
fn type_signature(name: &str, type_def: &TypeDef) -> String {
    match *type_def {
        TypeDef::Sum { ref variants, .. } => {
            let mut signature = format!("type {name}: ");
            for (index, variant) in variants.iter().enumerate() {
                if index > 0 {
                    signature.push_str(" | ");
                }
                signature.push_str(&variant.name);
            }
            signature
        }
        TypeDef::Product { ref fields, .. } => {
            let mut signature = format!("type {name}: ");
            for (index, field) in fields.iter().enumerate() {
                if index > 0 {
                    signature.push_str(", ");
                }
                signature.push_str(&field.name);
                signature.push_str(": ");
                signature.push_str(&field.type_annotation.to_signature_string());
            }
            signature
        }
        TypeDef::Alias {
            ref target_type, ..
        } => {
            format!("type {name}: {}", target_type.to_signature_string())
        }
    }
}
