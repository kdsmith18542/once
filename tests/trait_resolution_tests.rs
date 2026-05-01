//! Trait resolution tests for the Once language
//!
//! Verifies that trait definitions and implementations are tracked correctly.

use once_ty::{TypeChecker, TraitDef, TraitImpl, Type, TypeScheme};

/// Test trait registration and lookup
#[test]
fn test_trait_registry() {
    let mut checker = TypeChecker::new();

    let show_trait = TraitDef {
        name: "Show".to_string(),
        type_params: vec!["T".to_string()],
        methods: vec![(
            "show".to_string(),
            TypeScheme {
                vars: vec![],
                ty: Type::Function {
                    params: vec![Type::UserDefined { name: "T".to_string(), args: vec![] }],
                    return_type: Box::new(Type::Str),
                },
                constraints: vec![],
            },
        )],
    };

    checker.register_trait(show_trait);
    assert!(checker.traits.contains_key("Show"));
}

/// Test trait implementation lookup
#[test]
fn test_trait_impl_resolution() {
    let mut checker = TypeChecker::new();

    let int_show = TraitImpl {
        trait_name: "Show".to_string(),
        type_name: "Int".to_string(),
        methods: vec![(
            "show".to_string(),
            TypeScheme {
                vars: vec![],
                ty: Type::Function {
                    params: vec![Type::Int],
                    return_type: Box::new(Type::Str),
                },
                constraints: vec![],
            },
        )],
    };

    checker.register_trait_impl(int_show);

    let resolved = checker.resolve_trait("Show", &Type::Int);
    assert!(resolved.is_some(), "Int should implement Show");
    assert_eq!(resolved.unwrap().type_name, "Int");

    let not_resolved = checker.resolve_trait("Show", &Type::Float);
    assert!(not_resolved.is_none(), "Float should not implement Show (not registered)");
}

/// Test multiple trait implementations
#[test]
fn test_multiple_trait_impls() {
    let mut checker = TypeChecker::new();

    checker.register_trait_impl(TraitImpl {
        trait_name: "Show".to_string(),
        type_name: "Int".to_string(),
        methods: vec![],
    });

    checker.register_trait_impl(TraitImpl {
        trait_name: "Show".to_string(),
        type_name: "Bool".to_string(),
        methods: vec![],
    });

    assert!(checker.resolve_trait("Show", &Type::Int).is_some());
    assert!(checker.resolve_trait("Show", &Type::Bool).is_some());
    assert!(checker.resolve_trait("Show", &Type::Str).is_none());
}
