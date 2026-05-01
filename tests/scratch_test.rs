use once_lex::Lexer;
use once_parse::OnceParser;

fn main() {
    let source = "
    fn main() -> Unit {
        let x = 1;
        {
            let y = 2;
            {
                let z = 3;
            }
        }
    }";
    let tokens: Vec<_> = Lexer::new(source).collect();
    match OnceParser::parse(tokens) {
        Ok(_) => println!("Success!"),
        Err(e) => println!("Error: {}", e),
    }
}
