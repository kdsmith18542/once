//! MIR (Mid-level IR) for the Once language
//! 
//! Implements lowered IR with:
//! - Explicit moves and drops
//! - Region frees
//! - Bounds check annotations
//! - Actor/channel operations
//! - Linear value tracking

use once_hir::*;
use once_ty::{Type, TypeVar};
use once_rinf::{Region, RegionDag};
use once_lex::Span;
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// MIR generation errors
#[derive(Error, Debug, Clone)]
pub enum MirError {
    #[error("MIR generation failed: {0}")]
    GenerationFailed(String),
    
    #[error("Move analysis failed: {0}")]
    MoveAnalysisFailed(String),
    
    #[error("Drop placement failed: {0}")]
    DropPlacementFailed(String),
    
    #[error("Region integration failed: {0}")]
    RegionIntegrationFailed(String),
}

/// MIR operation types
#[derive(Debug, Clone, PartialEq)]
pub enum MirOp {
    /// Move a value from one location to another
    Move { from: MirLocation, to: MirLocation },
    /// Drop a value (consume it)
    Drop { location: MirLocation },
    /// Free a region
    FreeRegion { region: Region },
    /// Allocate in a region
    Allocate { region: Region, size: usize },
    /// Bounds check with proof status
    BoundsCheck { 
        index: MirLocation, 
        bound: MirLocation, 
        proven: bool 
    },
    /// Channel send operation
    ChannelSend { 
        channel: MirLocation, 
        value: MirLocation 
    },
    /// Channel receive operation
    ChannelRecv { 
        channel: MirLocation, 
        result: MirLocation 
    },
    /// Spawn task operation
    SpawnTask { 
        function: String, 
        args: Vec<MirLocation>,
        result: MirLocation 
    },
    /// Await task operation
    AwaitTask { 
        task: MirLocation, 
        result: MirLocation 
    },
    /// Function call
    Call { 
        function: String, 
        args: Vec<MirLocation>,
        result: MirLocation 
    },
    /// Return from function
    Return { value: Option<MirLocation> },
    /// Load a literal value
    LoadLiteral { value: MirValue, dest: MirLocation },
}

/// MIR location (where values are stored)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MirLocation {
    /// Local variable
    Local(usize),
    /// Function parameter
    Param(usize),
    /// Return value slot
    Return,
    /// Temporary value
    Temp(usize),
    /// Field access
    Field { base: Box<MirLocation>, field: String },
    /// Array element
    Index { base: Box<MirLocation>, index: Box<MirLocation> },
}

/// MIR value types
#[derive(Debug, Clone, PartialEq)]
pub enum MirValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Unit,
}

impl fmt::Display for MirLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MirLocation::Local(id) => write!(f, "local_{}", id),
            MirLocation::Param(id) => write!(f, "param_{}", id),
            MirLocation::Return => write!(f, "return"),
            MirLocation::Temp(id) => write!(f, "temp_{}", id),
            MirLocation::Field { base, field } => write!(f, "{}.{}", base, field),
            MirLocation::Index { base, index } => write!(f, "{}[{}]", base, index),
        }
    }
}

/// MIR statement
#[derive(Debug, Clone)]
pub struct MirStmt {
    pub op: MirOp,
    pub span: Span,
    pub region: Option<Region>,
}

/// MIR block
#[derive(Debug, Clone)]
pub struct MirBlock {
    pub statements: Vec<MirStmt>,
    pub region: Option<Region>,
}

/// MIR function
#[derive(Debug, Clone)]
pub struct MirFunction {
    pub name: String,
    pub params: Vec<(String, HirType)>,
    pub return_type: HirType,
    pub body: MirBlock,
    pub local_count: usize,
    pub temp_count: usize,
}

/// MIR program
#[derive(Debug, Clone)]
pub struct MirProgram {
    pub functions: Vec<MirFunction>,
    pub global_region: Region,
}

/// MIR generator
pub struct MirGenerator {
    next_local: usize,
    next_temp: usize,
    region_dag: Option<RegionDag>,
    errors: Vec<MirError>,
}

impl MirGenerator {
    pub fn new() -> Self {
        Self {
            next_local: 0,
            next_temp: 0,
            region_dag: None,
            errors: Vec::new(),
        }
    }

    pub fn generate(&mut self, hir: &HirProgram, region_dag: RegionDag) -> Result<MirProgram, Vec<MirError>> {
        self.region_dag = Some(region_dag);
        self.next_local = 0;
        self.next_temp = 0;
        self.errors.clear();

        let mut functions = Vec::new();
        let global_region = Region {
            id: 0,
            name: "global".to_string(),
            is_primary: true,
        };

        for item in &hir.items {
            match item {
                HirItem::FnDecl(fn_decl) => {
                    let mir_fn = self.generate_function(fn_decl)?;
                    functions.push(mir_fn);
                }
                HirItem::LetDecl(_) => {
                    // Global let declarations are handled differently
                }
            }
        }

        if self.errors.is_empty() {
            Ok(MirProgram {
                functions,
                global_region,
            })
        } else {
            Err(self.errors.clone())
        }
    }

