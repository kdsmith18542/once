//! Performance tests for the Once language compiler
//! 
//! These tests measure compilation performance and identify bottlenecks
//! to ensure the compiler can handle real-world programs efficiently.

use once_lex::{Lexer, TokenWithSpan};
use once_parse::{OnceParser, Program};
use std::time::Instant;

/// Test lexer performance
#[test]
fn test_lexer_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    let start = Instant::now();
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let duration = start.elapsed();
    
    assert!(!tokens.is_empty(), "Should produce tokens");
    println!("Lexed {} characters in {:?}", input.len(), duration);
    
    // Should complete in reasonable time (adjust threshold as needed)
    assert!(duration.as_millis() < 1000, "Lexing should be fast");
}

/// Test parser performance
#[test]
fn test_parser_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let token_count = tokens.len();
    
    let start = Instant::now();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    assert!(!ast.items.is_empty(), "Should have items");
    println!("Parsed {} tokens in {:?}", token_count, duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Parsing should be fast");
}

/// Test large input lexer performance
#[test]
fn test_lexer_large_input() {
    let input = "let x = 1; let y = 2; let z = 3; ".repeat(500);
    
    let start = Instant::now();
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let duration = start.elapsed();
    
    assert!(!tokens.is_empty(), "Should produce tokens");
    println!("Lexed {} statements in {:?}", 500, duration);
    assert!(duration.as_millis() < 2000, "Lexing large input should be fast");
}

/// Test large input parser performance
#[test]
fn test_parser_large_input() {
    let input = "let x = 1; let y = 2; let z = 3; ".repeat(500);
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    
    let start = Instant::now();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    assert!(!ast.items.is_empty(), "Should have items");
    println!("Parsed 500 statements in {:?}", duration);
    assert!(duration.as_millis() < 2000, "Parsing large input should be fast");
}

/// Test nested expression performance
#[test]
fn test_nested_expression_parsing() {
    // Create deeply nested expression: ((((...1 + 1...))) at 40 levels
    let mut expr = "1".to_string();
    for _ in 0..40 {
        expr = format!("({} + 1)", expr);
    }
    let input = format!("fn main() -> Int {{ {} }}", expr);
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    
    let start = Instant::now();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    println!("Parsed 40 nested additions in {:?}", duration);
    assert!(duration.as_millis() < 2000, "Nested expression parsing should be fast");
}

/// Test many functions parsing performance
#[test]
fn test_many_functions_parsing() {
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&format!("fn f_{}() -> Int {{ {} }}\n", i, i));
    }
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    
    let start = Instant::now();
    let ast = OnceParser::parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    assert!(ast.items.len() == 100, "Should have 100 functions");
    println!("Parsed 100 functions in {:?}", duration);
    assert!(duration.as_millis() < 2000, "Many functions parsing should be fast");
}