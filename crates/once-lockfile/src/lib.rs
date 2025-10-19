use once_onceo::{OnceoModule, EffectSummary, TypeSummary, RegionSummary};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use thiserror::Error;
use sha2::{Sha256, Digest};
use hex;

/// Errors for lockfile operations
#[derive(Error, Debug, Clone)]
pub enum LockfileError {
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),
    #[error("Version conflict: {0}")]
    VersionConflict(String),
    #[error("Effect conflict: {0}")]
    EffectConflict(String),
    #[error("Capability conflict: {0}")]
    CapabilityConflict(String),
    #[error("Hash mismatch: {0}")]
    HashMismatch(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

/// Dependency entry in lockfile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEntry {
    pub name: String,
    pub version: String,
    pub source: DependencySource,
    pub hash: String,
    pub effects: Vec<String>,
    pub capabilities: Vec<String>,
    pub type_summary: Vec<String>,
    pub region_constraints: Vec<String>,
    pub transitive_deps: Vec<String>,
}

/// Source of a dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencySource {
    Registry { url: String },
    Git { url: String, rev: String },
    Path { path: String },
    Local,
}

/// Lockfile structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: String,
    pub dependencies: HashMap<String, DependencyEntry>,
    pub capability_graph: CapabilityGraph,
    pub effect_constraints: HashMap<String, Vec<String>>,
    pub generated_at: String,
    pub lockfile_hash: String,
}

/// Capability graph for tracking capability flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityGraph {
    pub nodes: Vec<CapabilityNode>,
    pub edges: Vec<CapabilityEdge>,
}

/// Node in capability graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityNode {
    pub id: String,
    pub dependency_name: String,
    pub capabilities: Vec<String>,
}

/// Edge in capability graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityEdge {
    pub from: String,
    pub to: String,
    pub capability: String,
}

/// Lockfile manager
pub struct LockfileManager {
    lockfile: Lockfile,
}

impl LockfileManager {
    pub fn new() -> Self {
        Self {
            lockfile: Lockfile {
                version: "1.0".to_string(),
                dependencies: HashMap::new(),
                capability_graph: CapabilityGraph {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                },
                effect_constraints: HashMap::new(),
                generated_at: chrono::Utc::now().to_rfc3339(),
                lockfile_hash: String::new(),
            },
        }
    }

