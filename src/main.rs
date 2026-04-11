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
mod lexer;
mod parser;
#[path = "runtime.rs"]
pub mod runtime;
#[path = "hot_reload.rs"]
pub mod hot_reload;
mod token;
mod type_system;

use lexer::Lexer;
use parser::Parser;
use std::fs;

fn main() {
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
                for error in &lex_errors.errors {
                    println!("  {error}");
                }
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
                for error in &parse_errors.errors {
                    println!("  {error}");
                }
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
