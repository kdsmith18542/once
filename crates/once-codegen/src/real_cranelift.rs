//! Real Cranelift integration using the cranelift-object backend.
//! This module replaces the previous stub implementation with actual code generation.
//! It compiles MIR functions to native object files using Cranelift.

use once_mir::{MirProgram, MirFunction, MirStmt, MirOp, MirLocation, MirValue};
use once_hir::HirType;
use once_rinf::Region;
use thiserror::Error;
use std::collections::HashMap;
use cranelift_codegen::ir::{self, types, AbiParam, Signature, Value};
use cranelift_codegen::isa::CallConv;
use cranelift_codegen::settings;
use cranelift_codegen::Context;
use cranelift_codegen::ir::InstBuilder;
use cranelift_module::{default_libcall_names, Module, Linkage, FuncId, DataId, DataDescription};
use cranelift_object::{ObjectBuilder, ObjectModule};
use cranelift_native::builder as native_builder;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};

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
    /// Mapping from string literals to data object ids.
    string_data: HashMap<String, DataId>,
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
            string_data: HashMap::new(),
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

            // Declare malloc: size_t -> void*
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("malloc", Linkage::Import, &sig) {
                    declared.push(("malloc".to_string(), func_id));
                }
            }

            // Declare free: void* -> void
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("free", Linkage::Import, &sig) {
                    declared.push(("free".to_string(), func_id));
                }
            }

            // Declare runtime concurrency functions (resolved at link time)
            // once_runtime_spawn(func_ptr: i64, args_ptr: i64) -> task_handle: i64
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("once_runtime_spawn", Linkage::Import, &sig) {
                    declared.push(("once_runtime_spawn".to_string(), func_id));
                }
            }
            // once_runtime_send(channel_id: i64, value: i64) -> status: i64
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("once_runtime_send", Linkage::Import, &sig) {
                    declared.push(("once_runtime_send".to_string(), func_id));
                }
            }
            // once_runtime_recv(channel_id: i64) -> value: i64
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("once_runtime_recv", Linkage::Import, &sig) {
                    declared.push(("once_runtime_recv".to_string(), func_id));
                }
            }
            // once_runtime_await(task_handle: i64) -> result: i64
            {
                let mut sig = Signature::new(CallConv::SystemV);
                sig.params.push(AbiParam::new(types::I64));
                sig.returns.push(AbiParam::new(types::I64));
                if let Ok(func_id) = module.declare_function("once_runtime_await", Linkage::Import, &sig) {
                    declared.push(("once_runtime_await".to_string(), func_id));
                }
            }
        } // module borrow ends

        // Populate func_map
        for (name, func_id) in declared {
            self.func_map.insert(name, func_id);
        }

        // Pre-declare string literal data
        {
            let module = self.module.as_mut().ok_or_else(||
                RealCodegenError::ModuleError("Module not initialized".to_string()))?;
            let mut seen_strings: HashMap<String, bool> = HashMap::new();
            let mut str_counter: usize = 0;
            for mir_fn in &mir.functions {
                for stmt in &mir_fn.body.statements {
                    if let MirOp::LoadLiteral { value: MirValue::String(s), .. } = &stmt.op {
                        if !seen_strings.contains_key(s) {
                            seen_strings.insert(s.clone(), true);
                            let data_name = format!("str_{}", str_counter);
                            str_counter += 1;
                            let mut data_desc = DataDescription::new();
                            let mut bytes = s.as_bytes().to_vec();
                            bytes.push(0); // null terminator
                            data_desc.define(bytes.into_boxed_slice());
                            let data_id = module
                                .declare_data(&data_name, Linkage::Local, false, false)
                                .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;
                            module
                                .define_data(data_id, &data_desc)
                                .map_err(|e| RealCodegenError::ModuleError(e.to_string()))?;
                            self.string_data.insert(s.clone(), data_id);
                        }
                    }
                }
            }
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

        // Variable tracking: each MirLocation maps to a Cranelift Variable
        let mut var_map: HashMap<MirLocation, Variable> = HashMap::new();
        let mut declared_vars: Vec<Variable> = Vec::new();
        let mut next_var_idx = 0u32;

        // Track allocations per region for FreeRegion support
        let mut region_allocations: HashMap<Region, Vec<Variable>> = HashMap::new();

        let param_values: Vec<Value> = builder.block_params(entry_block).to_vec();
        for (i, (_, _)) in mir_fn.params.iter().enumerate() {
            let value = param_values[i];
            let loc = MirLocation::Param(i);
            let var = *var_map.entry(loc).or_insert_with(|| {
                let v = Variable::from_u32(next_var_idx);
                next_var_idx += 1;
                v
            });
            if !declared_vars.contains(&var) {
                builder.declare_var(var, types::I64);
                declared_vars.push(var);
            }
            builder.def_var(var, value);
        }

        // Pre-create Cranelift blocks for all MIR labels
        let mut label_blocks: HashMap<usize, ir::Block> = HashMap::new();
        for stmt in &mir_fn.body.statements {
            if let MirOp::Label { id } = &stmt.op {
                let block = builder.create_block();
                label_blocks.insert(*id, block);
            }
        }

        let mut current_terminated = false;

        for stmt in &mir_fn.body.statements {
            match &stmt.op {
                MirOp::Label { id } => {
                    let new_block = *label_blocks.get(id).ok_or_else(|| 
                        RealCodegenError::UnsupportedOp(format!("Unknown label: {}", id)))?;
                    if !current_terminated {
                        builder.ins().jump(new_block, &[]);
                    }
                    builder.switch_to_block(new_block);
                    current_terminated = false;
                }
                MirOp::Jump { target } => {
                    let target_block = *label_blocks.get(target).ok_or_else(|| 
                        RealCodegenError::UnsupportedOp(format!("Unknown jump target: {}", target)))?;
                    builder.ins().jump(target_block, &[]);
                    current_terminated = true;
                }
                MirOp::Branch { condition, true_target, false_target } => {
                    let cond_var = *var_map.get(condition).ok_or_else(|| 
                        RealCodegenError::UnsupportedOp(format!("Branch condition variable not found: {:?}", condition)))?;
                    let cond_val = builder.use_var(cond_var);
                    let true_block = *label_blocks.get(true_target).ok_or_else(|| 
                        RealCodegenError::UnsupportedOp(format!("Unknown branch target: {}", true_target)))?;
                    let false_block = *label_blocks.get(false_target).ok_or_else(|| 
                        RealCodegenError::UnsupportedOp(format!("Unknown branch target: {}", false_target)))?;
                    builder.ins().brif(cond_val, true_block, &[], false_block, &[]);
                    current_terminated = true;
                }
                MirOp::Return { value } => {
                    if let Some(loc) = value {
                        let var = *var_map.get(loc).ok_or_else(|| 
                            RealCodegenError::UnsupportedOp(format!("Return variable not found: {:?}", loc)))?;
                        let val = builder.use_var(var);
                        builder.ins().return_(&[val]);
                    } else {
                        builder.ins().return_(&[]);
                    }
                    current_terminated = true;
                }
                _ => {
                    self.translate_non_terminator(&mut builder, stmt, &mut var_map, &mut declared_vars, &mut next_var_idx, &mut region_allocations)
                        .map_err(|e| RealCodegenError::FunctionCompilationFailed(e.to_string()))?;
                }
            }
        }

        // Ensure the last block is terminated
        if !current_terminated {
            builder.ins().return_(&[]);
        }

        // Seal all blocks and finalize
        builder.seal_all_blocks();
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
            HirType::Bool => types::I64,
            // Strings and complex types are represented as pointers (i64)
            HirType::Str => types::I64,
            HirType::Hole => types::I64, // type hole treated as i64 placeholder
            HirType::Unit => types::I64, // placeholder; used only as param maybe
            HirType::Linear(..) | HirType::Affine(..) => types::I64,
            HirType::Array(..) => types::I64,
            HirType::Generic(..) => types::I64,
            HirType::Tuple(..) => types::I64,
            HirType::Function(..) => types::I64,
            HirType::Ident(_) => types::I64, // assume identifier refers to some type, treat as pointer for now
        }
    }

    /// Translate a non-terminator MIR statement to Cranelift IR.
    /// Terminators (Return, Jump, Branch) and block headers (Label) are handled
    /// by define_function to manage Cranelift block switching correctly.
    fn translate_non_terminator(
        &mut self,
        builder: &mut FunctionBuilder,
        stmt: &MirStmt,
        var_map: &mut HashMap<MirLocation, Variable>,
        declared_vars: &mut Vec<Variable>,
        next_var_idx: &mut u32,
        region_allocations: &mut HashMap<Region, Vec<Variable>>,
    ) -> Result<(), RealCodegenError> {
        let get_or_create_var = |var_map: &mut HashMap<MirLocation, Variable>, next_var_idx: &mut u32, loc: &MirLocation| -> Variable {
            *var_map.entry(loc.clone()).or_insert_with(|| {
                let v = Variable::from_u32(*next_var_idx);
                *next_var_idx += 1;
                v
            })
        };

        let declare_var = |builder: &mut FunctionBuilder, declared_vars: &mut Vec<Variable>, var: Variable| {
            if !declared_vars.contains(&var) {
                builder.declare_var(var, types::I64);
                declared_vars.push(var);
            }
        };

        match &stmt.op {
            MirOp::LoadLiteral { value, dest } => {
                let clif_val = match value {
                    MirValue::Int(n) => builder.ins().iconst(types::I64, *n),
                    MirValue::Float(f) => builder.ins().f64const(*f),
                    MirValue::Bool(b) => builder.ins().iconst(types::I64, if *b { 1 } else { 0 }),
                    MirValue::String(s) => {
                        match self.string_data.get(s) {
                            Some(data_id) => {
                                let module = self.module.as_mut().ok_or_else(||
                                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                                let gv = module.declare_data_in_func(*data_id, &mut builder.func);
                                builder.ins().global_value(types::I64, gv)
                            }
                            None => builder.ins().iconst(types::I64, 0),
                        }
                    }
                    MirValue::Unit => builder.ins().iconst(types::I64, 0),
                };
                let var = get_or_create_var(var_map, next_var_idx, dest);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, clif_val);
                Ok(())
            }
            MirOp::Move { from, to } => {
                let from_var = *var_map.get(from).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("Move from uninitialized variable: {:?}", from))
                })?;
                let val = builder.use_var(from_var);
                let to_var = get_or_create_var(var_map, next_var_idx, to);
                declare_var(builder, declared_vars, to_var);
                builder.def_var(to_var, val);
                Ok(())
            }
            MirOp::Drop { location } => {
                let loc_var = *var_map.get(location).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("Drop of uninitialized variable: {:?}", location))
                })?;
                let ptr = builder.use_var(loc_var);
                let free_id = self.func_map.get("free").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("free not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*free_id, &mut builder.func);
                builder.ins().call(func_ref, &[ptr]);
                Ok(())
            }
            MirOp::FreeRegion { region } => {
                // Iterate over all allocations in this region and free them
                if let Some(allocations) = region_allocations.get(region) {
                    let free_id = self.func_map.get("free").ok_or_else(|| {
                        RealCodegenError::UnsupportedOp("free not declared".to_string())
                    })?;
                    let module = self.module.as_mut().ok_or_else(||
                        RealCodegenError::ModuleError("Module not available".to_string()))?;
                    for alloc_var in allocations.clone() {
                        let ptr = builder.use_var(alloc_var);
                        let func_ref = module.declare_func_in_func(*free_id, &mut builder.func);
                        builder.ins().call(func_ref, &[ptr]);
                    }
                }
                Ok(())
            }
            MirOp::Allocate { region, size, dest } => {
                let size_val = builder.ins().iconst(types::I64, *size as i64);
                let malloc_id = self.func_map.get("malloc").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("malloc not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*malloc_id, &mut builder.func);
                let call_inst = builder.ins().call(func_ref, &[size_val]);
                let ptr = builder.func.dfg.first_result(call_inst);
                let var = get_or_create_var(var_map, next_var_idx, dest);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, ptr);
                // Track allocation in region for FreeRegion
                region_allocations.entry(region.clone()).or_default().push(var);
                Ok(())
            }
            MirOp::BoundsCheck { index, bound, proven } => {
                if *proven {
                    return Ok(());
                }
                let index_var = *var_map.get(index).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("BoundsCheck index variable not found: {:?}", index))
                })?;
                let index_val = builder.use_var(index_var);
                let bound_var = *var_map.get(bound).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("BoundsCheck bound variable not found: {:?}", bound))
                })?;
                let bound_val = builder.use_var(bound_var);

                let continue_block = builder.create_block();
                let trap_block = builder.create_block();

                let cmp = builder.ins().icmp(ir::condcodes::IntCC::UnsignedLessThan, index_val, bound_val);
                builder.ins().brif(cmp, continue_block, &[], trap_block, &[]);

                builder.switch_to_block(trap_block);
                builder.ins().trap(ir::TrapCode::User(0));
                builder.seal_block(trap_block);

                builder.switch_to_block(continue_block);
                Ok(())
            }
            MirOp::ChannelSend { channel, value } => {
                let channel_var = *var_map.get(channel).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("ChannelSend channel variable not found: {:?}", channel))
                })?;
                let channel_val = builder.use_var(channel_var);
                let value_var = *var_map.get(value).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("ChannelSend value variable not found: {:?}", value))
                })?;
                let value_val = builder.use_var(value_var);
                let send_id = self.func_map.get("once_runtime_send").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("once_runtime_send not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*send_id, &mut builder.func);
                builder.ins().call(func_ref, &[channel_val, value_val]);
                Ok(())
            }
            MirOp::ChannelRecv { channel, result } => {
                let channel_var = *var_map.get(channel).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("ChannelRecv channel variable not found: {:?}", channel))
                })?;
                let channel_val = builder.use_var(channel_var);
                let recv_id = self.func_map.get("once_runtime_recv").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("once_runtime_recv not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*recv_id, &mut builder.func);
                let call_inst = builder.ins().call(func_ref, &[channel_val]);
                let recv_val = builder.func.dfg.first_result(call_inst);
                let var = get_or_create_var(var_map, next_var_idx, result);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, recv_val);
                Ok(())
            }
            MirOp::SpawnTask { function, args, result } => {
                // Look up function pointer by name
                let func_ptr = if let Some(func_id) = self.func_map.get(function) {
                    let module = self.module.as_mut().ok_or_else(||
                        RealCodegenError::ModuleError("Module not available".to_string()))?;
                    let func_ref = module.declare_func_in_func(*func_id, &mut builder.func);
                    builder.ins().func_addr(types::I64, func_ref)
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                let mut arg_values = Vec::new();
                for arg_loc in args {
                    let arg_var = *var_map.get(arg_loc).ok_or_else(|| {
                        RealCodegenError::UnsupportedOp(format!("SpawnTask arg variable not found: {:?}", arg_loc))
                    })?;
                    let val = builder.use_var(arg_var);
                    arg_values.push(val);
                }
                // For simplicity, pass the first arg (or 0 if none)
                let arg_ptr = arg_values.first().copied().unwrap_or_else(|| builder.ins().iconst(types::I64, 0));
                let spawn_id = self.func_map.get("once_runtime_spawn").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("once_runtime_spawn not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*spawn_id, &mut builder.func);
                let call_inst = builder.ins().call(func_ref, &[func_ptr, arg_ptr]);
                let task_handle = builder.func.dfg.first_result(call_inst);
                let var = get_or_create_var(var_map, next_var_idx, result);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, task_handle);
                Ok(())
            }
            MirOp::AwaitTask { task, result } => {
                let task_var = *var_map.get(task).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp(format!("AwaitTask task variable not found: {:?}", task))
                })?;
                let task_val = builder.use_var(task_var);
                let await_id = self.func_map.get("once_runtime_await").ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("once_runtime_await not declared".to_string())
                })?;
                let module = self.module.as_mut().ok_or_else(||
                    RealCodegenError::ModuleError("Module not available".to_string()))?;
                let func_ref = module.declare_func_in_func(*await_id, &mut builder.func);
                let call_inst = builder.ins().call(func_ref, &[task_val]);
                let await_result = builder.func.dfg.first_result(call_inst);
                let var = get_or_create_var(var_map, next_var_idx, result);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, await_result);
                Ok(())
            }
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
                    let arg_var = *var_map.get(arg_loc).ok_or_else(|| {
                        RealCodegenError::UnsupportedOp(format!("Call arg variable not found: {:?}", arg_loc))
                    })?;
                    let val = builder.use_var(arg_var);
                    arg_values.push(val);
                }
                
                let call_inst = builder.ins().call(func_ref, &arg_values);
                let result_val = builder.func.dfg.first_result(call_inst);
                let var = get_or_create_var(var_map, next_var_idx, result);
                declare_var(builder, declared_vars, var);
                builder.def_var(var, result_val);
                
                 Ok(())
            }
            MirOp::BinOp { op, left, right, dest } => {
                let left_var = *var_map.get(left).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("BinOp left operand not initialized".to_string())
                })?;
                let right_var = *var_map.get(right).ok_or_else(|| {
                    RealCodegenError::UnsupportedOp("BinOp right operand not initialized".to_string())
                })?;
                let lhs = builder.use_var(left_var);
                let rhs = builder.use_var(right_var);
                let result = match op {
                    once_mir::MirBinOp::Add => builder.ins().iadd(lhs, rhs),
                    once_mir::MirBinOp::Sub => builder.ins().isub(lhs, rhs),
                    once_mir::MirBinOp::Mul => builder.ins().imul(lhs, rhs),
                    once_mir::MirBinOp::Div => builder.ins().udiv(lhs, rhs),
                    once_mir::MirBinOp::Eq => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::Equal, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::Ne => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::NotEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::Lt => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::SignedLessThan, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::Le => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::SignedLessThanOrEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::Gt => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::SignedGreaterThan, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::Ge => {
                        let cmp = builder.ins().icmp(ir::condcodes::IntCC::SignedGreaterThanOrEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    once_mir::MirBinOp::And => builder.ins().band(lhs, rhs),
                    once_mir::MirBinOp::Or => builder.ins().bor(lhs, rhs),
                    once_mir::MirBinOp::Move => lhs,
                };
                let dest_var = get_or_create_var(var_map, next_var_idx, dest);
                declare_var(builder, declared_vars, dest_var);
                builder.def_var(dest_var, result);
                Ok(())
            }
            MirOp::TryBlock { result: _ } => {
                // Try block: placeholder for error context capture
                // In a full implementation, this would instrument error handling with location info
                Ok(())
            }
            MirOp::Return { .. } | MirOp::Jump { .. } | MirOp::Branch { .. } | MirOp::Label { .. } => {
                // These should never reach translate_non_terminator; they are handled in define_function.
                Err(RealCodegenError::UnsupportedOp(
                    "Terminator or label reached translate_non_terminator".to_string()
                ))
            }
        }
    }
}
