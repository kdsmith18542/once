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
use std::collections::HashMap;
use std::fmt;
use thiserror::Error;

pub mod verify;
pub use verify::MirVerifier;

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
    /// Allocate in a region, storing the pointer in dest
    Allocate { region: Region, size: usize, dest: MirLocation },
    /// Bounds check with proof status
    BoundsCheck { 
        index: MirLocation, 
        bound: MirLocation, 
        proven: bool 
    },
    /// Binary arithmetic / comparison operation
    BinOp {
        op: MirBinOp,
        left: MirLocation,
        right: MirLocation,
        dest: MirLocation,
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
    /// Create a new task group for structured concurrency
    CreateGroup {
        result: MirLocation
    },
    /// Spawn a task within a group
    SpawnInGroup {
        group: MirLocation,
        function: String,
        args: Vec<MirLocation>,
        result: MirLocation
    },
    /// Await all tasks in a group (blocks until all children complete)
    AwaitGroup {
        group: MirLocation,
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
    /// Unconditional jump to a label
    Jump { target: usize },
    /// Conditional branch: if condition is true, jump to true_target, else false_target
    Branch {
        condition: MirLocation,
        true_target: usize,
        false_target: usize,
    },
    /// Label marker for jump targets
    Label { id: usize },
    /// Try block: instruments error context capture
    TryBlock { result: MirLocation },
    /// Load the length of a collection (array/vec) into dest
    LoadLength { base: MirLocation, dest: MirLocation },
}

/// MIR binary operation types
#[derive(Debug, Clone, PartialEq)]
pub enum MirBinOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    /// Assignment (=) - maps to a move operation in MIR
    Move,
}

impl MirBinOp {
    pub fn from_hir(op: &HirBinaryOp) -> Self {
        use HirBinaryOp::*;
        match op {
            Add => MirBinOp::Add,
            Sub => MirBinOp::Sub,
            Mul => MirBinOp::Mul,
            Div => MirBinOp::Div,
            Eq => MirBinOp::Eq,
            Ne => MirBinOp::Ne,
            Lt => MirBinOp::Lt,
            Le => MirBinOp::Le,
            Gt => MirBinOp::Gt,
            Ge => MirBinOp::Ge,
            And => MirBinOp::And,
            Or => MirBinOp::Or,
            Assign => MirBinOp::Move,
        }
    }
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
    next_label: usize,
    region_dag: Option<RegionDag>,
    errors: Vec<MirError>,
    /// Maps variant names to their discriminant index across all enum types
    variant_discriminants: HashMap<String, usize>,
}

impl MirGenerator {
    pub fn new() -> Self {
        Self {
            next_local: 0,
            next_temp: 0,
            next_label: 0,
            region_dag: None,
            errors: Vec::new(),
            variant_discriminants: HashMap::new(),
        }
    }

    fn fresh_label(&mut self) -> usize {
        let label = self.next_label;
        self.next_label += 1;
        label
    }

