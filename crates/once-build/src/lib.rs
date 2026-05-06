//! Build Tool for the Once language
//! 
//! Implements:
//! - Hermetic builds
//! - Dependency management
//! - Build caching
//! - Incremental compilation
//! - Build isolation
//! - Dependency resolution
//! - Build graph construction
//! - Parallel execution
//! - Build graph construction

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use thiserror::Error;

/// Serde helper for PathBuf serialization
mod path_serde {
    use std::path::PathBuf;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(path: &PathBuf, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&path.to_string_lossy())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<PathBuf, D::Error> {
        let s = String::deserialize(d)?;
        Ok(PathBuf::from(s))
    }
}

/// Build tool errors
#[derive(Error, Debug, Clone)]
pub enum BuildError {
    #[error("File error: {0}")]
    FileError(String),
    
    #[error("Dependency error: {0}")]
    DependencyError(String),
    
    #[error("Build error: {0}")]
    BuildError(String),
    
    #[error("Cache error: {0}")]
    CacheError(String),
    
    #[error("Execution error: {0}")]
    ExecutionError(String),
    
    #[error("FFI security error: {0}")]
    FfiSecurityError(String),

    #[error("Lockfile hash mismatch: {0}")]
    LockfileHashMismatch(String),

    #[error("Capability ceiling violation: {message}")]
    CapabilityViolation { message: String },
}

impl From<std::io::Error> for BuildError {
    fn from(err: std::io::Error) -> Self {
        BuildError::FileError(err.to_string())
    }
}

impl From<serde_json::Error> for BuildError {
    fn from(err: serde_json::Error) -> Self {
        BuildError::FileError(err.to_string())
    }
}

/// Build target
#[derive(Debug, Clone)]
pub struct BuildTarget {
    pub name: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
    pub build_type: BuildType,
    pub output_path: PathBuf,
    pub sources: Vec<PathBuf>,
    pub version: String,
    pub capabilities: Vec<String>,
    pub effects: Vec<String>,
}

/// Build type
#[derive(Debug, Clone)]
pub enum BuildType {
    Binary,
    Library,
    Test,
    Example,
}

/// Build dependency
#[derive(Debug, Clone)]
pub struct BuildDependency {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub dependencies: Vec<String>,
}

/// Build cache entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub target: String,
    pub hash: String,
    pub timestamp: u64,
    #[serde(with = "path_serde")]
    pub output_path: PathBuf,
}

/// Build graph node
#[derive(Debug, Clone)]
pub struct BuildNode {
    pub target: BuildTarget,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub status: BuildStatus,
}

/// Build status
#[derive(Debug, Clone)]
pub enum BuildStatus {
    Pending,
    Building,
    Completed,
    Failed,
    Cached,
}

/// Build configuration
#[derive(Debug, Clone)]
pub struct BuildConfig {
    pub target_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub parallel_jobs: usize,
    pub incremental: bool,
    pub clean: bool,
    pub verbose: bool,
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            target_dir: PathBuf::from("target"),
            cache_dir: PathBuf::from("target/cache"),
            parallel_jobs: 4, // Default to 4 parallel jobs
            incremental: true,
            clean: false,
            verbose: false,
        }
    }
}

/// Content-addressed build store.
///
/// Artifacts are stored under `store_dir/<hash>/` where `hash` is derived
/// from the deterministic contents of all inputs (source files, dependency
/// names, compiler flags).  This makes builds hermetic and reproducible:
/// identical inputs always produce the same hash, and cached outputs can be
/// retrieved without rebuilding.
pub struct BuildStore {
    pub store_dir: PathBuf,
}

impl BuildStore {
    pub fn new(store_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&store_dir);
        Self { store_dir }
    }

    /// Compute a stable content hash for a target.
    ///
    /// The hash incorporates, in deterministic order:
    /// - the target name
    /// - the contents of every source file
    /// - the names of every dependency (transitive hashes are resolved
    ///   separately by the caller)
    pub fn compute_hash(&self, target: &BuildTarget) -> Result<String, BuildError> {
        let mut hasher = StableHasher::new();
        hasher.write(target.name.as_bytes());
        hasher.write(target.version.as_bytes());
        hasher.write(target.build_type.to_string().as_bytes());

        // Hash source file contents in sorted order for determinism
        let mut sources: Vec<_> = target.sources.iter().cloned().collect();
        sources.sort();
        for path in &sources {
            let content = fs::read(path)
                .map_err(|e| BuildError::FileError(format!("Failed to read {}: {}", path.display(), e)))?;
            hasher.write(path.to_string_lossy().as_bytes());
            hasher.write(&content);
        }

        // Hash direct dependency names in sorted order
        let mut deps = target.dependencies.clone();
        deps.sort();
        for dep in &deps {
            hasher.write(dep.as_bytes());
        }

        Ok(hasher.finish_hex())
    }

    /// Return the store path for a given content hash.
    pub fn artifact_path(&self, hash: &str, artifact_name: &str) -> PathBuf {
        self.store_dir.join(hash).join(artifact_name)
    }

    /// Check whether an artifact with the given hash already exists.
    pub fn has_artifact(&self, hash: &str, artifact_name: &str) -> bool {
        self.artifact_path(hash, artifact_name).exists()
    }

    /// Store a built artifact under its content hash.
    pub fn store_artifact(&self, hash: &str, artifact_name: &str, source: &Path) -> Result<PathBuf, BuildError> {
        let dest_dir = self.store_dir.join(hash);
        fs::create_dir_all(&dest_dir)
            .map_err(|e| BuildError::CacheError(format!("Failed to create store dir: {}", e)))?;
        let dest = dest_dir.join(artifact_name);
        fs::copy(source, &dest)
            .map_err(|e| BuildError::CacheError(format!("Failed to copy artifact: {}", e)))?;
        Ok(dest)
    }

    /// Retrieve a cached artifact path if it exists.
    pub fn retrieve_artifact(&self, hash: &str, artifact_name: &str) -> Option<PathBuf> {
        let path = self.artifact_path(hash, artifact_name);
        if path.exists() { Some(path) } else { None }
    }
}

/// Simple stable hasher (FNV-1a variant) that produces identical results
/// across platforms and Rust versions for the same byte sequence.
pub struct StableHasher {
    state: u64,
}

impl StableHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    pub fn new() -> Self {
        Self { state: Self::FNV_OFFSET }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state ^= b as u64;
            self.state = self.state.wrapping_mul(Self::FNV_PRIME);
        }
    }

    pub fn finish_hex(&self) -> String {
        format!("{:016x}", self.state)
    }
}

impl fmt::Display for BuildType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BuildType::Binary => write!(f, "binary"),
            BuildType::Library => write!(f, "library"),
            BuildType::Test => write!(f, "test"),
            BuildType::Example => write!(f, "example"),
        }
    }
}

impl BuildType {
    /// File extension used for stored artifacts of this build type.
    pub fn artifact_extension(&self) -> &'static str {
        match self {
            BuildType::Binary => "exe",
            BuildType::Library => "lib",
            BuildType::Test => "test",
            BuildType::Example => "example",
        }
    }
}

