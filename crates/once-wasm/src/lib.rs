use once_hir::HirProgram;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use thiserror::Error;
use anyhow::Result;
use sha2::{Sha256, Digest};
use hex;

/// Errors for Wasm Component FFI
#[derive(Error, Debug, Clone)]
pub enum WasmComponentError {
    #[error("Component parsing error: {0}")]
    ComponentParsingError(String),
    #[error("Component encoding error: {0}")]
    ComponentEncodingError(String),
    #[error("Component validation error: {0}")]
    ComponentValidationError(String),
    #[error("PCC-lite validation error: {0}")]
    PccLiteValidationError(String),
    #[error("Type mismatch: {0}")]
    TypeMismatch(String),
    #[error("Effect mismatch: {0}")]
    EffectMismatch(String),
    #[error("Region mismatch: {0}")]
    RegionMismatch(String),
    #[error("Capability violation: {0}")]
    CapabilityViolation(String),
    #[error("Hash verification failed: {0}")]
    HashVerificationFailed(String),
    #[error("Component instantiation error: {0}")]
    ComponentInstantiationError(String),
}

/// PCC-lite proof for component verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PccLiteProof {
    pub component_hash: String,
    pub type_hash: String,
    pub effect_hash: String,
    pub region_hash: String,
    pub capability_hash: String,
    pub signature: String,
    pub timestamp: String,
}

/// Component capability requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentCapabilities {
    pub required_effects: Vec<String>,
    pub required_types: Vec<String>,
    pub required_regions: Vec<String>,
    pub permissions: Vec<String>,
    pub memory_limits: Option<MemoryLimits>,
    pub execution_limits: Option<ExecutionLimits>,
}

/// Memory limits for component execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimits {
    pub max_memory: u32,
    pub max_stack: u32,
    pub max_heap: u32,
}

/// Execution limits for component execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLimits {
    pub max_instructions: u64,
    pub max_calls: u32,
    pub timeout_ms: u64,
}

/// Component interface definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentInterface {
    pub imports: Vec<ComponentImport>,
    pub exports: Vec<ComponentExport>,
    pub types: Vec<String>,
    pub capabilities: ComponentCapabilities,
}

/// Component import definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentImport {
    pub name: String,
    pub interface: String,
    pub type_def: String,
    pub effects: Vec<String>,
    pub regions: Vec<String>,
}

/// Component export definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentExport {
    pub name: String,
    pub interface: String,
    pub type_def: String,
    pub effects: Vec<String>,
    pub regions: Vec<String>,
}

/// Simple component model
#[derive(Debug, Clone)]
pub enum ComponentModel {
    Bytes(Vec<u8>),
    Interface(ComponentInterface),
}

/// Wasm Component FFI handler
pub struct WasmComponentFfi {
    components: HashMap<String, ComponentModel>,
    interfaces: HashMap<String, ComponentInterface>,
    proofs: HashMap<String, PccLiteProof>,
}

