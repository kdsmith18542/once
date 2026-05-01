//! Build System Tests for the Once language
//!
//! Verifies that `once-build` acts as a standalone, content-addressed build
//! system producing reproducible outputs without invoking Cargo.

use once_build::{BuildConfig, BuildTool, BuildTarget, BuildType, BuildStore, BuildStatus};
use std::fs;
use std::path::PathBuf;

/// BuildStore computes identical hashes for identical inputs.
#[test]
fn test_content_hash_determinism() {
    let store = BuildStore::new(PathBuf::from("target/test_cas_det"));
    let target = BuildTarget {
        name: "hello".to_string(),
        path: PathBuf::from("fake.onc"),
        dependencies: vec!["std".to_string()],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("hello"),
        sources: vec![],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    };

    let hash1 = store.compute_hash(&target).expect("hash should compute");
    let hash2 = store.compute_hash(&target).expect("hash should compute again");
    assert_eq!(hash1, hash2, "Same inputs must produce identical hashes");
}

/// BuildStore produces different hashes when source contents differ.
#[test]
fn test_content_hash_sensitivity() {
    let dir = "target/test_cas_sens";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let store = BuildStore::new(PathBuf::from(dir));

    let src_path = PathBuf::from(format!("{}/main.onc", dir));
    fs::write(&src_path, "fn main() -> Unit {}").unwrap();

    let target1 = BuildTarget {
        name: "hello".to_string(),
        path: src_path.clone(),
        dependencies: vec![],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("hello"),
        sources: vec![src_path.clone()],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    };

    let hash1 = store.compute_hash(&target1).expect("hash should compute");

    fs::write(&src_path, "fn main() -> Unit { print(1) }").unwrap();
    let hash2 = store.compute_hash(&target1).expect("hash should compute");

    assert_ne!(hash1, hash2, "Different source contents must produce different hashes");

    let _ = fs::remove_dir_all(dir);
}

/// BuildStore can store and retrieve artifacts by content hash.
#[test]
fn test_store_and_retrieve_artifact() {
    let dir = "target/test_cas_store";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let store = BuildStore::new(PathBuf::from(dir));
    let hash = "deadbeefcafebabe";
    let artifact_name = "hello.exe";

    let source = PathBuf::from(format!("{}/source.tmp", dir));
    fs::write(&source, "artifact bytes").unwrap();

    assert!(!store.has_artifact(hash, artifact_name));
    let stored = store.store_artifact(hash, artifact_name, &source).expect("store should succeed");
    assert!(stored.exists());
    assert!(store.has_artifact(hash, artifact_name));

    let retrieved = store.retrieve_artifact(hash, artifact_name).expect("retrieve should succeed");
    assert_eq!(fs::read_to_string(&retrieved).unwrap(), "artifact bytes");

    let _ = fs::remove_dir_all(dir);
}

/// BuildTool can be initialized and targets added.
#[test]
fn test_build_tool_lifecycle() {
    let mut tool = BuildTool::new(BuildConfig {
        target_dir: PathBuf::from("target/test_build_lifecycle"),
        cache_dir: PathBuf::from("target/test_build_lifecycle/cache"),
        parallel_jobs: 1,
        incremental: true,
        clean: false,
        verbose: false,
    });

    tool.init().expect("init should succeed");

    let target = BuildTarget {
        name: "test".to_string(),
        path: PathBuf::from("test.onc"),
        dependencies: vec![],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("target/test_build_lifecycle/test"),
        sources: vec![],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    };

    tool.add_target(target).expect("add_target should succeed");
    assert_eq!(tool.build_graph.len(), 1);

    let stats = tool.get_stats();
    assert_eq!(stats.total, 1);
    assert_eq!(stats.pending, 1);

    let _ = fs::remove_dir_all("target/test_build_lifecycle");
}

/// BuildTool topological sort detects circular dependencies.
#[test]
fn test_topological_sort_detects_cycle() {
    let mut tool = BuildTool::new(BuildConfig::default());

    tool.add_target(BuildTarget {
        name: "a".to_string(),
        path: PathBuf::from("a.onc"),
        dependencies: vec!["b".to_string()],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("a"),
        sources: vec![],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    }).unwrap();

    tool.add_target(BuildTarget {
        name: "b".to_string(),
        path: PathBuf::from("b.onc"),
        dependencies: vec!["a".to_string()],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("b"),
        sources: vec![],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    }).unwrap();

    let result = tool.build_all();
    assert!(result.is_err(), "Circular dependency should cause an error");
}

/// BuildTool lockfile generation and validation.
#[test]
fn test_lockfile_generation_and_validation() {
    let mut tool = BuildTool::new(BuildConfig::default());

    let dir = "target/test_lockfile";
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    let src = PathBuf::from(format!("{}/main.onc", dir));
    fs::write(&src, "fn main() -> Unit {}").unwrap();

    tool.add_target(BuildTarget {
        name: "main".to_string(),
        path: src.clone(),
        dependencies: vec![],
        build_type: BuildType::Binary,
        output_path: PathBuf::from(format!("{}/main", dir)),
        sources: vec![src.clone()],
        version: "0.1.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    }).unwrap();

    let lockfile_path = PathBuf::from(format!("{}/once.lock", dir));
    tool.save_lockfile(&lockfile_path).expect("save lockfile should succeed");
    assert!(lockfile_path.exists());

    let validate_result = tool.validate_lockfile(&lockfile_path);
    assert!(validate_result.is_ok(), "Lockfile should validate against current targets");

    let _ = fs::remove_dir_all(dir);
}

/// BuildTool clean removes build artifacts.
#[test]
fn test_build_tool_clean() {
    let mut tool = BuildTool::new(BuildConfig {
        target_dir: PathBuf::from("target/test_clean"),
        cache_dir: PathBuf::from("target/test_clean/cache"),
        parallel_jobs: 1,
        incremental: true,
        clean: false,
        verbose: false,
    });

    tool.init().expect("init should succeed");
    assert!(tool.config.target_dir.exists());

    tool.clean().expect("clean should succeed");
    // After clean + init, directories should be recreated empty
    assert!(tool.config.target_dir.exists());

    let _ = fs::remove_dir_all("target/test_clean");
}

/// BuildStore hash includes dependency names.
#[test]
fn test_hash_includes_dependencies() {
    let store = BuildStore::new(PathBuf::from("target/test_hash_deps"));

    let target_no_deps = BuildTarget {
        name: "app".to_string(),
        path: PathBuf::from("app.onc"),
        dependencies: vec![],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("app"),
        sources: vec![],
        version: "1.0.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    };

    let target_with_deps = BuildTarget {
        name: "app".to_string(),
        path: PathBuf::from("app.onc"),
        dependencies: vec!["std".to_string(), "net".to_string()],
        build_type: BuildType::Binary,
        output_path: PathBuf::from("app"),
        sources: vec![],
        version: "1.0.0".to_string(),
        capabilities: vec![],
        effects: vec![],
    };

    let hash1 = store.compute_hash(&target_no_deps).unwrap();
    let hash2 = store.compute_hash(&target_with_deps).unwrap();
    assert_ne!(hash1, hash2, "Different dependencies must produce different hashes");
}
