use once_hir::*;
use once_linear::{LinearityChecker, LinearityError};

#[test]
fn test_linear_variable_branch_mismatch_fails() {
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
                        HirStmt::Expr(HirExpr::If {
                            condition: Box::new(HirExpr::Literal(HirLiteral::Bool(true), None)),
                            then_branch: HirBlock {
                                statements: vec![
                                    HirStmt::Expr(HirExpr::Ident("f".to_string(), None)),
                                ],
                                span: None,
                            },
                            else_branch: Some(Box::new(HirExpr::Block(HirBlock {
                                statements: vec![],
                                span: None,
                            }, None))),
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
    // This SHOULD fail because 'f' is not consumed in the else branch
    assert!(result.is_err(), "Linear variable not consumed in one branch should fail");
}
