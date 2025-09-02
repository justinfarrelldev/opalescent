use crate::lexer::Lexer;

fn main() {
    let input = "'Hello {world}'";
    let lexer = Lexer::new(input);
    let (tokens, errors) = lexer.tokenize();
    
    println!("Input: {}", input);
    println!("Errors: {:?}", errors);
    for (i, token) in tokens.iter().enumerate() {
        println!("Token {}: {:?}", i, token);
    }
}
