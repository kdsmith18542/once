//! Control-flow codegen tests for the Once language
//!
//! Verifies that the Cranelift backend correctly compiles
//! functions containing if/else, match, and for constructs.

use once_hir::*;
use once_mir::MirGenerator;
use once_rinf::RegionDag;
use once_codegen::RealCraneliftCodegen;
use std::collections::HashMap;

/// Test that a function with if/else compiles through the Cranelift backend.
#[test]
fn test_codegen_if_else() {
    let hir = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(HirType::Int),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Return(HirReturnStmt {
                            value: Some(HirExpr::If {
                                condition: Box::new(HirExpr::Literal(HirLiteral::Bool(true))),
                                then_branch: HirBlock {
                                    statements: vec![HirStmt::Expr(HirExpr::Literal(HirLiteral::Int(1)))],
                                    span: None,
                                },
                                else_branch: Some(Box::new(HirExpr::Literal(HirLiteral::Int(2)))),
                            }),
                            span: None,
                        }),
                    ],
                    span: None,
                },
                is_public: false,
                span: None,
            }),
        ],
        imports: vec![],
    };

    let mut generator = MirGenerator::new();
    let region_dag = RegionDag {
        nodes: HashMap::new(),
        edges: vec![],
        free_points: vec![],
    };
    let mir = generator.generate(&hir, region_dag).expect("Should lower to MIR");

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "If/else should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Test that a function with match compiles through the Cranelift backend.
#[test]
fn test_codegen_match() {
    let hir = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(HirType::Int),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Return(HirReturnStmt {
                            value: Some(HirExpr::Match {
                                expr: Box::new(HirExpr::Literal(HirLiteral::Bool(true))),
                                arms: vec![
                                    (HirPattern::Literal(HirLiteral::Bool(true)), HirExpr::Literal(HirLiteral::Int(10))),
                                    (HirPattern::Wildcard, HirExpr::Literal(HirLiteral::Int(0))),
                                ],
                            }),
                            span: None,
                        }),
                    ],
                    span: None,
                },
                is_public: false,
                span: None,
            }),
        ],
        imports: vec![],
    };

    let mut generator = MirGenerator::new();
    let region_dag = RegionDag {
        nodes: HashMap::new(),
        edges: vec![],
        free_points: vec![],
    };
    let mir = generator.generate(&hir, region_dag).expect("Should lower to MIR");

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Match should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Test that a function with a for loop compiles through the Cranelift backend.
/// Note: the current MIR generator lowers for loops as inline blocks (no back-edges),
/// so this primarily exercises non-terminator codegen across block boundaries.
#[test]
fn test_codegen_for_loop() {
    let hir = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![],
                return_type: Some(HirType::Unit),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Expr(HirExpr::For {
                            item: "x".to_string(),
                            collection: Box::new(HirExpr::Literal(HirLiteral::Int(1))),
                            body: HirBlock {
                                statements: vec![
                                    HirStmt::Expr(HirExpr::Call {
                                        function: "print".to_string(),
                                        args: vec![HirExpr::Literal(HirLiteral::Int(42))],
                                    }),
                                ],
                                span: None,
                            },
                        }),
                        HirStmt::Return(HirReturnStmt { value: None, span: None }),
                    ],
                    span: None,
                },
                is_public: false,
                span: None,
            }),
        ],
        imports: vec![],
    };

    let mut generator = MirGenerator::new();
    let region_dag = RegionDag {
        nodes: HashMap::new(),
        edges: vec![],
        free_points: vec![],
    };
    let mir = generator.generate(&hir, region_dag).expect("Should lower to MIR");

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "For loop should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}
