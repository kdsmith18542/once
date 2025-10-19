//! Unit tests for individual Once language compiler components
//! 
//! These tests verify individual functions and modules in isolation,
//! ensuring each component works correctly on its own.

use once_lex::*;
use once_parse::*;
use once_hir::*;
use once_ty::*;
use once_effects::*;
use once_linear::*;
use once_rinf::*;
use once_mir::*;
use once_codegen::*;

/// Test lexer functionality
#[test]
fn test_lexer_basic_tokens() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    
    assert!(!tokens.is_empty(), "Should produce tokens");
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Keyword(Keyword::Fn))), "Should contain fn keyword");
    assert!(tokens.iter().any(|t| matches!(t.token, Token::Identifier(_))), "Should contain identifiers");
}

#[test]
fn test_lexer_string_literals() {
    let input = r#""Hello, World!""#;
    let tokens = tokenize(input);
    
    assert_eq!(tokens.len(), 1, "Should produce one token");
    match &tokens[0].token {
        Token::StringLiteral(s) => assert_eq!(s, "Hello, World!"),
        _ => panic!("Should be a string literal"),
    }
}

#[test]
fn test_lexer_integer_literals() {
    let input = "42 0x2A 0o52 0b101010";
    let tokens = tokenize(input);
    
    assert_eq!(tokens.len(), 4, "Should produce four tokens");
    match &tokens[0].token {
        Token::IntegerLiteral(n) => assert_eq!(*n, 42),
        _ => panic!("Should be an integer literal"),
    }
}

#[test]
fn test_lexer_float_literals() {
    let input = "3.14 1.23e-4";
    let tokens = tokenize(input);
    
    assert_eq!(tokens.len(), 2, "Should produce two tokens");
    match &tokens[0].token {
        Token::FloatLiteral(f) => assert!((f - 3.14).abs() < 1e-10),
        _ => panic!("Should be a float literal"),
    }
}

#[test]
fn test_lexer_boolean_literals() {
    let input = "true false";
    let tokens = tokenize(input);
    
    assert_eq!(tokens.len(), 2, "Should produce two tokens");
    match &tokens[0].token {
        Token::BooleanLiteral(b) => assert!(*b),
        _ => panic!("Should be a boolean literal"),
    }
}

/// Test parser functionality
#[test]
fn test_parser_function_declaration() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    assert!(!ast.items.is_empty(), "Should have items");
    match &ast.items[0] {
        AstItem::FnDecl(fn_decl) => {
            assert_eq!(fn_decl.name, "main");
            assert!(matches!(fn_decl.return_type, HirType::Unit));
        }
        _ => panic!("Should be a function declaration"),
    }
}

#[test]
fn test_parser_variable_declaration() {
    let input = "let x: Int = 42;";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    assert!(!ast.items.is_empty(), "Should have items");
    match &ast.items[0] {
        AstItem::LetDecl(let_decl) => {
            assert_eq!(let_decl.name, "x");
            assert!(matches!(let_decl.type_annotation, Some(HirType::Int)));
        }
        _ => panic!("Should be a let declaration"),
    }
}

#[test]
fn test_parser_expression_parsing() {
    let input = "x + y * z";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    // Should parse as a statement with binary expression
    assert!(!ast.items.is_empty(), "Should have items");
}

/// Test HIR generation
#[test]
fn test_hir_function_generation() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    assert!(!hir.items.is_empty(), "Should have HIR items");
    match &hir.items[0] {
        HirItem::FnDecl(fn_decl) => {
            assert_eq!(fn_decl.name, "main");
            assert!(matches!(fn_decl.return_type, HirType::Unit));
        }
        _ => panic!("Should be a function declaration"),
    }
}

#[test]
fn test_hir_expression_generation() {
    let input = "let x: Int = 42 + 8;";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    assert!(!hir.items.is_empty(), "Should have HIR items");
}

/// Test type system
#[test]
fn test_type_inference() {
    let mut checker = TypeChecker::new();
    
    // Test basic type inference
    let input = "let x = 42;";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Type checking should succeed");
}

