//! MIR lowering tests for the Once language
//!
//! Verifies that HIR correctly lowers to MIR with control flow.

use once_hir::*;
use once_mir::{MirGenerator, MirOp};
use once_rinf::{RegionDag};
use std::collections::HashMap;

/// Test basic function lowering
#[test]
fn test_mir_function_lowering() {
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
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "Basic function should lower to MIR");
    let mir = result.unwrap();
    assert_eq!(mir.functions.len(), 1);
}

/// Test if/else lowering generates Branch and Labels
#[test]
fn test_mir_if_lowering() {
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
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "If expression should lower to MIR");
    let mir = result.unwrap();
    let stmts = &mir.functions[0].body.statements;

    // Should contain Branch, Labels, and Jumps
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Branch { .. })), "Should have Branch");
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Label { .. })), "Should have Labels");
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Jump { .. })), "Should have Jumps");
}

/// Test match lowering generates Branch and Labels
#[test]
fn test_mir_match_lowering() {
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
                                expr: Box::new(HirExpr::Literal(HirLiteral::Int(1))),
                                arms: vec![
                                    (HirPattern::Literal(HirLiteral::Int(1)), HirExpr::Literal(HirLiteral::Int(10))),
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
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "Match expression should lower to MIR");
    let mir = result.unwrap();
    let stmts = &mir.functions[0].body.statements;

    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Branch { .. })), "Should have Branch");
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Label { .. })), "Should have Labels");
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Jump { .. })), "Should have Jumps");
}

/// Test using block generates Drop
#[test]
fn test_mir_using_block_drop() {
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
                        HirStmt::Using(HirUsingStmt {
                            name: "f".to_string(),
                            init: HirExpr::Call {
                                function: "open_file".to_string(),
                                args: vec![],
                            },
                            body: HirBlock {
                                statements: vec![
                                    HirStmt::Expr(HirExpr::Call {
                                        function: "read".to_string(),
                                        args: vec![HirExpr::Ident("f".to_string())],
                                    }),
                                ],
                                span: None,
                            },
                            is_linear: true,
                            span: None,
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
    let result = generator.generate(&hir, region_dag);
    assert!(result.is_ok(), "Using block should lower to MIR");
    let mir = result.unwrap();
    let stmts = &mir.functions[0].body.statements;

    // Should contain a Drop at the end of the using block
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Drop { .. })), "Using block should generate Drop");
}
