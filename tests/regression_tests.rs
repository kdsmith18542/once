//! Regression tests for the Once language compiler
//! 
//! These tests ensure that previously fixed bugs don't regress
//! and that the compiler maintains consistent behavior over time.

use once_lex::{Lexer, Token, TokenWithSpan};
use once_parse::{OnceParser, Program, Item, FnDecl, Type, Expr, Stmt, Block};

/// Test lexer basic tokens
#[test]
fn test_lexer_basic_tokens() {
    let source = "fn main() -> Unit { return }";
    let mut lexer = Lexer::new(source);
    
    let tokens: Vec<_> = lexer.collect();
    assert!(tokens.len() >= 3, "Should produce tokens");
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Fn)), "Should contain fn keyword");
}

/// Test parser with valid input
#[test]
fn test_parser_valid_input() {
    let input = "fn main() -> Unit { return }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok(), "Should parse successfully");
    let program = result.unwrap();
    assert!(!program.items.is_empty(), "Should have items");
}

/// Test parser rejects invalid input
#[test]
fn test_parser_rejects_invalid() {
    let input = "fn main() -> Unit { print("; // Missing closing
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_err(), "Should fail to parse invalid input");
}

/// Test string literal token
#[test]
fn test_string_literal_token() {
    let source = r#""hello""#;
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 1);
    match &tokens[0].token {
        Token::StringLit(s) => assert_eq!(s, "hello"),
        _ => panic!("Expected StringLit"),
    }
}

/// Test integer literal token  
#[test]
fn test_integer_literal_token() {
    let source = "42";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 1);
    match &tokens[0].token {
        Token::IntLit(n) => assert_eq!(*n, 42),
        _ => panic!("Expected IntLit"),
    }
}

/// Test float literal token
#[test]
fn test_float_literal_token() {
    let source = "3.14";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 1);
    match &tokens[0].token {
        Token::FloatLit(f) => assert!((*f - 3.14).abs() < 1e-10),
        _ => panic!("Expected FloatLit"),
    }
}

/// Test boolean literal tokens
#[test]
fn test_boolean_literal_tokens() {
    let source = "true false";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 2);
    assert!(matches!(tokens[0].token, Token::True));
    assert!(matches!(tokens[1].token, Token::False));
}

/// Test identifier tokens
#[test]
fn test_identifier_tokens() {
    let source = "foo bar baz";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 3);
    assert!(matches!(&tokens[0].token, Token::Ident(x) if x == "foo"));
    assert!(matches!(&tokens[1].token, Token::Ident(x) if x == "bar"));
    assert!(matches!(&tokens[2].token, Token::Ident(x) if x == "baz"));
}

/// Test operator tokens
#[test]
fn test_operator_tokens() {
    let source = "+ - * /";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 4);
    assert!(matches!(tokens[0].token, Token::Plus));
    assert!(matches!(tokens[1].token, Token::Minus));
    assert!(matches!(tokens[2].token, Token::Star));
    assert!(matches!(tokens[3].token, Token::Slash));
}

/// Test comparison operator tokens
#[test]
fn test_comparison_tokens() {
    let source = "== != < > <= >=";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert!(tokens.len() >= 4);
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::EqEq)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Ne)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Lt)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Gt)));
}

/// Test keyword tokens
#[test]
fn test_keyword_tokens() {
    let source = "fn let var return if else for match spawn async await";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Fn)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Let)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Var)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Return)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::If)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Else)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::For)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Match)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Spawn)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Async)));
    assert!(tokens.iter().any(|t| matches!(&t.token, Token::Await)));
}

/// Test type keywords
#[test]
fn test_type_keywords() {
    let source = "Int Float Bool Str Unit Option Result Vec";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Int)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Float)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Bool)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Str)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Unit)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Option)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Result)));
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Vec)));
}

/// Test punctuation tokens
#[test]
fn test_punctuation_tokens() {
    let source = "() {} [] , : -> ;";
    let tokens: Vec<TokenWithSpan> = Lexer::new(source).collect();
    assert_eq!(tokens.len(), 10);
    assert!(matches!(tokens[0].token, Token::LParen));
    assert!(matches!(tokens[1].token, Token::RParen));
    assert!(matches!(tokens[2].token, Token::LBrace));
    assert!(matches!(tokens[3].token, Token::RBrace));
    assert!(matches!(tokens[4].token, Token::LBracket));
    assert!(matches!(tokens[5].token, Token::RBracket));
}

/// Test parse simple function
#[test]
fn test_parse_simple_function() {
    let input = "fn main() -> Unit { }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
    if let Item::FnDecl(fd) = &program.items[0] {
        assert_eq!(fd.name, "main");
    } else {
        panic!("Expected FnDecl");
    }
}

/// Test parse function with params
#[test]
fn test_parse_function_with_params() {
    let input = "fn add(x: Int, y: Int) -> Int { x + y }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
}

/// Test parse let statement
#[test]
fn test_parse_let_statement() {
    let input = "let x: Int = 42;";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse if expression
#[test]
fn test_parse_if_expression() {
    let input = "fn main() -> Int { if x > 0 { 1 } else { 0 } }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse match expression
#[test]
fn test_parse_match_expression() {
    let input = "fn main() -> Int { match x { 1 => 10, _ => 0 } }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse nested expressions
#[test]
fn test_parse_nested_expressions() {
    let input = "fn test() -> Int { 1 + 2 * 3 }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse array type
#[test]
fn test_parse_array_type() {
    let input = "fn test(arr: [Int; 5]) -> Int { arr[0] }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse generic type
#[test]
fn test_parse_generic_type() {
    let input = "fn test(opt: Option<Int>) -> Int { 0 }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse tuple type
#[test]
fn test_parse_tuple_type() {
    let input = "fn test(t: (Int, Str)) -> Int { 0 }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse ADT type
#[test]
fn test_parse_adt_type() {
    let input = "type Result<T, E> = Ok(T) | Err(E)";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test pipeline operator
#[test]
fn test_pipeline_operator() {
    let input = "fn test(x: Int) -> Int { x |> add(1) }";
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}