#[test]
fn test_type_constraints() {
    let mut checker = TypeChecker::new();
    
    // Test type constraints
    let input = "fn add(x: Int, y: Int) -> Int { x + y }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Type checking should succeed");
}

#[test]
fn test_type_errors() {
    let mut checker = TypeChecker::new();
    
    // Test type errors
    let input = "let x: Int = \"Hello\";";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_err(), "Type checking should fail");
}

/// Test effects system
#[test]
fn test_effects_checking() {
    let mut checker = EffectChecker::new();
    
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Effects checking should succeed");
}

#[test]
fn test_effects_propagation() {
    let mut checker = EffectChecker::new();
    
    let input = "fn read_file(path: Str) -> Str !io { \"content\" }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Effects checking should succeed");
}

/// Test linearity system
#[test]
fn test_linearity_checking() {
    let mut checker = LinearityChecker::new();
    
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Linearity checking should succeed");
}

#[test]
fn test_linearity_errors() {
    let mut checker = LinearityChecker::new();
    
    // Test linearity errors (simplified)
    let input = "fn main() -> Unit { let x = 42; print(x); print(x) }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    // This should succeed for non-linear values
    assert!(result.is_ok(), "Linearity checking should succeed for non-linear values");
}

/// Test region inference
#[test]
fn test_region_inference() {
    let mut checker = RegionChecker::new();
    
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Region inference should succeed");
}

#[test]
fn test_region_analysis() {
    let mut checker = RegionChecker::new();
    
    let input = "fn create_string() -> Str { \"Hello, World!\" }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_ok(), "Region analysis should succeed");
}

/// Test MIR generation
#[test]
fn test_mir_generation() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "MIR generation should succeed");
}

#[test]
fn test_mir_operations() {
    let input = "fn add(x: Int, y: Int) -> Int { x + y }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "MIR generation should succeed");
}

/// Test code generation
#[test]
fn test_code_generation() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let mut codegen = CodeGenerator::new(region_dag);
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Code generation should succeed");
}

#[test]
fn test_code_generation_with_cranelift() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let mut codegen = CodeGenerator::new_with_cranelift(region_dag).expect("Should create Cranelift codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Cranelift code generation should succeed");
}

/// Test error handling
#[test]
fn test_lexer_error_handling() {
    let input = "fn main() -> Unit { print(\"Hello\" }"; // Missing closing parenthesis
    let tokens = tokenize(input);
    
    // Should still produce tokens up to the error
    assert!(!tokens.is_empty(), "Should produce some tokens");
}

#[test]
fn test_parser_error_handling() {
    let input = "fn main() -> Unit { print(\"Hello\" }"; // Missing closing parenthesis
    let tokens = tokenize(input);
    let result = parse(tokens);
    
    assert!(result.is_err(), "Should fail to parse");
}

#[test]
fn test_type_error_handling() {
    let mut checker = TypeChecker::new();
    
    let input = "let x: Int = \"Hello\";"; // Type mismatch
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let result = checker.check(&hir);
    assert!(result.is_err(), "Should fail type checking");
}

/// Test edge cases
#[test]
fn test_empty_program() {
    let input = "";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse empty program");
    
    assert!(ast.items.is_empty(), "Should have no items");
}

#[test]
fn test_whitespace_handling() {
    let input = "   fn   main   (   )   ->   Unit   {   print   (   \"Hello\"   )   }   ";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse with whitespace");
    
    assert!(!ast.items.is_empty(), "Should have items");
}

#[test]
fn test_comment_handling() {
    let input = "// This is a comment\nfn main() -> Unit { print(\"Hello\") }";
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse with comments");
    
    assert!(!ast.items.is_empty(), "Should have items");
}

#[test]
fn test_multiline_strings() {
    let input = r#""Hello,
World!""#;
    let tokens = tokenize(input);
    
    assert_eq!(tokens.len(), 1, "Should produce one token");
    match &tokens[0].token {
        Token::StringLiteral(s) => assert!(s.contains("Hello") && s.contains("World")),
        _ => panic!("Should be a string literal"),
    }
}