    pub fn generate(&mut self, hir: &HirProgram, region_dag: RegionDag) -> Result<MirProgram, Vec<MirError>> {
        self.region_dag = Some(region_dag);
        self.next_local = 0;
        self.next_temp = 0;
        self.errors.clear();
        self.variant_discriminants.clear();

        // Build variant discriminant index from type declarations
        for item in &hir.items {
            if let HirItem::TypeDecl(type_decl) = item {
                for (discriminant, variant) in type_decl.variants.iter().enumerate() {
                    self.variant_discriminants.insert(variant.name.clone(), discriminant);
                }
            }
        }

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
                HirItem::TypeDecl(_) => {
                    // Type declarations don't generate MIR directly
                }
                HirItem::TraitDecl(trait_decl) => {
                    for method in &trait_decl.methods {
                        functions.push(self.generate_function(method)?);
                    }
                }
                HirItem::ImplBlock(impl_block) => {
                    for method in &impl_block.methods {
                        functions.push(self.generate_function(method)?);
                    }
                }
                HirItem::StructDecl(_) => {}
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
        for param in fn_decl.params.iter() {
            params.push((param.name.clone(), param.type_annotation.clone().unwrap_or(HirType::Unit)));
        }

        // Generate function body
        let body = self.generate_block(&fn_decl.body, &mut statements, &mut local_count, &mut temp_count)?;

        // Add implicit return if the last statement isn't a return
        let mut statements = body.statements.clone();
        if statements.is_empty() || !matches!(statements.last().unwrap().op, MirOp::Return { .. }) {
            let return_type = fn_decl.return_type.clone().unwrap_or(HirType::Unit);
            let return_value = if return_type != HirType::Unit && !statements.is_empty() {
                // The last expression's result is in the most recently allocated temp
                Some(MirLocation::Temp(temp_count.saturating_sub(1)))
            } else {
                None
            };
            statements.push(MirStmt {
                op: MirOp::Return { value: return_value },
                span: Span::new(0, 0, 0, 0),
                region: None,
            });
        }

        Ok(MirFunction {
            name: fn_decl.name.clone(),
            params,
            return_type: fn_decl.return_type.clone().unwrap_or(HirType::Unit),
            body: MirBlock {
                statements,
                region: None,
            },
            local_count,
            temp_count,
        })
    }