    /// Load lockfile from file
    pub fn load_from_file(path: &str) -> Result<Self, LockfileError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LockfileError::IoError(e.to_string()))?;
        let lockfile: Lockfile = toml::from_str(&content)
            .map_err(|e| LockfileError::DeserializationError(e.to_string()))?;
        
        // Verify lockfile hash
        let computed_hash = Self::compute_lockfile_hash(&lockfile)?;
        if lockfile.lockfile_hash != computed_hash {
            return Err(LockfileError::HashMismatch(format!(
                "Lockfile hash mismatch: expected {}, got {}",
                computed_hash, lockfile.lockfile_hash
            )));
        }

        Ok(Self { lockfile })
    }

    /// Save lockfile to file
    pub fn save_to_file(&mut self, path: &str) -> Result<(), LockfileError> {
        // Update generated_at timestamp
        self.lockfile.generated_at = chrono::Utc::now().to_rfc3339();
        
        // Compute and update lockfile hash
        self.lockfile.lockfile_hash = Self::compute_lockfile_hash(&self.lockfile)?;
        
        let content = toml::to_string_pretty(&self.lockfile)
            .map_err(|e| LockfileError::SerializationError(e.to_string()))?;
        std::fs::write(path, content)
            .map_err(|e| LockfileError::IoError(e.to_string()))?;
        Ok(())
    }

    /// Add dependency to lockfile
    pub fn add_dependency(&mut self, entry: DependencyEntry) -> Result<(), LockfileError> {
        // Check for version conflicts
        if let Some(existing) = self.lockfile.dependencies.get(&entry.name) {
            if existing.version != entry.version {
                return Err(LockfileError::VersionConflict(format!(
                    "Dependency {} has conflicting versions: {} vs {}",
                    entry.name, existing.version, entry.version
                )));
            }
        }

        // Check for effect conflicts
        self.check_effect_conflicts(&entry)?;

        // Check for capability conflicts
        self.check_capability_conflicts(&entry)?;

        // Add to capability graph
        self.add_to_capability_graph(&entry)?;

        // Add dependency
        self.lockfile.dependencies.insert(entry.name.clone(), entry);
        Ok(())
    }

    /// Check for effect conflicts
    fn check_effect_conflicts(&self, entry: &DependencyEntry) -> Result<(), LockfileError> {
        for (dep_name, dep) in &self.lockfile.dependencies {
            for effect in &entry.effects {
                if dep.effects.contains(effect) {
                    // Check if effects are compatible
                    if !self.are_effects_compatible(effect, &entry.name, dep_name) {
                        return Err(LockfileError::EffectConflict(format!(
                            "Effect {} conflicts between {} and {}",
                            effect, entry.name, dep_name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if effects are compatible
    fn are_effects_compatible(&self, _effect: &str, _dep1: &str, _dep2: &str) -> bool {
        // Simplified - in a real implementation, this would check effect subsumption
        true
    }

    /// Check for capability conflicts
    fn check_capability_conflicts(&self, entry: &DependencyEntry) -> Result<(), LockfileError> {
        for (dep_name, dep) in &self.lockfile.dependencies {
            for capability in &entry.capabilities {
                if dep.capabilities.contains(capability) {
                    // Check if capabilities are compatible
                    if !self.are_capabilities_compatible(capability, &entry.name, dep_name) {
                        return Err(LockfileError::CapabilityConflict(format!(
                            "Capability {} conflicts between {} and {}",
                            capability, entry.name, dep_name
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if capabilities are compatible
    fn are_capabilities_compatible(&self, _capability: &str, _dep1: &str, _dep2: &str) -> bool {
        // Simplified - in a real implementation, this would check capability subsumption
        true
    }

    /// Add dependency to capability graph
    fn add_to_capability_graph(&mut self, entry: &DependencyEntry) -> Result<(), LockfileError> {
        // Create node for this dependency
        let node = CapabilityNode {
            id: entry.name.clone(),
            dependency_name: entry.name.clone(),
            capabilities: entry.capabilities.clone(),
        };
        self.lockfile.capability_graph.nodes.push(node);

        // Create edges for transitive dependencies
        for transitive_dep in &entry.transitive_deps {
            for capability in &entry.capabilities {
                let edge = CapabilityEdge {
                    from: entry.name.clone(),
                    to: transitive_dep.clone(),
                    capability: capability.clone(),
                };
                self.lockfile.capability_graph.edges.push(edge);
            }
        }

        Ok(())
    }

    /// Create dependency entry from OnceoModule
    pub fn create_entry_from_module(
        name: String,
        version: String,
        source: DependencySource,
        module: &OnceoModule,
    ) -> Result<DependencyEntry, LockfileError> {
        // Extract effects from module
        let effects: Vec<String> = module
            .effect_summaries
            .iter()
            .flat_map(|s| s.effect_labels.clone())
            .collect();

        // Extract capabilities (simplified)
        let capabilities: Vec<String> = effects.clone();

        // Extract type summaries
        let type_summary: Vec<String> = module
            .type_summaries
            .iter()
            .map(|s| format!("{}: {}", s.name, s.type_scheme))
            .collect();

        // Extract region constraints
        let region_constraints: Vec<String> = module
            .region_summaries
            .iter()
            .flat_map(|s| s.region_constraints.clone())
            .collect();

        // Compute hash
        let hash = Self::compute_module_hash(module)?;

        Ok(DependencyEntry {
            name,
            version,
            source,
            hash,
            effects,
            capabilities,
            type_summary,
            region_constraints,
            transitive_deps: Vec::new(),
        })
    }

    /// Compute hash for a module
    fn compute_module_hash(module: &OnceoModule) -> Result<String, LockfileError> {
        let module_json = serde_json::to_string(module)
            .map_err(|e| LockfileError::SerializationError(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(module_json.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute lockfile hash
    fn compute_lockfile_hash(lockfile: &Lockfile) -> Result<String, LockfileError> {
        // Create a copy without the hash field
        let mut lockfile_copy = lockfile.clone();
        lockfile_copy.lockfile_hash = String::new();
        
        let lockfile_json = serde_json::to_string(&lockfile_copy)
            .map_err(|e| LockfileError::SerializationError(e.to_string()))?;
        let mut hasher = Sha256::new();
        hasher.update(lockfile_json.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Get dependency by name
    pub fn get_dependency(&self, name: &str) -> Option<&DependencyEntry> {
        self.lockfile.dependencies.get(name)
    }

    /// Get all dependencies
    pub fn get_all_dependencies(&self) -> Vec<&DependencyEntry> {
        self.lockfile.dependencies.values().collect()
    }

    /// Get capability graph
    pub fn get_capability_graph(&self) -> &CapabilityGraph {
        &self.lockfile.capability_graph
    }

    /// Verify lockfile integrity
    pub fn verify_integrity(&self) -> Result<(), LockfileError> {
        // Verify lockfile hash
        let computed_hash = Self::compute_lockfile_hash(&self.lockfile)?;
        if self.lockfile.lockfile_hash != computed_hash {
            return Err(LockfileError::HashMismatch(format!(
                "Lockfile hash mismatch: expected {}, got {}",
                computed_hash, self.lockfile.lockfile_hash
            )));
        }

        // Verify all dependencies have valid hashes
        for (name, entry) in &self.lockfile.dependencies {
            if entry.hash.is_empty() {
                return Err(LockfileError::HashMismatch(format!(
                    "Dependency {} has no hash",
                    name
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_onceo::{OnceoBuilder, ObjectMetadata, DebugInfo};

    #[test]
    fn test_lockfile_manager_creation() {
        let manager = LockfileManager::new();
        assert!(manager.lockfile.dependencies.is_empty());
        assert_eq!(manager.lockfile.version, "1.0");
    }

    #[test]
    fn test_add_dependency() {
        let mut manager = LockfileManager::new();
        let entry = DependencyEntry {
            name: "test-dep".to_string(),
            version: "1.0.0".to_string(),
            source: DependencySource::Local,
            hash: "abc123".to_string(),
            effects: vec!["Async".to_string()],
            capabilities: vec!["network".to_string()],
            type_summary: vec!["main: Function".to_string()],
            region_constraints: vec![],
            transitive_deps: vec![],
        };

        manager.add_dependency(entry.clone()).unwrap();
        assert_eq!(manager.lockfile.dependencies.len(), 1);
        assert_eq!(manager.get_dependency("test-dep").unwrap().name, "test-dep");
    }

    #[test]
    fn test_version_conflict() {
        let mut manager = LockfileManager::new();
        let entry1 = DependencyEntry {
            name: "test-dep".to_string(),
            version: "1.0.0".to_string(),
            source: DependencySource::Local,
            hash: "abc123".to_string(),
            effects: vec![],
            capabilities: vec![],
            type_summary: vec![],
            region_constraints: vec![],
            transitive_deps: vec![],
        };

        manager.add_dependency(entry1).unwrap();

        let entry2 = DependencyEntry {
            name: "test-dep".to_string(),
            version: "2.0.0".to_string(),
            source: DependencySource::Local,
            hash: "def456".to_string(),
            effects: vec![],
            capabilities: vec![],
            type_summary: vec![],
            region_constraints: vec![],
            transitive_deps: vec![],
        };

        let result = manager.add_dependency(entry2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LockfileError::VersionConflict(_)));
    }

    #[test]
    fn test_create_entry_from_module() {
        let mut builder = OnceoBuilder::new("test-module".to_string());
        let module = builder.build();

        let entry = LockfileManager::create_entry_from_module(
            "test-module".to_string(),
            "1.0.0".to_string(),
            DependencySource::Local,
            &module,
        ).unwrap();

        assert_eq!(entry.name, "test-module");
        assert_eq!(entry.version, "1.0.0");
        assert!(!entry.hash.is_empty());
    }

    #[test]
    fn test_capability_graph() {
        let mut manager = LockfileManager::new();
        let entry = DependencyEntry {
            name: "test-dep".to_string(),
            version: "1.0.0".to_string(),
            source: DependencySource::Local,
            hash: "abc123".to_string(),
            effects: vec!["Async".to_string()],
            capabilities: vec!["network".to_string()],
            type_summary: vec![],
            region_constraints: vec![],
            transitive_deps: vec!["sub-dep".to_string()],
        };

        manager.add_dependency(entry).unwrap();
        
        let graph = manager.get_capability_graph();
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.edges.len(), 1);
    }
}


