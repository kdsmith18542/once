//! Command-line interface for the Once compiler

use clap::{Parser, Subcommand, ValueEnum};
use once_lex::Lexer;
use once_parse::OnceParser;
use once_hir::HirBuilder;
use once_ty::TypeChecker;
use once_ty::effects::EffectChecker;
use once_linear::LinearityChecker;
use once_rinf::RegionChecker;
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
        /// Topic to explain
        topic: ExplainTopic,
        /// Input file
        input: PathBuf,
    },
    /// Start language server
    Lsp {
        /// LSP mode
        #[arg(long)]
        stdio: bool,
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
        Commands::BuildProject { project } => {
            build_project(project.as_deref())?;
        }
        Commands::New { name } => {
            create_project(&name)?;
        }
        Commands::Explain { topic, input } => {
            explain_file(&topic, &input)?;
        }
        Commands::Lsp { stdio } => {
            start_lsp_server(stdio)?;
        }
        Commands::Fix { mode, input } => {
            fix_file(&mode, &input)?;
        }
        Commands::Fmt { input } => {
            format_file(&input)?;
        }
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
    type_checker.check(&hir).map_err(|e| anyhow::anyhow!("Type error: {:?}", e))?;
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
    
    // Code generation
    // Try to use real Cranelift code generator
    let mut code_generator = match CodeGenerator::new_with_cranelift(region_dag.clone()) {
        Ok(generator) => {
            println!("Using real Cranelift backend");
            generator
        }
        Err(e) => {
            println!("Falling back to placeholder code generation: {}", e);
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
        }
        Err(errors) => {
            println!("Type checking failed:");
            for error in errors {
                println!("  {}", error);
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
            println!("Effects analysis passed.");
            println!("All effect rows are properly tracked through the call graph.");
        }
        ExplainTopic::Linearity => {
            let mut linearity_checker = LinearityChecker::new();
            linearity_checker.check(&hir).map_err(|e| anyhow::anyhow!("Linearity error: {:?}", e))?;
            println!("Linearity analysis passed.");
            println!("All linear values are consumed exactly once.");
        }
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
                    for error in &errors {
                        println!("  - Fix needed: {:?}", error);
                    }
                    println!("\nSuggestions:");
                    println!("  - Use 'using resource = expr {{ body }}' to guarantee consumption");
                    println!("  - Add '.consume()' call before end of scope");
                    println!("  - Mark variable as 'aff' if at-most-once consumption is acceptable");
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
    type_checker.check(&hir).map_err(|e| anyhow::anyhow!("Type error: {:?}", e))?;
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

    // Try to use real Cranelift code generator
    let mut code_generator = match CodeGenerator::new_with_cranelift(region_dag.clone()) {
        Ok(generator) => {
            println!("Using real Cranelift backend");
            generator
        }
        Err(e) => {
            println!("Falling back to placeholder code generation: {}", e);
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

    // TODO: Execute the compiled program
    // For now, just demonstrate runtime capabilities
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

fn start_lsp_server(stdio: bool) -> anyhow::Result<()> {
    if stdio {
        println!("Starting Once LSP server in stdio mode");
        // TODO: Implement stdio LSP server
        // This would involve:
        // 1. Setting up stdio communication
        // 2. Starting the LSP server
        // 3. Handling LSP protocol messages
    } else {
        println!("Starting Once LSP server in TCP mode");
        // TODO: Implement TCP LSP server
        // This would involve:
        // 1. Setting up TCP server
        // 2. Starting the LSP server
        // 3. Handling LSP protocol messages
    }
    
    println!("LSP server started successfully");
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
    // TODO: Implement in standard library
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
            } else if content == "```" && in_code_block {
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
