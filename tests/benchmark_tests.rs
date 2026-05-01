//! Benchmark tests for the Once language compiler
//! 
//! These tests provide baseline performance benchmarks for comparison
//! and regression testing over time.

use once_lex::{Lexer, TokenWithSpan};
use once_parse::{OnceParser, Program};
use std::time::Instant;

/// Benchmark: Simple function parsing
#[test]
fn benchmark_simple_function() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Simple function parse: {:?}", duration);
    assert!(duration.as_nanos() < 1_000_000, "Should parse in under 1ms");
}

/// Benchmark: Multiple functions parsing
#[test]
fn benchmark_multiple_functions() {
    let input = (0..50)
        .map(|i| format!("fn f_{}(x: Int) -> Int {{ x + {} }}", i, i))
        .collect::<Vec<_>>()
        .join("\n");
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("50 functions parse: {:?}", duration);
    assert!(duration.as_millis() < 100, "Should parse in under 100ms");
}

/// Benchmark: Complex expression parsing
#[test]
fn benchmark_complex_expression() {
    // ((1 + 2) * 3 - 4) / 5 + 6 * 7 - 8
    let input = "fn test(x: Int) -> Int { ((x + 2) * 3 - 4) / 5 + 6 * 7 - 8 }";
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Complex expression parse: {:?}", duration);
    assert!(duration.as_nanos() < 1_000_000, "Should parse in under 1ms");
}

/// Benchmark: Type-rich function parsing
#[test]
fn benchmark_type_rich_function() {
    let input = "fn process(data: Vec<Int>, opts: Options) -> Result<Int, Error> { Ok(0) }";
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Type-rich function parse: {:?}", duration);
    assert!(duration.as_nanos() < 1_000_000, "Should parse in under 1ms");
}

/// Benchmark: Large string literal
#[test]
fn benchmark_large_string() {
    let content = "x".repeat(10000);
    let input = format!("fn main() -> Unit {{ print(\"{}\") }}", content);
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Large string parse: {:?}", duration);
    assert!(duration.as_millis() < 100, "Should parse in under 100ms");
}

/// Benchmark: Many let bindings
#[test]
fn benchmark_many_let_bindings() {
    let input = (0..100)
        .map(|i| format!("let x_{}: Int = {};", i, i))
        .collect::<Vec<_>>()
        .join("\n");
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(&input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("100 let bindings parse: {:?}", duration);
    assert!(duration.as_millis() < 100, "Should parse in under 100ms");
}

/// Benchmark: Match expression
#[test]
fn benchmark_match_expression() {
    let input = "fn test(x: Int) -> Int { match x { 1 => 10, 2 => 20, 3 => 30, _ => 0 } }";
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Match expression parse: {:?}", duration);
    assert!(duration.as_nanos() < 1_000_000, "Should parse in under 1ms");
}

/// Benchmark: Nested blocks
#[test]
fn benchmark_nested_blocks() {
    let input = "fn main() -> Unit { { { { let x = 1; } } } }";
    
    let tokens: Vec<TokenWithSpan> = Lexer::new(input).collect();
    let start = Instant::now();
    let _ast = OnceParser::parse(tokens).expect("Should parse");
    let duration = start.elapsed();
    
    println!("Nested blocks parse: {:?}", duration);
    assert!(duration.as_nanos() < 1_000_000, "Should parse in under 1ms");
}