impl WasmComponentFfi {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
            interfaces: HashMap::new(),
            proofs: HashMap::new(),
        }
    }

    /// Load a component from bytes
    pub fn load_component(&mut self, name: String, bytes: &[u8]) -> Result<(), WasmComponentError> {
        // Extract interface information
        let interface = self.extract_interface_from_bytes(bytes)?;
        
        // Validate PCC-lite proof if present
        if let Some(proof) = self.extract_pcc_lite_proof_from_bytes(bytes)? {
            self.validate_pcc_lite_proof(&proof, &interface)?;
            self.proofs.insert(name.clone(), proof);
        }

        // Store component bytes and interface
        self.components.insert(name.clone(), ComponentModel::Bytes(bytes.to_vec()));
        self.interfaces.insert(name, interface);
        Ok(())
    }

    /// Extract interface from component bytes
    fn extract_interface_from_bytes(&self, _bytes: &[u8]) -> Result<ComponentInterface, WasmComponentError> {
        let imports = Vec::new();
        let exports = Vec::new();
        let types = Vec::new();
        let capabilities = ComponentCapabilities {
            required_effects: Vec::new(),
            required_types: Vec::new(),
            required_regions: Vec::new(),
            permissions: Vec::new(),
            memory_limits: None,
            execution_limits: None,
        };

        Ok(ComponentInterface {
            imports,
            exports,
            types,
            capabilities,
        })
    }

    /// Extract PCC-lite proof from component bytes
    fn extract_pcc_lite_proof_from_bytes(&self, _bytes: &[u8]) -> Result<Option<PccLiteProof>, WasmComponentError> {
        // Simplified - just return None for now
        Ok(None)
    }

    /// Validate PCC-lite proof
    fn validate_pcc_lite_proof(&self, proof: &PccLiteProof, interface: &ComponentInterface) -> Result<(), WasmComponentError> {
        // Verify component hash
        let component_hash = self.compute_component_hash(interface)?;
        if proof.component_hash != component_hash {
            return Err(WasmComponentError::HashVerificationFailed(
                format!("Component hash mismatch: expected {}, got {}", component_hash, proof.component_hash)
            ));
        }

        // Verify type hash
        let type_hash = self.compute_type_hash(&interface.types)?;
        if proof.type_hash != type_hash {
            return Err(WasmComponentError::HashVerificationFailed(
                format!("Type hash mismatch: expected {}, got {}", type_hash, proof.type_hash)
            ));
        }

        // Verify effect hash
        let effect_hash = self.compute_effect_hash(&interface.capabilities.required_effects)?;
        if proof.effect_hash != effect_hash {
            return Err(WasmComponentError::HashVerificationFailed(
                format!("Effect hash mismatch: expected {}, got {}", effect_hash, proof.effect_hash)
            ));
        }

        // Verify region hash
        let region_hash = self.compute_region_hash(&interface.capabilities.required_regions)?;
        if proof.region_hash != region_hash {
            return Err(WasmComponentError::HashVerificationFailed(
                format!("Region hash mismatch: expected {}, got {}", region_hash, proof.region_hash)
            ));
        }

        // Verify capability hash
        let capability_hash = self.compute_capability_hash(&interface.capabilities)?;
        if proof.capability_hash != capability_hash {
            return Err(WasmComponentError::HashVerificationFailed(
                format!("Capability hash mismatch: expected {}, got {}", capability_hash, proof.capability_hash)
            ));
        }

        Ok(())
    }

    /// Compute component hash
    fn compute_component_hash(&self, interface: &ComponentInterface) -> Result<String, WasmComponentError> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(interface).map_err(|e| WasmComponentError::ComponentParsingError(e.to_string()))?.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute type hash
    fn compute_type_hash(&self, types: &[String]) -> Result<String, WasmComponentError> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(types).map_err(|e| WasmComponentError::ComponentParsingError(e.to_string()))?.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute effect hash
    fn compute_effect_hash(&self, effects: &[String]) -> Result<String, WasmComponentError> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(effects).map_err(|e| WasmComponentError::ComponentParsingError(e.to_string()))?.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute region hash
    fn compute_region_hash(&self, regions: &[String]) -> Result<String, WasmComponentError> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(regions).map_err(|e| WasmComponentError::ComponentParsingError(e.to_string()))?.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Compute capability hash
    fn compute_capability_hash(&self, capabilities: &ComponentCapabilities) -> Result<String, WasmComponentError> {
        let mut hasher = Sha256::new();
        hasher.update(serde_json::to_string(capabilities).map_err(|e| WasmComponentError::ComponentParsingError(e.to_string()))?.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }

    /// Generate PCC-lite proof for a component
    pub fn generate_pcc_lite_proof(&self, interface: &ComponentInterface) -> Result<PccLiteProof, WasmComponentError> {
        let component_hash = self.compute_component_hash(interface)?;
        let type_hash = self.compute_type_hash(&interface.types)?;
        let effect_hash = self.compute_effect_hash(&interface.capabilities.required_effects)?;
        let region_hash = self.compute_region_hash(&interface.capabilities.required_regions)?;
        let capability_hash = self.compute_capability_hash(&interface.capabilities)?;
        
        // Generate signature (simplified for now)
        let signature = format!("{}-{}-{}-{}-{}", component_hash, type_hash, effect_hash, region_hash, capability_hash);
        
        Ok(PccLiteProof {
            component_hash,
            type_hash,
            effect_hash,
            region_hash,
            capability_hash,
            signature,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Create component from HIR program
    pub fn create_component_from_hir(&self, _name: String, hir: &HirProgram) -> Result<Vec<u8>, WasmComponentError> {
        // Simplified component creation - just return serialized interface and proof
        // In a full implementation, this would use wasm-encoder to create a proper Wasm module
        
        let interface = self.extract_interface_from_hir(hir)?;
        let proof = self.generate_pcc_lite_proof(&interface)?;
        
        // Create a simple JSON representation for now
        let component_data = serde_json::json!({
            "interface": interface,
            "proof": proof,
        });
        
        serde_json::to_vec(&component_data)
            .map_err(|e| WasmComponentError::ComponentEncodingError(e.to_string()))
    }

    /// Extract capabilities from HIR
    fn extract_capabilities_from_hir(&self, hir: &HirProgram) -> Result<ComponentCapabilities, WasmComponentError> {
        let mut capabilities = ComponentCapabilities {
            required_effects: Vec::new(),
            required_types: Vec::new(),
            required_regions: Vec::new(),
            permissions: Vec::new(),
            memory_limits: None,
            execution_limits: None,
        };

        // Extract capabilities from HIR analysis
        // This would involve analyzing the program for effects, types, and regions
        for item in &hir.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    // Analyze function for capabilities
                    self.analyze_function_capabilities(fn_decl, &mut capabilities)?;
                }
                _ => {}
            }
        }

        Ok(capabilities)
    }

    /// Analyze function capabilities
    fn analyze_function_capabilities(&self, fn_decl: &once_hir::HirFnDecl, capabilities: &mut ComponentCapabilities) -> Result<(), WasmComponentError> {
        // Analyze function body for effects, types, and regions
        // This is a simplified analysis
        if fn_decl.name.contains("async") {
            capabilities.required_effects.push("Async".to_string());
        }
        if fn_decl.name.contains("channel") {
            capabilities.required_effects.push("Channel".to_string());
        }
        if fn_decl.name.contains("spawn") {
            capabilities.required_effects.push("Spawn".to_string());
        }
        
        Ok(())
    }

    /// Extract interface from HIR
    fn extract_interface_from_hir(&self, hir: &HirProgram) -> Result<ComponentInterface, WasmComponentError> {
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        let types = Vec::new();
        let capabilities = self.extract_capabilities_from_hir(hir)?;

        // Convert HIR imports to component imports
        for import in &hir.imports {
            let component_import = ComponentImport {
                name: import.alias.clone().unwrap_or_else(|| "unknown".to_string()),
                interface: "function".to_string(),
                type_def: "func".to_string(),
                effects: Vec::new(),
                regions: Vec::new(),
            };
            imports.push(component_import);
        }

        // Convert HIR functions to component exports
        for item in &hir.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    let component_export = ComponentExport {
                        name: fn_decl.name.clone(),
                        interface: "function".to_string(),
                        type_def: "func".to_string(),
                        effects: Vec::new(),
                        regions: Vec::new(),
                    };
                    exports.push(component_export);
                }
                _ => {}
            }
        }

        Ok(ComponentInterface {
            imports,
            exports,
            types,
            capabilities,
        })
    }

    /// Get component interface
    pub fn get_interface(&self, name: &str) -> Option<&ComponentInterface> {
        self.interfaces.get(name)
    }

    /// Get PCC-lite proof
    pub fn get_proof(&self, name: &str) -> Option<&PccLiteProof> {
        self.proofs.get(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral, HirParam};
    // use once_lex::Span;

    #[test]
    fn test_wasm_component_ffi_creation() {
        let ffi = WasmComponentFfi::new();
        assert!(ffi.components.is_empty());
        assert!(ffi.interfaces.is_empty());
        assert!(ffi.proofs.is_empty());
    }

    #[test]
    fn test_component_interface_extraction() {
        let ffi = WasmComponentFfi::new();
        let hir = create_test_hir();
        let interface = ffi.extract_interface_from_hir(&hir).unwrap();
        
        assert!(!interface.exports.is_empty());
        assert_eq!(interface.exports[0].name, "main");
    }

    #[test]
    fn test_pcc_lite_proof_generation() {
        let ffi = WasmComponentFfi::new();
        let hir = create_test_hir();
        let interface = ffi.extract_interface_from_hir(&hir).unwrap();
        let proof = ffi.generate_pcc_lite_proof(&interface).unwrap();
        
        assert!(!proof.component_hash.is_empty());
        assert!(!proof.type_hash.is_empty());
        assert!(!proof.effect_hash.is_empty());
        assert!(!proof.region_hash.is_empty());
        assert!(!proof.capability_hash.is_empty());
        assert!(!proof.signature.is_empty());
        assert!(!proof.timestamp.is_empty());
    }

    #[test]
    fn test_component_creation_from_hir() {
        let ffi = WasmComponentFfi::new();
        let hir = create_test_hir();
        let component_bytes = ffi.create_component_from_hir("test".to_string(), &hir).unwrap();
        
        assert!(!component_bytes.is_empty());
    }

    fn create_test_hir() -> HirProgram {
        let mut program = HirProgram {
            items: Vec::new(),
            imports: Vec::new(),
        };
        
        let fn_decl = HirFnDecl {
            name: "main".to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            return_type: None,
            effects: None,
            body: HirBlock {
                statements: vec![],
                span: None,
            },
            is_public: false,
            span: None,
        };
        
        program.items.push(HirItem::FnDecl(fn_decl));
        program
    }
}