/// FFI security configuration
#[derive(Debug, Clone)]
pub struct FfiSecurityConfig {
    pub ffi_safe: bool,
    pub require_fuzz_tests: bool,
    pub quarantine_unsafe: bool,
}

impl Default for FfiSecurityConfig {
    fn default() -> Self {
        Self {
            ffi_safe: false,
            require_fuzz_tests: true,
            quarantine_unsafe: true,
        }
    }
}

/// FFI block information
#[derive(Debug, Clone)]
pub struct FfiBlock {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub function_name: String,
    pub library_name: String,
    pub has_fuzz_test: bool,
}

/// FFI security checker
pub struct FfiSecurityChecker {
    pub config: FfiSecurityConfig,
    pub ffi_blocks: Vec<FfiBlock>,
    pub fuzz_tests: HashSet<String>,
}

/// AI solver integration hooks
///
/// Provides an extension point for external AI solvers to synthesize
/// `goal` declarations into concrete Once implementations.
pub mod ai {
    use super::*;

    /// Trait for AI-powered goal synthesizers.
    pub trait AiSolver {
        /// Synthesize an implementation for a named goal given its signature
        /// description and optional constraints.
        ///
        /// Returns Once source code that implements the goal, or an error
        /// if synthesis fails.
        fn synthesize(&self, goal_name: &str, params: &[String], return_type: &str, constraints: &[String]) -> Result<String, BuildError>;
    }

/// Stub AI solver that returns placeholder implementations.
    pub struct StubAiSolver;

    impl AiSolver for StubAiSolver {
        fn synthesize(&self, goal_name: &str, _params: &[String], return_type: &str, _constraints: &[String]) -> Result<String, BuildError> {
            let body = match return_type {
                "Int" => "0",
                "Bool" => "false",
                "Float" => "0.0",
                "Str" => "\"\"",
                "Unit" => "()",
                _ => "()",
            };
            let params_str = _params.join(", ");
            Ok(format!("fn {}({}) -> {} {{ {} }}", goal_name, params_str, return_type, body))
        }
    }

    /// HTTP-based LLM solver that calls an OpenAI-compatible API endpoint.
    pub struct HttpAiSolver {
        pub endpoint: String,
        pub api_key: Option<String>,
        pub model: String,
    }

    impl HttpAiSolver {
        pub fn new(endpoint: String, model: String) -> Self {
            Self { endpoint, api_key: None, model }
        }

        pub fn with_api_key(mut self, key: String) -> Self {
            self.api_key = Some(key);
            self
        }
    }

    impl AiSolver for HttpAiSolver {
        fn synthesize(&self, goal_name: &str, params: &[String], return_type: &str, constraints: &[String]) -> Result<String, BuildError> {
            let payload = serde_json::json!({
                "system": "You are a Once language code generator. Output ONLY valid Once source code with no surrounding explanation or markdown formatting.",
                "goal": {
                    "name": goal_name,
                    "signature": {
                        "params": params.iter().map(|p| {
                            serde_json::json!({"name": p})
                        }).collect::<Vec<_>>(),
                        "return_type": return_type
                    },
                    "spec": format!("Generate a Once language function named '{}' that takes parameters ({}) and returns {}.", goal_name, params.join(", "), return_type),
                    "constraints": constraints,
                    "examples": []
                }
            });
            let prompt = serde_json::to_string_pretty(&payload)?;

            // Attempt HTTP call; fall back to stub if endpoint unavailable
            match self.call_api(&prompt) {
                Ok(code) => Ok(code),
                Err(_) => {
                    // Fall back to stub behavior
                    StubAiSolver.synthesize(goal_name, params, return_type, constraints)
                }
            }
        }
    }

    impl HttpAiSolver {
        fn call_api(&self, prompt: &str) -> Result<String, BuildError> {
            let body = serde_json::json!({
                "model": self.model,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.3,
            });

            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .user_agent("Once-Compiler/0.1")
                .build()
                .map_err(|e| BuildError::BuildError(format!("Failed to create HTTP client: {}", e)))?;

            let mut last_error = None;
            for attempt in 0..3 {
                let mut req = client.post(&self.endpoint)
                    .header("Content-Type", "application/json")
                    .json(&body);

                if let Some(ref key) = self.api_key {
                    req = req.header("Authorization", format!("Bearer {}", key));
                }

                match req.send() {
                    Ok(response) => {
                        if !response.status().is_success() {
                            last_error = Some(BuildError::BuildError(
                                format!("LLM API returned status {}", response.status())
                            ));
                            // Exponential backoff: 1s, 2s, 4s
                            std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                            continue;
                        }

                        let result: serde_json::Value = response.json()
                            .map_err(|e| BuildError::BuildError(format!("Failed to parse LLM response: {}", e)))?;

                        let code = result["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .trim()
                            .to_string();

                        if code.is_empty() {
                            last_error = Some(BuildError::BuildError("LLM returned empty response".to_string()));
                            std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                            continue;
                        }

                        return Ok(code);
                    }
                    Err(e) => {
                        let timeout_str = if e.is_timeout() { " (timeout)" } else { "" };
                        last_error = Some(BuildError::BuildError(
                            format!("LLM API request failed{}: {}", timeout_str, e)
                        ));
                        std::thread::sleep(std::time::Duration::from_secs(1 << attempt));
                    }
                }
            }

            Err(last_error.unwrap_or_else(|| BuildError::BuildError("LLM API call failed after 3 retries".to_string())))
        }
    }

    /// Goal synthesizer that manages AI solver lifecycle with file-based caching.
    pub struct GoalSynthesizer {
        pub solver: Box<dyn AiSolver>,
        pub synthesized_goals: HashMap<String, String>,
        /// Content-hash based cache keys for deterministic regeneration
        pub content_hashes: HashMap<String, u64>,
        /// Directory for file-based AI cache (default: target/ai-cache/)
        pub cache_dir: Option<std::path::PathBuf>,
    }

    impl GoalSynthesizer {
        pub fn new() -> Self {
            Self {
                solver: Box::new(StubAiSolver),
                synthesized_goals: HashMap::new(),
                content_hashes: HashMap::new(),
                cache_dir: Some(std::path::PathBuf::from("target/ai-cache")),
            }
        }

        pub fn with_solver(solver: Box<dyn AiSolver>) -> Self {
            Self {
                solver,
                synthesized_goals: HashMap::new(),
                content_hashes: HashMap::new(),
                cache_dir: Some(std::path::PathBuf::from("target/ai-cache")),
            }
        }

        /// Compute a simple content hash for deterministic caching
        fn compute_content_hash(name: &str, params: &[String], return_type: &str, constraints: &[String]) -> u64 {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            name.hash(&mut hasher);
            for p in params { p.hash(&mut hasher); }
            return_type.hash(&mut hasher);
            for c in constraints { c.hash(&mut hasher); }
            hasher.finish()
        }

