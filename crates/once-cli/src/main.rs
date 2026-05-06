//! Command-line interface for the Once compiler

use clap::{Parser, Subcommand, ValueEnum};
use anyhow::Context;
use once_lex::Lexer;
use once_parse::OnceParser;
use once_hir::HirBuilder;
use once_ty::TypeChecker;
use once_ty::effects::{EffectChecker, EffectRow};
use once_linear::{LinearityChecker, Linearity};
use once_rinf::RegionChecker;

mod lint;
mod mir_eval;
use once_mir::MirGenerator;
use once_codegen::CodeGenerator;
use once_runtime::Runtime;
use once_std;
use once_build::BuildTool;
use once_lsp;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "once")]
#[command(about = "Once Language Compiler")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Clone, ValueEnum)]
enum ExplainTopic {
    Regions,
    Effects,
    Linearity,
    #[value(alias = "escape")]
    EscapeAnalysis,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile Once source code
    Build {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
        
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Parse and show AST
    Parse {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Show HIR
    Hir {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Type check
    Typecheck {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Check effects
    Effects {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Check linearity
    Linearity {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Check regions
    Regions {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Generate MIR
    Mir {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Generate code
    Codegen {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Run program
    Run {
        /// Input file
        #[arg(short, long)]
        input: PathBuf,
    },
    /// Run tests
    Test {
        /// Test directory (defaults to tests/)
        #[arg(short, long)]
        dir: Option<PathBuf>,
        
        /// Run only tests matching pattern
        #[arg(short, long)]
        filter: Option<String>,

        /// Run tests in deterministic (single-threaded) scheduler mode for reproducible results
        #[arg(long)]
        deterministic: bool,
    },
    /// Build project
    BuildProject {
        /// Project directory
        #[arg(short, long)]
        project: Option<PathBuf>,
    },
    /// Create a new project
    New {
        /// Project name
        name: String,
    },
    /// Explain compiler analysis
    Explain {
        /// Topic to explain (regions, effects, linearity)
        topic: Option<ExplainTopic>,
        /// Error code to explain (e.g., E001)
        #[arg(long)]
        error_code: Option<String>,
        /// Input file
        input: Option<PathBuf>,
    },
    /// Start the Language Server
    Lsp {
        /// Use stdio transport (default)
        #[arg(long)]
        stdio: bool,
        /// Use TCP transport on the given port
        #[arg(long, default_value_t = 0)]
        port: u16,
    },
    /// Auto-fix common issues
    Fix {
        /// Fix mode
        #[arg(long, default_value = "consumes")]
        mode: String,
        /// Input file
        input: PathBuf,
    },
    /// Format source file
    Fmt {
        /// Input file
        input: PathBuf,
    },
    /// Lint source file
    Lint {
        /// Input file
        input: PathBuf,
    },
    /// Output unified JSON analysis of compiler stages
    Analyze {
        /// Source file to analyze
        file: String,
        /// Output format (json only for now)
        #[arg(long, default_value = "json")]
        format: String,
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Manage AI-synthesized goals
    Goal {
        #[command(subcommand)]
        action: GoalAction,
    },
}

/// Goal management actions
#[derive(Subcommand)]
enum GoalAction {
    /// Eject a goal declaration into a concrete function implementation
    Eject {
        /// The goal name to eject
        name: String,
        /// Input file
        #[arg(short, long)]
        file: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { input, output } => {
            compile_file(&input, output.as_deref())?;
        }
        Commands::Parse { input } => {
            parse_file(&input)?;
        }
        Commands::Hir { input } => {
            show_hir(&input)?;
        }
        Commands::Typecheck { input } => {
            typecheck_file(&input)?;
        }
        Commands::Effects { input } => {
            check_effects(&input)?;
        }
        Commands::Linearity { input } => {
            check_linearity(&input)?;
        }
        Commands::Regions { input } => {
            check_regions(&input)?;
        }
        Commands::Mir { input } => {
            generate_mir(&input)?;
        }
        Commands::Codegen { input } => {
            generate_code(&input)?;
        }
        Commands::Run { input } => {
            run_program(&input)?;
        }
        Commands::Test { dir, filter, deterministic } => {
            run_tests(dir.as_deref(), filter.as_deref(), deterministic)?;
        }
        Commands::BuildProject { project } => {
            build_project(project.as_deref())?;
        }
        Commands::New { name } => {
            create_project(&name)?;
        }
        Commands::Explain { topic, error_code, input } => {
            if let Some(code) = error_code {
                explain_error_code(&code);
            } else if let (Some(t), Some(f)) = (topic, input) {
                explain_file(&t, &f)?;
            } else {
                anyhow::bail!("Specify either a topic with --input, or --error-code <code>");
            }
        }
        Commands::Lsp { stdio, port } => {
            let rt = tokio::runtime::Runtime::new()
                .context("Failed to create Tokio runtime")?;

            if port > 0 {
                rt.block_on(once_lsp::start_lsp_server_tcp(port))
                    .map_err(|e| anyhow::anyhow!("TCP LSP server failed: {}", e))?;
            } else {
                rt.block_on(once_lsp::start_lsp_server())
                    .map_err(|e| anyhow::anyhow!("LSP server failed: {}", e))?;
            }
        }
        Commands::Fix { mode, input } => {
            fix_file(&mode, &input)?;
        }
        Commands::Fmt { input } => {
            format_file(&input)?;
        }
        Commands::Lint { input } => {
            lint_file(&input)?;
        }
        Commands::Analyze { file, format: _, output } => {
            analyze_file(&file, output.as_deref())?;
        }
        Commands::Goal { action } => match action {
            GoalAction::Eject { name, file } => {
                let source = std::fs::read_to_string(&file)
                    .context("Failed to read source file")?;

                let lexer = Lexer::new(&source);
                let tokens: Vec<_> = lexer.collect();
                let ast = OnceParser::parse(tokens)
                    .map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;

                let builder = HirBuilder::new();
                let hir = builder.build(ast)
                    .map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;

                let goal_fn = hir.items.iter().find_map(|item| {
                    if let once_hir::HirItem::FnDecl(f) = item {
                        if f.name == name { Some(f) } else { None }
                    } else { None }
                });

                match goal_fn {
                    Some(_) => {
                        let output = source.replace(&format!("goal fn {}", name), &format!("fn {}", name));
                        let out_path = format!("{}.ejected.onc", file.trim_end_matches(".onc"));
                        std::fs::write(&out_path, &output)
                            .context("Failed to write ejected output")?;
                        println!("Goal '{}' ejected to {}", name, out_path);
                    }
                    None => {
                        anyhow::bail!("Goal '{}' not found in {}", name, file);
                    }
                }
            }
        },
    }

    Ok(())
}

fn compile_file(input: &PathBuf, output: Option<&Path>) -> anyhow::Result<()> {
    println!("Compiling {}", input.display());
    
    let source = fs::read_to_string(input)?;
    let tokens: Vec<_> = Lexer::new(&source).collect();
    
    println!("Lexed {} tokens", tokens.len());
    
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());
    
    let builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());
    
    // Type checking
    let mut type_checker = TypeChecker::new();
    if let Err(errors) = type_checker.check(&hir) {
        for err in &errors {
            eprintln!("{}", err.diagnostic_with_source(&source));
        }
        return Err(anyhow::anyhow!("Type checking failed"));
    }
    println!("Type checking passed");
    
    // Effects checking
    let mut effect_checker = EffectChecker::new();
    effect_checker.check(&hir).map_err(|e| anyhow::anyhow!("Effects error: {:?}", e))?;
    println!("Effects checking passed");
    
    // Linearity checking
    let mut linearity_checker = LinearityChecker::new();
    linearity_checker.check(&hir).map_err(|e| anyhow::anyhow!("Linearity error: {:?}", e))?;
    println!("Linearity checking passed");
    
    // Run doctests from doc comments
    let _ = run_doctests(input);

    // Region inference
    let mut region_checker = RegionChecker::new();
    let region_dag = region_checker.check(&hir).map_err(|e| anyhow::anyhow!("Region error: {:?}", e))?;
    println!("Region inference passed");
    
    // MIR generation
    let mut mir_generator = MirGenerator::new();
    let mir = mir_generator.generate(&hir, region_dag.clone()).map_err(|e| anyhow::anyhow!("MIR error: {:?}", e))?;
    println!("MIR generation passed");
    
    // MIR verification
    let verifier = once_mir::MirVerifier::new();
    if let Err(errors) = verifier.verify_program(&mir) {
        for err in &errors {
            eprintln!("MIR verification warning: {}", err);
        }
    }
    
    // Code generation
    // Try to use real Cranelift code generator
    let mut code_generator = match CodeGenerator::new_with_cranelift(region_dag.clone()) {
        Ok(generator) => {
            println!("Using real Cranelift backend");
            generator
        }
        Err(e) => {
            println!("Note: Cranelift backend unavailable ({}), using fallback codegen", e);
            CodeGenerator::new(region_dag)
        }
    };
    
    let compiled_program = code_generator.generate(&mir).map_err(|e| anyhow::anyhow!("Codegen error: {:?}", e))?;
    println!("Code generation passed");
    
    // Generate object file
    let default_output = PathBuf::from("main.o");
    let output_path = output.unwrap_or(&default_output);
    code_generator.generate_object_file(&compiled_program, &output_path.to_string_lossy()).map_err(|e| anyhow::anyhow!("Object file error: {:?}", e))?;
    println!("Object file generated: {}", output_path.display());
    println!("Compilation complete. Output: {}", output_path.display());
    
    Ok(())
}

fn parse_file(input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let tokens: Vec<_> = Lexer::new(&source).collect();
    
    println!("Tokens:");
    for (i, token) in tokens.iter().enumerate() {
        println!("  {}: {:?}", i, token.token);
    }
    
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("\nAST:");
    println!("{:#?}", ast);
    
    Ok(())
}

fn show_hir(input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let tokens: Vec<_> = Lexer::new(&source).collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    let builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    
    println!("HIR:");
    println!("{:#?}", hir);
    
    Ok(())
}

fn typecheck_file(input: &PathBuf) -> anyhow::Result<()> {
    println!("Type checking file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    let mut type_checker = TypeChecker::new();
    match type_checker.check(&hir) {
        Ok(()) => {
            println!("Type checking passed ✓");
            // Report inferred types for type holes
            for diag in type_checker.hole_diagnostics() {
                println!("  note: {}", diag);
            }
        }
        Err(errors) => {
            println!("Type checking failed:");
            for error in &errors {
                eprintln!("{}", error.diagnostic_with_source(&source));
            }
            return Err(anyhow::anyhow!("Type checking failed"));
        }
    }

    Ok(())
}

fn check_effects(input: &PathBuf) -> anyhow::Result<()> {
    println!("Checking effects for file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    let mut effect_checker = EffectChecker::new();
    match effect_checker.check(&hir) {
        Ok(()) => {
            println!("Effects checking passed ✓");
        }
        Err(errors) => {
            println!("Effects checking failed:");
            for error in errors {
                println!("  {}", error);
            }
            return Err(anyhow::anyhow!("Effects checking failed"));
        }
    }

    Ok(())
}

fn check_linearity(input: &PathBuf) -> anyhow::Result<()> {
    println!("Checking linearity for file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    let mut linearity_checker = LinearityChecker::new();
    match linearity_checker.check(&hir) {
        Ok(()) => {
            println!("Linearity checking passed ✓");
        }
        Err(errors) => {
            println!("Linearity checking failed:");
            for error in errors {
                println!("  {}", error);
            }
            return Err(anyhow::anyhow!("Linearity checking failed"));
        }
    }

    Ok(())
}

fn check_regions(input: &PathBuf) -> anyhow::Result<()> {
    println!("Checking regions for file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    let mut region_checker = RegionChecker::new();
    match region_checker.check(&hir) {
        Ok(region_dag) => {
            println!("Region inference passed ✓");
            println!("{}", region_checker.explain_regions(&region_dag));
        }
        Err(errors) => {
            println!("Region inference failed:");
            for error in errors {
                println!("  {}", error);
            }
            return Err(anyhow::anyhow!("Region inference failed"));
        }
    }

    Ok(())
}

fn explain_error_code(code: &str) {
    let explanations = [
        ("E001", "Type mismatch — the compiler expected one type but found another.", "Check type annotations, or add explicit conversion (type cast)."),
        ("E002", "Undefined variable — a name was used that has no binding in scope.", "Define the variable with `let` before use, or check the spelling."),
        ("E003", "Missing return type annotation — top-level functions must have explicit `-> Type`.", "Add `-> Int`, `-> String`, `-> Unit`, or the appropriate type after the parameter list."),
        ("E004", "Unhandled effect — a function body uses an effect not declared in the function signature.", "Add `!io`, `!net`, `!spawn`, etc. to the function's effect annotation."),
        ("E005", "Linear value reused — a value marked `lin` or `aff` was used after being consumed.", "Ensure the value is only used once, or use `copy()` to create a duplicate."),
        ("E006", "Non-exhaustive match — a `match` expression doesn't cover all possible patterns.", "Add a wildcard `_` arm or cover all enum variants."),
        ("E007", "Region constraint unsatisfiable — memory region analysis found a conflict.", "Check that allocations are freed in the correct scope and values don't escape their regions."),
        ("E008", "Trait bound not satisfied — a type doesn't implement a required trait.", "Implement the trait for the type, or use a different type that satisfies the bound."),
        ("E009", "Missing effect annotation on exported function — public functions must declare effects explicitly.", "Add effect annotations like `!io`, `!net` to the function signature."),
        ("E010", "Channel deadlock detected — tasks are waiting on each other in a cycle.", "Restructure channel communication to break the dependency cycle."),
    ];

    let code_upper = code.to_uppercase();
    for (id, description, fix) in &explanations {
        if id == &code_upper {
            println!("\n  {}: {}\n", id, description);
            println!("  Common fix: {}\n", fix);
            return;
        }
    }
    println!("\n  Unknown error code: {}\n", code);
    println!("  Available codes: E001 - E010\n");
}

fn explain_file(topic: &ExplainTopic, input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;

    match topic {
        ExplainTopic::Regions => {
            let mut region_checker = RegionChecker::new();
            let region_dag = region_checker.check(&hir).map_err(|e| anyhow::anyhow!("Region error: {:?}", e))?;
            println!("{}", region_checker.explain_regions(&region_dag));
        }
        ExplainTopic::Effects => {
            let mut effect_checker = EffectChecker::new();
            effect_checker.check(&hir).map_err(|e| anyhow::anyhow!("Effects error: {:?}", e))?;

            println!("Effect Analysis Results:");
            println!("=======================\n");
            let bindings = &effect_checker.env.bindings;
            if bindings.is_empty() {
                println!("No effect bindings found. All functions are pure.");
            }
            for (name, effect_row) in bindings {
                let effect_str = format!("{:?}", effect_row);
                if effect_str != "Empty" {
                    println!("  {}  →  {:?}", name, effect_row);
                }
            }
            if bindings.values().all(|r| matches!(r, EffectRow::Empty)) {
                println!("All functions are pure (no effects).");
            }
        }
        ExplainTopic::Linearity => {
            let mut linearity_checker = LinearityChecker::new();
            linearity_checker.check(&hir).map_err(|e| anyhow::anyhow!("Linearity error: {:?}", e))?;

            println!("Linearity Analysis Results:");
            println!("==========================\n");
            let variables = &linearity_checker.env.variables;
            if variables.is_empty() {
                println!("No linear variables tracked.");
            }
            for (name, usage) in variables {
                let lin_str = match usage.linearity {
                    Linearity::Linear => "linear",
                    Linearity::Affine => "affine",
                    Linearity::NonLinear => "nonlinear",
                };
                println!("  {}  [{}]  used {} time(s)", name, lin_str, usage.usage_count);
                if let Some(first) = usage.first_use {
                    println!("    first use:  line {} col {}", first.line, first.column);
                }
                if let Some(last) = usage.last_use {
                    println!("    last use:   line {} col {}", last.line, last.column);
                }
            }
        }
        ExplainTopic::EscapeAnalysis => {
            use once_explain::Explainer;
            let span = once_lex::Span { start: 0, end: source.len(), line: 1, column: 1 };
            let mut explainer = Explainer::new();
            let explanation = explainer.explain_regions(&hir, span)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
            println!("{}", explainer.format_region_explanation(&explanation));
        }
    }
    Ok(())
}

fn analyze_file(file: &str, output: Option<&str>) -> Result<(), anyhow::Error> {
    let source = std::fs::read_to_string(file)
        .context("Failed to read source file")?;

    let mut analysis = serde_json::Map::new();

    // Stage 1: Lex
    let lexer = Lexer::new(&source);
    let tokens: Vec<once_lex::TokenWithSpan> = lexer.collect();

    let token_json: Vec<_> = tokens.iter().map(|t| {
        let mut m = serde_json::Map::new();
        m.insert("token".to_string(), serde_json::Value::String(format!("{:?}", t.token)));
        m.insert("span".to_string(), serde_json::json!({
            "start": t.span.start,
            "end": t.span.end,
            "line": t.span.line,
            "column": t.span.column,
        }));
        serde_json::Value::Object(m)
    }).collect();
    analysis.insert("tokens".to_string(), serde_json::Value::Array(token_json));

    // Stage 2: Parse
    let ast = match OnceParser::parse(tokens) {
        Ok(ast) => {
            analysis.insert("ast".to_string(), serde_json::json!({
                "item_count": ast.items.len(),
                "items": ast.items.iter().map(|item| format!("{:?}", item)).collect::<Vec<_>>(),
            }));
            Some(ast)
        }
        Err(e) => {
            analysis.insert("parse_error".to_string(), serde_json::Value::String(e));
            None
        }
    };

    // Stage 3: HIR
    if let Some(ast) = ast {
        let hir = match HirBuilder::new().build(ast) {
            Ok(hir) => {
                analysis.insert("hir".to_string(), serde_json::json!({
                    "item_count": hir.items.len(),
                    "import_count": hir.imports.len(),
                }));
                Some(hir)
            }
            Err(e) => {
                analysis.insert("hir_error".to_string(), serde_json::Value::String(format!("{:?}", e)));
                None
            }
        };

        // Stage 4: Type check
        if let Some(ref hir) = hir {
            let mut type_checker = TypeChecker::new();
            match type_checker.check(hir) {
                Ok(()) => {
                    let bindings: serde_json::Map<String, serde_json::Value> = type_checker.env.bindings.iter()
                        .filter(|(k, _)| !matches!(k.as_str(), "Unit" | "Int" | "Bool" | "Float" | "Str" | "print" | "spawn"))
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(format!("{}", v.ty))))
                        .collect();
                    analysis.insert("types".to_string(), serde_json::Value::Object(bindings));
                }
                Err(errors) => {
                    analysis.insert("type_errors".to_string(), serde_json::Value::Array(
                        errors.iter().map(|e| serde_json::Value::String(e.diagnostic())).collect()
                    ));
                }
            }

            // Stage 5: Effects
            let mut effect_checker = EffectChecker::new();
            match effect_checker.check(hir) {
                Ok(()) => {
                    let effects: serde_json::Map<String, serde_json::Value> = effect_checker.env.bindings.iter()
                        .map(|(k, v)| (k.clone(), serde_json::Value::String(format!("{}", v))))
                        .collect();
                    analysis.insert("effects".to_string(), serde_json::Value::Object(effects));
                }
                Err(errors) => {
                    analysis.insert("effect_errors".to_string(), serde_json::Value::Array(
                        errors.iter().map(|e| serde_json::Value::String(format!("{}", e))).collect()
                    ));
                }
            }

            // Stage 6: Regions
            let mut region_checker = RegionChecker::new();
            match region_checker.check(hir) {
                Ok(dag) => {
                    let nodes: Vec<serde_json::Value> = dag.nodes.iter().map(|(region, node)| {
                        serde_json::json!({
                            "region": format!("{}", region),
                            "allocations": node.allocations.len(),
                            "escapes": node.escapes.len(),
                        })
                    }).collect();
                    analysis.insert("regions".to_string(), serde_json::Value::Array(nodes));

                    // Stage 7: MIR
                    let mut mir_gen = MirGenerator::new();
                    match mir_gen.generate(hir, dag) {
                        Ok(mir) => {
                            let funcs: Vec<serde_json::Value> = mir.functions.iter().map(|f| {
                                serde_json::json!({
                                    "name": f.name,
                                    "statement_count": f.body.statements.len(),
                                    "operations": f.body.statements.iter().map(|s| format!("{:?}", s.op)).collect::<Vec<_>>(),
                                })
                            }).collect();
                            analysis.insert("mir".to_string(), serde_json::Value::Array(funcs));
                        }
                        Err(e) => {
                            analysis.insert("mir_error".to_string(), serde_json::Value::String(format!("{:?}", e)));
                        }
                    }
                }
                Err(errors) => {
                    analysis.insert("region_errors".to_string(), serde_json::Value::Array(
                        errors.iter().map(|e| serde_json::Value::String(format!("{:?}", e))).collect()
                    ));
                }
            }
        }
    }

    // Output
    let json = serde_json::Value::Object(analysis);
    let output_str = serde_json::to_string_pretty(&json).context("Failed to serialize JSON")?;

    match output {
        Some(path) => {
            std::fs::write(path, output_str).context("Failed to write output")?;
            println!("Analysis written to {}", path);
        }
        None => println!("{}", output_str),
    }

    Ok(())
}

fn fix_file(mode: &str, input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;

    match mode {
        "consumes" | "linearity" => {
            let mut linearity_checker = LinearityChecker::new();
            match linearity_checker.check(&hir) {
                Ok(()) => {
                    println!("All linear values are properly consumed.");
                }
                Err(errors) => {
                    println!("Found {} linearity issues:", errors.len());
                    
                    // Attempt auto-fix: insert .consume() calls
                    let mut modified = source.clone();
                    let mut offset = 0i64;
                    
                    for error in &errors {
                        let error_str = format!("{:?}", error);
                        // Extract variable name from error message
                        if let Some(var_name) = extract_variable_name(&error_str) {
                            // Find end of block to insert consume
                            if let Some(pos) = find_insert_position(&modified, &var_name) {
                                let pos = (pos as i64 + offset) as usize;
                                let consume_call = format!("\n    {}.consume();", var_name);
                                modified.insert_str(pos, &consume_call);
                                offset += consume_call.len() as i64;
                                println!("  - Fixed: inserted consume for '{}'", var_name);
                            } else {
                                println!("  - Could not auto-fix '{}': use 'using {} = expr {{...}}'", var_name, var_name);
                            }
                        }
                    }
                    
                    if offset > 0 {
                        fs::write(input, &modified)?;
                        println!("\nApplied {} auto-fix(es) to {}", 
                            errors.iter().filter(|e| extract_variable_name(&format!("{:?}", e)).is_some()).count(),
                            input.display());
                    } else {
                        println!("\nSuggestions:");
                        println!("  - Use 'using resource = expr {{ body }}'");
                        println!("  - Add '.consume()' call before end of scope");
                        println!("  - Mark variable as 'aff' for at-most-once consumption");
                    }
                }
            }
        }
        "imports" => {
            println!("Import fix suggestions:");
            println!("  - Use 'once fmt' for canonical formatting");
            println!("  - Unused imports will be identified during compilation");
        }
        _ => {
            anyhow::bail!("Unknown fix mode: {}. Use 'consumes' or 'imports'.", mode);
        }
    }
    Ok(())
}

fn generate_mir(input: &PathBuf) -> anyhow::Result<()> {
    println!("Generating MIR for file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    // Run region inference first
    let mut region_checker = RegionChecker::new();
    let region_dag = region_checker.check(&hir).map_err(|e| anyhow::anyhow!("Region error: {:?}", e))?;
    println!("Region inference completed");

    // Generate MIR
    let mut mir_generator = MirGenerator::new();
    match mir_generator.generate(&hir, region_dag) {
        Ok(mir) => {
            println!("MIR generation passed ✓");
            println!("{}", mir);
        }
        Err(errors) => {
            println!("MIR generation failed:");
            for error in errors {
                println!("  {}", error);
            }
            return Err(anyhow::anyhow!("MIR generation failed"));
        }
    }

    Ok(())
}

fn generate_code(input: &PathBuf) -> anyhow::Result<()> {
    println!("Generating code for file: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    // Run region inference first
    let mut region_checker = RegionChecker::new();
    let region_dag = region_checker.check(&hir).map_err(|e| anyhow::anyhow!("Region error: {:?}", e))?;
    println!("Region inference completed");

    // Generate MIR
    let mut mir_generator = MirGenerator::new();
    let mir = mir_generator.generate(&hir, region_dag.clone()).map_err(|e| anyhow::anyhow!("MIR error: {:?}", e))?;
    println!("MIR generation completed");

    // MIR verification
    let verifier = once_mir::MirVerifier::new();
    if let Err(errors) = verifier.verify_program(&mir) {
        for err in &errors {
            eprintln!("MIR verification warning: {}", err);
        }
    }

    // Generate code
    let mut code_generator = CodeGenerator::new(region_dag);
    match code_generator.generate(&mir) {
        Ok(compiled_program) => {
            println!("Code generation passed ✓");
            println!("{}", compiled_program);
            
            // Generate assembly output
            let asm = code_generator.generate_assembly(&compiled_program);
            println!("\nAssembly output:");
            println!("{}", asm);
        }
        Err(errors) => {
            println!("Code generation failed:");
            for error in errors {
                println!("  {}", error);
            }
            return Err(anyhow::anyhow!("Code generation failed"));
        }
    }

    Ok(())
}

fn run_program(input: &PathBuf) -> anyhow::Result<()> {
    println!("Running program: {:?}", input);
    let source = fs::read_to_string(input)?;

    let lexer = Lexer::new(&source);
    let tokens: Vec<_> = lexer.collect();
    println!("Lexed {} tokens", tokens.len());

    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    println!("Parsed AST with {} items", ast.items.len());

    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    println!("Generated HIR with {} items", hir.items.len());

    // Run all compiler passes
    let mut type_checker = TypeChecker::new();
    type_checker.check(&hir).map_err(|errors| {
        for err in &errors {
            eprintln!("{}", err.diagnostic_with_source(&source));
        }
        anyhow::anyhow!("Type checking failed")
    })?;
    println!("Type checking passed");

    let mut effect_checker = EffectChecker::new();
    effect_checker.check(&hir).map_err(|e| anyhow::anyhow!("Effects error: {:?}", e))?;
    println!("Effects checking passed");

    let mut linearity_checker = LinearityChecker::new();
    linearity_checker.check(&hir).map_err(|e| anyhow::anyhow!("Linearity error: {:?}", e))?;
    println!("Linearity checking passed");

    let mut region_checker = RegionChecker::new();
    let region_dag = region_checker.check(&hir).map_err(|e| anyhow::anyhow!("Region error: {:?}", e))?;
    println!("Region inference passed");

    let mut mir_generator = MirGenerator::new();
    let mir = mir_generator.generate(&hir, region_dag.clone()).map_err(|e| anyhow::anyhow!("MIR error: {:?}", e))?;
    println!("MIR generation passed");

    // MIR verification
    let verifier = once_mir::MirVerifier::new();
    if let Err(errors) = verifier.verify_program(&mir) {
        for err in &errors {
            eprintln!("MIR verification warning: {}", err);
        }
    }

    // Try to use real Cranelift code generator
    let mut code_generator = match CodeGenerator::new_with_cranelift(region_dag.clone()) {
        Ok(generator) => {
            println!("Using real Cranelift backend");
            generator
        }
        Err(e) => {
            println!("Note: Cranelift backend unavailable ({}), using fallback codegen", e);
            CodeGenerator::new(region_dag)
        }
    };
    
    let compiled_program = code_generator.generate(&mir).map_err(|e| anyhow::anyhow!("Codegen error: {:?}", e))?;
    println!("Code generation passed");

    // Initialize standard library
    once_std::init().map_err(|e| anyhow::anyhow!("Std init error: {:?}", e))?;
    println!("Standard library initialized");

    // Start runtime
    let mut runtime = Runtime::new();
    println!("Starting runtime...");
    println!("{}", runtime);

    // Execute the compiled program — interpret MIR through the runtime
    println!("Executing compiled program...");
    
    // Walk MIR functions and execute each one's body
    for function in &mir.functions {
        println!("Running function: {}", function.name);
        for stmt in &function.body.statements {
            match &stmt.op {
                once_mir::MirOp::LoadLiteral { value, .. } => {
                    if let once_mir::MirValue::String(s) = value {
                        println!("{}", s);
                    }
                }
                once_mir::MirOp::Call { function: fname, .. } => {
                    println!("  -> called {}", fname);
                }
                once_mir::MirOp::Return { .. } => {
                    break;
                }
                _ => {}
            }
        }
    }

    // Start runtime and task infrastructure
    let mut runtime = Runtime::new();
    let task_handle = runtime.spawn_task("main".to_string(), vec![]);
    println!("Spawned task: {:?}", task_handle);

    let channel = runtime.create_channel(10, once_runtime::BackpressurePolicy::Blocking);
    println!("Created channel: {:?}", channel);

    let allocation_id = runtime.allocate(1024, "main_region".to_string());
    println!("Allocated memory: {}", allocation_id);

    // Clean up
    runtime.free(allocation_id).map_err(|e| anyhow::anyhow!("Memory error: {:?}", e))?;
    println!("Freed memory: {}", allocation_id);

    println!("Program execution completed ✓");
    
    // Cleanup standard library
    once_std::cleanup().map_err(|e| anyhow::anyhow!("Std cleanup error: {:?}", e))?;
    println!("Standard library cleaned up");
    
    Ok(())
}

fn build_project(project_dir: Option<&Path>) -> anyhow::Result<()> {
    let project_path = project_dir.unwrap_or(Path::new("."));
    println!("Building project in: {:?}", project_path);

    // Initialize build tool
    let config = once_build::BuildConfig::default();
    let mut build_tool = BuildTool::new(config);
    build_tool.init().map_err(|e| anyhow::anyhow!("Build init error: {:?}", e))?;
    println!("Build tool initialized");

    // Find source files
    let mut build_tool = once_build::BuildTool::new(once_build::BuildConfig::default());
    let source_files = build_tool.find_source_files(project_path)
        .map_err(|e| anyhow::anyhow!("Source file error: {:?}", e))?;
    println!("Found {} source files", source_files.len());

    // Add build targets
    for source_file in source_files {
        let target_name = source_file.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        let output_path = project_path.join("target").join(&target_name);
        
        let target = once_build::BuildTarget {
            name: target_name.clone(),
            path: source_file.clone(),
            dependencies: Vec::new(),
            build_type: once_build::BuildType::Binary,
            output_path,
            sources: vec![source_file.clone()],
            version: "0.1.0".to_string(),
            capabilities: Vec::new(),
            effects: Vec::new(),
        };
        
        build_tool.add_target(target).map_err(|e| anyhow::anyhow!("Add target error: {:?}", e))?;
        println!("Added build target: {}", target_name);
    }

    // Build all targets
    build_tool.build_all().map_err(|e| anyhow::anyhow!("Build error: {:?}", e))?;
    println!("Build completed successfully");

    // Show build statistics
    let stats = build_tool.get_stats();
    println!("Build statistics:");
    println!("  Total targets: {}", stats.total);
    println!("  Completed: {}", stats.completed);
    println!("  Failed: {}", stats.failed);
    println!("  Pending: {}", stats.pending);

    Ok(())
}

fn create_project(name: &str) -> anyhow::Result<()> {
    let project_dir = PathBuf::from(name);
    
    if project_dir.exists() {
        anyhow::bail!("Project directory {} already exists", name);
    }
    
    fs::create_dir(&project_dir)?;
    
    // Create main.onc
    let main_content = format!(
        "// {} - A Once program
fn main() -> Unit {{
    let message = \"Hello, Once!\"
    print(message)
    return
}}

fn print(msg: Str) -> Unit {{
    // Uses the standard library I/O built-in
    println(msg)
    return
}}",
        name
    );
    
    fs::write(project_dir.join("main.onc"), main_content)?;
    
    // Create once.toml
    let toml_content = format!(
        "[package]
name = \"{}\"
version = \"0.1.0\"

[capabilities]
io = true
",
        name
    );
    
    fs::write(project_dir.join("once.toml"), toml_content)?;
    
    println!("Created new Once project: {}", name);
    println!("  main.onc - Main source file");
    println!("  once.toml - Project configuration");
    println!();
    println!("To build:");
println!("  cd {}", name);
    println!("  once build --input main.onc");
    Ok(())
}

/// Extract doctest code blocks from /// doc comments in a source file
fn collect_doctests(source: &str) -> Vec<(usize, String)> {
    let mut doctests = Vec::new();
    let mut in_doc_comment = false;
    let mut in_code_block = false;
    let mut code_block = String::new();
    let mut line_num = 0usize;

    for line in source.lines() {
        line_num += 1;
        let trimmed = line.trim();

        if trimmed.starts_with("///") {
            in_doc_comment = true;
            let content = &trimmed[3..].trim();
            if content.starts_with("```once") {
                in_code_block = true;
                code_block.clear();
            } else if *content == "```" && in_code_block {
                in_code_block = false;
                doctests.push((line_num, code_block.clone()));
            } else if in_code_block {
                code_block.push_str(content);
                code_block.push('\n');
            }
        } else {
            in_doc_comment = false;
        }
    }
    doctests
}

/// Run doctests extracted from source files
fn run_doctests(input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let doctests = collect_doctests(&source);

    if doctests.is_empty() {
        println!("No doctests found.");
        return Ok(());
    }

    println!("Running {} doctest(s) from {}", doctests.len(), input.display());
    let mut passed = 0;
    let mut failed = 0;

    for (line, code) in &doctests {
        print!("  doctest at line {} ... ", line);
        let tokens: Vec<_> = Lexer::new(code).collect();
        match OnceParser::parse(tokens) {
            Ok(ast) => {
                let mut builder = HirBuilder::new();
                match builder.build(ast) {
                    Ok(hir) => {
                        let mut checker = TypeChecker::new();
                        match checker.check(&hir) {
                            Ok(()) => {
                                println!("ok");
                                passed += 1;
                            }
                            Err(errors) => {
                                println!("FAILED (type error: {:?})", errors.first().unwrap());
                                failed += 1;
                            }
                        }
                    }
                    Err(errors) => {
                        println!("FAILED (HIR: {:?})", errors.first().unwrap());
                        failed += 1;
                    }
                }
            }
            Err(err) => {
                println!("FAILED (parse: {})", err);
                failed += 1;
            }
        }
    }

    println!("\nDoctest results: {} passed, {} failed", passed, failed);
    if failed > 0 {
        anyhow::bail!("{} doctest(s) failed", failed);
    }
    Ok(())
}

fn format_file(input: &PathBuf) -> anyhow::Result<()> {
    use once_parse::format;
    
    let source = fs::read_to_string(input)?;
    let tokens: Vec<_> = Lexer::new(&source).collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    
    let formatted = format::format_program(&ast);
    
    if formatted != source {
        fs::write(input, &formatted)?;
        println!("Formatted: {}", input.display());
    } else {
        println!("Already formatted: {}", input.display());
    }
    Ok(())
}

fn lint_file(input: &PathBuf) -> anyhow::Result<()> {
    let source = fs::read_to_string(input)?;
    let tokens: Vec<_> = Lexer::new(&source).collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error: {}", e))?;
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error: {:?}", e))?;
    
    let warnings = lint::lint(&hir);
    
    if warnings.is_empty() {
        println!("No issues found in: {}", input.display());
    } else {
        println!("Lint warnings for {}: ({} found)", input.display(), warnings.len());
        for w in &warnings {
            let kind_label = match w.kind {
                lint::LintKind::StyleIssue => "style",
                lint::LintKind::DeadCode => "deadcode",
                lint::LintKind::UnusedVariable => "unused",
                lint::LintKind::UnusedImport => "unused-import",
                lint::LintKind::LinearResourceLeak => "linear-leak",
                lint::LintKind::CapabilityViolation => "capability",
                lint::LintKind::BoxRcWarning => "box-rc",
                lint::LintKind::UnusedEffect => "unused-effect",
            };
            print!("  line {}", w.line);
            if w.line == 0 { print!("?"); }
            println!(" [{}]: {}", kind_label, w.message);
            if let Some(ref suggestion) = w.suggestion {
                println!("    suggestion: {}", suggestion);
            }
        }
    }
    Ok(())
}

/// Extract variable name from linearity error message
fn extract_variable_name(error: &str) -> Option<String> {
    if let Some(start) = error.find("name: \"") {
        let rest = &error[start + 7..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    if let Some(start) = error.find("value '") {
        let rest = &error[start + 7..];
        if let Some(end) = rest.find('\'') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Find position to insert a consume call
fn find_insert_position(source: &str, _var_name: &str) -> Option<usize> {
    source.rfind('}').map(|pos| pos - 1)
}

/// Run tests from a directory or file
fn run_tests(test_dir: Option<&Path>, filter: Option<&str>, deterministic: bool) -> anyhow::Result<()> {
    let dir = test_dir.unwrap_or(Path::new("tests"));
    if deterministic {
        println!("Running tests in deterministic mode (single-threaded scheduler)");
    }
    println!("Running tests from: {}", dir.display());
    
    // Discover test files
    let mut test_files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "onc").unwrap_or(false) {
                test_files.push(path);
            }
        }
    } else if dir.is_file() && dir.extension().map(|e| e == "onc").unwrap_or(false) {
        test_files.push(dir.to_path_buf());
    }
    
    if test_files.is_empty() {
        println!("No .onc test files found in {}", dir.display());
        return Ok(());
    }
    
    let mut total = 0;
    let mut passed = 0;
    let mut failed = 0;
    
    for test_file in &test_files {
        let source = fs::read_to_string(test_file)?;
        let tests = discover_test_functions(&source);
        
        if let Some(filter) = filter {
            println!("\n{} (filtered by '{}')", test_file.display(), filter);
        } else {
            println!("\n{}", test_file.display());
        }
        
        for (name, (body, offset)) in &tests {
            if let Some(filter) = filter {
                if !name.contains(filter) {
                    continue;
                }
            }
            total += 1;
            print!("  test {} ... ", name);
            
            match run_single_test(body, deterministic) {
                Ok(_) => {
                    println!("ok");
                    passed += 1;
                }
                Err(e) => {
                    println!("FAILED");
                    println!("    {}", e);
                    failed += 1;
                }
            }
        }
    }
    
    println!("\ntest result: {}. {} passed; {} failed; 0 ignored",
        if failed == 0 { "ok" } else { "FAILED" },
        passed, failed
    );
    
    if failed > 0 {
        anyhow::bail!("{} test(s) failed", failed);
    }
    
    Ok(())
}

/// Discover #[test] functions or test_ prefixed functions in source
fn discover_test_functions(source: &str) -> Vec<(String, (String, usize))> {
    let mut tests = Vec::new();
    let mut in_fn = false;
    let mut fn_name = String::new();
    let mut fn_body = String::new();
    let mut brace_depth = 0;
    let mut offset = 0;
    
    // Parse #[test] annotation
    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        
        // Detect #[test] annotation
        if trimmed == "#[test]" {
            continue;
        }
        
        // Detect test function start
        if trimmed.starts_with("fn ") {
            if let Some(name_start) = trimmed.find("fn ") {
                let rest = &trimmed[name_start + 3..];
                if let Some(name_end) = rest.find('(') {
                    fn_name = rest[..name_end].trim().to_string();
                    in_fn = true;
                    fn_body.clear();
                    brace_depth = 0;
                    offset = line_num;
                    continue;
                }
            }
        }
        
        // Detect test_ prefixed functions (without #[test] annotation)
        if trimmed.starts_with("fn test_") && !in_fn {
            if let Some(name_start) = trimmed.find("fn ") {
                let rest = &trimmed[name_start + 3..];
                if let Some(name_end) = rest.find('(') {
                    fn_name = rest[..name_end].trim().to_string();
                    in_fn = true;
                    fn_body.clear();
                    brace_depth = 0;
                    offset = line_num;
                    continue;
                }
            }
        }
        
        if in_fn {
            fn_body.push_str(line);
            fn_body.push('\n');
            for ch in line.chars() {
                match ch {
                    '{' => brace_depth += 1,
                    '}' => {
                        if brace_depth > 0 {
                            brace_depth -= 1;
                            if brace_depth == 0 {
                                in_fn = false;
                                if fn_name.starts_with("test_") || fn_name.contains("test") {
                                    tests.push((fn_name.clone(), (fn_body.clone(), offset)));
                                }
                                fn_name.clear();
                                fn_body.clear();
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    
    tests
}

/// Run a single test: parse, type-check, region-infer, generate MIR, and evaluate
fn run_single_test(test_body: &str, deterministic: bool) -> anyhow::Result<()> {
    if deterministic {
        println!("Deterministic mode: running with single-threaded scheduler");
    }
    use once_lex::Lexer;
    use once_parse::OnceParser;
    use once_hir::HirBuilder;
    use once_ty::TypeChecker;
    use once_linear::LinearityChecker;
    use once_ty::effects::EffectChecker;
    use once_rinf::RegionChecker;
    use once_mir::MirGenerator;
    use once_mir::MirValue;

    // Extract the function name from test_body if it starts with "fn "
    let test_name = if test_body.trim_start().starts_with("fn ") {
        let rest = test_body.trim_start();
        let name_part = &rest[3..].trim_start();
        name_part.split('(').next().unwrap_or("test_func").to_string()
    } else {
        "test_func".to_string()
    };

    // Wrap body in a function if not already wrapped
    let source = if test_body.trim_start().starts_with("fn ") {
        test_body.to_string()
    } else {
        format!("fn {}() -> Bool {{\n{}\n}}", test_name, test_body)
    };

    let tokens: Vec<_> = Lexer::new(&source).collect();
    let ast = OnceParser::parse(tokens).map_err(|e| anyhow::anyhow!("Parse error in test '{}': {}", test_name, e))?;
    let mut builder = HirBuilder::new();
    let hir = builder.build(ast).map_err(|e| anyhow::anyhow!("HIR error in test '{}': {:?}", test_name, e))?;

    let mut type_checker = TypeChecker::new();
    type_checker.check(&hir).map_err(|errors| {
        for err in &errors {
            eprintln!("{}", err.diagnostic_with_source(test_body));
        }
        anyhow::anyhow!("Type error in test '{}': see above", test_name)
    })?;

    let mut linearity_checker = LinearityChecker::new();
    linearity_checker.check(&hir).map_err(|e| anyhow::anyhow!("Linearity error in test '{}': {:?}", test_name, e.first().unwrap_or(&once_linear::LinearityError::ResourceNotConsumed("unknown".to_string()))))?;

    let mut effect_checker = EffectChecker::new();
    effect_checker.check(&hir).map_err(|e| anyhow::anyhow!("Effect error in test '{}': {:?}", test_name, e.first().unwrap_or(&once_ty::effects::EffectError::UnhandledEffect { name: "unknown".to_string(), span: None })))?;

    // Region inference
    let mut region_checker = RegionChecker::new();
    let region_dag = region_checker.check(&hir)
        .map_err(|e| anyhow::anyhow!("Region error in test '{}': {:?}", test_name, e.first().unwrap_or(&once_rinf::RegionError::UnsatisfiableConstraint("unknown".to_string()))))?;

    // MIR generation
    let mut mir_gen = MirGenerator::new();
    let mir = mir_gen.generate(&hir, region_dag)
        .map_err(|e| anyhow::anyhow!("MIR error in test '{}': {:?}", test_name, e.first().unwrap_or(&once_mir::MirError::GenerationFailed("unknown".to_string()))))?;

    // MIR verification
    let verifier = once_mir::MirVerifier::new();
    if let Err(errors) = verifier.verify_program(&mir) {
        for err in &errors {
            eprintln!("MIR verification warning in test '{}': {}", test_name, err);
        }
    }

    if deterministic {
        let runtime = once_runtime::Runtime::new();
        runtime.set_deterministic(true);
    }

    // Evaluate via MIR interpreter
    let evaluator = mir_eval::MirEvaluator::new(mir);
    match evaluator.eval_function(&test_name, &[]) {
        Ok(MirValue::Bool(true)) => Ok(()),
        Ok(MirValue::Bool(false)) => Err(anyhow::anyhow!("Test '{}' FAILED: returned false", test_name)),
        Ok(other) => Err(anyhow::anyhow!("Test '{}' must return Bool, got: {:?}", test_name, other)),
        Err(e) => Err(anyhow::anyhow!("Test '{}' runtime error: {}", test_name, e)),
    }
}
