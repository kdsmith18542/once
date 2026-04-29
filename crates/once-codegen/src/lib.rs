//! Code generation for the Once language
//! 
//! Implements Cranelift backend for:
//! - MIR to machine code translation
//! - Function compilation
//! - Memory management code generation
//! - Async runtime integration
//! - Object file generation

use once_mir::*;
use once_rinf::{Region, RegionDag};
use once_lex::Span;
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

// Real Cranelift integration
pub mod real_cranelift;
pub use real_cranelift::{RealCraneliftCodegen, RealCodegenError};

// Import Cranelift types for compatibility
use cranelift_codegen::ir::Signature;
use cranelift_codegen::isa::CallConv;

/// Code generation errors
#[derive(Error, Debug, Clone)]
pub enum CodegenError {
    #[error("Code generation failed: {0}")]
    GenerationFailed(String),
    
    #[error("Function compilation failed: {0}")]
    FunctionCompilationFailed(String),
    
    #[error("Memory management code generation failed: {0}")]
    MemoryManagementFailed(String),
    
    #[error("Async runtime integration failed: {0}")]
    AsyncRuntimeFailed(String),
    
    #[error("Object file generation failed: {0}")]
    ObjectFileFailed(String),
}

/// Code generation context
pub struct CodegenContext {
    pub functions: HashMap<String, CompiledFunction>,
    pub globals: HashMap<String, GlobalVariable>,
    pub regions: RegionDag,
    pub next_label: usize,
    pub next_temp: usize,
}

/// Compiled function
#[derive(Debug, Clone)]
pub struct CompiledFunction {
    pub name: String,
    pub signature: FunctionSignature,
    pub instructions: Vec<Instruction>,
    pub locals: Vec<LocalVariable>,
    pub region_info: Option<Region>,
}

/// Function signature
#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub calling_convention: CallingConvention,
}

/// Parameter
#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
    pub location: ParameterLocation,
}

/// Parameter location
#[derive(Debug, Clone)]
pub enum ParameterLocation {
    Register(String),
    Stack(usize),
    Memory(usize),
}

/// Type representation
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Unit,
    Int(IntWidth),
    Float(FloatWidth),
    Bool,
    String,
    Pointer(Box<Type>),
    Function(FunctionType),
    Struct(Vec<Type>),
    Array(Box<Type>, usize),
}

/// Integer width
#[derive(Debug, Clone, PartialEq)]
pub enum IntWidth {
    I8, I16, I32, I64, I128,
}

/// Float width
#[derive(Debug, Clone, PartialEq)]
pub enum FloatWidth {
    F32, F64,
}

/// Function type
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

/// Calling convention
#[derive(Debug, Clone, PartialEq)]
pub enum CallingConvention {
    SystemV,
    Windows,
    C,
    Once,
}

/// Instruction types
#[derive(Debug, Clone)]
pub enum Instruction {
    /// Load immediate value
    LoadImm { dest: String, value: Immediate, ty: Type },
    /// Load from memory
    Load { dest: String, src: String, ty: Type },
    /// Store to memory
    Store { dest: String, src: String, ty: Type },
    /// Move between locations
    Move { dest: String, src: String, ty: Type },
    /// Function call
    Call { dest: Option<String>, func: String, args: Vec<String>, ty: Type },
    /// Return from function
    Return { value: Option<String> },
    /// Branch instruction
    Branch { condition: String, true_label: String, false_label: String },
    /// Jump instruction
    Jump { label: String },
    /// Label definition
    Label { name: String },
    /// Allocate memory
    Allocate { dest: String, size: usize, alignment: usize },
    /// Free memory
    Free { ptr: String },
    /// Region free
    FreeRegion { region: Region },
    /// Bounds check
    BoundsCheck { index: String, bound: String, proven: bool },
    /// Channel send
    ChannelSend { channel: String, value: String },
    /// Channel receive
    ChannelRecv { channel: String, dest: String },
    /// Spawn task
    SpawnTask { func: String, args: Vec<String>, dest: String },
    /// Await task
    AwaitTask { task: String, dest: String },
    /// Arithmetic operations
    Add { dest: String, left: String, right: String, ty: Type },
    Sub { dest: String, left: String, right: String, ty: Type },
    Mul { dest: String, left: String, right: String, ty: Type },
    Div { dest: String, left: String, right: String, ty: Type },
    /// Comparison operations
    Eq { dest: String, left: String, right: String, ty: Type },
    Ne { dest: String, left: String, right: String, ty: Type },
    Lt { dest: String, left: String, right: String, ty: Type },
    Le { dest: String, left: String, right: String, ty: Type },
    Gt { dest: String, left: String, right: String, ty: Type },
    Ge { dest: String, left: String, right: String, ty: Type },
}

