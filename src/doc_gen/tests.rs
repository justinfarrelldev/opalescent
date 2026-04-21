extern crate alloc;

use crate::ast::Decl;
use crate::doc_gen::attributes::parse_doc_attributes;
use crate::doc_gen::cross_refs::{build_cross_reference_index, link_text};
use crate::doc_gen::extractor::{ApiSymbolKind, extract_public_api_docs};
use crate::doc_gen::renderer::{RenderFormat, render_documentation, render_markdown};
use crate::lexer::Lexer;
use crate::parser::Parser;
use alloc::string::String;
use alloc::vec;

fn parse_program(source: &str) -> Option<crate::ast::Program> {
    let lexer = Lexer::new(source);
    let (tokens, lex_errors) = lexer.tokenize();
    assert!(
        lex_errors.is_empty(),
        "lexer should succeed for test source"
    );

    let parser = Parser::new(tokens);
    let (program, parse_errors) = parser.parse();
    assert!(
        parse_errors.is_empty(),
        "parser should succeed for test source"
    );

    program
}

#[test]
fn test_parse_doc_attributes_extracts_param_returns_and_example() {
    let raw = "Description: Converts a UserId into a User.\n@param user_id: UserId value.\n@returns: User value.\n@example create_user(1)";
    let parsed = parse_doc_attributes(raw);

    assert_eq!(
        parsed.description,
        Some(String::from("Converts a UserId into a User."))
    );
    assert_eq!(parsed.params.len(), 1, "expected one @param entry");
    assert_eq!(parsed.params[0].name, "user_id");
    assert_eq!(parsed.params[0].description, "UserId value.");
    assert_eq!(
        parsed
            .returns
            .as_ref()
            .map(|returns| returns.description.as_str()),
        Some("User value.")
    );
    assert_eq!(parsed.examples.len(), 1, "expected one @example entry");
    assert_eq!(parsed.examples[0].code, "create_user(1)");
}

#[test]
fn test_extractor_includes_only_public_symbols() {
    let source = "##\n  Description: Public user type.\n##\npublic type User:\n    Person\n##\n  Description: Build a User from a User id.\n  @param id: User id.\n  @returns: User value.\n##\npublic create_user = f(id: int32): User => {\n    return id\n}\n##\n  Description: Internal helper should not be documented.\n##\nlet helper = f(): void => { return void }";

    let program_option = parse_program(source);
    assert!(
        program_option.is_some(),
        "program should parse successfully"
    );
    let Some(program) = program_option else {
        return;
    };
    let symbols = extract_public_api_docs(&program);

    assert_eq!(symbols.len(), 2, "only public symbols should be extracted");
    assert!(
        symbols
            .iter()
            .any(|symbol| symbol.name == "User" && symbol.kind == ApiSymbolKind::Type),
        "public type should be present"
    );
    assert!(
        symbols.iter().any(|symbol| {
            symbol.name == "create_user" && symbol.kind == ApiSymbolKind::Function
        }),
        "public function should be present"
    );
    assert!(
        !symbols.iter().any(|symbol| symbol.name == "helper"),
        "private symbol should be excluded"
    );
}

#[test]
fn test_cross_reference_linking_rewrites_known_symbols() {
    let names = vec![String::from("User"), String::from("UserId")];
    let index = build_cross_reference_index(names.as_slice());
    let linked = link_text("Create User from UserId safely.", &index);

    assert_eq!(
        linked, "Create [User](#User) from [UserId](#UserId) safely.",
        "known symbol names should be linked"
    );
}

#[test]
fn test_renderer_markdown_contains_api_sections_and_attributes() {
    let source = "##\n  Description: Build a User from a UserId.\n  @param id: UserId input value.\n  @returns: User output value.\n  @example create_user(1)\n##\npublic create_user = f(id: int32): User => {\n    return id\n}\n##\n  Description: Public user type.\n##\npublic type User:\n    Person";

    let program_option = parse_program(source);
    assert!(
        program_option.is_some(),
        "program should parse successfully"
    );
    let Some(program) = program_option else {
        return;
    };
    let symbols = extract_public_api_docs(&program);
    let markdown = render_markdown(symbols.as_slice());

    assert!(
        markdown.contains("# API Documentation"),
        "markdown should start with a heading"
    );
    assert!(
        markdown.contains("## `create_user`"),
        "function section heading should be present"
    );
    assert!(
        markdown.contains("**Parameters:**"),
        "parameter section should be rendered. markdown={markdown}"
    );
    assert!(
        markdown.contains("**Returns:**"),
        "returns section should be rendered"
    );
    assert!(
        markdown.contains("**Example:**"),
        "example section should be rendered"
    );
    assert!(
        markdown.contains("[User](#User)"),
        "cross-reference links should be emitted for known type names"
    );
}

#[test]
fn test_renderer_html_mode_renders_html_headings() {
    let source = "##\n  Description: Public user type.\n##\npublic type User:\n    Person";
    let program_option = parse_program(source);
    assert!(
        program_option.is_some(),
        "program should parse successfully"
    );
    let Some(program) = program_option else {
        return;
    };
    let symbols = extract_public_api_docs(&program);

    let html = render_documentation(symbols.as_slice(), RenderFormat::Html);

    assert!(html.contains("<h1>API Documentation</h1>"));
    assert!(html.contains("<h2 id=\"User\"><code>User</code></h2>"));
}

#[test]
fn test_generate_markdown_for_program_wires_pipeline() {
    let source = "##\n  Description: Public user type.\n##\npublic type User:\n    Person";
    let program_option = parse_program(source);
    assert!(
        program_option.is_some(),
        "program should parse successfully"
    );
    let Some(program) = program_option else {
        return;
    };
    let markdown = crate::doc_gen::generate_markdown_for_program(&program);

    assert!(
        markdown.contains("## `User`"),
        "top-level API generation should include extracted symbol sections"
    );
}

#[test]
fn test_source_contains_documentation_struct_on_declarations() {
    let source = "##\n  Description: Public user type.\n##\npublic type User:\n    Person";
    let program_option = parse_program(source);
    assert!(
        program_option.is_some(),
        "program should parse successfully"
    );
    let Some(program) = program_option else {
        return;
    };

    assert_eq!(program.declarations.len(), 1);
    let has_doc = match &program.declarations[0] {
        &Decl::Type {
            ref doc_comment, ..
        }
        | &Decl::Function {
            ref doc_comment, ..
        }
        | &Decl::Let {
            ref doc_comment, ..
        } => doc_comment.is_some(),
        &Decl::Import { .. } | &Decl::Comment { .. } => false,
    };
    assert!(has_doc, "declaration should preserve parsed doc comments");
}