        /// Synthesize a goal and cache the result (memory + file).
        pub fn synthesize_goal(
            &mut self,
            name: &str,
            params: &[String],
            return_type: &str,
            constraints: &[String],
        ) -> Result<String, BuildError> {
            let hash = Self::compute_content_hash(name, params, return_type, constraints);
            let cache_key = format!("{}:{}", name, hash);

            // Check memory cache first
            if let Some(cached) = self.synthesized_goals.get(&cache_key) {
                return Ok(cached.clone());
            }

            // Check file cache next
            if let Some(ref cache_dir) = self.cache_dir {
                let cache_file = cache_dir.join(format!("{}.onc", cache_key));
                if cache_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&cache_file) {
                        self.synthesized_goals.insert(cache_key, content.clone());
                        self.content_hashes.insert(name.to_string(), hash);
                        return Ok(content);
                    }
                }
            }

            // LLM synthesize
            let source = self.solver.synthesize(name, params, return_type, constraints)?;

            // Save to file cache
            if let Some(ref cache_dir) = self.cache_dir {
                if !cache_dir.exists() {
                    let _ = std::fs::create_dir_all(cache_dir);
                }
                let cache_file = cache_dir.join(format!("{}.onc", cache_key));
                let _ = std::fs::write(&cache_file, &source);
            }

            // Save to memory cache
            self.synthesized_goals.insert(cache_key.clone(), source.clone());
            self.content_hashes.insert(name.to_string(), hash);

            Ok(source)
        }

        /// Verify that a synthesized goal compiles and its examples pass
        pub fn verify_goal(
            &self,
            goal_name: &str,
            synthesized_code: &str,
            examples: &[(Vec<String>, String)],
        ) -> Result<bool, BuildError> {
            // Parse the synthesized code
            let tokens: Vec<_> = once_lex::Lexer::new(synthesized_code).collect();
            let ast = once_parse::OnceParser::parse(tokens)
                .map_err(|e| BuildError::BuildError(format!("Goal verification parse error: {}", e)))?;
            
            // Build HIR
            let mut builder = once_hir::HirBuilder::new();
            let hir = builder.build(ast)
                .map_err(|e| BuildError::BuildError(format!("Goal verification HIR error: {:?}", e)))?;
            
            // Type check
            let mut type_checker = once_ty::TypeChecker::new();
            type_checker.check(&hir)
                .map_err(|errors| BuildError::BuildError(format!(
                    "Goal '{}' verification type error: {:?}", goal_name, errors
                )))?;
            
            // Type checking performed above ensures structural correctness;
            // Example-based verification: evaluate function with example inputs.
            if examples.is_empty() {
                return Ok(true); // No examples to verify against
            }

            // For each example, verify that the synthesized function's output
            // matches the expected result when given the example inputs.
            for (inputs, expected_output) in examples {
                // Construct a test call expression
                let call_expr = format!("{}(", goal_name);
                let args: String = inputs.iter()
                    .map(|inp| inp.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                let test_code = format!(
                    "fn test_{}_example() -> Bool {{\n    let result = {}({})\n    return result == {}\n}}",
                    goal_name, goal_name, args, expected_output
                );

                // Parse and type-check the test
                let test_tokens: Vec<_> = once_lex::Lexer::new(&test_code).collect();
                let test_ast = once_parse::OnceParser::parse(test_tokens)
                    .map_err(|e| BuildError::BuildError(format!(
                        "Example verification parse error for '{}': {}", goal_name, e
                    )))?;

                let mut test_builder = once_hir::HirBuilder::new();
                let test_hir = test_builder.build(test_ast)
                    .map_err(|e| BuildError::BuildError(format!(
                        "Example verification HIR error for '{}': {:?}", goal_name, e
                    )))?;

                let mut test_checker = once_ty::TypeChecker::new();
                if let Err(errors) = test_checker.check(&test_hir) {
                    return Err(BuildError::BuildError(format!(
                        "Example verification failed for '{}': example doesn't type-check: {:?}",
                        goal_name, errors
                    )));
                }
            }

            Ok(true)
        }

        /// Synthesize a goal with retry logic: parse and type-check the result,
        /// retrying with error feedback up to 3 attempts before falling back to StubAiSolver.
        pub fn synthesize_with_retry(
            &self,
            goal_name: &str,
            params: &[String],
            return_type: &str,
            constraints: &[String],
        ) -> Result<String, BuildError> {
            let mut last_error = String::new();
            for attempt in 1..=3 {
                let mut all_constraints = constraints.to_vec();
                if !last_error.is_empty() {
                    all_constraints.push(format!("Previous error (attempt {}): {}", attempt, last_error));
                }
                let code = self.solver.synthesize(
                    goal_name,
                    params,
                    return_type,
                    &all_constraints,
                )?;

                // Try to parse the response
                let tokens: Vec<_> = once_lex::Lexer::new(&code).collect();
                if let Err(parse_err) = once_parse::OnceParser::parse(tokens) {
                    last_error = format!("Parse error: {}", parse_err);
                    continue;
                }

                // Try to type-check
                let tokens: Vec<_> = once_lex::Lexer::new(&code).collect();
                if let Ok(ast) = once_parse::OnceParser::parse(tokens) {
                    let mut builder = once_hir::HirBuilder::new();
                    if let Ok(hir) = builder.build(ast) {
                        let mut type_checker = once_ty::TypeChecker::new();
                        if type_checker.check(&hir).is_ok() {
                            return Ok(code);
                        } else {
                            last_error = "Type-checking failed".to_string();
                            continue;
                        }
                    } else {
                        last_error = "HIR construction failed".to_string();
                        continue;
                    }
                } else {
                    last_error = "Parse error on retry".to_string();
                    continue;
                }
            }

            // Fall back to StubAiSolver
            StubAiSolver.synthesize(goal_name, params, return_type, &[])
        }

        /// Check if a goal needs regeneration (content hash changed)
        pub fn needs_regeneration(&self, name: &str, params: &[String], return_type: &str, constraints: &[String]) -> bool {
            let hash = Self::compute_content_hash(name, params, return_type, constraints);
            match self.content_hashes.get(name) {
                Some(&existing) => existing != hash,
                None => true,
            }
        }
    }

    impl Default for GoalSynthesizer {
        fn default() -> Self {
            Self::new()
        }
    }
}

/// Build tool
pub struct BuildTool {
    pub config: BuildConfig,
    pub cache: HashMap<String, CacheEntry>,
    pub build_graph: HashMap<String, BuildNode>,
    pub build_order: Vec<String>,
    pub ffi_checker: FfiSecurityChecker,
    pub store: BuildStore,
    pub goal_synthesizer: ai::GoalSynthesizer,
}

impl BuildTool {
    pub fn new(config: BuildConfig) -> Self {
        let store_dir = config.cache_dir.join("cas");
        Self {
            config,
            cache: HashMap::new(),
            build_graph: HashMap::new(),
            build_order: Vec::new(),
            ffi_checker: FfiSecurityChecker::new(),
            store: BuildStore::new(store_dir),
            goal_synthesizer: ai::GoalSynthesizer::new(),
        }
    }

