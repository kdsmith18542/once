use once_hir::HirProgram;
use once_ty::{Type, TypeVar, Region};
use once_ty::effects::{EffectRow, EffectLabel};
use once_rinf::{RegionConstraint, RegionDag};
use std::collections::HashMap;
use thiserror::Error;

/// Errors for .onceo object module format
#[derive(Error, Debug, Clone)]
pub enum OnceoError {
    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Deserialization error: {0}")]
    DeserializationError(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Invalid format: {0}")]
    InvalidFormat(String),
}

/// Type summary for a function or variable
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeSummary {
    pub name: String,
    pub type_scheme: String, // Simplified to avoid serde issues
    pub type_variables: Vec<String>,
    pub constraints: Vec<TypeConstraint>,
}

/// Type constraint information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TypeConstraint {
    pub constraint_type: ConstraintType,
    pub left: String,
    pub right: String,
}

/// Type of constraint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ConstraintType {
    Equality,
    Linearity,
    Affinity,
    Region,
}

/// Effect summary for a function
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EffectSummary {
    pub function_name: String,
    pub effect_row: String, // Simplified to avoid serde issues
    pub effect_labels: Vec<String>,
    pub effect_constraints: Vec<EffectConstraint>,
}

/// Effect constraint information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EffectConstraint {
    pub constraint_type: EffectConstraintType,
    pub left: String,
    pub right: String,
}

/// Type of effect constraint
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum EffectConstraintType {
    Subsumption,
    Union,
    Intersection,
}

/// Region summary for a function
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RegionSummary {
    pub function_name: String,
    pub region_variables: Vec<String>,
    pub region_constraints: Vec<String>,
    pub region_dag: String, // Simplified to avoid serde issues
    pub escape_analysis: EscapeAnalysis,
}

/// Escape analysis results
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EscapeAnalysis {
    pub escaping_variables: Vec<String>,
    pub non_escaping_variables: Vec<String>,
    pub region_assignments: HashMap<String, String>,
}

/// Object module metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectMetadata {
    pub module_name: String,
    pub version: String,
    pub dependencies: Vec<String>,
    pub compilation_time: String,
    pub target_architecture: String,
    pub optimization_level: String,
}

/// Complete .onceo object module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnceoModule {
    pub metadata: ObjectMetadata,
    pub type_summaries: Vec<TypeSummary>,
    pub effect_summaries: Vec<EffectSummary>,
    pub region_summaries: Vec<RegionSummary>,
    pub machine_code: Vec<u8>,
    pub debug_info: DebugInfo,
}

/// Debug information for the module
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DebugInfo {
    pub source_files: Vec<String>,
    pub line_mappings: HashMap<usize, usize>,
    pub symbol_table: HashMap<String, SymbolInfo>,
}

/// Symbol information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolInfo {
    pub name: String,
    pub type_summary: TypeSummary,
    pub effect_summary: Option<EffectSummary>,
    pub region_summary: Option<RegionSummary>,
    pub address: Option<usize>,
    pub size: Option<usize>,
}

/// Onceo module builder
pub struct OnceoBuilder {
    metadata: ObjectMetadata,
    type_summaries: Vec<TypeSummary>,
    effect_summaries: Vec<EffectSummary>,
    region_summaries: Vec<RegionSummary>,
    machine_code: Vec<u8>,
    debug_info: DebugInfo,
}

impl OnceoBuilder {
    pub fn new(module_name: String) -> Self {
        Self {
            metadata: ObjectMetadata {
                module_name,
                version: "0.1.0".to_string(),
                dependencies: Vec::new(),
                compilation_time: chrono::Utc::now().to_rfc3339(),
                target_architecture: "x86_64".to_string(),
                optimization_level: "O2".to_string(),
            },
            type_summaries: Vec::new(),
            effect_summaries: Vec::new(),
            region_summaries: Vec::new(),
            machine_code: Vec::new(),
            debug_info: DebugInfo {
                source_files: Vec::new(),
                line_mappings: HashMap::new(),
                symbol_table: HashMap::new(),
            },
        }
    }

    pub fn add_type_summary(&mut self, summary: TypeSummary) {
        self.type_summaries.push(summary);
    }

    pub fn add_effect_summary(&mut self, summary: EffectSummary) {
        self.effect_summaries.push(summary);
    }

    pub fn add_region_summary(&mut self, summary: RegionSummary) {
        self.region_summaries.push(summary);
    }

    pub fn set_machine_code(&mut self, code: Vec<u8>) {
        self.machine_code = code;
    }

    pub fn add_debug_info(&mut self, source_file: String, line_mapping: HashMap<usize, usize>) {
        self.debug_info.source_files.push(source_file);
        for (line, mapping) in line_mapping {
            self.debug_info.line_mappings.insert(line, mapping);
        }
    }

    pub fn add_symbol(&mut self, symbol: SymbolInfo) {
        self.debug_info.symbol_table.insert(symbol.name.clone(), symbol);
    }

    pub fn build(self) -> OnceoModule {
        OnceoModule {
            metadata: self.metadata,
            type_summaries: self.type_summaries,
            effect_summaries: self.effect_summaries,
            region_summaries: self.region_summaries,
            machine_code: self.machine_code,
            debug_info: self.debug_info,
        }
    }
}

