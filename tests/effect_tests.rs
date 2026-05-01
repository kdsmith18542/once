//! Effect system tests for the Once language
//!
//! Verifies row-polymorphic effect tracking.

use once_hir::*;
use once_ty::effects::{EffectChecker, EffectRow, EffectLabel};

/// Test that pure literals produce no effects
#[test]
fn test_pure_literal_effects() {
    let program = HirProgram {
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

    let mut checker = EffectChecker::new();
    let result = checker.check(&program);
    assert!(result.is_ok(), "Pure function should have no effect errors");
}

/// Test effect row union
#[test]
fn test_effect_row_union() {
    use once_ty::Type;

    let checker = EffectChecker::new();

    let io = EffectRow::Single {
        label: EffectLabel::Io,
        ty: Type::Unit,
    };
    let spawn = EffectRow::Single {
        label: EffectLabel::Spawn,
        ty: Type::Unit,
    };

    let union = checker.union_effect_rows(io.clone(), spawn.clone());
    assert!(matches!(union, EffectRow::Union(_, _)));

    // Union with empty should return the other
    let union_empty = checker.union_effect_rows(EffectRow::Empty, io.clone());
    assert_eq!(union_empty, io);
}

/// Test effect contains check
#[test]
fn test_effect_contains() {
    use once_ty::Type;

    let checker = EffectChecker::new();

    let row = EffectRow::Single {
        label: EffectLabel::Io,
        ty: Type::Unit,
    };

    assert!(checker.contains_effect(&row, &EffectLabel::Io));
    assert!(!checker.contains_effect(&row, &EffectLabel::Spawn));
}

/// Test empty effect row
#[test]
fn test_empty_effect_row() {
    let checker = EffectChecker::new();
    let row = EffectRow::Empty;

    assert!(!checker.contains_effect(&row, &EffectLabel::Io));
    assert!(!checker.contains_effect(&row, &EffectLabel::Spawn));
}
