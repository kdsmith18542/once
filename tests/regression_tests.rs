//! Regression tests for the Once language compiler
//! 
//! These tests ensure that previously fixed bugs don't regress
//! and that the compiler maintains consistent behavior over time.

use once_lex::*;
use once_parse::*;
use once_hir::*;
use once_ty::*;
use once_effects::*;
use once_linear::*;
use once_rinf::*;
use once_mir::*;
use once_codegen::*;

/// Test that basic compilation still works
#[test]
fn test_basic_compilation_regression() {
    let input = "fn main() -> Unit { print(\"Hello, World!\") }";
    
    let tokens = tokenize(input);
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
    
    // All steps should succeed
    assert!(type_result.is_ok(), "Type checking should succeed");
    assert!(effects_result.is_ok(), "Effects checking should succeed");
    assert!(linearity_result.is_ok(), "Linearity checking should succeed");
    assert!(region_result.is_ok(), "Region inference should succeed");
    assert!(mir_result.is_ok(), "MIR generation should succeed");
    assert!(codegen_result.is_ok(), "Code generation should succeed");
}

/// Test that error handling still works correctly
#[test]
fn test_error_handling_regression() {
    // Test parsing errors
    let invalid_input = "fn main() -> Unit { print(\"Hello\" }"; // Missing closing parenthesis
    let tokens = tokenize(invalid_input);
    let parse_result = parse(tokens);
    assert!(parse_result.is_err(), "Should fail to parse invalid input");
    
    // Test type errors
    let type_error_input = "let x: Int = \"Hello\";"; // Type mismatch
    let tokens = tokenize(type_error_input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut type_checker = TypeChecker::new();
    let type_result = type_checker.check(&hir);
    assert!(type_result.is_err(), "Should fail type checking");
}

/// Test that string literals are handled correctly
#[test]
fn test_string_literals_regression() {
    let input = r#"fn main() -> Unit { print("Hello, World!") }"#;
    
    let tokens = tokenize(input);
    assert!(!tokens.is_empty(), "Should produce tokens");
    
    // Check that string literal is preserved
    let string_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token, Token::StringLiteral(_))).collect();
    assert_eq!(string_tokens.len(), 1, "Should have one string literal");
    
    match &string_tokens[0].token {
        Token::StringLiteral(s) => assert_eq!(s, "Hello, World!"),
        _ => panic!("Should be a string literal"),
    }
}

/// Test that integer literals are handled correctly
#[test]
fn test_integer_literals_regression() {
    let input = "fn main() -> Unit { print(42) }";
    
    let tokens = tokenize(input);
    assert!(!tokens.is_empty(), "Should produce tokens");
    
    // Check that integer literal is preserved
    let int_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token, Token::IntegerLiteral(_))).collect();
    assert_eq!(int_tokens.len(), 1, "Should have one integer literal");
    
    match &int_tokens[0].token {
        Token::IntegerLiteral(n) => assert_eq!(*n, 42),
        _ => panic!("Should be an integer literal"),
    }
}

/// Test that float literals are handled correctly
#[test]
fn test_float_literals_regression() {
    let input = "fn main() -> Unit { print(3.14) }";
    
    let tokens = tokenize(input);
    assert!(!tokens.is_empty(), "Should produce tokens");
    
    // Check that float literal is preserved
    let float_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token, Token::FloatLiteral(_))).collect();
    assert_eq!(float_tokens.len(), 1, "Should have one float literal");
    
    match &float_tokens[0].token {
        Token::FloatLiteral(f) => assert!((f - 3.14).abs() < 1e-10),
        _ => panic!("Should be a float literal"),
    }
}

/// Test that boolean literals are handled correctly
#[test]
fn test_boolean_literals_regression() {
    let input = "fn main() -> Unit { print(true) }";
    
    let tokens = tokenize(input);
    assert!(!tokens.is_empty(), "Should produce tokens");
    
    // Check that boolean literal is preserved
    let bool_tokens: Vec<_> = tokens.iter().filter(|t| matches!(t.token, Token::BooleanLiteral(_))).collect();
    assert_eq!(bool_tokens.len(), 1, "Should have one boolean literal");
    
    match &bool_tokens[0].token {
        Token::BooleanLiteral(b) => assert!(*b),
        _ => panic!("Should be a boolean literal"),
    }
}

/// Test that function declarations are handled correctly
#[test]
fn test_function_declarations_regression() {
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

/// Test that variable declarations are handled correctly
#[test]
fn test_variable_declarations_regression() {
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

/// Test that type annotations are preserved
#[test]
fn test_type_annotations_regression() {
    let input = "fn add(x: Int, y: Int) -> Int { x + y }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    assert!(!ast.items.is_empty(), "Should have items");
    match &ast.items[0] {
        AstItem::FnDecl(fn_decl) => {
            assert_eq!(fn_decl.name, "add");
            assert!(matches!(fn_decl.return_type, HirType::Int));
            assert_eq!(fn_decl.params.len(), 2, "Should have two parameters");
        }
        _ => panic!("Should be a function declaration"),
    }
}

/// Test that HIR generation preserves structure
#[test]
fn test_hir_structure_regression() {
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

/// Test that type checking preserves semantics
#[test]
fn test_type_checking_semantics_regression() {
    let input = "fn add(x: Int, y: Int) -> Int { x + y }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut type_checker = TypeChecker::new();
    let result = type_checker.check(&hir);
    assert!(result.is_ok(), "Type checking should succeed");
}

/// Test that effects checking preserves semantics
#[test]
fn test_effects_checking_semantics_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut effects_checker = EffectChecker::new();
    let result = effects_checker.check(&hir);
    assert!(result.is_ok(), "Effects checking should succeed");
}

/// Test that linearity checking preserves semantics
#[test]
fn test_linearity_checking_semantics_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut linearity_checker = LinearityChecker::new();
    let result = linearity_checker.check(&hir);
    assert!(result.is_ok(), "Linearity checking should succeed");
}

/// Test that region inference preserves semantics
#[test]
fn test_region_inference_semantics_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut region_checker = RegionChecker::new();
    let result = region_checker.check(&hir);
    assert!(result.is_ok(), "Region inference should succeed");
}

