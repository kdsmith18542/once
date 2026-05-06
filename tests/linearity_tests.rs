//! Linearity checking tests for the Once language
//!
//! Verifies linear and affine type usage rules.

use once_hir::*;
use once_linear::{LinearityChecker, LinearityEnv, Linearity, LinearityError};
use once_lex::Span;

/// Test that linear variables used once pass
#[test]
fn test_linear_used_once() {
    let program = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![
                    HirParam {
                        name: "f".to_string(),
                        type_annotation: Some(HirType::Linear(Box::new(HirType::Ident("File".to_string())))),
                        is_linear: true,
                    },
                ],
                return_type: Some(HirType::Unit),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Expr(HirExpr::Call {
                            function: "consume".to_string(),
                            args: vec![HirExpr::Ident("f".to_string(), None)],
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

    let mut checker = LinearityChecker::new();
    let result = checker.check(&program);
    assert!(result.is_ok(), "Linear variable used once should pass");
}

/// Test that linear variables used twice fail
#[test]
fn test_linear_used_twice_fails() {
    let program = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![
                    HirParam {
                        name: "f".to_string(),
                        type_annotation: Some(HirType::Linear(Box::new(HirType::Ident("File".to_string())))),
                        is_linear: true,
                    },
                ],
                return_type: Some(HirType::Unit),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Expr(HirExpr::Ident("f".to_string(), None)),
                        HirStmt::Expr(HirExpr::Ident("f".to_string(), None)),
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

    let mut checker = LinearityChecker::new();
    let result = checker.check(&program);
    assert!(result.is_err(), "Linear variable used twice should fail");
}

/// Test unconsumed linear value fails
#[test]
fn test_unconsumed_linear_fails() {
    let program = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![
                    HirParam {
                        name: "f".to_string(),
                        type_annotation: Some(HirType::Linear(Box::new(HirType::Ident("File".to_string())))),
                        is_linear: true,
                    },
                ],
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

    let mut checker = LinearityChecker::new();
    let result = checker.check(&program);
    assert!(result.is_err(), "Unconsumed linear parameter should fail");
    let errors = result.unwrap_err();
    assert!(errors.iter().any(|e| matches!(e, LinearityError::LinearValueNotConsumed { .. })));
}

/// Test linear environment usage tracking
#[test]
fn test_linear_env_tracking() {
    let mut env = LinearityEnv::new();
    env.add_variable("x".to_string(), Linearity::Linear);

    // First use should succeed
    assert!(env.use_variable("x", Span::new(0, 0, 0, 0)).is_ok());

    // Second use should fail
    let result = env.use_variable("x", Span::new(1, 1, 1, 1));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), LinearityError::LinearValueReused { .. }));
}

/// Test non-linear variable can be used multiple times
#[test]
fn test_non_linear_multiple_use() {
    let program = HirProgram {
        items: vec![
            HirItem::FnDecl(HirFnDecl {
                name: "main".to_string(),
                type_params: vec![],
                params: vec![
                    HirParam {
                        name: "x".to_string(),
                        type_annotation: Some(HirType::Int),
                        is_linear: false,
                    },
                ],
                return_type: Some(HirType::Unit),
                effects: None,
                body: HirBlock {
                    statements: vec![
                        HirStmt::Expr(HirExpr::Ident("x".to_string(), None)),
                        HirStmt::Expr(HirExpr::Ident("x".to_string(), None)),
                        HirStmt::Expr(HirExpr::Ident("x".to_string(), None)),
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

    let mut checker = LinearityChecker::new();
    let result = checker.check(&program);
    assert!(result.is_ok(), "Non-linear variable used multiple times should pass");
}
