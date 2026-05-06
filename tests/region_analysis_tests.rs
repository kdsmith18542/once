//! Region analysis tests for the Once language
//!
//! Verifies that the Region DAG integrates correctly with MIR generation.

use once_hir::*;
use once_mir::{MirGenerator, MirOp};
use once_rinf::{RegionDag, RegionSolver};
use std::collections::HashMap;

/// Test that a basic HIR program produces a valid Region DAG
#[test]
fn test_region_dag_from_hir() {
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

    let mut solver = RegionSolver::new();
    let dag = solver.solve(&hir);
    assert!(dag.is_ok(), "Region solver should produce a DAG for simple program");
    let dag = dag.unwrap();
    // Should have at least one region node for the function
    assert!(!dag.nodes.is_empty(), "DAG should contain region nodes");
}

/// Test that MIR generation accepts a Region DAG
#[test]
fn test_mir_generation_with_region_dag() {
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

    let mut solver = RegionSolver::new();
    let dag = solver.solve(&hir).expect("Should solve regions");

    let mut generator = MirGenerator::new();
    let mir = generator.generate(&hir, dag);
    assert!(mir.is_ok(), "MIR generation should succeed with Region DAG");
}

/// Test that using blocks emit Drop operations
#[test]
fn test_using_block_emits_drop() {
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
                                span: None,
                            },
                            body: HirBlock {
                                statements: vec![
                                    HirStmt::Expr(HirExpr::Call {
                                        function: "read".to_string(),
                                        args: vec![HirExpr::Ident("f".to_string(), None)],
                                        span: None,
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

    let mut solver = RegionSolver::new();
    let dag = solver.solve(&hir).expect("Should solve regions");

    let mut generator = MirGenerator::new();
    let mir = generator.generate(&hir, dag).expect("Should generate MIR");
    let stmts = &mir.functions[0].body.statements;

    // Should contain a Drop at the end of the using block
    assert!(stmts.iter().any(|s| matches!(s.op, MirOp::Drop { .. })),
        "Using block should generate Drop operation");
}

/// Test that region frees are inserted into MIR
#[test]
fn test_region_frees_inserted() {
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

    let mut solver = RegionSolver::new();
    let dag = solver.solve(&hir).expect("Should solve regions");

    let mut generator = MirGenerator::new();
    let mut mir = generator.generate(&hir, dag).expect("Should generate MIR");

    // Add region frees based on the DAG
    let result = generator.add_region_frees(&mut mir);
    assert!(result.is_ok(), "add_region_frees should succeed");

    // The MIR should now contain FreeRegion operations
    let has_free = mir.functions.iter()
        .any(|f| f.body.statements.iter().any(|s| matches!(s.op, MirOp::FreeRegion { .. })));
    assert!(has_free, "MIR should contain FreeRegion operations after add_region_frees");
}

/// Test that nested blocks create subregions in the DAG
#[test]
fn test_nested_blocks_create_subregions() {
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
                        HirStmt::Expr(HirExpr::Block(HirBlock {
                            statements: vec![
                                HirStmt::Return(HirReturnStmt { value: None, span: None }),
                            ],
                            span: None,
                        }, None)),
                    ],
                    span: None,
                },
                is_public: false,
                span: None,
            }),
        ],
        imports: vec![],
    };

    let mut solver = RegionSolver::new();
    let dag = solver.solve(&hir).expect("Should solve regions");

    // Should have multiple regions: one for function, one for block
    assert!(dag.nodes.len() >= 2, "Nested blocks should create subregions");
}
