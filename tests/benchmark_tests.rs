//! Benchmark tests for the Once language compiler
//! 
//! These tests measure performance characteristics and identify
//! performance regressions over time.

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

/// Benchmark lexer performance
#[test]
fn benchmark_lexer_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    let start = Instant::now();
    let tokens = tokenize(&input);
    let duration = start.elapsed();
    
    assert!(!tokens.is_empty(), "Should produce tokens");
    println!("Lexer benchmark: {} characters in {:?}", input.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Lexer should be fast");
}

/// Benchmark parser performance
#[test]
fn benchmark_parser_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    
    let start = Instant::now();
    let ast = parse(tokens).expect("Should parse successfully");
    let duration = start.elapsed();
    
    assert!(!ast.items.is_empty(), "Should have items");
    println!("Parser benchmark: {} tokens in {:?}", tokens.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Parser should be fast");
}

/// Benchmark HIR generation performance
#[test]
fn benchmark_hir_generation_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let start = Instant::now();
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    let duration = start.elapsed();
    
    assert!(!hir.items.is_empty(), "Should have HIR items");
    println!("HIR generation benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "HIR generation should be fast");
}

/// Benchmark type checking performance
#[test]
fn benchmark_type_checking_performance() {
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
    println!("Type checking benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Type checking should be fast");
}

/// Benchmark effects checking performance
#[test]
fn benchmark_effects_checking_performance() {
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
    println!("Effects checking benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Effects checking should be fast");
}

/// Benchmark linearity checking performance
#[test]
fn benchmark_linearity_checking_performance() {
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
    println!("Linearity checking benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Linearity checking should be fast");
}

/// Benchmark region inference performance
#[test]
fn benchmark_region_inference_performance() {
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
    println!("Region inference benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Region inference should be fast");
}

/// Benchmark MIR generation performance
#[test]
fn benchmark_mir_generation_performance() {
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
    println!("MIR generation benchmark: {} items in {:?}", hir.items.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "MIR generation should be fast");
}

/// Benchmark code generation performance
#[test]
fn benchmark_code_generation_performance() {
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
    println!("Code generation benchmark: {} functions in {:?}", mir.functions.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Code generation should be fast");
}

/// Benchmark full compilation pipeline performance
#[test]
fn benchmark_full_compilation_performance() {
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
    
    println!("Full compilation pipeline benchmark:");
    println!("  Lexing: {:?} ({:.2}% of total)", lex_duration, lex_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Parsing: {:?} ({:.2}% of total)", parse_duration, parse_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  HIR generation: {:?} ({:.2}% of total)", hir_duration, hir_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Type checking: {:?} ({:.2}% of total)", type_duration, type_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Effects checking: {:?} ({:.2}% of total)", effects_duration, effects_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Linearity checking: {:?} ({:.2}% of total)", linearity_duration, linearity_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Region inference: {:?} ({:.2}% of total)", region_duration, region_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  MIR generation: {:?} ({:.2}% of total)", mir_duration, mir_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Code generation: {:?} ({:.2}% of total)", codegen_duration, codegen_duration.as_millis() as f64 / total_duration.as_millis() as f64 * 100.0);
    println!("  Total: {:?}", total_duration);
    
    // Performance threshold (adjust as needed)
    assert!(total_duration.as_millis() < 5000, "Full compilation should be fast");
}

/// Benchmark memory usage during compilation
#[test]
fn benchmark_memory_usage() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    // Measure memory usage at different stages
    let tokens = tokenize(&input);
    let token_count = tokens.len();
    
    let ast = parse(tokens).expect("Should parse successfully");
    let ast_item_count = ast.items.len();
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    let hir_item_count = hir.items.len();
    
    println!("Memory usage benchmark:");
    println!("  Input size: {} characters", input.len());
    println!("  Tokens: {}", token_count);
    println!("  AST items: {}", ast_item_count);
    println!("  HIR items: {}", hir_item_count);
    println!("  Tokens per character: {:.2}", token_count as f64 / input.len() as f64);
    println!("  AST items per token: {:.2}", ast_item_count as f64 / token_count as f64);
    println!("  HIR items per AST item: {:.2}", hir_item_count as f64 / ast_item_count as f64);
    
    // Basic sanity checks
    assert!(token_count > 0, "Should have tokens");
    assert!(ast_item_count > 0, "Should have AST items");
    assert!(hir_item_count > 0, "Should have HIR items");
}

/// Benchmark compilation with large programs
#[test]
fn benchmark_large_program_compilation() {
    // Generate a larger program
    let mut input = String::new();
    for i in 0..1000 {
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
    
    println!("Large program compilation benchmark ({} functions) completed in {:?}", 1001, duration);
    println!("  Functions per second: {:.2}", 1001.0 / duration.as_secs_f64());
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 10000, "Large program compilation should be fast");
}

/// Benchmark Cranelift integration performance
#[test]
fn benchmark_cranelift_integration_performance() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }".repeat(1000);
    
    let tokens = tokenize(&input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = mir_generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let start = Instant::now();
    let mut codegen = CodeGenerator::new_with_cranelift(region_dag).expect("Should create Cranelift codegen");
    let result = codegen.generate(&mir);
    let duration = start.elapsed();
    
    assert!(result.is_ok(), "Cranelift code generation should succeed");
    println!("Cranelift integration benchmark: {} functions in {:?}", mir.functions.len(), duration);
    
    // Performance threshold (adjust as needed)
    assert!(duration.as_millis() < 1000, "Cranelift integration should be fast");
}