/// Onceo module reader/writer
pub struct OnceoModuleHandler;

impl OnceoModuleHandler {
    /// Write a .onceo module to a file
    pub fn write_module(module: &OnceoModule, path: &str) -> Result<(), OnceoError> {
        let serialized = bincode::serialize(module)
            .map_err(|e| OnceoError::SerializationError(format!("Failed to serialize module: {}", e)))?;
        
        std::fs::write(path, serialized)
            .map_err(|e| OnceoError::IoError(format!("Failed to write module to {}: {}", path, e)))?;
        
        Ok(())
    }

    /// Read a .onceo module from a file
    pub fn read_module(path: &str) -> Result<OnceoModule, OnceoError> {
        let data = std::fs::read(path)
            .map_err(|e| OnceoError::IoError(format!("Failed to read module from {}: {}", path, e)))?;
        
        let module = bincode::deserialize(&data)
            .map_err(|e| OnceoError::DeserializationError(format!("Failed to deserialize module: {}", e)))?;
        
        Ok(module)
    }

    /// Validate a .onceo module
    pub fn validate_module(module: &OnceoModule) -> Result<(), OnceoError> {
        // Check metadata
        if module.metadata.module_name.is_empty() {
            return Err(OnceoError::InvalidFormat("Module name cannot be empty".to_string()));
        }

        // Check type summaries
        for summary in &module.type_summaries {
            if summary.name.is_empty() {
                return Err(OnceoError::InvalidFormat("Type summary name cannot be empty".to_string()));
            }
        }

        // Check effect summaries
        for summary in &module.effect_summaries {
            if summary.function_name.is_empty() {
                return Err(OnceoError::InvalidFormat("Effect summary function name cannot be empty".to_string()));
            }
        }

        // Check region summaries
        for summary in &module.region_summaries {
            if summary.function_name.is_empty() {
                return Err(OnceoError::InvalidFormat("Region summary function name cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Extract type information from a module
    pub fn extract_type_info<'a>(module: &'a OnceoModule, symbol_name: &str) -> Option<&'a TypeSummary> {
        module.type_summaries.iter().find(|s| s.name == symbol_name)
    }

    /// Extract effect information from a module
    pub fn extract_effect_info<'a>(module: &'a OnceoModule, function_name: &str) -> Option<&'a EffectSummary> {
        module.effect_summaries.iter().find(|s| s.function_name == function_name)
    }

    /// Extract region information from a module
    pub fn extract_region_info<'a>(module: &'a OnceoModule, function_name: &str) -> Option<&'a RegionSummary> {
        module.region_summaries.iter().find(|s| s.function_name == function_name)
    }

    /// Get all symbols in a module
    pub fn get_symbols(module: &OnceoModule) -> Vec<&SymbolInfo> {
        module.debug_info.symbol_table.values().collect()
    }

    /// Get dependencies of a module
    pub fn get_dependencies(module: &OnceoModule) -> &Vec<String> {
        &module.metadata.dependencies
    }
}

/// Create a .onceo module from a HIR program
pub fn create_module_from_hir(
    hir: &HirProgram,
    module_name: String,
    machine_code: Vec<u8>,
) -> Result<OnceoModule, OnceoError> {
    let mut builder = OnceoBuilder::new(module_name);
    
    // Extract type information from HIR
    for item in &hir.items {
        match item {
            once_hir::HirItem::FnDecl(fn_decl) => {
                // Create type summary
                let type_summary = TypeSummary {
                    name: fn_decl.name.clone(),
                    type_scheme: "Function".to_string(), // Simplified for now
                    type_variables: Vec::new(), // TODO: Extract type variables
                    constraints: Vec::new(), // TODO: Extract constraints
                };
                builder.add_type_summary(type_summary);
            }
            _ => {} // Handle other items as needed
        }
    }
    
    builder.set_machine_code(machine_code);
    Ok(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_onceo_builder() {
        let mut builder = OnceoBuilder::new("test_module".to_string());
        
        let type_summary = TypeSummary {
            name: "test_function".to_string(),
            type_scheme: "Int".to_string(),
            type_variables: Vec::new(),
            constraints: Vec::new(),
        };
        builder.add_type_summary(type_summary);
        
        let module = builder.build();
        assert_eq!(module.metadata.module_name, "test_module");
        assert_eq!(module.type_summaries.len(), 1);
        assert_eq!(module.type_summaries[0].name, "test_function");
    }

    #[test]
    fn test_module_validation() {
        let mut builder = OnceoBuilder::new("".to_string()); // Empty name
        let module = builder.build();
        
        let result = OnceoModuleHandler::validate_module(&module);
        assert!(result.is_err());
    }

    #[test]
    fn test_symbol_extraction() {
        let mut builder = OnceoBuilder::new("test_module".to_string());
        
        let symbol = SymbolInfo {
            name: "test_symbol".to_string(),
            type_summary: TypeSummary {
                name: "test_symbol".to_string(),
                type_scheme: "Int".to_string(),
                type_variables: Vec::new(),
                constraints: Vec::new(),
            },
            effect_summary: None,
            region_summary: None,
            address: Some(0x1000),
            size: Some(8),
        };
        builder.add_symbol(symbol);
        
        let module = builder.build();
        let symbols = OnceoModuleHandler::get_symbols(&module);
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "test_symbol");
    }
}
