//! Concurrency codegen tests for the Once language
//!
//! Verifies that the Cranelift backend correctly generates
//! SpawnTask, ChannelSend, ChannelRecv, and AwaitTask sequences.

use once_hir::*;
use once_mir::{MirGenerator, MirOp, MirLocation, MirValue};
use once_rinf::{RegionDag};
use once_codegen::RealCraneliftCodegen;
use std::collections::HashMap;

/// Build a minimal MIR function that uses SpawnTask and compile it
#[test]
fn test_codegen_spawn_task() {
    let mir_fn = once_mir::MirFunction {
        name: "test_spawn".to_string(),
        params: vec![],
        return_type: HirType::Int,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::SpawnTask {
                        function: "worker".to_string(),
                        args: vec![],
                        result: MirLocation::Temp(0),
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
    assert!(result.is_ok(), "SpawnTask should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Build a minimal MIR function that uses ChannelSend and compile it
#[test]
fn test_codegen_channel_send() {
    let mir_fn = once_mir::MirFunction {
        name: "test_send".to_string(),
        params: vec![],
        return_type: HirType::Unit,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Int(1),
                        dest: MirLocation::Temp(0),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Int(42),
                        dest: MirLocation::Temp(1),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::ChannelSend {
                        channel: MirLocation::Temp(0),
                        value: MirLocation::Temp(1),
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
        temp_count: 2,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "ChannelSend should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Build a minimal MIR function that uses ChannelRecv and compile it
#[test]
fn test_codegen_channel_recv() {
    let mir_fn = once_mir::MirFunction {
        name: "test_recv".to_string(),
        params: vec![],
        return_type: HirType::Int,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Int(1),
                        dest: MirLocation::Temp(0),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::ChannelRecv {
                        channel: MirLocation::Temp(0),
                        result: MirLocation::Temp(1),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: Some(MirLocation::Temp(1)) },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 2,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "ChannelRecv should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}

/// Build a minimal MIR function that uses AwaitTask and compile it
#[test]
fn test_codegen_await_task() {
    let mir_fn = once_mir::MirFunction {
        name: "test_await".to_string(),
        params: vec![],
        return_type: HirType::Int,
        body: once_mir::MirBlock {
            statements: vec![
                once_mir::MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Int(1),
                        dest: MirLocation::Temp(0),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::AwaitTask {
                        task: MirLocation::Temp(0),
                        result: MirLocation::Temp(1),
                    },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
                once_mir::MirStmt {
                    op: MirOp::Return { value: Some(MirLocation::Temp(1)) },
                    span: once_lex::Span::new(0, 0, 0, 0),
                    region: None,
                },
            ],
            region: None,
        },
        local_count: 0,
        temp_count: 2,
    };

    let mir = once_mir::MirProgram {
        functions: vec![mir_fn],
        global_region: once_rinf::Region { id: 0, name: "global".to_string(), is_primary: true },
    };

    let mut codegen = RealCraneliftCodegen::new().expect("Should create codegen");
    let result = codegen.generate(&mir);
    assert!(result.is_ok(), "AwaitTask should compile: {:?}", result.err());
    let bytes = result.unwrap();
    assert!(!bytes.is_empty(), "Should produce object bytes");
}
