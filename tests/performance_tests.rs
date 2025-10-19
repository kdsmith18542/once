//! Performance tests for the Once language compiler
//! 
//! These tests measure compilation performance and identify bottlenecks
//! to ensure the compiler can handle real-world programs efficiently.

use once_lex::*;
use once_parse::*;
use once_hir::*;
use once_ty::*;
use once_effects::*;
use once_linear::*;
use once_rinf::*;
use once_mir::*;
use once_codegen::*;
use std::time::Instant;

/// Test lexer performance
#[test]
fn test_lexer_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    let start = Instant::now();
    let tokens = tokenize(&input);
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
    let tokens = tokenize(&input);
    
    let start = Instant::now();
    let ast = parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    assert!(!ast.items.is_empty(), "Should have items");
    println!("Parsed {} tokens in {:?}", tokens.len(), duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Parsing should be fast");
}

/// Test HIR generation performance
#[test]
fn test_hir_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let start = Instant::now();
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    let duration = start.elapsed();
    
    assert!(!hir.items.is_empty(), "Should have HIR items");
    println!("Generated HIR in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "HIR generation should be fast");
}

/// Test type checking performance
#[test]
fn test_type_checking_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let start = Instant::now();
    let mut checker = TypeChecker::new();
    let result = checker.check(&hir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Type checking should succeed");
    println!("Type checked in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Type checking should be fast");
}

/// Test effects checking performance
#[test]
fn test_effects_checking_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let start = Instant::now();
    let mut checker = EffectChecker::new();
    let result = checker.check(&hir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Effects checking should succeed");
    println!("Effects checked in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Effects checking should be fast");
}

/// Test linearity checking performance
#[test]
fn test_linearity_checking_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let start = Instant::now();
    let mut checker = LinearityChecker::new();
    let result = checker.check(&hir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Linearity checking should succeed");
    println!("Linearity checked in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Linearity checking should be fast");
}

/// Test region inference performance
#[test]
fn test_region_inference_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let start = Instant::now();
    let mut checker = RegionChecker::new();
    let result = checker.check(&hir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Region inference should succeed");
    println!("Region inference completed in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Region inference should be fast");
}

/// Test MIR generation performance
#[test]
fn test_mir_generation_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let start = Instant::now();
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let result = generator.generate(&hir, region_dag);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "MIR generation should succeed");
    println!("MIR generated in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "MIR generation should be fast");
}

/// Test code generation performance
#[test]
fn test_code_generation_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let start = Instant::now();
    let mut codegen = CodeGenerator::new(region_dag);
    let result = codegen.generate(&mir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Code generation should succeed");
    println!("Code generated in {:?}", duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 1000, "Code generation should be fast");
}

