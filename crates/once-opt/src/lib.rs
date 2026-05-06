//! Optimization Passes for Once Language
//!
//! Implements:
//! - Constant folding
//! - Dead code elimination
//! - Inline small functions
//! - Loop unrolling for known iterations

use once_hir::{HirProgram, HirItem, HirExpr, HirStmt, HirFnDecl, HirBinaryOp, HirLiteral, HirBlock};
use once_mir::{MirProgram, MirOp};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OptError {
    #[error("Optimization failed: {0}")]
    Failed(String),
}

pub struct Optimizer {
    pub inline_threshold: usize,
    pub max_iterations: usize,
}

impl Default for Optimizer {
    fn default() -> Self {
        Self {
            inline_threshold: 10,
            max_iterations: 3,
        }
    }
}

impl Optimizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn optimize_hir(&mut self, hir: &mut HirProgram) -> Result<(), OptError> {
        for _ in 0..self.max_iterations {
            let mut changed = false;
            for item in &mut hir.items {
                if let HirItem::FnDecl(fn_decl) = item {
                    changed |= self.optimize_function(fn_decl);
                }
            }
            if !changed {
                break;
            }
        }
        Ok(())
    }

    fn optimize_function(&mut self, fn_decl: &mut HirFnDecl) -> bool {
        let mut changed = false;
        changed |= self.constant_fold_block(&mut fn_decl.body);
        changed
    }

    fn constant_fold_block(&mut self, block: &mut HirBlock) -> bool {
        let mut changed = false;
        for stmt in &mut block.statements {
            changed |= self.constant_fold_stmt(stmt);
        }
        changed
    }

    fn constant_fold_stmt(&mut self, stmt: &mut HirStmt) -> bool {
        match stmt {
            HirStmt::Let(let_stmt) => self.constant_fold_expr(&mut let_stmt.value),
            HirStmt::Return(return_stmt) => {
                if let Some(ref mut expr) = return_stmt.value {
                    self.constant_fold_expr(expr)
                } else {
                    false
                }
            }
            HirStmt::Expr(expr) => self.constant_fold_expr(expr),
            HirStmt::Using(using_stmt) => {
                let mut changed = self.constant_fold_expr(&mut using_stmt.init);
                changed |= self.constant_fold_block(&mut using_stmt.body);
                changed
            }
            _ => false,
        }
    }

    fn constant_fold_expr(&mut self, expr: &mut HirExpr) -> bool {
        match expr {
            HirExpr::Binary { left, op, right, .. } => {
                let mut changed = self.constant_fold_expr(left);
                changed |= self.constant_fold_expr(right);
                if let (HirExpr::Literal(HirLiteral::Int(l), _), HirExpr::Literal(HirLiteral::Int(r), _)) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        HirBinaryOp::Add => HirLiteral::Int(l + r),
                        HirBinaryOp::Sub => HirLiteral::Int(l - r),
                        HirBinaryOp::Mul => HirLiteral::Int(l * r),
                        HirBinaryOp::Div if *r != 0 => HirLiteral::Int(l / r),
                        HirBinaryOp::Eq => HirLiteral::Bool(l == r),
                        HirBinaryOp::Ne => HirLiteral::Bool(l != r),
                        HirBinaryOp::Lt => HirLiteral::Bool(l < r),
                        HirBinaryOp::Le => HirLiteral::Bool(l <= r),
                        HirBinaryOp::Gt => HirLiteral::Bool(l > r),
                        HirBinaryOp::Ge => HirLiteral::Bool(l >= r),
                        _ => return changed,
                    };
                    *expr = HirExpr::Literal(result, None);
                    true
                } else if let (HirExpr::Literal(HirLiteral::Bool(l), _), HirExpr::Literal(HirLiteral::Bool(r), _)) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        HirBinaryOp::And => HirLiteral::Bool(*l && *r),
                        HirBinaryOp::Or => HirLiteral::Bool(*l || *r),
                        HirBinaryOp::Eq => HirLiteral::Bool(l == r),
                        HirBinaryOp::Ne => HirLiteral::Bool(l != r),
                        _ => return changed,
                    };
                    *expr = HirExpr::Literal(result, None);
                    true
                } else if let (HirExpr::Literal(HirLiteral::Float(l), _), HirExpr::Literal(HirLiteral::Float(r), _)) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        HirBinaryOp::Add => HirLiteral::Float(l + r),
                        HirBinaryOp::Sub => HirLiteral::Float(l - r),
                        HirBinaryOp::Mul => HirLiteral::Float(l * r),
                        HirBinaryOp::Div if *r != 0.0 => HirLiteral::Float(l / r),
                        HirBinaryOp::Eq => HirLiteral::Bool(l == r),
                        HirBinaryOp::Ne => HirLiteral::Bool(l != r),
                        HirBinaryOp::Lt => HirLiteral::Bool(l < r),
                        HirBinaryOp::Le => HirLiteral::Bool(l <= r),
                        HirBinaryOp::Gt => HirLiteral::Bool(l > r),
                        HirBinaryOp::Ge => HirLiteral::Bool(l >= r),
                        _ => return changed,
                    };
                    *expr = HirExpr::Literal(result, None);
                    true
                } else {
                    changed
                }
            }
            HirExpr::Block(block, _) => self.constant_fold_block(block),
            HirExpr::If { condition, then_branch, else_branch, .. } => {
                let mut changed = self.constant_fold_expr(condition);
                changed |= self.constant_fold_block(then_branch);
                if let Some(ref mut else_expr) = else_branch {
                    changed |= self.constant_fold_expr(else_expr);
                }
                changed
            }
            HirExpr::Call { args, .. } => {
                let mut changed = false;
                for arg in args {
                    changed |= self.constant_fold_expr(arg);
                }
                changed
            }
            _ => false,
        }
    }

    pub fn optimize_mir(&mut self, mir: &mut MirProgram) -> Result<(), OptError> {
        for func in &mut mir.functions {
            self.optimize_mir_function(func);
        }
        Ok(())
    }

    fn optimize_mir_function(&mut self, func: &mut once_mir::MirFunction) {
        self.constant_fold_mir_function(func);
        self.dce_mir_function(func);
    }

    fn constant_fold_mir_function(&self, func: &mut once_mir::MirFunction) {
        use std::collections::HashMap;
        use once_mir::{MirBinOp, MirValue};

        let mut constants: HashMap<once_mir::MirLocation, MirValue> = HashMap::new();
        let mut folded_stmts = Vec::new();

        for stmt in &func.body.statements {
            match &stmt.op {
                MirOp::LoadLiteral { value, dest } => {
                    constants.insert(dest.clone(), value.clone());
                    folded_stmts.push(stmt.clone());
                }
                MirOp::BinOp { op, left, right, dest } => {
                    if let (Some(l_val), Some(r_val)) = (constants.get(left), constants.get(right)) {
                        match (l_val, r_val) {
                            (MirValue::Int(a), MirValue::Int(b)) => {
                                let result = match op {
                                    MirBinOp::Add => MirValue::Int(a + b),
                                    MirBinOp::Sub => MirValue::Int(a - b),
                                    MirBinOp::Mul => MirValue::Int(a * b),
                                    MirBinOp::Div if *b != 0 => MirValue::Int(a / b),
                                    MirBinOp::Eq => MirValue::Bool(a == b),
                                    MirBinOp::Ne => MirValue::Bool(a != b),
                                    MirBinOp::Lt => MirValue::Bool(a < b),
                                    MirBinOp::Le => MirValue::Bool(a <= b),
                                    MirBinOp::Gt => MirValue::Bool(a > b),
                                    MirBinOp::Ge => MirValue::Bool(a >= b),
                                    _ => { folded_stmts.push(stmt.clone()); constants.remove(dest); continue; }
                                };
                                constants.insert(dest.clone(), result.clone());
                                folded_stmts.push(once_mir::MirStmt {
                                    op: MirOp::LoadLiteral { value: result, dest: dest.clone() },
                                    span: stmt.span,
                                    region: stmt.region.clone(),
                                });
                                continue;
                            }
                            (MirValue::Bool(a), MirValue::Bool(b)) => {
                                let result = match op {
                                    MirBinOp::And => MirValue::Bool(*a && *b),
                                    MirBinOp::Or => MirValue::Bool(*a || *b),
                                    MirBinOp::Eq => MirValue::Bool(a == b),
                                    MirBinOp::Ne => MirValue::Bool(a != b),
                                    _ => { folded_stmts.push(stmt.clone()); constants.remove(dest); continue; }
                                };
                                constants.insert(dest.clone(), result.clone());
                                folded_stmts.push(once_mir::MirStmt {
                                    op: MirOp::LoadLiteral { value: result, dest: dest.clone() },
                                    span: stmt.span,
                                    region: stmt.region.clone(),
                                });
                                continue;
                            }
                            (MirValue::Float(a), MirValue::Float(b)) => {
                                let result = match op {
                                    MirBinOp::Add => MirValue::Float(a + b),
                                    MirBinOp::Sub => MirValue::Float(a - b),
                                    MirBinOp::Mul => MirValue::Float(a * b),
                                    MirBinOp::Div if *b != 0.0 => MirValue::Float(a / b),
                                    MirBinOp::Eq => MirValue::Bool(a == b),
                                    MirBinOp::Ne => MirValue::Bool(a != b),
                                    MirBinOp::Lt => MirValue::Bool(a < b),
                                    MirBinOp::Le => MirValue::Bool(a <= b),
                                    MirBinOp::Gt => MirValue::Bool(a > b),
                                    MirBinOp::Ge => MirValue::Bool(a >= b),
                                    _ => { folded_stmts.push(stmt.clone()); constants.remove(dest); continue; }
                                };
                                constants.insert(dest.clone(), result.clone());
                                folded_stmts.push(once_mir::MirStmt {
                                    op: MirOp::LoadLiteral { value: result, dest: dest.clone() },
                                    span: stmt.span,
                                    region: stmt.region.clone(),
                                });
                                continue;
                            }
                            _ => {}
                        }
                    }
                    folded_stmts.push(stmt.clone());
                    constants.remove(dest);
                }
                _ => {
                    folded_stmts.push(stmt.clone());
                }
            }
        }

        func.body.statements = folded_stmts;
    }

    /// Dead code elimination at MIR level
    pub fn dead_code_eliminate_mir(&mut self, mir: &mut MirProgram) -> Result<(), OptError> {
        for func in &mut mir.functions {
            self.dce_mir_function(func);
        }
        Ok(())
    }

    fn dce_mir_function(&self, func: &mut once_mir::MirFunction) {
        use std::collections::HashSet;
        
        // Mark all destinations that are used (read) before they are written again
        let mut used_locations: HashSet<once_mir::MirLocation> = HashSet::new();
        let mut alive_stmts = Vec::new();
        
        // First pass: mark all locations that are ever used as function results or args
        for stmt in &func.body.statements {
            match &stmt.op {
                MirOp::Return { value: Some(v) } => { used_locations.insert(v.clone()); }
                MirOp::Call { args, .. } => { for a in args { used_locations.insert(a.clone()); } }
                MirOp::BinOp { left, right, .. } => { used_locations.insert(left.clone()); used_locations.insert(right.clone()); }
                MirOp::Branch { condition, .. } => { used_locations.insert(condition.clone()); }
                MirOp::Move { from, .. } => { used_locations.insert(from.clone()); }
                MirOp::ChannelSend { value, .. } => { used_locations.insert(value.clone()); }
                MirOp::SpawnTask { args, .. } => { for a in args { used_locations.insert(a.clone()); } }
                MirOp::AwaitTask { task, .. } => { used_locations.insert(task.clone()); }
                MirOp::SpawnInGroup { group, args, .. } => { used_locations.insert(group.clone()); for a in args { used_locations.insert(a.clone()); } }
                MirOp::AwaitGroup { group, .. } => { used_locations.insert(group.clone()); }
                _ => {}
            }
        }
        
        // Second pass: keep only statements whose destination is used
        for stmt in &func.body.statements {
            let produces = match &stmt.op {
                MirOp::LoadLiteral { dest, .. } => Some(dest.clone()),
                MirOp::BinOp { dest, .. } => Some(dest.clone()),
                MirOp::Call { result, .. } => Some(result.clone()),
                MirOp::Move { to, .. } => Some(to.clone()),
                MirOp::ChannelRecv { result, .. } => Some(result.clone()),
                MirOp::SpawnTask { result, .. } => Some(result.clone()),
                MirOp::AwaitTask { result, .. } => Some(result.clone()),
                MirOp::CreateGroup { result } => Some(result.clone()),
                MirOp::SpawnInGroup { result, .. } => Some(result.clone()),
                MirOp::AwaitGroup { result, .. } => Some(result.clone()),
                _ => None,
            };
            
            let should_keep = match &produces {
                Some(loc) => used_locations.contains(loc),
                None => true, // Keep control flow statements (Return, Jump, Branch, Label)
            };
            
            if should_keep {
                alive_stmts.push(stmt.clone());
            }
        }
        
        func.body.statements = alive_stmts;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimizer_creation() {
        let optimizer = Optimizer::new();
        assert_eq!(optimizer.inline_threshold, 10);
        assert_eq!(optimizer.max_iterations, 3);
    }
}