/// Immediate value
#[derive(Debug, Clone)]
pub enum Immediate {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Null,
}

/// Local variable
#[derive(Debug, Clone)]
pub struct LocalVariable {
    pub name: String,
    pub ty: Type,
    pub location: LocalLocation,
}

/// Local variable location
#[derive(Debug, Clone)]
pub enum LocalLocation {
    Register(String),
    Stack(usize),
    Memory(usize),
}

/// Global variable
#[derive(Debug, Clone)]
pub struct GlobalVariable {
    pub name: String,
    pub ty: Type,
    pub initial_value: Option<Immediate>,
    pub is_constant: bool,
}

/// Code generator
pub struct CodeGenerator {
    context: CodegenContext,
    errors: Vec<CodegenError>,
    real_cranelift: Option<RealCraneliftCodegen>,
}

impl CodeGenerator {
    pub fn new(region_dag: RegionDag) -> Self {
        Self {
            context: CodegenContext {
                functions: HashMap::new(),
                globals: HashMap::new(),
                regions: region_dag,
                next_label: 0,
                next_temp: 0,
            },
            errors: Vec::new(),
            real_cranelift: None,
        }
    }

    /// Create a new code generator with real Cranelift integration
    pub fn new_with_cranelift(region_dag: RegionDag) -> Result<Self, RealCodegenError> {
        let real_cranelift = RealCraneliftCodegen::new()?;
        Ok(Self {
            context: CodegenContext {
                functions: HashMap::new(),
                globals: HashMap::new(),
                regions: region_dag,
                next_label: 0,
                next_temp: 0,
            },
            errors: Vec::new(),
            real_cranelift: Some(real_cranelift),
        })
    }

    /// Generate code using real Cranelift backend
    fn generate_with_cranelift(mir: &MirProgram, cranelift: &mut RealCraneliftCodegen) -> Result<CompiledProgram, Vec<CodegenError>> {
        match cranelift.generate(mir) {
            Ok(object_data) => {
                // Create a compiled program with the object data
                let mut program = CompiledProgram {
                    functions: HashMap::new(),
                    globals: HashMap::new(),
                    object_data: Some(object_data),
                };
                
                // Add placeholder functions for compatibility
                for function in &mir.functions {
                    let compiled_fn = CompiledFunction {
                        name: function.name.clone(),
                        instructions: vec![], // Real instructions are in object_data
                        locals: vec![],
                        signature: FunctionSignature {
                            params: vec![],
                            return_type: Type::Unit,
                            calling_convention: CallingConvention::SystemV,
                        },
                        region_info: None,
                    };
                    program.functions.insert(function.name.clone(), compiled_fn);
                }
                
                Ok(program)
            }
            Err(e) => {
                let mut errors = Vec::new();
                errors.push(CodegenError::GenerationFailed(e.to_string()));
                Err(errors)
            }
        }
    }

    pub fn generate(&mut self, mir: &MirProgram) -> Result<CompiledProgram, Vec<CodegenError>> {
        self.errors.clear();

        // Use real Cranelift if available
        if let Some(cranelift) = self.real_cranelift.as_mut() {
            return Self::generate_with_cranelift(mir, cranelift);
        }

        for function in &mir.functions {
            let compiled_fn = self.compile_function(function)?;
            self.context.functions.insert(function.name.clone(), compiled_fn);
        }

        if self.errors.is_empty() {
            Ok(CompiledProgram {
                functions: self.context.functions.clone(),
                globals: self.context.globals.clone(),
                object_data: None,
            })
        } else {
            Err(self.errors.clone())
        }
    }