/// Test that MIR generation preserves semantics
#[test]
fn test_mir_generation_semantics_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let result = mir_generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "MIR generation should succeed");
}

/// Test that code generation preserves semantics
#[test]
fn test_code_generation_semantics_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = mir_generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let mut codegen = CodeGenerator::new(region_dag);
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Code generation should succeed");
}

/// Test that Cranelift integration still works
#[test]
fn test_cranelift_integration_regression() {
    let input = "fn main() -> Unit { print(\"Hello\") }";
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut mir_generator = MirGenerator::new();
    let region_dag = RegionDag::new(); // Simplified
    let mir = mir_generator.generate(&hir, region_dag).expect("Should generate MIR");
    
    let mut codegen = CodeGenerator::new_with_cranelift(region_dag).expect("Should create Cranelift codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Cranelift code generation should succeed");
}

/// Test that error messages are consistent
#[test]
fn test_error_message_consistency() {
    let input = "let x: Int = \"Hello\";"; // Type mismatch
    
    let tokens = tokenize(input);
    let ast = parse(tokens).expect("Should parse successfully");
    
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).expect("Should build HIR successfully");
    
    let mut type_checker = TypeChecker::new();
    let result = type_checker.check(&hir);
    
    assert!(result.is_err(), "Should fail type checking");
    
    // Error message should be consistent
    let error = result.unwrap_err();
    assert!(!error.is_empty(), "Should have error messages");
}

/// Test that whitespace handling is consistent
#[test]
fn test_whitespace_handling_consistency() {
    let input1 = "fn main() -> Unit { print(\"Hello\") }";
    let input2 = "   fn   main   (   )   ->   Unit   {   print   (   \"Hello\"   )   }   ";
    
    let tokens1 = tokenize(input1);
    let tokens2 = tokenize(input2);
    
    let ast1 = parse(tokens1).expect("Should parse successfully");
    let ast2 = parse(tokens2).expect("Should parse successfully");
    
    // Both should produce the same structure
    assert_eq!(ast1.items.len(), ast2.items.len(), "Should have same number of items");
    
    match (&ast1.items[0], &ast2.items[0]) {
        (AstItem::FnDecl(fn1), AstItem::FnDecl(fn2)) => {
            assert_eq!(fn1.name, fn2.name, "Function names should match");
            assert_eq!(fn1.return_type, fn2.return_type, "Return types should match");
        }
        _ => panic!("Should be function declarations"),
    }
}

/// Test that comment handling is consistent
#[test]
fn test_comment_handling_consistency() {
    let input1 = "fn main() -> Unit { print(\"Hello\") }";
    let input2 = "// This is a comment\nfn main() -> Unit { print(\"Hello\") }";
    
    let tokens1 = tokenize(input1);
    let tokens2 = tokenize(input2);
    
    let ast1 = parse(tokens1).expect("Should parse successfully");
    let ast2 = parse(tokens2).expect("Should parse successfully");
    
    // Both should produce the same structure
    assert_eq!(ast1.items.len(), ast2.items.len(), "Should have same number of items");
    
    match (&ast1.items[0], &ast2.items[0]) {
        (AstItem::FnDecl(fn1), AstItem::FnDecl(fn2)) => {
            assert_eq!(fn1.name, fn2.name, "Function names should match");
            assert_eq!(fn1.return_type, fn2.return_type, "Return types should match");
        }
        _ => panic!("Should be function declarations"),
    }
}

/// Test that multiline string handling is consistent
#[test]
fn test_multiline_string_handling_consistency() {
    let input1 = r#""Hello, World!""#;
    let input2 = r#""Hello,
World!""#;
    
    let tokens1 = tokenize(input1);
    let tokens2 = tokenize(input2);
    
    // Both should produce string literals
    let string1 = tokens1.iter().find(|t| matches!(t.token, Token::StringLiteral(_)));
    let string2 = tokens2.iter().find(|t| matches!(t.token, Token::StringLiteral(_)));
    
    assert!(string1.is_some(), "Should have string literal");
    assert!(string2.is_some(), "Should have string literal");
    
    match (&string1.unwrap().token, &string2.unwrap().token) {
        (Token::StringLiteral(s1), Token::StringLiteral(s2)) => {
            assert!(s1.contains("Hello") && s1.contains("World"), "First string should contain content");
            assert!(s2.contains("Hello") && s2.contains("World"), "Second string should contain content");
        }
        _ => panic!("Should be string literals"),
    }
}
