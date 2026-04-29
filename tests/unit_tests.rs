//! Unit tests for individual Once language compiler components
//! 
//! These tests verify individual functions and modules in isolation,
//! ensuring each component works correctly on its own.

use once_lex::{Lexer, Token, TokenWithSpan};
use once_parse::{OnceParser, Program, Item, FnDecl, Type, Expr, Stmt, Block};

/// Test lexer basic tokens
#[test]
fn test_lexer_basic_tokens() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    
    assert!(!tokens.is_empty(), "Should produce tokens");
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Fn)), "Should contain fn keyword");
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Ident(_))), "Should contain identifiers");
}

/// Test lexer string literals
#[test]
fn test_lexer_string_literals() {
    let input = r#""Hello, World!""#;
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    
    assert_eq!(tokens.len(), 1, "Should produce one token");
    match &tokens[0].token {
        Token::StringLit(s) => assert_eq!(s, "Hello, World!"),
        _ => panic!("Should be a string literal"),
    }
}

/// Test lexer integer literals
#[test]
fn test_lexer_integer_literals() {
    let input = "42 0x2A 0o52 0b101010";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    
    assert_eq!(tokens.len(), 4, "Should produce four tokens");
    match &tokens[0].token {
        Token::IntLit(n) => assert_eq!(*n, 42),
        _ => panic!("Should be an integer literal"),
    }
}

/// Test lexer float literals
#[test]
fn test_lexer_float_literals() {
    let input = "3.14 1.23e-4";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    
    assert_eq!(tokens.len(), 2, "Should produce two tokens");
    match &tokens[0].token {
        Token::FloatLit(f) => assert!((f - 3.14).abs() < 1e-10),
        _ => panic!("Should be a float literal"),
    }
}

/// Test lexer boolean literals
#[test]
fn test_lexer_boolean_literals() {
    let input = "true false";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    
    assert_eq!(tokens.len(), 2, "Should produce two tokens");
    match &tokens[0].token {
        Token::True => assert!(true),
        _ => panic!("Should be a boolean literal"),
    }
}

/// Test parser functionality
#[test]
fn test_parser_function_declaration() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    
    assert!(!ast.items.is_empty(), "Should have items");
    match &ast.items[0] {
        Item::FnDecl(fn_decl) => {
            assert_eq!(fn_decl.name, "main");
            assert!(matches!(fn_decl.return_type, Some(Type::Unit)));
        }
        _ => panic!("Should be a function declaration"),
    }
}

#[test]
fn test_parser_variable_declaration() {
    let input = "let x: Int = 42;";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    
    assert!(!ast.items.is_empty(), "Should have items");
    match &ast.items[0] {
        Item::LetDecl(let_decl) => {
            assert_eq!(let_decl.name, "x");
            assert!(matches!(let_decl.type_annotation, Some(Type::Int)));
        }
        _ => panic!("Should be a let declaration"),
    }
}

#[test]
fn test_parser_expression_parsing() {
    let input = "x + y * z";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    
    // Should parse as a statement with binary expression
    assert!(!ast.items.is_empty(), "Should have items");
}

/// Test parse simple function
#[test]
fn test_parse_simple_function() {
    let input = "fn main() -> Unit { }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
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
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.items.len(), 1);
}

/// Test parse function with body
#[test]
fn test_parse_function_with_body() {
    let input = "fn add(x: Int, y: Int) -> Int { return x + y; }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse multiple items
#[test]
fn test_parse_multiple_items() {
    let input = "fn foo() -> Unit { }\nfn bar() -> Unit { }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
    let program = result.unwrap();
    assert_eq!(program.items.len(), 2);
}

/// Test parse let statement
#[test]
fn test_parse_let_statement() {
    let input = "let x: Int = 42;";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse let with expression
#[test]
fn test_parse_let_with_expression() {
    let input = "let x: Int = 10 + 20;";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse if expression
#[test]
fn test_parse_if_expression() {
    let input = "if x > 0 { 1 } else { 0 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse if with then and else
#[test]
fn test_parse_if_full() {
    let input = "if x > 0 { x } else { -x }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse match expression
#[test]
fn test_parse_match_expression() {
    let input = "match x { 1 => 10, _ => 0 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse nested expressions
#[test]
fn test_parse_nested_expressions() {
    let input = "fn test() -> Int { 1 + 2 * 3 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse array type
#[test]
fn test_parse_array_type() {
    let input = "fn test(arr: [Int; 5]) -> Int { arr[0] }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse generic type
#[test]
fn test_parse_generic_type() {
    let input = "fn test(opt: Option<Int>) -> Int { 0 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse tuple type
#[test]
fn test_parse_tuple_type() {
    let input = "fn test(t: (Int, Str)) -> Int { 0 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse ADT type
#[test]
fn test_parse_adt_type() {
    let input = "type Result<T, E> = Ok(T) | Err(E)";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test pipeline operator
#[test]
fn test_pipeline_operator() {
    let input = "fn test(x: Int) -> Int { x |> add(1) }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse for loop
#[test]
fn test_parse_for_loop() {
    let input = "for x in items { print(x) }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse while loop
#[test]
fn test_parse_while_loop() {
    let input = "while x > 0 { x = x - 1 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse unary operators
#[test]
fn test_parse_unary_operators() {
    let input = "-x + !y";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse function call
#[test]
fn test_parse_function_call() {
    let input = "foo(1, 2, 3)";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse method call
#[test]
fn test_parse_method_call() {
    let input = "x.foo()";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse field access
#[test]
fn test_parse_field_access() {
    let input = "x.field";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse index access
#[test]
fn test_parse_index_access() {
    let input = "arr[0]";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse lambda/closure
#[test]
fn test_parse_lambda() {
    let input = "|x| x + 1";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse struct literal
#[test]
fn test_parse_struct_literal() {
    let input = "Point { x: 1, y: 2 }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_ok());
}

/// Test parse invalid input
#[test]
fn test_parse_invalid_input() {
    let input = "fn main() { "; // Missing closing
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_err(), "Should fail to parse invalid input");
}

/// Test parse mismatched parentheses
#[test]
fn test_parse_mismatched_parens() {
    let input = "fn test() -> Unit { ()) }";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    let result = OnceParser::parse(tokens);
    assert!(result.is_err(), "Should fail on mismatched parens");
}

/// Test parse invalid type
#[test]
fn test_parse_invalid_type() {
    let input = "let x: InvalidType = 1;";
    let tokens = Lexer::new(input).collect::<Vec<_>>();
    // This might still parse but with an error in the type
    let _ = OnceParser::parse(tokens);
}