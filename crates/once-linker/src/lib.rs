use once_onceo::{OnceoModule, EffectSummary, TypeSummary, RegionSummary};
use once_ty::effects::{EffectRow, EffectLabel};
use once_ty::{Type, TypeVar};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

/// Errors for the capability-aware linker
#[derive(Error, Debug, Clone)]
pub enum LinkerError {
    #[error("Effect mismatch: {0}")]
    EffectMismatch(String),

    #[error("Type mismatch: {0}")]
    TypeMismatch(String),

    #[error("Region mismatch: {0}")]
    RegionMismatch(String),

    #[error("Capability violation: {0}")]
    CapabilityViolation(String),

    #[error("Symbol not found: {0}")]
    SymbolNotFound(String),

    #[error("Circular dependency: {0}")]
    CircularDependency(String),

    #[error("IO error: {0}")]
    IoError(String),
}

/// Capability requirements for a module
#[derive(Debug, Clone)]
pub struct CapabilityRequirements {
    pub required_effects: Vec<String>,
    pub required_types: Vec<Type>,
    pub required_regions: Vec<String>,
    pub permissions: Vec<Permission>,
}

/// Permission types
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    FileSystem(String), // Path access
    Network(String),    // Network access
    System(String),     // System calls
    Memory(String),     // Memory access
    Custom(String),     // Custom capability
}

/// Capability-aware linker
pub struct CapabilityLinker {
    modules: HashMap<String, OnceoModule>,
    dependencies: HashMap<String, Vec<String>>,
    capability_requirements: HashMap<String, CapabilityRequirements>,
    effect_constraints: HashMap<String, String>,
    type_constraints: HashMap<String, String>,
    region_constraints: HashMap<String, String>,
}