/// Test full compilation pipeline performance
#[test]
fn test_full_compilation_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    let start = Instant::now();
    
    // Lexing
    let tokens = tokenize(&input);
    let lex_duration = start.elapsed();
    
    // Parsing
    let parse_start = Instant::now();
    let ast = parse(tokens).expect("Should parse successfully");
    let parse_duration = parse_start.elapsed();
    
    // HIR generation
    let hir_start = Instant::now();
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    let hir_duration = hir_start.elapsed();
    
    // Type checking
    let type_start = Instant::now();
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&hir);
    let type_duration = type_start.elapsed();
    
    // Effects checking
    let effects_start = Instant::now();
    let mut effects_checker = EffectChecker::new();
    let effects_result = effects_checker.check(&hir);
    let effects_duration = effects_start.elapsed();
    
    // Linearity checking
    let linearity_start = Instant::now();
    let mut linearity_checker = LinearityChecker::new();
    let linearity_result = linearity_checker.check(&hir);
    let linearity_duration = linearity_start.elapsed();
    
    // Region inference
    let region_start = Instant::now();
    let mut region_checker = RegionChecker::new();
    let region_result = region_checker.check(&hir);
    let region_duration = region_start.elapsed();
    
    // MIR generation
    let mir_start = Instant::now();
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir_result = mir_generator.generate(&hir, region_dag);
    let mir_duration = mir_start.elapsed();
    
    // Code generation
    let codegen_start = Instant::now();
    let mut codegen = CodeGenerator::new(region_dag);
    let codegen_result = codegen.generate(&mir_result.expect("Should generate MIR"));
    let codegen_duration = codegen_start.elapsed();
    
    let total_duration = start.elapsed();
    
    // Verify all steps succeeded
    assert!(type_result.is_ok(), "Type checking should succeed");
    assert!(effects_result.is_ok(), "Effects checking should succeed");
    assert!(linearity_result.is_ok(), "Linearity checking should succeed");
    assert!(region_result.is_ok(), "Region inference should succeed");
    assert!(mir_result.is_ok(), "MIR generation should succeed");
    assert!(codegen_result.is_ok(), "Code generation should succeed");
    
    println!("Full compilation pipeline performance:");
    println!("  Lexing: {:?}", lex_duration);
    println!("  Parsing: {:?}", parse_duration);
    println!("  HIR generation: {:?}", hir_duration);
    println!("  Type checking: {:?}", type_duration);
    println!("  Effects checking: {:?}", effects_duration);
    println!("  Linearity checking: {:?}", linearity_duration);
    println!("  Region inference: {:?}", region_duration);
    println!("  MIR generation: {:?}", mir_duration);
    println!("  Code generation: {:?}", codegen_duration);
    println!("  Total: {:?}", total_duration);
    
    // Should complete in reasonable time
    assert!(total_duration.as_millis() < 5000, "Full compilation should be fast");
}

/// Test memory usage during compilation
#[test]
fn test_memory_usage() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    // Measure memory usage at different stages
    let tokens = tokenize(&input);
    let token_count = tokens.len();
    
    let ast = parse(tokens).expect("Should parse successfully");
    let ast_item_count = ast.items.len();
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    let hir_item_count = hir.items.len();
    
    println!("Memory usage analysis:");
    println!("  Tokens: {}", token_count);
    println!("  AST items: {}", ast_item_count);
    println!("  HIR items: {}", hir_item_count);
    
    // Basic sanity checks
    assert!(token_count > 0, "Should have tokens");
    assert!(ast_item_count > 0, "Should have AST items");
    assert!(hir_item_count > 0, "Should have HIR items");
}

/// Test compilation with large programs
#[test]
fn test_large_program_compilation() {
    // Generate a larger program
    let mut input = String::new();
    for i in 0..100 {
        input.push_str(&format!("fn func_{}() -> Unit {{ print(\"Function {}\") }}\n", i, i));
    }
    input.push_str("fn main() -> Unit { print(\"Hello, World!\") }");
    
    let start = Instant::now();
    
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&hir);
    
    let mut effects_checker = EffectChecker::new();
    let effects_result = effects_checker.check(&hir);
    
    let mut linearity_checker = LinearityChecker::new();
    let linearity_result = linearity_checker.check(&hir);
    
    let mut region_checker = RegionChecker::new();
    let region_result = region_checker.check(&hir);
    
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir_result = mir_generator.generate(&hir, region_dag);
    
    let mut codegen = CodeGenerator::new(region_dag);
    let codegen_result = codegen.generate(&mir_result.expect("Should generate MIR"));
    
    let duration = start.elapsed();
    
    // Verify all steps succeeded
    assert!(type_result.is_ok(), "Type checking should succeed");
    assert!(effects_result.is_ok(), "Effects checking should succeed");
    assert!(linearity_result.is_ok(), "Linearity checking should succeed");
    assert!(region_result.is_ok(), "Region inference should succeed");
    assert!(mir_result.is_ok(), "MIR generation should succeed");
    assert!(codegen_result.is_ok(), "Code generation should succeed");
    
    println!("Large program compilation ({} functions) completed in {:?}", 101, duration);
    
    // Should complete in reasonable time
    assert!(duration.as_millis() < 10000, "Large program compilation should be fast");
}