    fn generate_function(&mut self, fn_decl: &HirFnDecl) -> Result<MirFunction, Vec<MirError>> {
        let mut statements = Vec::new();
        let mut local_count = 0;
        let mut temp_count = 0;

        // Generate parameter locations
        let mut params = Vec::new();
        for (i, param) in fn_decl.params.iter().enumerate() {
            params.push((param.name.clone(), param.type_annotation.clone().unwrap_or(HirType::Unit)));
        }

        // Generate function body
        let body = self.generate_block(&fn_decl.body, &mut statements, &mut local_count, &mut temp_count)?;

        Ok(MirFunction {
            name: fn_decl.name.clone(),
            params,
            return_type: fn_decl.return_type.clone().unwrap_or(HirType::Unit),
            body,
            local_count,
            temp_count,
        })
    }

    fn generate_block(
        &mut self,
        block: &HirBlock,
        statements: &mut Vec<MirStmt>,
        local_count: &mut usize,
        temp_count: &mut usize,
    ) -> Result<MirBlock, Vec<MirError>> {
        let mut block_statements = Vec::new();

        for stmt in &block.statements {
            let stmt_ops = self.generate_stmt(stmt, local_count, temp_count)?;
            block_statements.extend(stmt_ops);
        }

        statements.extend(block_statements.clone());

        Ok(MirBlock {
            statements: block_statements,
            region: None, // TODO: Determine region from region_dag
        })
    }