impl CapabilityLinker {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            dependencies: HashMap::new(),
            capability_requirements: HashMap::new(),
            effect_constraints: HashMap::new(),
            type_constraints: HashMap::new(),
            region_constraints: HashMap::new(),
        }
    }

    /// Add a module to the linker
    pub fn add_module(&mut self, name: String, module: OnceoModule) -> Result<(), LinkerError> {
        // Check for circular dependencies
        if self.has_circular_dependency(&name, &module.metadata.dependencies) {
            return Err(LinkerError::CircularDependency(format!("Circular dependency detected for module {}", name)));
        }

        // Track dependencies for future circular checks
        self.dependencies.insert(name.clone(), module.metadata.dependencies.clone());

        // Extract capability requirements
        let requirements = self.extract_capability_requirements(&module)?;
        self.capability_requirements.insert(name.clone(), requirements);

        // Extract effect constraints
        for effect_summary in &module.effect_summaries {
            self.effect_constraints.insert(
                effect_summary.function_name.clone(),
                effect_summary.effect_row.clone(),
            );
        }

        // Extract type constraints
        for type_summary in &module.type_summaries {
            self.type_constraints.insert(
                type_summary.name.clone(),
                type_summary.type_scheme.clone(),
            );
        }

        // Extract region constraints
        for region_summary in &module.region_summaries {
            self.region_constraints.insert(
                region_summary.function_name.clone(),
                region_summary.region_dag.clone(),
            );
        }

        self.modules.insert(name, module);
        Ok(())
    }

    /// Link modules with capability enforcement
    pub fn link(&mut self, entry_point: &str) -> Result<LinkedModule, LinkerError> {
        let mut linked_functions = HashMap::new();
        let mut linked_types = HashMap::new();
        let mut linked_effects = HashMap::new();
        let mut linked_regions = HashMap::new();
        let mut resolved_dependencies = HashSet::new();

        // Resolve dependencies starting from entry point
        self.resolve_dependencies(entry_point, &mut resolved_dependencies)?;

        // Link all resolved modules
        for module_name in &resolved_dependencies {
            if let Some(module) = self.modules.get(module_name) {
                // Link functions with effect enforcement
                for effect_summary in &module.effect_summaries {
                    self.enforce_effect_constraints(effect_summary)?;
                    linked_effects.insert(
                        effect_summary.function_name.clone(),
                        effect_summary.clone(),
                    );
                }

                // Link types with type checking
                for type_summary in &module.type_summaries {
                    self.enforce_type_constraints(type_summary)?;
                    linked_types.insert(
                        type_summary.name.clone(),
                        type_summary.clone(),
                    );
                }

                // Link regions with region checking
                for region_summary in &module.region_summaries {
                    self.enforce_region_constraints(region_summary)?;
                    linked_regions.insert(
                        region_summary.function_name.clone(),
                        region_summary.clone(),
                    );
                }
            }
        }

        Ok(LinkedModule {
            functions: linked_functions,
            types: linked_types,
            effects: linked_effects,
            regions: linked_regions,
            entry_point: entry_point.to_string(),
        })
    }

    /// Check for circular dependencies
    fn has_circular_dependency(&self, module_name: &str, dependencies: &[String]) -> bool {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        
        self.dfs_circular_check_with_deps(module_name, &mut visited, &mut stack, Some(dependencies))
    }

    fn dfs_circular_check_with_deps(&self, module_name: &str, visited: &mut HashSet<String>, stack: &mut HashSet<String>, pending_deps: Option<&[String]>) -> bool {
        if stack.contains(module_name) {
            return true;
        }
        
        if visited.contains(module_name) {
            return false;
        }

        visited.insert(module_name.to_string());
        stack.insert(module_name.to_string());

        if let Some(deps) = self.dependencies.get(module_name) {
            for dep in deps {
                if self.dfs_circular_check_with_deps(dep, visited, stack, None) {
                    return true;
                }
            }
        } else if let Some(deps) = pending_deps {
            for dep in deps {
                if self.dfs_circular_check_with_deps(dep, visited, stack, None) {
                    return true;
                }
            }
        }

        stack.remove(module_name);
        false
    }

    /// Extract capability requirements from a module
    fn extract_capability_requirements(&self, module: &OnceoModule) -> Result<CapabilityRequirements, LinkerError> {
        let mut required_effects = Vec::new();
        let mut required_types = Vec::new();
        let mut required_regions = Vec::new();
        let mut permissions = Vec::new();

        // Extract from effect summaries
        for effect_summary in &module.effect_summaries {
            for label in &effect_summary.effect_labels {
                required_effects.push(label.clone());
            }
        }

        // Extract from type summaries
        for type_summary in &module.type_summaries {
            let parsed_type = self.parse_type_scheme(&type_summary.type_scheme);
            required_types.push(parsed_type);
        }

        // Extract from region summaries
        for region_summary in &module.region_summaries {
            for region_var in &region_summary.region_variables {
                required_regions.push(region_var.clone());
            }
        }

        // Extract permissions from metadata
        for dep in &module.metadata.dependencies {
            match dep.as_str() {
                "fs" | "std::fs" | "io" => permissions.push(Permission::FileSystem(dep.clone())),
                "net" | "std::net" | "http" => permissions.push(Permission::Network(dep.clone())),
                "os" | "std::os" | "sys" => permissions.push(Permission::System(dep.clone())),
                _ => permissions.push(Permission::Memory(dep.clone())),
            }
        }
        // Always grant basic memory access
        if permissions.is_empty() {
            permissions.push(Permission::Memory("heap".to_string()));
            permissions.push(Permission::Memory("stack".to_string()));
        }

        Ok(CapabilityRequirements {
            required_effects,
            required_types,
            required_regions,
            permissions,
        })
    }

    /// Resolve dependencies recursively
    fn resolve_dependencies(&self, module_name: &str, resolved: &mut HashSet<String>) -> Result<(), LinkerError> {
        if resolved.contains(module_name) {
            return Ok(()); // Already resolved
        }

        if let Some(module) = self.modules.get(module_name) {
            // Resolve dependencies first
            for dep in &module.metadata.dependencies {
                self.resolve_dependencies(dep, resolved)?;
            }
            resolved.insert(module_name.to_string());
        } else {
            return Err(LinkerError::SymbolNotFound(format!("Module {} not found", module_name)));
        }

        Ok(())
    }

    /// Enforce effect constraints
    fn enforce_effect_constraints(&self, effect_summary: &EffectSummary) -> Result<(), LinkerError> {
        // Check if effect row is compatible with existing constraints
        if let Some(existing_constraint) = self.effect_constraints.get(&effect_summary.function_name) {
            if existing_constraint != &effect_summary.effect_row {
                return Err(LinkerError::EffectMismatch(format!(
                    "Effect mismatch for function {}: expected {}, got {}",
                    effect_summary.function_name, existing_constraint, effect_summary.effect_row
                )));
            }
        }

        // Check for capability violations
        for label in &effect_summary.effect_labels {
            if self.is_capability_violation(label) {
                return Err(LinkerError::CapabilityViolation(format!(
                    "Capability violation: effect {} requires capabilities not available",
                    label
                )));
            }
        }

        Ok(())
    }

    /// Enforce type constraints
    fn enforce_type_constraints(&self, type_summary: &TypeSummary) -> Result<(), LinkerError> {
        // Check if type is compatible with existing constraints
        if let Some(existing_constraint) = self.type_constraints.get(&type_summary.name) {
            if existing_constraint != &type_summary.type_scheme {
                return Err(LinkerError::TypeMismatch(format!(
                    "Type mismatch for symbol {}: expected {}, got {}",
                    type_summary.name, existing_constraint, type_summary.type_scheme
                )));
            }
        }

        Ok(())
    }

    /// Enforce region constraints
    fn enforce_region_constraints(&self, region_summary: &RegionSummary) -> Result<(), LinkerError> {
        // Check if region DAG is compatible with existing constraints
        if let Some(existing_constraint) = self.region_constraints.get(&region_summary.function_name) {
            if existing_constraint != &region_summary.region_dag {
                return Err(LinkerError::RegionMismatch(format!(
                    "Region mismatch for function {}: expected {}, got {}",
                    region_summary.function_name, existing_constraint, region_summary.region_dag
                )));
            }
        }

        Ok(())
    }

    /// Check if an effect label represents a capability violation
    fn is_capability_violation(&self, label: &str) -> bool {
        matches!(label, "net" | "fs" | "ffi" | "spawn" | "io")
    }

    /// Parse a type scheme string (e.g. "Int -> Int", "fn(Int) -> Bool") into our Type
    fn parse_type_scheme(&self, scheme: &str) -> Type {
        let trimmed = scheme.trim();
        if let Some(arrow_pos) = trimmed.find("->") {
            let params_str = trimmed[..arrow_pos].trim();
            let ret_str = trimmed[arrow_pos + 2..].trim();
            let params = params_str
                .trim_start_matches("fn(")
                .trim_end_matches(')')
                .split(',')
                .filter(|s| !s.trim().is_empty())
                .map(|s| self.parse_simple_type(s.trim()))
                .collect::<Vec<_>>();
            let ret = self.parse_simple_type(ret_str.trim_matches(')').trim());
            Type::Function { params, return_type: Box::new(ret) }
        } else {
            self.parse_simple_type(trimmed)
        }
    }

    fn parse_simple_type(&self, name: &str) -> Type {
        match name {
            "Int" => Type::Int,
            "Bool" => Type::Bool,
            "Float" => Type::Float,
            "Str" => Type::Str,
            "Unit" => Type::Unit,
            "Result" => Type::Result {
                ok_type: Box::new(Type::Int),
                err_type: Box::new(Type::Str),
            },
            "Option" => Type::Option(Box::new(Type::Int)),
            "Array" => Type::Array { element_type: Box::new(Type::Int), size: None },
            other => {
                if let Some(open) = other.find('<') {
                    let close = other.rfind('>').unwrap_or(other.len() - 1);
                    let name_part = &other[..open];
                    let args_str = &other[open + 1..close];
                    let args = args_str.split(',')
                        .map(|s| self.parse_simple_type(s.trim()))
                        .collect();
                    Type::UserDefined { name: name_part.to_string(), args }
                } else if other == "Self" || other.chars().next().map_or(false, |c| c.is_uppercase()) {
                    Type::UserDefined { name: other.to_string(), args: vec![] }
                } else {
                    Type::Var(TypeVar(0))
                }
            }
        }
    }

    /// Get capability requirements for a module
    pub fn get_capability_requirements(&self, module_name: &str) -> Option<&CapabilityRequirements> {
        self.capability_requirements.get(module_name)
    }

    /// Check if all capability requirements are satisfied
    pub fn check_capabilities(&self, module_name: &str, available_capabilities: &[Permission]) -> Result<(), LinkerError> {
        if let Some(requirements) = self.capability_requirements.get(module_name) {
            for required_perm in &requirements.permissions {
                if !available_capabilities.contains(required_perm) {
                    return Err(LinkerError::CapabilityViolation(format!(
                        "Missing required capability: {:?}",
                        required_perm
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Linked module result
#[derive(Debug, Clone)]
pub struct LinkedModule {
    pub functions: HashMap<String, EffectSummary>,
    pub types: HashMap<String, TypeSummary>,
    pub effects: HashMap<String, EffectSummary>,
    pub regions: HashMap<String, RegionSummary>,
    pub entry_point: String,
}

impl LinkedModule {
    /// Get the entry point function
    pub fn get_entry_point(&self) -> Option<&EffectSummary> {
        self.functions.get(&self.entry_point)
    }

    /// Get all linked functions
    pub fn get_functions(&self) -> &HashMap<String, EffectSummary> {
        &self.functions
    }

    /// Get all linked types
    pub fn get_types(&self) -> &HashMap<String, TypeSummary> {
        &self.types
    }

    /// Get all linked effects
    pub fn get_effects(&self) -> &HashMap<String, EffectSummary> {
        &self.effects
    }

    /// Get all linked regions
    pub fn get_regions(&self) -> &HashMap<String, RegionSummary> {
        &self.regions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_onceo::{OnceoModule, ObjectMetadata, TypeSummary, EffectSummary, RegionSummary, EscapeAnalysis};

    #[test]
    fn test_capability_linker_new() {
        let linker = CapabilityLinker::new();
        assert!(linker.modules.is_empty());
        assert!(linker.dependencies.is_empty());
    }

    #[test]
    fn test_add_module() {
        let mut linker = CapabilityLinker::new();
        let module = create_test_module("test_module".to_string());
        
        let result = linker.add_module("test_module".to_string(), module);
        assert!(result.is_ok());
        assert!(linker.modules.contains_key("test_module"));
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut linker = CapabilityLinker::new();
        
        let mut module1 = create_test_module("module1".to_string());
        module1.metadata.dependencies = vec!["module2".to_string()];
        let mut module2 = create_test_module("module2".to_string());
        module2.metadata.dependencies = vec!["module1".to_string()];
        
        linker.add_module("module1".to_string(), module1).unwrap();
        let result = linker.add_module("module2".to_string(), module2);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LinkerError::CircularDependency(_)));
    }

    #[test]
    fn test_capability_requirements() {
        let mut linker = CapabilityLinker::new();
        let module = create_test_module("test_module".to_string());
        
        linker.add_module("test_module".to_string(), module).unwrap();
        
        let requirements = linker.get_capability_requirements("test_module");
        assert!(requirements.is_some());
    }

    fn create_test_module(name: String) -> OnceoModule {
        OnceoModule {
            metadata: ObjectMetadata {
                module_name: name,
                version: "0.1.0".to_string(),
                dependencies: Vec::new(),
                compilation_time: "2024-01-01T00:00:00Z".to_string(),
                target_architecture: "x86_64".to_string(),
                optimization_level: "O2".to_string(),
            },
            type_summaries: vec![TypeSummary {
                name: "test_function".to_string(),
                type_scheme: "Int -> Int".to_string(),
                type_variables: Vec::new(),
                constraints: Vec::new(),
            }],
            effect_summaries: vec![EffectSummary {
                function_name: "test_function".to_string(),
                effect_row: "[]".to_string(),
                effect_labels: Vec::new(),
                effect_constraints: Vec::new(),
            }],
            region_summaries: vec![RegionSummary {
                function_name: "test_function".to_string(),
                region_variables: Vec::new(),
                region_constraints: Vec::new(),
                region_dag: "{}".to_string(),
                escape_analysis: EscapeAnalysis {
                    escaping_variables: Vec::new(),
                    non_escaping_variables: Vec::new(),
                    region_assignments: HashMap::new(),
                },
            }],
            machine_code: Vec::new(),
            debug_info: once_onceo::DebugInfo {
                source_files: Vec::new(),
                line_mappings: HashMap::new(),
                symbol_table: HashMap::new(),
            },
        }
    }
}
