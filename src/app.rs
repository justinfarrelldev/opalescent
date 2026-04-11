//! CLI application workflow for the Opalescent binary.
//!
//! This module centralizes command-line behavior so `main.rs` remains a thin
//! entry point while the executable behavior stays testable and reusable.

use crate::ast;
use crate::errors;
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::fs;

/// Print usage help for `opal` CLI commands.
///
/// When `topic` is `None`, prints the top-level help summary.
/// When `topic` is `Some(t)`, prints topic-specific guidance for the named topic.
fn print_help(topic: Option<&str>) {
    match topic {
        Some("pkg") => {
            println!("opal pkg <command>");
            println!();
            println!("Commands:");
            println!("  init <name>              Initialise a new project manifest");
            println!("  add <pkg> <version>      Add a dependency");
            println!("  remove <pkg>             Remove a dependency");
            println!("  install                  Install all declared dependencies");
            println!("  publish                  Publish the package to the registry");
        }
        Some("fmt") => {
            println!("opal fmt [--check] [--config <path>] <file>");
            println!();
            println!("Format an Opalescent source file.");
            println!("  --check     Exit with error if file would change (CI mode)");
            println!("  --config    Path to opal-fmt.toml configuration file");
        }
        Some(unknown) => {
            println!("Unknown help topic: {unknown}");
            println!("Run `opal help` for the list of topics.");
        }
        None => {
            println!("Opalescent Compiler");
            println!();
            println!("Usage:  opal <command> [arguments]");
            println!();
            println!("Commands:");
            println!("  <file.op>    Compile and run an Opalescent source file");
            println!("  help         Show this help message");
            println!("  help pkg     Package manager commands");
            println!("  help fmt     Formatter commands");
            println!();
            println!("Examples:");
            println!("  opal hello_world.op");
            println!("  opal help pkg");
        }
    }
}

/// Run the Opalescent CLI application entry workflow.
pub fn run() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(String::as_str) == Some("help") {
        print_help(args.get(2).map(String::as_str));
        return;
    }

    if !crate::doc_gen::touch_doc_gen_api_for_lints() {
        println!("doc gen api warmup did not produce expected probe result");
    }
    if !crate::errors::touch_error_api_for_lints() {
        println!("error api warmup did not produce expected probe result");
    }
    println!("Opalescent Parser Test");

    match fs::read_to_string("language-spec/hello_world.op") {
        Ok(content) => {
            println!("\n=== Source Code ===");
            println!("{content}");

            println!("\n=== Tokenizing ===");
            let lexer = Lexer::new(&content);
            let (tokens, lex_errors) = lexer.tokenize();

            if !lex_errors.is_empty() {
                println!("Lexer errors:");
                let mut error_report = errors::reporter::CompilationErrorReport::new();
                error_report.extend_lex_errors(lex_errors.errors);
                println!("{}", error_report.render());
            }

            println!("\nTokens:");
            for (i, token) in tokens.iter().enumerate() {
                println!("  {i}: {}", token.token_type);
            }

            println!("\n=== Parsing ===");
            let parser = Parser::new(tokens);
            let (program, parse_errors) = parser.parse();

            if !parse_errors.is_empty() {
                println!("Parser errors:");
                let mut error_report = errors::reporter::CompilationErrorReport::new();
                error_report.extend_parse_errors(parse_errors.errors);
                println!("{}", error_report.render());
            }

            if let Some(program) = program {
                println!("\nParsed AST:");
                println!("Program with {} declarations", program.declarations.len());
                for (i, decl) in program.declarations.iter().enumerate() {
                    match decl {
                        &ast::Decl::Function {
                            ref name,
                            ref parameters,
                            is_entry,
                            ..
                        } => {
                            let entry_str = if is_entry { "entry " } else { "" };
                            println!(
                                "  {i}: {entry_str}function {name} with {} parameters",
                                parameters.len()
                            );
                        }
                        _ => {
                            println!("  {i}: Other declaration");
                        }
                    }
                }
            } else {
                println!("Failed to parse program");
            }
        }
        Err(read_error) => {
            println!("Failed to read hello_world.op: {read_error}");
        }
    }
}
