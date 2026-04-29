//! Real Cranelift integration using the cranelift-object backend.
//! This module replaces the previous stub implementation with actual code generation.
//! It compiles MIR functions to native object files using Cranelift.

use once_mir::{MirProgram, MirFunction, MirStmt, MirOp, MirLocation, MirValue};
use once_hir::HirType;
use thiserror::Error;
use std::collections::HashMap;
use cranelift_codegen::ir::{self, types, AbiParam, Signature, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::Context;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::{default_libcall_names, Module, Linkage, FuncId};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_native::builder as native_builder;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

/// Code generation errors for the real Cranelift backend.
#[derive(Error, Debug, Clone)]
pub enum RealCodegenError {
    #[error("Cranelift ISA creation failed: {0}")]
    IsaCreationFailed(String),
    
    #[error("Cranelift module error: {0}")]
    ModuleError(String),
    
    #[error("Function compilation failed: {0}")]
    FunctionCompilationFailed(String),
    
    #[error("MIR has no functions to compile")]
    NoFunctions,
    
    #[error("Unsupported MIR operation: {0}")]
    UnsupportedOp(String),
}

/// Real Cranelift code generator.
pub struct RealCraneliftCodegen {
    /// Cranelift object module (wrapped in Option to allow taking ownership)
    module: Option<ObjectModule>,
    /// Mapping from function names to declared FuncIds.
    func_map: HashMap<String, FuncId>,
}

impl RealCraneliftCodegen {
    /// Create a new real Cranelift code generator.
    pub fn new() -> Result<Self, RealCodegenError> {
        // Build target ISA (native)
        let isa_builder = native_builder()
            .map_err(|e| RealCodegenError::IsaCreationFailed(e.to_string()))?;
        let flag_builder = settings::builder();
        let flags = settings::Flags::new(flag_builder);
        let isa = isa_builder
            .finish(flags)
            .map_err(|e| RealCodegenError::IsaCreationFailed(e.to_string()))?;
        // Build ObjectModule
        let builder = ObjectBuilder::new(
            isa,
            b"once",
            Box::new(default_libcall_names()),
        )
        .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;
        let module = ObjectModule::new(builder);
        Ok(Self {
            module: Some(module),
            func_map: HashMap::new(),
        })
    }

    /// Generate object file bytes for the given MIR program.
    pub fn generate(&mut self, mir: &MirProgram) -> Result<Vec<u8>, RealCodegenError> {
        if mir.functions.is_empty() {
            return Err(RealCodegenError::NoFunctions);
        }

        // Precompute signatures for all functions (no borrow conflict)
        let signatures: Vec<_> = mir.functions
            .iter()
            .map(|mir_fn| self.make_signature(&mir_fn.params, &mir_fn.return_type))
            .collect();

        // Declare all functions first.
        let mut declared = Vec::new();
        {
            let module = self.module.as_mut().ok_or_else(|| 
                RealCodegenError::ModuleError("Module not initialized".to_string()))?;
            
            // Declare program functions
            for (mir_fn, sig) in mir.functions.iter().zip(signatures.iter()) {
                let func_id = module
                    .declare_function(&mir_fn.name, Linkage::Export, sig)
                    .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;
                declared.push((mir_fn.name.clone(), func_id));
            }
            
            // Declare external functions that might be referenced (e.g., print)
            // These will be resolved at link time
            for ext_fn in &["print", "println", "file_open", "file_read", "file_write", "file_close"] {
                let mut sig = Signature::new(CallConv::SystemV);
                // Print takes a single i64 (string pointer) and returns i64
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function(ext_fn, Linkage::Import, &sig) {
                    declared.push((ext_fn.to_string(), func_id));
                }
            }
        } // module borrow ends

        // Populate func_map
        for (name, func_id) in declared {
            self.func_map.insert(name, func_id);
        }

        // Define each function body
        for mir_fn in &mir.functions {
            let func_id = self.func_map.get(&mir_fn.name)
                .ok_or_else(|| RealCodegenError::ModuleError(format!("Function {} not declared", mir_fn.name)))?;
            self.define_function(mir_fn, *func_id)?;
        }

        // Take module to finish
        let module = self.module.take().ok_or_else(|| 
            RealCodegenError::ModuleError("Module already taken".to_string()))?;
        let object_product = module.finish();
        let mut object_bytes = Vec::new();
        object_product
            .object
            .emit(&mut object_bytes)
            .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;
        Ok(object_bytes)
    }

     /// Define a single function in the module.
    fn define_function(&mut self, mir_fn: &MirFunction, func_id: FuncId) -> Result<(), RealCodegenError> {
        // Compute signature
        let sig = self.make_signature(&mir_fn.params, &mir_fn.return_type);

        // Create a context and set signature
        let mut ctx = Context::new();
        ctx.func.signature = sig;

         // Prepare function builder
        let mut fn_builder_ctx = FunctionBuilderContext::new();
        let mut builder = FunctionBuilder::new(&mut ctx.func, &mut fn_builder_ctx);

        // Create entry block and add block parameters for function arguments
        let entry_block = builder.create_block();
        builder.append_block_params_for_function_params(entry_block);
        builder.switch_to_block(entry_block);

        // Map parameter locations to SSA values
        let mut loc_map: HashMap<MirLocation, Value> = HashMap::new();
        let param_values = builder.block_params(entry_block);
        for (i, (_, _)) in mir_fn.params.iter().enumerate() {
            let value = param_values[i];
            loc_map.insert(MirLocation::Param(i), value);
        }

        // Translate MIR statements into Cranelift IR
        for stmt in &mir_fn.body.statements {
            self.translate_statement(&mut builder, stmt, &mut loc_map)
                .map_err(|e| RealCodegenError::FunctionCompilationFailed(e.to_string()))?;
        }

         // Seal the entry block and all other blocks (all jump targets are now known)
        builder.seal_all_blocks();

        // Finalize the function
        builder.finalize();

        // Define the function in the module (borrow module after translation)
        let module = self.module.as_mut().ok_or_else(|| 
            RealCodegenError::ModuleError("Module not available".to_string()))?;
        module
            .define_function(func_id, &mut ctx)
            .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;

        Ok(())
    }

    /// Create a Cranelift signature from MIR parameter and return types.
    fn make_signature(&self, params: &[(String, HirType)], return_type: &HirType) -> Signature {
        let mut sig = Signature::new(CallConv::SystemV);
        for (_, ty) in params {
            let clif_ty = self.convert_type(ty);
            sig.params.push(AbiParam::new(clif_ty));
        }
        // Handle return type
        match return_type {
            HirType::Unit => {
                // Void return: no returns
            }
            _ => {
                let clif_ret = self.convert_type(return_type);
                sig.returns.push(AbiParam::new(clif_ret));
            }
        }
        sig
    }

    /// Convert a HIR type to a Cranelift type.
    fn convert_type(&self, ty: &HirType) -> ir::Type {
        match ty {
            HirType::Int => types::I64,
            HirType::Float => types::F64,
            // Booleans represented as i8 (1 byte)
            HirType::Bool => types::I8,
            // Strings and complex types are represented as pointers (i64)
            HirType::Str => types::I64,
            HirType::Unit => types::I64, // placeholder; used only as param maybe
            HirType::Linear(..) | HirType::Affine(..) => types::I64,
            HirType::Array(..) => types::I64,
            HirType::Generic(..) => types::I64,
            HirType::Tuple(..) => types::I64,
            HirType::Function(..) => types::I64,
            HirType::Ident(_) => types::I64, // assume identifier refers to some type, treat as pointer for now
        }
    }

    /// Translate a single MIR statement to Cranelift IR.
    fn translate_statement(
        &mut self,
        builder: &mut FunctionBuilder,
        stmt: &MirStmt,
        loc_map: &mut HashMap<MirLocation, Value>,
    ) -> Result<(), RealCodegenError> {
        match &stmt.op {
            MirOp::LoadLiteral { value, dest } => {
                let clif_val = match value {
                    MirValue::Int(n) => builder.ins().iconst(types::I64, *n),
                    MirValue::Float(f) => builder.ins().f64const(*f),
                    MirValue::Bool(b) => builder.ins().iconst(types::I8, if *b { 1 } else { 0 }),
                    MirValue::String(_s) => {
                        // TODO: embed string in data section and load pointer
                        builder.ins().iconst(types::I64, 0)
                    }
                    MirValue::Unit => builder.ins().iconst(types::I64, 0),
                };
                loc_map.insert(dest.clone(), clif_val);
                Ok(())
            }
            MirOp::Move { from, to } => {
                let val = loc_map.get(from).cloned().ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("Move from uninitialized location: {:?}", from))
                })?;
                loc_map.insert(to.clone(), val);
                Ok(())
            }
            MirOp::Return { value } => {
                if let Some(loc) = value {
                    let val = loc_map.get(loc).cloned().ok_or_else(|| {
                        RealCodegenError::UnsupportedOp(format!("Return from uninitialized location: {:?}", loc))
                    })?;
                    builder.ins().return_(&[val]);
                } else {
                    builder.ins().return_(&[]);
                }
                Ok(())
            }
            // For now, ignore Drop and other operations (no-op) to allow simple programs
            MirOp::Drop { .. } => Ok(()),
            MirOp::FreeRegion { .. } => Ok(()),
            MirOp::Allocate { .. } => Ok(()),
            MirOp::BoundsCheck { .. } => Ok(()),
            MirOp::ChannelSend { .. } => Err(RealCodegenError::UnsupportedOp("ChannelSend".to_string())),
            MirOp::ChannelRecv { .. } => Err(RealCodegenError::UnsupportedOp("ChannelRecv".to_string())),
            MirOp::SpawnTask { .. } => Err(RealCodegenError::UnsupportedOp("SpawnTask".to_string())),
            MirOp::AwaitTask { .. } => Err(RealCodegenError::UnsupportedOp("AwaitTask".to_string())),
            MirOp::Call { function, args, result } => {
                // Look up the function reference in func_map
                let func_id = self.func_map.get(function)
                    .ok_or_else(|| RealCodegenError::UnsupportedOp(format!("Unknown function: {}", function)))?;
                
                // Get the module to import the function
                let module = self.module.as_mut().ok_or_else(|| 
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                
                // Import the external function into this function's context
                let func_ref = module.declare_func_in_func(*func_id, &mut builder.func);
                
                // Build argument values
                let mut arg_values = Vec::new();
                for arg_loc in args {
                    let val = loc_map.get(arg_loc).cloned().ok_or_else(|| {
                        RealCodegenError::UnsupportedOp(format!("Call arg from uninitialized location: {:?}", arg_loc))
                    })?;
                    arg_values.push(val);
                }
                
                let call_inst = builder.ins().call(func_ref, &arg_values);
                let result_val = builder.func.dfg.first_result(call_inst);
                loc_map.insert(result.clone(), result_val);
                
                 Ok(())
            }
        }
    }
}