    fn literal_to_mir_value(&self, lit: &HirLiteral) -> MirValue {
        match lit {
            HirLiteral::Int(i) => MirValue::Int(*i),
            HirLiteral::Float(f) => MirValue::Float(*f),
            HirLiteral::String(s) => MirValue::String(s.clone()),
            HirLiteral::Bool(b) => MirValue::Bool(*b),
            HirLiteral::Unit => MirValue::Unit,
        }
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
            region: None, // Region is assigned by the caller via region_dag free points
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
                    span: Span::new(0, 0, 0, 0), // Span tracking requires HIR→MIR source mapping
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
            HirStmt::Continue(_) | HirStmt::Break(_) => {
                // Control flow — no MIR statements generated here
            }
            HirStmt::Using(using_stmt) => {
                // Generate init expression
                let init_ops = self.generate_expr(&using_stmt.init, temp_count)?;
                statements.extend(init_ops);
                
                // Allocate local for the variable
                let local_id = *local_count;
                *local_count += 1;
                let local = MirLocation::Local(local_id);
                
                // Move init result to local
                let result_temp = MirLocation::Temp(*temp_count - 1);
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: result_temp,
                        to: local.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                
                // Generate body statements
                for stmt in &using_stmt.body.statements {
                    let body_ops = self.generate_stmt(stmt, local_count, temp_count)?;
                    statements.extend(body_ops);
                }
                
                // Generate consume/drop at end of using block
                statements.push(MirStmt {
                    op: MirOp::Drop {
                        location: local,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
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
            HirExpr::Literal(lit, _) => {
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
            HirExpr::Ident(name, _) => {
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
            HirExpr::Call { function, args, .. } => {
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
            HirExpr::Binary { left, op, right, .. } => {
                // Generate operations for left and right operands
                let left_ops = self.generate_expr(left, temp_count)?;
                statements.extend(left_ops);
                let left_temp = MirLocation::Temp(*temp_count - 1);

                let right_ops = self.generate_expr(right, temp_count)?;
                statements.extend(right_ops);
                let right_temp = MirLocation::Temp(*temp_count - 1);

                // Create temporary for result
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Generate binary operation
                statements.push(MirStmt {
                    op: MirOp::BinOp {
                        op: MirBinOp::from_hir(op),
                        left: left_temp,
                        right: right_temp,
                        dest: result_temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Block(block, _) => {
                let _block_ops = self.generate_block(block, &mut statements, &mut 0, temp_count)?;
                // statements already extended by generate_block
            }
            HirExpr::If { condition, then_branch, else_branch, .. } => {
                // Generate condition
                let cond_ops = self.generate_expr(condition, temp_count)?;
                statements.extend(cond_ops);
                let cond_temp = MirLocation::Temp(*temp_count - 1);

                // Labels
                let then_label = self.fresh_label();
                let else_label = self.fresh_label();
                let end_label = self.fresh_label();

                // Result temp
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Branch
                statements.push(MirStmt {
                    op: MirOp::Branch {
                        condition: cond_temp,
                        true_target: then_label,
                        false_target: else_label,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Then block
                statements.push(MirStmt {
                    op: MirOp::Label { id: then_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                let _then_ops = self.generate_block(then_branch, &mut statements, &mut 0, temp_count)?;
                // statements already extended by generate_block
                // Move last value to result
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: MirLocation::Temp(*temp_count - 1),
                        to: result_temp.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                statements.push(MirStmt {
                    op: MirOp::Jump { target: end_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Else block
                statements.push(MirStmt {
                    op: MirOp::Label { id: else_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                if let Some(else_expr) = else_branch {
                    let else_ops = self.generate_expr(else_expr, temp_count)?;
                    statements.extend(else_ops);
                    statements.push(MirStmt {
                        op: MirOp::Move {
                            from: MirLocation::Temp(*temp_count - 1),
                            to: result_temp.clone(),
                        },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                } else {
                    // No else branch: load Unit
                    statements.push(MirStmt {
                        op: MirOp::LoadLiteral {
                            value: MirValue::Unit,
                            dest: result_temp.clone(),
                        },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                }
                statements.push(MirStmt {
                    op: MirOp::Jump { target: end_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // End
                statements.push(MirStmt {
                    op: MirOp::Label { id: end_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Match { expr, arms, .. } => {
                let scrutinee_ops = self.generate_expr(expr, temp_count)?;
                statements.extend(scrutinee_ops);
                let scrutinee_temp = MirLocation::Temp(*temp_count - 1);

                let end_label = self.fresh_label();
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                for arm in arms {
                    let arm_label = self.fresh_label();
                    let next_label = self.fresh_label();

                    match &arm.pattern {
                        HirPattern::Literal(lit) => {
                            // Load literal and compare with scrutinee
                            let lit_temp = MirLocation::Temp(*temp_count);
                            *temp_count += 1;
                            statements.push(MirStmt {
                                op: MirOp::LoadLiteral {
                                    value: self.literal_to_mir_value(lit),
                                    dest: lit_temp.clone(),
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            let cmp_temp = MirLocation::Temp(*temp_count);
                            *temp_count += 1;
                            statements.push(MirStmt {
                                op: MirOp::BinOp {
                                    op: MirBinOp::Eq,
                                    left: scrutinee_temp.clone(),
                                    right: lit_temp,
                                    dest: cmp_temp.clone(),
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            statements.push(MirStmt {
                                op: MirOp::Branch {
                                    condition: cmp_temp,
                                    true_target: arm_label,
                                    false_target: next_label,
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                        }
                        HirPattern::Ident(_name) => {
                            // Bind scrutinee value to a local slot, then always match
                            let bind_temp = MirLocation::Temp(*temp_count);
                            *temp_count += 1;
                            statements.push(MirStmt {
                                op: MirOp::Move {
                                    from: scrutinee_temp.clone(),
                                    to: bind_temp,
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            statements.push(MirStmt {
                                op: MirOp::Jump { target: arm_label },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                        }
                        HirPattern::Wildcard => {
                            // Wildcard always matches
                            statements.push(MirStmt {
                                op: MirOp::Jump { target: arm_label },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                        }
                        HirPattern::EnumVariant { name, fields } => {
                            let discriminant = self.variant_discriminants.get(name).copied().unwrap_or(0);
                            let variant_tag_temp = MirLocation::Temp(*temp_count);
                            *temp_count += 1;
                            statements.push(MirStmt {
                                op: MirOp::LoadLiteral {
                                    value: MirValue::Int(discriminant as i64),
                                    dest: variant_tag_temp.clone(),
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            let cmp_temp = MirLocation::Temp(*temp_count);
                            *temp_count += 1;
                            statements.push(MirStmt {
                                op: MirOp::BinOp {
                                    op: MirBinOp::Eq,
                                    left: scrutinee_temp.clone(),
                                    right: variant_tag_temp,
                                    dest: cmp_temp.clone(),
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            statements.push(MirStmt {
                                op: MirOp::Branch {
                                    condition: cmp_temp,
                                    true_target: arm_label,
                                    false_target: next_label,
                                },
                                span: Span::new(0, 0, 0, 0),
                                region: None,
                            });
                            let _ = fields;
                            let _ = name;
                        }
                    }

                    // Arm pattern match body
                    statements.push(MirStmt {
                        op: MirOp::Label { id: arm_label },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });

                    // Check guard if present
                    let guard_label = if arm.guard.is_some() {
                        let guard_ok = self.fresh_label();
                        let guard_ops = self.generate_expr(arm.guard.as_ref().unwrap(), temp_count)?;
                        statements.extend(guard_ops);
                        let guard_temp = MirLocation::Temp(*temp_count - 1);
                        statements.push(MirStmt {
                            op: MirOp::Branch {
                                condition: guard_temp,
                                true_target: guard_ok,
                                false_target: next_label,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                        Some(guard_ok)
                    } else {
                        None
                    };

                    // Arm commit (after guard passes)
                    if let Some(gk) = guard_label {
                        statements.push(MirStmt {
                            op: MirOp::Label { id: gk },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                    let arm_ops = self.generate_expr(&arm.body, temp_count)?;
                    statements.extend(arm_ops);
                    statements.push(MirStmt {
                        op: MirOp::Move {
                            from: MirLocation::Temp(*temp_count - 1),
                            to: result_temp.clone(),
                        },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                    statements.push(MirStmt {
                        op: MirOp::Jump { target: end_label },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });

                    // Next arm label
                    statements.push(MirStmt {
                        op: MirOp::Label { id: next_label },
                        span: Span::new(0, 0, 0, 0),
                        region: None,
                    });
                }

                // End label
                statements.push(MirStmt {
                    op: MirOp::Label { id: end_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::For { item, collection, body, .. } => {
                // Generate collection
                let coll_ops = self.generate_expr(collection, temp_count)?;
                statements.extend(coll_ops);

                let loop_start = self.fresh_label();
                let loop_body = self.fresh_label();
                let loop_end = self.fresh_label();
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Counter-based iteration: for collections, iterate idx 0..length
                let idx_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                let len_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                let one_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                let coll_temp = MirLocation::Temp(*temp_count - 5);

                // idx = 0
                statements.push(MirStmt {
                    op: MirOp::LoadLiteral { value: MirValue::Int(0), dest: idx_temp.clone() },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // len = LoadLength(collection)
                statements.push(MirStmt {
                    op: MirOp::LoadLength { base: coll_temp.clone(), dest: len_temp.clone() },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // one = 1
                statements.push(MirStmt {
                    op: MirOp::LoadLiteral { value: MirValue::Int(1), dest: one_temp.clone() },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // loop_start:
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_start },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // cmp = idx < len
                let cmp_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                statements.push(MirStmt {
                    op: MirOp::BinOp {
                        op: MirBinOp::Lt,
                        left: idx_temp.clone(),
                        right: len_temp.clone(),
                        dest: cmp_temp.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // if !cmp goto loop_end
                statements.push(MirStmt {
                    op: MirOp::Branch {
                        condition: cmp_temp,
                        true_target: loop_body,
                        false_target: loop_end,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // loop_body:
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_body },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Load element: item = collection[idx_temp]
                let item_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: MirLocation::Index {
                            base: Box::new(coll_temp),
                            index: Box::new(idx_temp.clone()),
                        },
                        to: item_temp.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                let _ = item;

                // Execute body
                let _body_ops = self.generate_block(body, &mut statements, &mut 0, temp_count)?;

                // idx = idx + 1
                let new_idx = MirLocation::Temp(*temp_count);
                *temp_count += 1;
                statements.push(MirStmt {
                    op: MirOp::BinOp {
                        op: MirBinOp::Add,
                        left: idx_temp.clone(),
                        right: one_temp,
                        dest: new_idx.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                statements.push(MirStmt {
                    op: MirOp::Move { from: new_idx, to: idx_temp },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // goto loop_start
                statements.push(MirStmt {
                    op: MirOp::Jump { target: loop_start },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // loop_end:
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_end },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // Load Unit as loop result
                statements.push(MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Unit,
                        dest: result_temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::While { condition, body, .. } => {
                // Generate condition evaluation
                let cond_ops = self.generate_expr(condition, temp_count)?;
                statements.extend(cond_ops);
                let _cond_temp = MirLocation::Temp(*temp_count - 1);

                let loop_start = self.fresh_label();
                let loop_body = self.fresh_label();
                let loop_end = self.fresh_label();

                // Result temp (Unit by default)
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Loop start: evaluate condition
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_start },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // Re-evaluate condition each iteration
                let cond_ops2 = self.generate_expr(condition, temp_count)?;
                statements.extend(cond_ops2);
                let cond_temp2 = MirLocation::Temp(*temp_count - 1);
                statements.push(MirStmt {
                    op: MirOp::Branch {
                        condition: cond_temp2,
                        true_target: loop_body,
                        false_target: loop_end,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Loop body
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_body },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                let _body_ops = self.generate_block(body, &mut statements, &mut 0, temp_count)?;
                // Jump back to condition check
                statements.push(MirStmt {
                    op: MirOp::Jump { target: loop_start },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Loop end
                statements.push(MirStmt {
                    op: MirOp::Label { id: loop_end },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                statements.push(MirStmt {
                    op: MirOp::LoadLiteral {
                        value: MirValue::Unit,
                        dest: result_temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Index { base, index, .. } => {
                // Generate operations for base and index
                let base_ops = self.generate_expr(base, temp_count)?;
                statements.extend(base_ops);
                let base_temp = MirLocation::Temp(*temp_count - 1);

                let index_ops = self.generate_expr(index, temp_count)?;
                statements.extend(index_ops);
                let index_temp = MirLocation::Temp(*temp_count - 1);

                // Emit bounds check
                statements.push(MirStmt {
                    op: MirOp::BoundsCheck {
                        index: index_temp.clone(),
                        bound: base_temp.clone(),
                        proven: false,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Create temporary for result
                let result_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Index access: move the indexed element to result
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: MirLocation::Index { base: Box::new(base_temp), index: Box::new(index_temp) },
                        to: result_temp,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Try(inner, _) => {
                // `try expr` lowers to: evaluate expr, if it's Err, return Err immediately;
                // if it's Ok, unwrap the value and continue.
                let inner_ops = self.generate_expr(inner, temp_count)?;
                statements.extend(inner_ops);
                let result_temp = MirLocation::Temp(*temp_count - 1);

                // Create a temp for the unwrapped success value
                let success_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Create a temp for the error value
                let error_temp = MirLocation::Temp(*temp_count);
                *temp_count += 1;

                // Labels for control flow
                let ok_label = self.fresh_label();
                let err_label = self.fresh_label();
                let end_label = self.fresh_label();

                // `try` context: check if result is a Result
                // Emit a TryBlock to instrument the error context capture
                statements.push(MirStmt {
                    op: MirOp::TryBlock { result: result_temp.clone() },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Branch on the Result discriminant (simplified: non-zero is Err)
                // A proper implementation would check the Result enum tag
                statements.push(MirStmt {
                    op: MirOp::Branch {
                        condition: result_temp.clone(),
                        true_target: ok_label,
                        false_target: err_label,
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Err path: extract error and return it from current function
                statements.push(MirStmt {
                    op: MirOp::Label { id: err_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // Move error to temp (result_temp holds Err variant's payload)
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: result_temp.clone(),
                        to: error_temp.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // Early return with the error
                statements.push(MirStmt {
                    op: MirOp::Return { value: Some(error_temp) },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // Ok path: unwrap the success value
                statements.push(MirStmt {
                    op: MirOp::Label { id: ok_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
                // Move success value to the result temp for downstream code
                statements.push(MirStmt {
                    op: MirOp::Move {
                        from: result_temp,
                        to: success_temp.clone(),
                    },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });

                // End label
                statements.push(MirStmt {
                    op: MirOp::Label { id: end_label },
                    span: Span::new(0, 0, 0, 0),
                    region: None,
                });
            }
            HirExpr::Struct { fields, .. } => {
                for (_name, field_expr) in fields {
                    let field_ops = self.generate_expr(field_expr, temp_count)?;
                    statements.extend(field_ops);
                }
            }
            HirExpr::FieldAccess { base, .. } => {
                let base_ops = self.generate_expr(base, temp_count)?;
                statements.extend(base_ops);
            }
        }

        Ok(statements)
    }

    /// Add region frees based on region DAG
    pub fn add_region_frees(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        if let Some(ref dag) = self.region_dag {
            for (region, _free_point) in &dag.free_points {
                // Associate region with functions
                // Regions named "fn_<name>" belong to function <name>
                let target_fn = if region.name.starts_with("fn_") {
                    Some(region.name.strip_prefix("fn_").unwrap().to_string())
                } else {
                    None
                };

                for function in &mut mir.functions {
                    let should_insert = match &target_fn {
                        Some(name) => function.name == *name,
                        None => true, // Global regions go in all functions
                    };

                    if should_insert {
                        let free_stmt = MirStmt {
                            op: MirOp::FreeRegion {
                                region: region.clone(),
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: Some(region.clone()),
                        };
                        function.body.statements.push(free_stmt);
                    }
                }
            }
        }
        Ok(())
    }

    /// Add drop operations for linear values
    /// Inserts Drop MIR ops after the last Move from a linear-typed location
    pub fn add_drop_operations(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        for function in &mut mir.functions {
            let mut new_stmts = Vec::new();
            for stmt in &function.body.statements {
                new_stmts.push(stmt.clone());
                // After a Move, insert a Drop on the source if it held a linear value
                // Currently this is tracked via a simple heuristic; full linear-type tracking
                // would consult the type-checker's linearity table.
                if let MirOp::Move { from, .. } = &stmt.op {
                    // For now, drop non-local, non-param sources to clean up temps
                    if !matches!(from, MirLocation::Local(_) | MirLocation::Param(_)) {
                        new_stmts.push(MirStmt {
                            op: MirOp::Drop { location: from.clone() },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                }
            }
            function.body.statements = new_stmts;
        }
        Ok(())
    }

    /// Add bounds checks with proof annotations
    /// Analyzes MIR for Index locations and inserts BoundsCheck ops where missing
    pub fn add_bounds_checks(&mut self, mir: &mut MirProgram) -> Result<(), Vec<MirError>> {
        for function in &mut mir.functions {
            let mut new_statements = Vec::new();
            
            for stmt in &function.body.statements {
                new_statements.push(stmt.clone());
                
                // Index accesses already emit BoundsCheck in generate_expr (Index arm).
                // This pass would add checks for accesses that slipped through,
                // or refine existing checks with proof annotations from the bounds checker.
                if let MirOp::Move { from, .. } = &stmt.op {
                    if let MirLocation::Index { base, index } = from {
                        new_statements.push(MirStmt {
                            op: MirOp::BoundsCheck {
                                index: *index.clone(),
                                bound: *base.clone(),
                                proven: false,
                            },
                            span: Span::new(0, 0, 0, 0),
                            region: None,
                        });
                    }
                }
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