    fn compile_function(&mut self, mir_fn: &MirFunction) -> Result<CompiledFunction, Vec<CodegenError>> {
        let mut instructions = Vec::new();
        let mut locals = Vec::new();

        // Create function signature
        let signature = self.create_function_signature(mir_fn)?;

        // Create local variables
        for i in 0..mir_fn.local_count {
            let local = LocalVariable {
                name: format!("local_{}", i),
                ty: Type::Int(IntWidth::I64), // TODO: Use actual types
                location: LocalLocation::Stack(i * 8),
            };
            locals.push(local);
        }

        // Compile MIR statements to instructions
        for stmt in &mir_fn.body.statements {
            let stmt_instructions = self.compile_statement(stmt)?;
            instructions.extend(stmt_instructions);
        }

        Ok(CompiledFunction {
            name: mir_fn.name.clone(),
            signature,
            instructions,
            locals,
            region_info: None, // TODO: Extract from region DAG
        })
    }

    fn create_function_signature(&mut self, mir_fn: &MirFunction) -> Result<FunctionSignature, Vec<CodegenError>> {
        let mut params = Vec::new();
        
        for (i, (name, ty)) in mir_fn.params.iter().enumerate() {
            let param = Parameter {
                name: name.clone(),
                ty: self.convert_type(ty),
                location: ParameterLocation::Register(format!("arg_{}", i)),
            };
            params.push(param);
        }

        let return_type = self.convert_type(&mir_fn.return_type);

        Ok(FunctionSignature {
            params,
            return_type,
            calling_convention: CallingConvention::Once,
        })
    }

    fn convert_type(&self, hir_type: &once_hir::HirType) -> Type {
        match hir_type {
            once_hir::HirType::Unit => Type::Unit,
            once_hir::HirType::Int => Type::Int(IntWidth::I64),
            once_hir::HirType::Float => Type::Float(FloatWidth::F64),
            once_hir::HirType::Bool => Type::Bool,
            once_hir::HirType::Str => Type::String,
            once_hir::HirType::Ident(_name) => {
                // TODO: Handle type aliases and user-defined types
                Type::Int(IntWidth::I64) // Placeholder
            }
            once_hir::HirType::Linear(inner) => {
                // Linear types are represented as pointers in the generated code
                Type::Pointer(Box::new(self.convert_type(inner)))
            }
            once_hir::HirType::Affine(inner) => {
                // Affine types are represented as pointers in the generated code
                Type::Pointer(Box::new(self.convert_type(inner)))
            }
            once_hir::HirType::Array(_ty, _n) => {
                // Arrays represented as pointers for now
                Type::Pointer(Box::new(Type::Int(IntWidth::I64)))
            }
            once_hir::HirType::Generic(_name, _args) => {
                // Generic types - placeholder
                Type::Int(IntWidth::I64)
            }
            once_hir::HirType::Tuple(_types) => {
                // Tuples - placeholder as pointer
                Type::Pointer(Box::new(Type::Unit))
            }
            once_hir::HirType::Function(_params, _ret) => {
                // Function types - pointer to function
                Type::Pointer(Box::new(Type::Unit))
            }
        }
    }

