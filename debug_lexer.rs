use once_lex::{Lexer, Token, TokenWithSpan};

fn main() {
    let input = "type Result<T, E> = Ok(T) | Err(E)";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    println!("Input: {}", input);
    println!("Tokens ({}):", tokens.len());
    for t in &tokens {
        println!("  {:?}", t.token);
    }
}