mod ast;
mod error;
mod lexer;
mod parser;
mod token;

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
                println!("{program:#?}");
            } else {
                println!("Failed to parse program");
            }
        }
        Err(e) => {
            println!("Failed to read hello_world.op: {e}");
        }
    }
}
