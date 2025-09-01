pub mod error;
pub mod lexer;
pub mod token;

use lexer::Lexer;
use std::fs;

fn main() {
    println!("Opalescent Lexer Test");

    // Test with hello_world.op
    match fs::read_to_string("language-spec/hello_world.op") {
        Ok(content) => {
            println!("\n=== Tokenizing hello_world.op ===");
            let lexer = Lexer::new(&content);
            let (tokens, errors) = lexer.tokenize();

            if !errors.is_empty() {
                println!("Errors found:");
                for error in &errors.errors {
                    println!("  {error}");
                }
            }

            println!("\nTokens:");
            for (i, token) in tokens.iter().enumerate() {
                println!("  {i}: {}", token.token_type);
            }
        }
        Err(e) => {
            println!("Failed to read hello_world.op: {e}");
        }
    }
}
