//! AI Integration Tests for the Once language
//!
//! Verifies that the `goal` syntax and AI solver hooks work end-to-end:
//! - `goal` declarations parse successfully
//! - The build system's `GoalSynthesizer` can produce placeholder implementations
//! - Synthesized goals can be lowered through the HIR pipeline

use once_build::ai::{AiSolver, GoalSynthesizer, StubAiSolver};
use once_build::BuildError;
use once_lex::Lexer;
use once_parse::OnceParser;
use once_hir::HirBuilder;

/// StubAiSolver produces a compilable placeholder for every supported return type.
#[test]
fn test_stub_ai_solver_synthesizes_placeholders() {
    let solver = StubAiSolver;

    let src = solver.synthesize("answer", &[], "Int", &[]).unwrap();
    assert!(src.contains("fn answer() -> Int { 0 }"), "Expected Int placeholder, got: {}", src);

    let src = solver.synthesize("flag", &[], "Bool", &[]).unwrap();
    assert!(src.contains("fn flag() -> Bool { false }"), "Expected Bool placeholder, got: {}", src);

    let src = solver.synthesize("greeting", &[], "Str", &[]).unwrap();
    assert!(src.contains("fn greeting() -> Str { \"\" }"), "Expected Str placeholder, got: {}", src);

    let src = solver.synthesize("noop", &[], "Unit", &[]).unwrap();
    assert!(src.contains("fn noop() -> Unit { () }"), "Expected Unit placeholder, got: {}", src);
}

/// GoalSynthesizer caches synthesized results.
#[test]
fn test_goal_synthesizer_caches_results() {
    let mut synthesizer = GoalSynthesizer::new();

    let src1 = synthesizer.synthesize_goal("cached_goal", &[], "Int", &[]).unwrap();
    let src2 = synthesizer.synthesize_goal("cached_goal", &[], "Int", &[]).unwrap();

    assert_eq!(src1, src2);
    assert_eq!(synthesizer.synthesized_goals.len(), 1);
}

/// A custom AI solver can be injected into the GoalSynthesizer.
#[test]
fn test_custom_ai_solver() {
    struct CustomSolver;
    impl AiSolver for CustomSolver {
        fn synthesize(&self, goal_name: &str, _params: &[String], return_type: &str, _constraints: &[String]) -> Result<String, BuildError> {
            Ok(format!("fn {}() -> {} {{ 42 }}", goal_name, return_type))
        }
    }

    let mut synthesizer = GoalSynthesizer::with_solver(Box::new(CustomSolver));
    let src = synthesizer.synthesize_goal("custom", &[], "Int", &[]).unwrap();
    assert_eq!(src, "fn custom() -> Int { 42 }");
}

/// `goal` declarations parse as distinct items from `fn` declarations.
#[test]
fn test_goal_parses_successfully() {
    let source = r#"
fn helper() -> Int { 1 }
goal optimize(x: Int) -> Int { x }
"#;
    let tokens: Vec<_> = Lexer::new(source).collect();
    let program = OnceParser::parse(tokens).expect("parse should succeed");

    assert_eq!(program.items.len(), 2);
    assert!(matches!(program.items[0], once_parse::Item::FnDecl(_)));
    assert!(matches!(program.items[1], once_parse::Item::GoalDecl(_)));
}

/// A `goal` declaration lowers through HIR as a regular function.
#[test]
fn test_goal_lowers_to_hir() {
    let source = "goal shortest_path(graph: Graph) -> Path { return }";
    let tokens: Vec<_> = Lexer::new(source).collect();
    let program = OnceParser::parse(tokens).expect("parse should succeed");

    let builder = HirBuilder::new();
    let hir = builder.build(program).expect("HIR build should succeed");

    assert_eq!(hir.items.len(), 1);
    assert!(matches!(hir.items[0], once_hir::HirItem::FnDecl(_)));

    if let once_hir::HirItem::FnDecl(fn_decl) = &hir.items[0] {
        assert_eq!(fn_decl.name, "shortest_path");
        assert_eq!(fn_decl.params.len(), 1);
    }
}

/// Synthesized goal source can be parsed and lowered to HIR.
#[test]
fn test_synthesized_goal_compiles_through_pipeline() {
    let mut synthesizer = GoalSynthesizer::new();
    let source = synthesizer.synthesize_goal("find_max", &["a: Int".to_string(), "b: Int".to_string()], "Int", &[]).unwrap();

    let tokens: Vec<_> = Lexer::new(&source).collect();
    let program = OnceParser::parse(tokens).expect("synthesized source should parse");
    assert_eq!(program.items.len(), 1);

    let builder = HirBuilder::new();
    let hir = builder.build(program).expect("synthesized source should lower to HIR");
    assert_eq!(hir.items.len(), 1);
}

/// BuildTool can carry a GoalSynthesizer for AI integration.
#[test]
fn test_build_tool_with_goal_synthesizer() {
    use once_build::{BuildConfig, BuildTool};

    let mut tool = BuildTool::new(BuildConfig::default());
    let src = tool.goal_synthesizer.synthesize_goal("route", &[], "Int", &[]).unwrap();
    assert!(src.contains("route"));
    assert!(src.contains("Int"));
}
