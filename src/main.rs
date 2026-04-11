//! Opalescent Programming Language Compiler
//!
//! This is the main entry point for the Opalescent compiler.
//! Currently supports lexical analysis and parsing.

#![allow(
    clippy::ref_patterns,
    reason = "Using ref patterns for consistency with other modules"
)]

mod ast;
#[path = "codegen.rs"]
mod codegen;
mod error;
/// Compiler-wide error reporting infrastructure modules.
#[path = "errors.rs"]
mod errors;
#[path = "hot_reload.rs"]
pub mod hot_reload;
mod lexer;
mod parser;
#[path = "runtime.rs"]
pub mod runtime;
mod token;
mod type_system;

use lexer::Lexer;
use parser::Parser;
use std::fs;

fn main() {
    if !errors::touch_error_api_for_lints() {
        println!("error api warmup did not produce expected probe result");
    }
    println!("Opalescent Parser Test");

    // Test with hello_world.op
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
        Err(e) => {
            println!("Failed to read hello_world.op: {e}");
        }
    }
}
