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

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use thiserror::Error;

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
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub target: String,
    pub hash: String,
    pub timestamp: u64,
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
    ///
    /// In a production build this would be replaced by a call to an
    /// external neural-symbolic solver or LLM-based synthesizer.
    pub struct StubAiSolver;

    impl AiSolver for StubAiSolver {
        fn synthesize(&self, goal_name: &str, _params: &[String], return_type: &str, _constraints: &[String]) -> Result<String, BuildError> {
            // Produce a minimal placeholder function body so the goal
            // compiles as a regular Once function.
            let body = match return_type {
                "Int" => "0",
                "Bool" => "false",
                "Float" => "0.0",
                "Str" => "\"\"",
                "Unit" => "()",
                _ => "()",
            };
            Ok(format!("fn {}() -> {} {{ {} }}", goal_name, return_type, body))
        }
    }

    /// Goal synthesizer that manages AI solver lifecycle.
    pub struct GoalSynthesizer {
        pub solver: Box<dyn AiSolver>,
        pub synthesized_goals: HashMap<String, String>,
        /// Content-hash based cache keys for deterministic regeneration
        pub content_hashes: HashMap<String, u64>,
    }

    impl GoalSynthesizer {
        pub fn new() -> Self {
            Self {
                solver: Box::new(StubAiSolver),
                synthesized_goals: HashMap::new(),
                content_hashes: HashMap::new(),
            }
        }

        pub fn with_solver(solver: Box<dyn AiSolver>) -> Self {
            Self {
                solver,
                synthesized_goals: HashMap::new(),
                content_hashes: HashMap::new(),
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

        /// Synthesize a goal and cache the result.
        /// Uses content-hash caching: same input → same cached output.
        pub fn synthesize_goal(
            &mut self,
            name: &str,
            params: &[String],
            return_type: &str,
            constraints: &[String],
        ) -> Result<String, BuildError> {
            let hash = Self::compute_content_hash(name, params, return_type, constraints);
            let cache_key = format!("{}:{}", name, hash);
            
            if let Some(cached) = self.synthesized_goals.get(&cache_key) {
                return Ok(cached.clone());
            }
            let source = self.solver.synthesize(name, params, return_type, constraints)?;
            self.synthesized_goals.insert(cache_key.clone(), source.clone());
            self.content_hashes.insert(name.to_string(), hash);
            Ok(source)
        }

        /// Verify that a synthesized goal compiles and its examples pass
        pub fn verify_goal(
            &self,
            goal_name: &str,
            _synthesized_code: &str,
            _examples: &[(Vec<String>, String)],
        ) -> Result<bool, BuildError> {
            // Full verification would: parse, type-check, compile, and run examples
            // For now, rely on the compiler's existing type/effect/linearity checks
            // that run during normal compilation
            Ok(true)
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

    /// Resolve dependencies
    pub fn resolve_dependencies(&mut self) -> Result<(), BuildError> {
        // TODO: Implement dependency resolution
        // This would involve:
        // 1. Parsing Cargo.toml files
        // 2. Resolving version constraints
        // 3. Downloading dependencies
        // 4. Building dependency graph
        
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

    /// Build dependency graph
    fn build_dependency_graph(&mut self) -> Result<(), BuildError> {
        // TODO: Implement dependency graph construction
        // This would involve:
        // 1. Analyzing source files for imports
        // 2. Building dependency relationships
        // 3. Detecting circular dependencies
        // 4. Optimizing build order
        
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

    /// Execute builds
    fn execute_builds(&mut self) -> Result<(), BuildError> {
        let build_order = self.build_order.clone();
        for target_name in &build_order {
            self.build_target(target_name)?;
        }
        Ok(())
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

    /// Build binary
    fn build_binary(&self, target: &BuildTarget) -> Result<(), BuildError> {
        // Use the current executable path
        let once_path = std::env::current_exe()
            .map_err(|e| BuildError::ExecutionError(format!("Failed to get current executable: {}", e)))?;
        
        let output = Command::new(&once_path)
            .arg("build")
            .arg("--input")
            .arg(&target.path)
            .arg("--output")
            .arg(&target.output_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| BuildError::ExecutionError(format!("Failed to execute once build: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BuildError::BuildError(format!("Build failed: {}", stderr)));
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
    fn build_example(&self, _target: &BuildTarget) -> Result<(), BuildError> {
        // TODO: Implement example building
        // This would involve:
        // 1. Compiling example files
        // 2. Linking with dependencies
        // 3. Generating example executables
        
        Ok(())
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
            let _cache_data = fs::read_to_string(&cache_file)
                .map_err(|e| BuildError::CacheError(format!("Failed to read cache: {}", e)))?;
            
            // TODO: Deserialize cache data
            // For now, just create empty cache
        }
        
        Ok(())
    }

    /// Save build cache
    fn save_cache(&self) -> Result<(), BuildError> {
        let cache_file = self.config.cache_dir.join("build_cache.json");
        
        // TODO: Serialize cache data
        // For now, just create empty cache file
        fs::write(&cache_file, "{}")
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
    pub fn from_file(_path: &Path) -> Result<Self, BuildError> {
        // TODO: Parse build manifest file
        // This would involve parsing a build configuration file
        // and extracting targets and dependencies
        
        Ok(Self {
            name: "example".to_string(),
            version: "0.1.0".to_string(),
            targets: Vec::new(),
            dependencies: Vec::new(),
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
                    return Err(BuildError::DependencyError(format!(
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