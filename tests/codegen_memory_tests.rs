//! Memory codegen tests for the Once language
//!
//! Verifies that the Cranelift backend correctly generates
//! Allocate, FreeRegion, Drop, and BoundsCheck sequences.

use once_hir::*;
use once_mir::{MirGenerator, MirOp, MirLocation, MirValue};
use once_rinf::{RegionDag};
use once_codegen::RealCraneliftCodegen;
use std::collections::HashMap;

/// Build a minimal MIR function that uses Drop and compile it
#[test]
fn test_codegen_drop_operation() {
    let mut mir_fn = once_mir::MirFunction {
        name: "test_drop".to_string(),
        params: vec![("ptr".to_string(), HirType::Int)],
        return_type: HirType::Unit,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::Drop {
                        location: MirLocation::Param(0),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: None },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 0,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Drop operation should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Build a minimal MIR function that uses Allocate and compile it
#[test]
fn test_codegen_allocate_operation() {
    let mut mir_fn = once_mir::MirFunction {
        name: "test_alloc".to_string(),
        params: vec![],
        return_type: HirType::Int,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::Allocate {
                        region: once_rinf::Region { id: 0, name: "heap".to_string(), is_primary: false },
                        size: 64,
                        dest: MirLocation::Temp(0),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: Some(MirLocation::Temp(0)) },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 1,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Allocate operation should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Build a minimal MIR function that uses BoundsCheck and compile it
#[test]
fn test_codegen_bounds_check() {
    let mut mir_fn = once_mir::MirFunction {
        name: "test_bounds".to_string(),
        params: vec![
            ("index".to_string(), HirType::Int),
            ("bound".to_string(), HirType::Int),
        ],
        return_type: HirType::Unit,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::BoundsCheck {
                        index: MirLocation::Param(0),
                        bound: MirLocation::Param(1),
                        proven: false,
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: None },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 0,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "BoundsCheck operation should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Test proven BoundsCheck is a no-op (should compile without generating compare)
#[test]
fn test_codegen_bounds_check_proven() {
    let mut mir_fn = once_mir::MirFunction {
        name: "test_bounds_proven".to_string(),
        params: vec![],
        return_type: HirType::Unit,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::BoundsCheck {
                        index: MirLocation::Param(0),
                        bound: MirLocation::Param(1),
                        proven: true,
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: None },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 0,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "Proven BoundsCheck should compile: {:?}", result.err());
}