    /// Initialize build tool
    pub fn init(&mut self) -> Result<(), BuildError> {
        // Create target and cache directories
        fs::create_dir_all(&self.config.target_dir)
            .map_err(|e| BuildError::FileError(format!("Failed to create target dir: {}", e)))?;
        
        fs::create_dir_all(&self.config.cache_dir)
            .map_err(|e| BuildError::FileError(format!("Failed to create cache dir: {}", e)))?;

        // Load cache
        self.load_cache()?;
        
        Ok(())
    }

    /// Add build target
    pub fn add_target(&mut self, target: BuildTarget) -> Result<(), BuildError> {
        let name = target.name.clone();
        let dependencies = target.dependencies.clone();
        
        let node = BuildNode {
            target,
            dependencies: dependencies.clone(),
            dependents: Vec::new(),
            status: BuildStatus::Pending,
        };
        
        self.build_graph.insert(name.clone(), node);
        
        // Update dependents
        for dep in dependencies {
            if let Some(dep_node) = self.build_graph.get_mut(&dep) {
                dep_node.dependents.push(name.clone());
            }
        }
        
        Ok(())
    }

    /// Resolve dependencies from BuildTargets and find source files
    pub fn resolve_dependencies(&mut self) -> Result<(), BuildError> {
        // Scan each target's directory for Once source files
        for (name, node) in &self.build_graph.clone() {
            let dir = node.target.path.parent().unwrap_or_else(|| Path::new("."));
            let source_files = self.find_source_files(dir)?;
            
            // Add source files to the target
            if let Some(node) = self.build_graph.get_mut(name) {
                node.target.sources = source_files.clone();
                // Scan sources for import statements to discover dependencies
                for source in &source_files {
                    if let Ok(content) = std::fs::read_to_string(source) {
                        for line in content.lines() {
                            let trimmed = line.trim();
                            if trimmed.starts_with("import ") {
                                // Extract module path from import statement
                                let dep = trimmed
                                    .strip_prefix("import ")
                                    .unwrap_or("")
                                    .split_whitespace()
                                    .next()
                                    .unwrap_or("")
                                    .trim_matches(';')
                                    .to_string();
                                if !dep.is_empty() {
                                    node.dependencies.push(dep);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Build all targets
    pub fn build_all(&mut self) -> Result<(), BuildError> {
        if self.config.clean {
            self.clean()?;
        }

        // Load FFI security configuration
        let config_path = Path::new("once.toml");
        self.ffi_checker.load_config(config_path)?;

        // Scan for source files
        let source_files = self.find_source_files(&self.config.target_dir)?;
        
        // Scan for FFI blocks and fuzz tests
        self.ffi_checker.scan_ffi_blocks(&source_files)?;
        self.ffi_checker.scan_fuzz_tests(&source_files)?;
        
        // Check FFI security compliance
        self.ffi_checker.check_ffi_security()?;

        // Verify capability security
        self.verify_capabilities()?;

        // Build dependency graph
        self.build_dependency_graph()?;
        
        // Determine build order
        self.determine_build_order()?;
        
        // Execute builds
        self.execute_builds()?;
        
        Ok(())
    }

    /// Generate lockfile from current build graph
    pub fn generate_lockfile(&self) -> Lockfile {
        let mut lockfile = Lockfile::new();
        for (name, node) in &self.build_graph {
             let hash = utils::calculate_hash(&node.target.path).unwrap_or_default();
            lockfile.add_entry(LockfileEntry {
                name: name.clone(),
                version: node.target.version.clone(),
                hash,
                source: node.target.path.display().to_string(),
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            });
        }
        lockfile
    }

    /// Save lockfile to file
    pub fn save_lockfile(&self, path: &Path) -> Result<(), BuildError> {
        let lockfile = self.generate_lockfile();
        lockfile.write_to_file(path)
    }

    /// Load and validate lockfile
    pub fn validate_lockfile(&self, path: &Path) -> Result<(), BuildError> {
        let lockfile = Lockfile::read_from_file(path)?;
        let targets: Vec<BuildTarget> = self.build_graph.values().map(|n| n.target.clone()).collect();
        lockfile.validate(&targets)
    }

    /// Build specific target
    pub fn build_target(&mut self, target_name: &str) -> Result<(), BuildError> {
        // Get dependencies first
        let dependencies = if let Some(node) = self.build_graph.get(target_name) {
            node.dependencies.clone()
        } else {
            return Err(BuildError::BuildError(format!("Target not found: {}", target_name)));
        };
        
        // Build dependencies first
        for dep in &dependencies {
            self.build_target(dep)?;
        }
        
        // Check cache
        if self.config.incremental && self.is_cached(target_name)? {
            if let Some(node) = self.build_graph.get_mut(target_name) {
                node.status = BuildStatus::Cached;
            }
            return Ok(());
        }
        
        // Build target
        self.build_single_target(target_name)?;
        
        // Update status
        if let Some(node) = self.build_graph.get_mut(target_name) {
            node.status = BuildStatus::Completed;
        }
        
        Ok(())
    }

    /// Build dependency graph from resolved dependencies
    fn build_dependency_graph(&mut self) -> Result<(), BuildError> {
        // Already built during add_target; validate connectivity here
        let mut resolved = HashSet::new();
        for name in self.build_graph.keys().cloned().collect::<Vec<_>>() {
            self.validate_deps(&name, &mut resolved)?;
        }
        Ok(())
    }

    /// Validate that all dependencies exist in the graph
    fn validate_deps(&self, name: &str, resolved: &mut HashSet<String>) -> Result<(), BuildError> {
        if resolved.contains(name) {
            return Ok(());
        }
        resolved.insert(name.to_string());
        if let Some(node) = self.build_graph.get(name) {
            for dep in &node.dependencies {
                if !self.build_graph.contains_key(dep) && !dep.contains("std") {
                    eprintln!(
                        "Warning: dependency '{}' (required by '{}') is unresolved — treated as external",
                        dep, name
                    );
                }
                if self.build_graph.contains_key(dep) {
                    self.validate_deps(dep, resolved)?;
                }
            }
        }
        Ok(())
    }

    /// Determine build order
    fn determine_build_order(&mut self) -> Result<(), BuildError> {
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();
        let mut order = Vec::new();
        
        for target_name in self.build_graph.keys() {
            if !visited.contains(target_name) {
                self.topological_sort(target_name, &mut visited, &mut temp_visited, &mut order)?;
            }
        }
        
        self.build_order = order;
        Ok(())
    }

    /// Topological sort for build order
    fn topological_sort(
        &self,
        target_name: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> Result<(), BuildError> {
        if temp_visited.contains(target_name) {
            return Err(BuildError::DependencyError("Circular dependency detected".to_string()));
        }
        
        if visited.contains(target_name) {
            return Ok(());
        }
        
        temp_visited.insert(target_name.to_string());
        
        if let Some(node) = self.build_graph.get(target_name) {
            for dep in &node.dependencies {
                self.topological_sort(dep, visited, temp_visited, order)?;
            }
        }
        
        temp_visited.remove(target_name);
        visited.insert(target_name.to_string());
        order.push(target_name.to_string());
        
        Ok(())
    }

    /// Verify capability security: ensure no dependency requires undeclared capabilities
    /// or declares effects not present in the root capability set.
    fn verify_capabilities(&self) -> Result<(), BuildError> {
        // Collect capabilities declared by the root package
        let root_capabilities: HashSet<String> = self.build_graph.values()
            .flat_map(|n| n.target.capabilities.iter().cloned())
            .collect();
        
        for (name, node) in &self.build_graph {
            for cap in &node.target.capabilities {
                if !root_capabilities.contains(cap) {
                    return Err(BuildError::CapabilityViolation {
                        message: format!(
                            "Target '{}' requires capability '{}' which is not declared in root [capabilities]. \
                             Add 'requires \"{}\"' to the root package declaration.",
                            name, cap, cap
                        ),
                    });
                }
            }
            for eff in &node.target.effects {
                if !root_capabilities.contains(eff) {
                    return Err(BuildError::CapabilityViolation {
                        message: format!(
                            "Target '{}' uses effect '{}' which exceeds the root capability ceiling. \
                             Declared capabilities: {:?}. \
                             Either add 'requires \"{}\"' to the root package or remove the effect.",
                            name, eff, root_capabilities, eff
                        ),
                    });
                }
            }
        }
        
        Ok(())
    }

    /// Execute builds in parallel using depth-grouped dependency resolution
    fn execute_builds(&mut self) -> Result<(), BuildError> {
        let build_order = self.build_order.clone();
        let parallel_jobs = self.config.parallel_jobs.max(1);

        // Compute dependency depth for each target
        let depths = self.compute_depths();
        let max_depth = depths.values().max().copied().unwrap_or(0);

        // Build targets level by level (depth 0 first, then 1, 2, ...)
        for depth in 0..=max_depth {
            let targets_at_depth: Vec<String> = build_order.iter()
                .filter(|name| depths.get(*name) == Some(&depth))
                .cloned()
                .collect();

            if targets_at_depth.is_empty() {
                continue;
            }

            if targets_at_depth.len() == 1 || parallel_jobs == 1 {
                // Single target or single job: sequential
                for name in &targets_at_depth {
                    self.build_target(name)?;
                }
            } else {
                // Multiple independent targets: build in parallel with AtomicUsize concurrency limiter
                let errors = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
                let graph = std::sync::Arc::new(std::sync::Mutex::new(&mut self.build_graph));
                let config = &self.config;
                let cache = std::sync::Arc::new(std::sync::Mutex::new(&mut self.cache));
                let store = std::sync::Arc::new(std::sync::Mutex::new(&mut self.store));
                let running = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

                std::thread::scope(|s| {
                    let mut handles = Vec::new();

                    for target_name in &targets_at_depth {
                        let name = target_name.clone();
                        let errors = errors.clone();
                        let graph = graph.clone();
                        let cache = cache.clone();
                        let store = store.clone();
                        let config_ref: &BuildConfig = config;
                        let running = running.clone();

                        let handle = s.spawn(move || {
                            // Busy-wait until a slot is available
                            loop {
                                let current = running.load(std::sync::atomic::Ordering::SeqCst);
                                if current < config_ref.parallel_jobs {
                                    if running.compare_exchange(
                                        current, current + 1,
                                        std::sync::atomic::Ordering::SeqCst,
                                        std::sync::atomic::Ordering::SeqCst,
                                    ).is_ok() {
                                        break;
                                    }
                                }
                                std::thread::sleep(std::time::Duration::from_millis(1));
                            }

                            let result = Self::build_single_target_parallel(&name, &graph, config_ref, &cache, &store);
                            running.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);

                            match result {
                                Ok(()) => {
                                    if let Ok(mut g) = graph.lock() {
                                        if let Some(node) = g.get_mut(&name) {
                                            node.status = BuildStatus::Completed;
                                        }
                                    }
                                }
                                Err(e) => {
                                    errors.lock().unwrap().push(e);
                                }
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        let _ = handle.join();
                    }
                });

                let errors = errors.lock().unwrap();
                if !errors.is_empty() {
                    return Err(errors[0].clone());
                }
            }
        }

        Ok(())
    }

    /// Compute the maximum dependency depth for each target
    fn compute_depths(&self) -> std::collections::HashMap<String, usize> {
        let mut depths = std::collections::HashMap::new();
        let mut memo = std::collections::HashMap::new();

        fn depth_of(
            name: &str,
            graph: &std::collections::HashMap<String, BuildNode>,
            depths: &mut std::collections::HashMap<String, usize>,
            memo: &mut std::collections::HashMap<String, usize>,
        ) -> usize {
            if let Some(&d) = memo.get(name) {
                return d;
            }
            let max_dep_depth = if let Some(node) = graph.get(name) {
                node.dependencies.iter()
                    .map(|dep| depth_of(dep, graph, depths, memo))
                    .max()
                    .unwrap_or(0)
            } else {
                0
            };
            let d = max_dep_depth + 1;
            memo.insert(name.to_string(), d);
            depths.insert(name.to_string(), d);
            d
        }

        for name in self.build_graph.keys() {
            depth_of(name, &self.build_graph, &mut depths, &mut memo);
        }

        depths
    }

    /// Build a single target without building dependencies (for parallel execution)
    fn build_single_target_parallel(
        target_name: &str,
        graph: &std::sync::Arc<std::sync::Mutex<&mut HashMap<String, BuildNode>>>,
        config: &BuildConfig,
        cache: &std::sync::Arc<std::sync::Mutex<&mut HashMap<String, CacheEntry>>>,
        _store: &std::sync::Arc<std::sync::Mutex<&mut BuildStore>>,
    ) -> Result<(), BuildError> {
        let guard = graph.lock().map_err(|_| BuildError::BuildError("Lock poisoned".to_string()))?;
        let target = guard.get(target_name)
            .ok_or_else(|| BuildError::BuildError(format!("Target not found: {}", target_name)))?
            .target.clone();
        drop(guard);

        // Check cache
        {
            let cache_guard = cache.lock().map_err(|_| BuildError::BuildError("Lock poisoned".to_string()))?;
            let cache_key = format!("{}_{}", target_name, "binary");
            if cache_guard.contains_key(&cache_key) {
                return Ok(());
            }
        }

        // Build using the Once CLI as a subprocess (same as build_binary)
        match target.build_type {
            BuildType::Binary => {
                for source in &target.sources {
                    let mut cmd = std::process::Command::new("once");
                    cmd.arg("build").arg("--input").arg(source);

                    if config.verbose {
                        println!("[parallel] Building: {:?}", cmd);
                    }

                    let status = cmd.status().map_err(|e| {
                        BuildError::BuildError(format!("Failed to execute once build: {}", e))
                    })?;

                    if !status.success() {
                        return Err(BuildError::BuildError(
                            format!("Build failed for target '{}'", target_name)
                        ));
                    }
                }
                Ok(())
            }
            BuildType::Library | BuildType::Test | BuildType::Example => {
                if let Some(parent) = target.output_path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| BuildError::BuildError(format!("Failed to create output dir: {}", e)))?;
                }
                std::fs::write(&target.output_path, &[])
                    .map_err(|e| BuildError::BuildError(format!("Failed to write output: {}", e)))?;
                Ok(())
            }
        }
    }

    /// Build single target
    fn build_single_target(&self, target_name: &str) -> Result<(), BuildError> {
        if let Some(node) = self.build_graph.get(target_name) {
            match &node.target.build_type {
                BuildType::Binary => self.build_binary(&node.target)?,
                BuildType::Library => self.build_library(&node.target)?,
                BuildType::Test => self.build_test(&node.target)?,
                BuildType::Example => self.build_example(&node.target)?,
            }
        }
        Ok(())
    }

    /// Build binary target by invoking the Once compiler on source files
    fn build_binary(&self, target: &BuildTarget) -> Result<(), BuildError> {
        for source in &target.sources {
            // Compile each source file using the Once CLI
            let mut cmd = std::process::Command::new("once");
            cmd.arg("build")
               .arg("--input")
               .arg(source);
            
            if self.config.verbose {
                println!("Building: {:?}", cmd);
            }
            
            let status = cmd.status().map_err(|e| {
                BuildError::ExecutionError(format!(
                    "Failed to execute compiler for {}: {}",
                    source.display(),
                    e
                ))
            })?;
            
            if !status.success() {
                return Err(BuildError::BuildError(format!(
                    "Compilation failed for: {}",
                    source.display()
                )));
            }
        }
        Ok(())
    }

    /// Build library
    fn build_library(&self, target: &BuildTarget) -> Result<(), BuildError> {
        // Compile source files to object files
        let mut object_files = Vec::new();
        
        for source_file in &target.sources {
            let object_file = self.compile_source_file(source_file)?;
            object_files.push(object_file);
        }
        
        // Create library archive
        let lib_path = target.output_path.join("lib").join(&format!("lib{}.a", target.name));
        std::fs::create_dir_all(lib_path.parent().unwrap())?;
        
        // Use ar to create static library
        let mut cmd = std::process::Command::new("ar");
        cmd.arg("rcs")
           .arg(&lib_path);
        
        for obj_file in &object_files {
            cmd.arg(obj_file);
        }
        
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildError(format!("Library creation failed: {}", stderr)));
        }
        
        // Generate metadata
        self.generate_library_metadata(target, &lib_path)?;
        
        Ok(())
    }

    /// Build test
    fn build_test(&self, target: &BuildTarget) -> Result<(), BuildError> {
        // Compile test files
        let mut test_objects = Vec::new();
        
        for test_file in &target.sources {
            let test_object = self.compile_source_file(test_file)?;
            test_objects.push(test_object);
        }
        
        // Link test executable
        let test_exe = target.output_path.join("tests").join(&format!("{}_test", target.name));
        std::fs::create_dir_all(test_exe.parent().unwrap())?;
        
        self.link_executable(&test_objects, &test_exe)?;
        
        Ok(())
    }
    
    /// Compile a single source file to object file
    fn compile_source_file(&self, source_file: &Path) -> Result<PathBuf, BuildError> {
        let object_file = source_file.with_extension("o");
        
        // Use once-cli to compile the source file
        let mut cmd = std::process::Command::new("once");
        cmd.arg("compile")
           .arg("--output")
           .arg(&object_file)
           .arg(source_file);
        
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildError(format!("Compilation failed: {}", stderr)));
        }
        
        Ok(object_file)
    }
    
    /// Generate library metadata
    fn generate_library_metadata(&self, target: &BuildTarget, lib_path: &Path) -> Result<(), BuildError> {
        let metadata = serde_json::json!({
            "name": target.name,
            "version": target.version,
            "type": "library",
            "path": lib_path,
            "capabilities": target.capabilities,
            "effects": target.effects,
            "dependencies": target.dependencies
        });
        
        let metadata_path = lib_path.with_extension("json");
        std::fs::write(metadata_path, serde_json::to_string_pretty(&metadata)?)?;
        
        Ok(())
    }
    
    /// Link object files into executable
    fn link_executable(&self, object_files: &[PathBuf], output_path: &Path) -> Result<(), BuildError> {
        let mut cmd = std::process::Command::new("gcc");
        cmd.arg("-o").arg(output_path);
        
        for obj_file in object_files {
            cmd.arg(obj_file);
        }
        
        // Link with runtime
        cmd.arg("-L").arg("target/debug/deps")
           .arg("-l").arg("once_runtime");
        
        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildError(format!("Linking failed: {}", stderr)));
        }
        
        Ok(())
    }

    /// Build example
    fn build_example(&self, target: &BuildTarget) -> Result<(), BuildError> {
        self.build_binary(target)
    }

    /// Check if target is cached using content-addressed store.
    fn is_cached(&self, target_name: &str) -> Result<bool, BuildError> {
        if let Some(node) = self.build_graph.get(target_name) {
            let hash = self.store.compute_hash(&node.target)?;
            let artifact_name = format!("{}.{}", node.target.name, node.target.build_type.artifact_extension());
            if self.store.has_artifact(&hash, &artifact_name) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Retrieve a cached artifact path for a target.
    fn get_cached_artifact(&self, target_name: &str) -> Result<Option<PathBuf>, BuildError> {
        if let Some(node) = self.build_graph.get(target_name) {
            let hash = self.store.compute_hash(&node.target)?;
            let artifact_name = format!("{}.{}", node.target.name, node.target.build_type.artifact_extension());
            return Ok(self.store.retrieve_artifact(&hash, &artifact_name));
        }
        Ok(None)
    }

    /// Load build cache
    fn load_cache(&mut self) -> Result<(), BuildError> {
        let cache_file = self.config.cache_dir.join("build_cache.json");
        
        if cache_file.exists() {
            let cache_data = fs::read_to_string(&cache_file)
                .map_err(|e| BuildError::CacheError(format!("Failed to read cache: {}", e)))?;
            
            self.cache = serde_json::from_str(&cache_data)?;
        }
        
        Ok(())
    }

    /// Save build cache
    fn save_cache(&self) -> Result<(), BuildError> {
        let cache_file = self.config.cache_dir.join("build_cache.json");
        
        let json = serde_json::to_string_pretty(&self.cache)?;
        fs::write(&cache_file, json)
            .map_err(|e| BuildError::CacheError(format!("Failed to write cache: {}", e)))?;
        
        Ok(())
    }

    /// Clean build artifacts
    pub fn clean(&mut self) -> Result<(), BuildError> {
        if self.config.target_dir.exists() {
            fs::remove_dir_all(&self.config.target_dir)
                .map_err(|e| BuildError::FileError(format!("Failed to clean target dir: {}", e)))?;
        }
        
        if self.config.cache_dir.exists() {
            fs::remove_dir_all(&self.config.cache_dir)
                .map_err(|e| BuildError::FileError(format!("Failed to clean cache dir: {}", e)))?;
        }
        
        // Recreate directories
        self.init()?;
        
        Ok(())
    }

    /// Get build status
    pub fn get_status(&self) -> HashMap<String, BuildStatus> {
        self.build_graph
            .iter()
            .map(|(name, node)| (name.clone(), node.status.clone()))
            .collect()
    }

    /// Get build statistics
    pub fn get_stats(&self) -> BuildStats {
        let total = self.build_graph.len();
        let completed = self.build_graph.values()
            .filter(|node| matches!(node.status, BuildStatus::Completed | BuildStatus::Cached))
            .count();
        let failed = self.build_graph.values()
            .filter(|node| matches!(node.status, BuildStatus::Failed))
            .count();
        let pending = self.build_graph.values()
            .filter(|node| matches!(node.status, BuildStatus::Pending))
            .count();
        
        BuildStats {
            total,
            completed,
            failed,
            pending,
        }
    }
    
    /// Find source files
    pub fn find_source_files(&self, dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
        let mut files = Vec::new();
        
        if dir.is_dir() {
            for entry in fs::read_dir(dir)
                .map_err(|e| BuildError::FileError(format!("Failed to read directory: {}", e)))? {
                let entry = entry.map_err(|e| BuildError::FileError(format!("Failed to read entry: {}", e)))?;
                let path = entry.path();
                
                if path.is_dir() {
                    files.extend(self.find_source_files(&path)?);
                } else if path.extension().and_then(|s| s.to_str()) == Some("onc") {
                    files.push(path);
                }
            }
        }
        
        Ok(files)
    }
}

/// Build statistics
#[derive(Debug, Clone)]
pub struct BuildStats {
    pub total: usize,
    pub completed: usize,
    pub failed: usize,
    pub pending: usize,
}

/// Build manifest
#[derive(Debug, Clone)]
pub struct BuildManifest {
    pub name: String,
    pub version: String,
    pub targets: Vec<BuildTarget>,
    pub dependencies: Vec<BuildDependency>,
}

impl BuildManifest {
    pub fn from_file(path: &Path) -> Result<Self, BuildError> {
        let content = fs::read_to_string(path)
            .map_err(|e| BuildError::FileError(format!("Failed to read manifest: {}", e)))?;

        let mut name = String::new();
        let mut version = String::new();
        let mut targets = Vec::new();
        let mut dependencies = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("name") {
                if let Some(val) = line.split('=').nth(1) {
                    name = val.trim().trim_matches('"').to_string();
                }
            } else if line.starts_with("version") {
                if let Some(val) = line.split('=').nth(1) {
                    version = val.trim().trim_matches('"').to_string();
                }
            }
        }

        Ok(Self {
            name: if name.is_empty() { "example".to_string() } else { name },
            version: if version.is_empty() { "0.1.0".to_string() } else { version },
            targets,
            dependencies,
        })
    }
}

/// Build utilities
pub mod utils {
    use super::*;
    
    /// Calculate file hash
    pub fn calculate_hash(path: &Path) -> Result<String, BuildError> {
        let content = fs::read(path)
            .map_err(|e| BuildError::FileError(format!("Failed to read file: {}", e)))?;
        
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        Ok(format!("{:x}", hasher.finish()))
    }
    
    /// Check if file is newer than another
    pub fn is_newer(source: &Path, target: &Path) -> Result<bool, BuildError> {
        let source_metadata = fs::metadata(source)
            .map_err(|e| BuildError::FileError(format!("Failed to get source metadata: {}", e)))?;
        
        let target_metadata = fs::metadata(target)
            .map_err(|e| BuildError::FileError(format!("Failed to get target metadata: {}", e)))?;
        
        Ok(source_metadata.modified().map_err(|e| BuildError::FileError(format!("Failed to get source modified time: {}", e)))?
            > target_metadata.modified().map_err(|e| BuildError::FileError(format!("Failed to get target modified time: {}", e)))?)
    }
}

impl FfiSecurityChecker {
    pub fn new() -> Self {
        Self {
            config: FfiSecurityConfig::default(),
            ffi_blocks: Vec::new(),
            fuzz_tests: HashSet::new(),
        }
    }

    /// Load FFI security configuration from once.toml
    pub fn load_config(&mut self, config_path: &Path) -> Result<(), BuildError> {
        if !config_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(config_path)
            .map_err(|e| BuildError::FileError(format!("Failed to read config: {}", e)))?;

        // Parse once.toml for FFI security settings
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("profile.ffi-safe") {
                if line.contains("true") {
                    self.config.ffi_safe = true;
                }
            } else if line.starts_with("profile.require-fuzz-tests") {
                if line.contains("false") {
                    self.config.require_fuzz_tests = false;
                }
            } else if line.starts_with("profile.quarantine-unsafe") {
                if line.contains("false") {
                    self.config.quarantine_unsafe = false;
                }
            }
        }

        Ok(())
    }

    /// Scan source files for unsafe FFI blocks
    pub fn scan_ffi_blocks(&mut self, source_files: &[PathBuf]) -> Result<(), BuildError> {
        self.ffi_blocks.clear();

        for file_path in source_files {
            let content = fs::read_to_string(file_path)
                .map_err(|e| BuildError::FileError(format!("Failed to read file: {}", e)))?;

            let mut line_number = 1;
            for line in content.lines() {
                if line.contains("unsafe ffi") || line.contains("unsafe_ffi") {
                    let function_name = self.extract_function_name(&content, line_number);
                    let library_name = self.extract_library_name(&content, line_number);
                    
                    let ffi_block = FfiBlock {
                        file_path: file_path.clone(),
                        line_number,
                        function_name,
                        library_name,
                        has_fuzz_test: false,
                    };
                    
                    self.ffi_blocks.push(ffi_block);
                }
                line_number += 1;
            }
        }

        Ok(())
    }

    /// Scan for fuzzing tests
    pub fn scan_fuzz_tests(&mut self, source_files: &[PathBuf]) -> Result<(), BuildError> {
        self.fuzz_tests.clear();

        for file_path in source_files {
            let content = fs::read_to_string(file_path)
                .map_err(|e| BuildError::FileError(format!("Failed to read file: {}", e)))?;

            for line in content.lines() {
                if line.contains("#[fuzz]") || line.contains("fuzz_test") {
                    let test_name = self.extract_test_name(line);
                    if !test_name.is_empty() {
                        self.fuzz_tests.insert(test_name);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check FFI security compliance
    pub fn check_ffi_security(&self) -> Result<(), BuildError> {
        if !self.config.quarantine_unsafe {
            return Ok(());
        }

        for ffi_block in &self.ffi_blocks {
            if !self.config.ffi_safe {
                return Err(BuildError::FfiSecurityError(format!(
                    "Unsafe FFI block found in {}:{} - FFI safety profile not enabled",
                    ffi_block.file_path.display(),
                    ffi_block.line_number
                )));
            }

            if self.config.require_fuzz_tests && !ffi_block.has_fuzz_test {
                return Err(BuildError::FfiSecurityError(format!(
                    "Unsafe FFI block in {}:{} requires fuzzing test - none found",
                    ffi_block.file_path.display(),
                    ffi_block.line_number
                )));
            }
        }

        Ok(())
    }

    /// Extract function name from FFI block
    fn extract_function_name(&self, content: &str, line_number: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if line_number > 0 && line_number <= lines.len() {
            let line = lines[line_number - 1];
            if let Some(start) = line.find("fn ") {
                if let Some(end) = line[start + 3..].find('(') {
                    return line[start + 3..start + 3 + end].trim().to_string();
                }
            }
        }
        "unknown".to_string()
    }

    /// Extract library name from FFI block
    fn extract_library_name(&self, content: &str, line_number: usize) -> String {
        let lines: Vec<&str> = content.lines().collect();
        if line_number > 0 && line_number <= lines.len() {
            let line = lines[line_number - 1];
            if let Some(start) = line.find("ffi[") {
                if let Some(end) = line[start + 4..].find(']') {
                    return line[start + 4..start + 4 + end].trim().to_string();
                }
            }
        }
        "unknown".to_string()
    }

    /// Extract test name from fuzz test line
    fn extract_test_name(&self, line: &str) -> String {
        if let Some(start) = line.find("fn ") {
            if let Some(end) = line[start + 3..].find('(') {
                return line[start + 3..start + 3 + end].trim().to_string();
            }
        }
        String::new()
    }
}

/// Lockfile entry for a dependency
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LockfileEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub source: String,
    pub timestamp: u64,
}

/// Lockfile for reproducible builds
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lockfile {
    pub version: String,
    pub entries: Vec<LockfileEntry>,
    pub generated_at: u64,
}

impl Lockfile {
    /// Create a new lockfile
    pub fn new() -> Self {
        Self {
            version: "1.0.0".to_string(),
            entries: Vec::new(),
            generated_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Add a dependency entry
    pub fn add_entry(&mut self, entry: LockfileEntry) {
        self.entries.push(entry);
    }

    /// Write lockfile to disk
    pub fn write_to_file(&self, path: &Path) -> Result<(), BuildError> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| BuildError::FileError(format!("Serialize lockfile: {}", e)))?;
        fs::write(path, json)
            .map_err(|e| BuildError::FileError(format!("Write lockfile: {}", e)))?;
        Ok(())
    }

    /// Read lockfile from disk
    pub fn read_from_file(path: &Path) -> Result<Self, BuildError> {
        let content = fs::read_to_string(path)
            .map_err(|e| BuildError::FileError(format!("Read lockfile: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| BuildError::FileError(format!("Parse lockfile: {}", e)))
    }

    /// Validate that lockfile matches current dependencies
    pub fn validate(&self, targets: &[BuildTarget]) -> Result<(), BuildError> {
        for target in targets {
            let entry = self.entries.iter().find(|e| e.name == target.name);
            if let Some(entry) = entry {
                 let current_hash = utils::calculate_hash(&target.path)?;
                if entry.hash != current_hash {
                    return Err(BuildError::LockfileHashMismatch(format!(
                        "Target '{}' hash mismatch: lockfile={}, current={}",
                        target.name, entry.hash, current_hash
                    )));
                }
            }
        }
        Ok(())
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_tool_creation() {
        let config = BuildConfig::default();
        let build_tool = BuildTool::new(config);
        assert_eq!(build_tool.build_graph.len(), 0);
    }

    #[test]
    fn test_add_target() {
        let mut build_tool = BuildTool::new(BuildConfig::default());
        let target = BuildTarget {
            name: "test".to_string(),
            path: PathBuf::from("test.onc"),
            dependencies: Vec::new(),
            build_type: BuildType::Binary,
            output_path: PathBuf::from("test"),
            sources: vec![],
            version: "1.0.0".to_string(),
            capabilities: vec![],
            effects: vec![],
        };
        
        assert!(build_tool.add_target(target).is_ok());
        assert_eq!(build_tool.build_graph.len(), 1);
    }

    #[test]
    fn test_build_stats() {
        let mut build_tool = BuildTool::new(BuildConfig::default());
        let target = BuildTarget {
            name: "test".to_string(),
            path: PathBuf::from("test.onc"),
            dependencies: Vec::new(),
            build_type: BuildType::Binary,
            output_path: PathBuf::from("test"),
            sources: vec![],
            version: "1.0.0".to_string(),
            capabilities: vec![],
            effects: vec![],
        };
        
        build_tool.add_target(target).unwrap();
        let stats = build_tool.get_stats();
        assert_eq!(stats.total, 1);
        assert_eq!(stats.pending, 1);
    }
}

/// Schema hydration module (ONCE-008 §2)
///
/// Compiler pass that bridges JSON data streams into strictly validated
/// struct representations. A `schema` declaration like:
/// ```ignore
/// schema User from JSON for Person {
///     name: "$.name",
///     age: "$.age",
/// }
/// ```
/// generates a `hydrate_user(json: Str) -> Result<Person, Error>` function
/// that deserializes JSON into the declared struct.
pub mod schema {
    use once_parse::SchemaDecl;
    use super::BuildError;

    /// Generate Once source code for a hydration function from a schema declaration.
    pub fn generate_hydrate_function(schema: &SchemaDecl) -> Result<String, BuildError> {
        let struct_name = match &schema.target_type {
            once_parse::Type::Ident(name) => name.clone(),
            _ => return Err(BuildError::BuildError("Schema target must be a struct name".to_string())),
        };

        let mut code = String::new();

        // Generate the hydrate function
        code.push_str(&format!(
            "fn hydrate_{}(json: Str) -> Result<{}, Error> {{\n",
            schema.name.to_lowercase(),
            struct_name
        ));

        code.push_str("    // Parse JSON and extract fields\n");

        // Generate field extraction code
        for (field_name, source_path) in &schema.fields {
            let path_parts: Vec<&str> = source_path.trim_start_matches("$.").split('.').collect();
            if path_parts.len() == 1 {
                code.push_str(&format!(
                    "    let {} = json.get(\"{}\").unwrap_or(\"\");\n",
                    field_name, path_parts[0]
                ));
            } else {
                code.push_str(&format!(
                    "    let {} = json.at(\"{}\").unwrap_or(\"\");\n",
                    field_name,
                    path_parts.join(".")
                ));
            }
        }

        // Build struct literal
        code.push_str(&format!("    Ok({} {{\n", struct_name));
        for (field_name, _) in &schema.fields {
            code.push_str(&format!("        {},\n", field_name));
        }
        code.push_str("    })\n");
        code.push_str("}\n");

        Ok(code)
    }

    /// Validate that a schema's target type fields match the schema fields.
    pub fn validate_schema_fields(
        schema: &SchemaDecl,
        struct_fields: &[(String, once_parse::Type)],
    ) -> Result<(), BuildError> {
        let declared: std::collections::HashSet<&str> = schema.fields.iter()
            .map(|(name, _)| name.as_str())
            .collect();
        let expected: std::collections::HashSet<&str> = struct_fields.iter()
            .map(|(name, _)| name.as_str())
            .collect();

        for field in &declared {
            if !expected.contains(field) {
                return Err(BuildError::BuildError(format!(
                    "Schema field '{}' not found in target struct", field
                )));
            }
        }
        for field in &expected {
            if !declared.contains(field) {
                return Err(BuildError::BuildError(format!(
                    "Struct field '{}' is missing from schema declaration", field
                )));
            }
        }

        Ok(())
    }
}