    fn compile_statement(&mut self, stmt: &MirStmt) -> Result<Vec<Instruction>, Vec<CodegenError>> {
        let mut instructions = Vec::new();

        match &stmt.op {
            MirOp::Move { from, to } => {
                let from_reg = self.get_register(from);
                let to_reg = self.get_register(to);
                instructions.push(Instruction::Move {
                    dest: to_reg,
                    src: from_reg,
                    ty: Type::Int(IntWidth::I64), // TODO: Use actual type
                });
            }
            MirOp::Drop { location } => {
                // Implement drop logic
                let reg = self.get_register(location);
                
                // Check if the value needs explicit dropping
                // For now, always free the memory
                instructions.push(Instruction::Free { ptr: reg });
                
                // In a real implementation, we would:
                // 1. Check if the type implements Drop
                // 2. Call the drop function if needed
                // 3. Free any associated memory
            }
            MirOp::FreeRegion { region } => {
                instructions.push(Instruction::FreeRegion {
                    region: region.clone(),
                });
            }
            MirOp::Allocate { region: _, size } => {
                let dest = self.get_temp_register();
                instructions.push(Instruction::Allocate {
                    dest,
                    size: *size,
                    alignment: 8,
                });
            }
            MirOp::BoundsCheck { index, bound, proven } => {
                let index_reg = self.get_register(index);
                let bound_reg = self.get_register(bound);
                instructions.push(Instruction::BoundsCheck {
                    index: index_reg,
                    bound: bound_reg,
                    proven: *proven,
                });
            }
            MirOp::ChannelSend { channel, value } => {
                let channel_reg = self.get_register(channel);
                let value_reg = self.get_register(value);
                instructions.push(Instruction::ChannelSend {
                    channel: channel_reg,
                    value: value_reg,
                });
            }
            MirOp::ChannelRecv { channel, result } => {
                let channel_reg = self.get_register(channel);
                let result_reg = self.get_register(result);
                instructions.push(Instruction::ChannelRecv {
                    channel: channel_reg,
                    dest: result_reg,
                });
            }
            MirOp::SpawnTask { function, args, result } => {
                let _func_reg = self.get_register(&MirLocation::Temp(0)); // TODO: Handle function references
                let result_reg = self.get_register(result);
                let arg_regs: Vec<String> = args.iter().map(|arg| self.get_register(arg)).collect();
                instructions.push(Instruction::SpawnTask {
                    func: function.clone(),
                    args: arg_regs,
                    dest: result_reg,
                });
            }
            MirOp::AwaitTask { task, result } => {
                let task_reg = self.get_register(task);
                let result_reg = self.get_register(result);
                instructions.push(Instruction::AwaitTask {
                    task: task_reg,
                    dest: result_reg,
                });
            }
            MirOp::Call { function, args, result } => {
                let _func_reg = self.get_register(&MirLocation::Temp(0)); // TODO: Handle function references
                let result_reg = self.get_register(result);
                let arg_regs: Vec<String> = args.iter().map(|arg| self.get_register(arg)).collect();
                instructions.push(Instruction::Call {
                    dest: Some(result_reg),
                    func: function.clone(),
                    args: arg_regs,
                    ty: Type::Int(IntWidth::I64), // TODO: Use actual return type
                });
            }
            MirOp::LoadLiteral { value, dest } => {
                // Handle literal loading
                let dest_reg = self.get_register(&dest);
                match value {
                    once_mir::MirValue::Int(i) => {
                        instructions.push(Instruction::LoadImm {
                            dest: dest_reg,
                            value: Immediate::Int(*i),
                            ty: Type::Int(IntWidth::I64),
                        });
                    }
                    once_mir::MirValue::Float(f) => {
                        instructions.push(Instruction::LoadImm {
                            dest: dest_reg,
                            value: Immediate::Float(*f),
                            ty: Type::Float(FloatWidth::F64),
                        });
                    }
                    once_mir::MirValue::Bool(b) => {
                        instructions.push(Instruction::LoadImm {
                            dest: dest_reg,
                            value: Immediate::Bool(*b),
                            ty: Type::Bool,
                        });
                    }
                    once_mir::MirValue::String(s) => {
                        instructions.push(Instruction::LoadImm {
                            dest: dest_reg,
                            value: Immediate::String(s.clone()),
                            ty: Type::String,
                        });
                    }
                    once_mir::MirValue::Unit => {
                        instructions.push(Instruction::LoadImm {
                            dest: dest_reg,
                            value: Immediate::Null,
                            ty: Type::Unit,
                        });
                    }
                }
            }
            MirOp::Return { value } => {
                let value_reg = value.as_ref().map(|v| self.get_register(v));
                instructions.push(Instruction::Return { value: value_reg });
            }
        }

        Ok(instructions)
    }

    fn get_register(&mut self, location: &MirLocation) -> String {
        match location {
            MirLocation::Local(id) => format!("local_{}", id),
            MirLocation::Param(id) => format!("arg_{}", id),
            MirLocation::Return => "return".to_string(),
            MirLocation::Temp(id) => format!("temp_{}", id),
            MirLocation::Field { base, field } => {
                let base_reg = self.get_register(base);
                format!("{}.{}", base_reg, field)
            }
            MirLocation::Index { base, index } => {
                let base_reg = self.get_register(base);
                let index_reg = self.get_register(index);
                format!("{}[{}]", base_reg, index_reg)
            }
        }
    }

    fn get_temp_register(&mut self) -> String {
        let reg = format!("temp_{}", self.context.next_temp);
        self.context.next_temp += 1;
        reg
    }