    fn generate_stmt(
        &mut self,
        stmt: &HirStmt,
        local_count: &mut usize,
        temp_count: &mut usize,
    ) -> Result<Vec<MirStmt>, Vec<MirError>> {
        let mut statements = Vec::new();

        match stmt {
            HirStmt::Let(let_stmt) => {
                // Allocate local for the variable
                let local_id = *local_count;
                *local_count += 1;
                let local = MirLocation::Local(local_id);

                // Generate operations for the value expression
                let value_ops = self.generate_expr(&let_stmt.value, temp_count)?;
                statements.extend(value_ops);

                // Move the result to the local
                let result_temp = MirLocation::Temp(*temp_count - 1);
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: result_temp,
                        to: local,
                    },
                    span: Span::new(0, 0, 0, 0), // TODO: Use actual span
                    region: None,
                });
            }
            HirStmt::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    // Generate operations for the return expression
                    let expr_ops = self.generate_expr(expr, temp_count)?;
                    statements.extend(expr_ops);

                    // Move result to return location
                    let result_temp = MirLocation::Temp(*temp_count - 1);
                    statements.push(MirStmt {
                        op: MirOp::Return {
                            value: Some(result_temp),
                        },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                } else {
                    statements.push(MirStmt {
                        op: MirOp::Return { value: None },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                }
            }
            HirStmt::Expr(expr) => {
                let expr_ops = self.generate_expr(expr, temp_count)?;
                statements.extend(expr_ops);
            }
        }

        Ok(statements)
    }

    fn generate_expr(
        &mut self,
        expr: &HirExpr,
        temp_count: &mut usize,
    ) -> Result<Vec<MirStmt>, Vec<MirError>> {
        let mut statements = Vec::new();

        match expr {
            HirExpr::Literal(lit) => {
                // Create temporary for literal
                let temp_id = *temp_count;
                *temp_count += 1;
                let temp = MirLocation::Temp(temp_id);

                // Generate literal loading operation
                match lit {
                    once_hir::HirLiteral::Int(value) => {
                        statements.push(MirStmt {
                            op: MirOp::LoadLiteral {
                                value: MirValue::Int(*value),
                                dest: temp,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                    once_hir::HirLiteral::Float(value) => {
                        statements.push(MirStmt {
                            op: MirOp::LoadLiteral {
                                value: MirValue::Float(*value),
                                dest: temp,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                    once_hir::HirLiteral::Bool(value) => {
                        statements.push(MirStmt {
                            op: MirOp::LoadLiteral {
                                value: MirValue::Bool(*value),
                                dest: temp,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                    once_hir::HirLiteral::String(value) => {
                        statements.push(MirStmt {
                            op: MirOp::LoadLiteral {
                                value: MirValue::String(value.clone()),
                                dest: temp,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                    once_hir::HirLiteral::Unit => {
                        statements.push(MirStmt {
                            op: MirOp::LoadLiteral {
                                value: MirValue::Unit,
                                dest: temp,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                }
            }
            HirExpr::Ident(name) => {
                // Look up variable location (simplified - in real implementation would use symbol table)
                let temp_id = *temp_count;
                *temp_count += 1;
                let temp = MirLocation::Temp(temp_id);

                // Generate variable loading operation
                // For now, assume variables are stored in local slots
                let local_slot = self.get_variable_slot(name);
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: MirLocation::Local(local_slot),
                        to: temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Call { function, args } => {
                // Generate operations for arguments
                let mut arg_locations = Vec::new();
                for arg in args {
                    let arg_ops = self.generate_expr(arg, temp_count)?;
                    statements.extend(arg_ops);
                    arg_locations.push(MirLocation::Temp(*temp_count - 1));
                }

                // Create temporary for result
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Generate call operation
                let call_op = match function.as_str() {
                    "spawn" => MirOp::SpawnTask {
                        function: "spawn".to_string(),
                        args: arg_locations,
                        result: result_temp,
                    },
                    "await" => MirOp::AwaitTask {
                        task: arg_locations[0].clone(),
                        result: result_temp,
                    },
                    _ => MirOp::Call {
                        function: function.clone(),
                        args: arg_locations,
                        result: result_temp,
                    },
                };

                statements.push(MirStmt {
                    op: call_op,
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Binary { left, op: _, right } => {
                // Generate operations for left and right operands
                let left_ops = self.generate_expr(left, temp_count)?;
                statements.extend(left_ops);
                let left_temp = MirLocation::Temp(*temp_count - 1);

                let right_ops = self.generate_expr(right, temp_count)?;
                statements.extend(right_ops);
                let _right_temp = MirLocation::Temp(*temp_count - 1);

                // Create temporary for result
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // TODO: Generate binary operation
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: left_temp,
                        to: result_temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Block(block) => {
                let block_ops = self.generate_block(block, &mut statements, &mut 0, temp_count)?;
                statements.extend(block_ops.statements);
            }
        }

        Ok(statements)
    }

    /// Add region frees based on region DAG
    pub fn add_region_frees(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        if let Some(ref dag) = self.region_dag {
            for (region, _free_point) in &dag.free_points {
                // Find the appropriate function and insert free operation
                for function in &mut mir.functions {
                    if function.name.contains(&region.name) {
                        // Insert free operation at the calculated point
                        let free_stmt = MirStmt {
                            op: MirOp::FreeRegion {
                                region: region.clone(),
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: Some(region.clone()),
                        };

                        // TODO: Insert at the correct position based on free_point
                        function.body.statements.push(free_stmt);
                    }
                }
            }
        }
        Ok(())
    }

    /// Add drop operations for linear values
    pub fn add_drop_operations(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        for function in &mut mir.functions {
            // TODO: Analyze function to determine where drops are needed
            // This is a simplified implementation
            for stmt in &mut function.body.statements {
                match &stmt.op {
                    MirOp::Move { from: _, to: _ } => {
                        // If moving a linear value, we might need to drop the source
                        // TODO: Implement proper linear value tracking
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    /// Add bounds checks with proof annotations
    pub fn add_bounds_checks(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        for function in &mut mir.functions {
            let mut new_statements = Vec::new();
            
            for stmt in &function.body.statements {
                new_statements.push(stmt.clone());
                
                // TODO: Add bounds checks for array accesses
                // This would analyze the MIR to find array accesses and insert bounds checks
            }
            
            function.body.statements = new_statements;
        }
        Ok(())
    }

    /// Get variable slot for a variable name (simplified implementation)
    fn get_variable_slot(&self, name: &str) -> usize {
        // In a real implementation, this would use a symbol table
        // For now, use a simple hash-based approach
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        (hasher.finish() % 100) as usize
    }
}

/// MIR pretty printer
impl fmt::Display for MirProgram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "MIR Program:")?;
        writeln!(f, "============")?;
        
        for function in &self.functions {
            writeln!(f, "\nFunction {}:", function.name)?;
            writeln!(f, "  Parameters: {}", function.params.len())?;
            writeln!(f, "  Locals: {}", function.local_count)?;
            writeln!(f, "  Statements:")?;
            
            for (i, stmt) in function.body.statements.iter().enumerate() {
                writeln!(f, "    {}: {:?}", i, stmt.op)?;
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral};

    #[test]
    fn test_mir_generation() {
        let mut generator = MirGenerator::new();
        let hir = HirProgram { items: Vec::new(), imports: Vec::new() };
        let region_dag = once_rinf::RegionDag {
            nodes: HashMap::new(),
            edges: Vec::new(),
            free_points: Vec::new(),
        };
        
        let result = generator.generate(&hir, region_dag);
        assert!(result.is_ok());
    }

    #[test]
    fn test_location_display() {
        let local = MirLocation::Local(0);
        assert_eq!(format!("{}", local), "local_0");
        
        let temp = MirLocation::Temp(1);
        assert_eq!(format!("{}", temp), "temp_1");
    }

    #[test]
    fn test_operation_types() {
        let move_op = MirOp::Move {
            from: MirLocation::Temp(0),
            to: MirLocation::Local(1),
        };
        
        assert!(matches!(move_op, MirOp::Move { .. }));
    }
}