    /// Generate object file
    pub fn generate_object_file(&self, program: &CompiledProgram, output_path: &str) -> Result<(), Vec<CodegenError>> {
        // If real Cranelift generated raw object bytes, write them directly.
        if let Some(bytes) = &program.object_data {
            std::fs::write(output_path, bytes)
                .map_err(|e| vec![CodegenError::ObjectFileFailed(format!("Failed to write object file: {}", e))])?;
            return Ok(());
        }

        // Fallback: create a simple custom object file format (placeholder)
        let mut object_data = Vec::new();
        
        // Write object file header
        object_data.extend_from_slice(b"ONCE_OBJ\0");
        
        // Write function count
        let func_count = program.functions.len() as u32;
        object_data.extend_from_slice(&func_count.to_le_bytes());
        
        // Write functions
        for (name, func) in &program.functions {
            // Function name length and name
            let name_bytes = name.as_bytes();
            object_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            object_data.extend_from_slice(name_bytes);
            
            // Function signature (simplified)
            object_data.push(0); // Return type placeholder
            object_data.push(func.signature.params.len() as u8); // Parameter count
        }
        
        // Write global count
        let global_count = program.globals.len() as u32;
        object_data.extend_from_slice(&global_count.to_le_bytes());
        
        // Write globals
        for (name, global) in &program.globals {
            // Global name length and name
            let name_bytes = name.as_bytes();
            object_data.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            object_data.extend_from_slice(name_bytes);
            
            // Global type and constant flag
            object_data.push(0); // Type placeholder
            object_data.push(if global.is_constant { 1 } else { 0 });
        }
        
        // Write to file
        std::fs::write(output_path, object_data)
            .map_err(|e| vec![CodegenError::ObjectFileFailed(format!("Failed to write object file: {}", e))])?;
        
        Ok(())
    }
    

    /// Generate assembly code
    pub fn generate_assembly(&self, program: &CompiledProgram) -> String {
        let mut asm = String::new();
        asm.push_str("; Once language assembly output\n");
        asm.push_str("; Generated by once-codegen\n\n");

        for (name, function) in &program.functions {
            asm.push_str(&format!("; Function: {}\n", name));
            asm.push_str(&format!("{}:\n", name));
            
            for instruction in &function.instructions {
                asm.push_str(&format!("  ; {:?}\n", instruction));
            }
            
            asm.push_str("\n");
        }

        asm
    }
}

/// Compiled program
#[derive(Debug, Clone)]
pub struct CompiledProgram {
    pub functions: HashMap<String, CompiledFunction>,
    pub globals: HashMap<String, GlobalVariable>,
    pub object_data: Option<Vec<u8>>,
}

impl fmt::Display for CompiledProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Compiled Program:")?;
        writeln!(f, "=================")?;
        
        for (name, function) in &self.functions {
            writeln!(f, "\nFunction {}:", name)?;
            writeln!(f, "  Parameters: {}", function.signature.params.len())?;
            writeln!(f, "  Locals: {}", function.locals.len())?;
            writeln!(f, "  Instructions: {}", function.instructions.len())?;
            
            for (i, instruction) in function.instructions.iter().enumerate() {
                writeln!(f, "    {}: {:?}", i, instruction)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_mir::{MirProgram, MirFunction, MirBlock, MirStmt, MirOp, MirLocation};

    #[test]
    fn test_codegen_creation() {
        let region_dag = once_rinf::RegionDag {
            nodes: std::collections::HashMap::new(),
            edges: Vec::new(),
            free_points: Vec::new(),
        };
        
        let generator = CodeGenerator::new(region_dag);
        assert!(generator.context.functions.is_empty());
    }

    #[test]
    fn test_type_conversion() {
        let region_dag = once_rinf::RegionDag {
            nodes: std::collections::HashMap::new(),
            edges: Vec::new(),
            free_points: Vec::new(),
        };
        
        let generator = CodeGenerator::new(region_dag);
        let hir_type = once_hir::HirType::Int;
        let converted = generator.convert_type(&hir_type);
        assert_eq!(converted, Type::Int(IntWidth::I64));
    }

    #[test]
    fn test_instruction_generation() {
        let region_dag = once_rinf::RegionDag {
            nodes: std::collections::HashMap::new(),
            edges: Vec::new(),
            free_points: Vec::new(),
        };
        
        let mut generator = CodeGenerator::new(region_dag);
        let move_op = MirOp::Move {
            from: MirLocation::Temp(0),
            to: MirLocation::Local(1),
        };
        let stmt = MirStmt {
            op: move_op,
            span: once_lex::Span::new(0, 0, 0, 0),
            region: None,
        };
        
        let instructions = generator.compile_statement(&stmt).unwrap();
        assert!(!instructions.is_empty());
    }
}