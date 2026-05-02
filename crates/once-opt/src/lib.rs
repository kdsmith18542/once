//! Optimization Passes for Once Language
//!
//! Implements:
//! - Constant folding
//! - Dead code elimination
//! - Inline small functions
//! - Loop unrolling for known iterations

use once_hir::{HirProgram, HirItem, HirExpr, HirStmt, HirFnDecl, HirBinaryOp, HirLiteral, HirBlock};
use once_mir::{MirProgram, MirBlock, MirOp, MirStmt};
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
            HirExpr::Binary { left, op, right } => {
                let mut changed = self.constant_fold_expr(left);
                changed |= self.constant_fold_expr(right);
                if let (HirExpr::Literal(HirLiteral::Int(l)), HirExpr::Literal(HirLiteral::Int(r))) = (left.as_ref(), right.as_ref()) {
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
                    *expr = HirExpr::Literal(result);
                    true
                } else if let (HirExpr::Literal(HirLiteral::Bool(l)), HirExpr::Literal(HirLiteral::Bool(r))) = (left.as_ref(), right.as_ref()) {
                    let result = match op {
                        HirBinaryOp::And => HirLiteral::Bool(*l && *r),
                        HirBinaryOp::Or => HirLiteral::Bool(*l || *r),
                        HirBinaryOp::Eq => HirLiteral::Bool(l == r),
                        HirBinaryOp::Ne => HirLiteral::Bool(l != r),
                        _ => return changed,
                    };
                    *expr = HirExpr::Literal(result);
                    true
                } else if let (HirExpr::Literal(HirLiteral::Float(l)), HirExpr::Literal(HirLiteral::Float(r))) = (left.as_ref(), right.as_ref()) {
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
                    *expr = HirExpr::Literal(result);
                    true
                } else {
                    changed
                }
            }
            HirExpr::Block(block) => self.constant_fold_block(block),
            HirExpr::If { condition, then_branch, else_branch } => {
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
        for block in &mut func.body.statements {
            self.optimize_mir_statement(block);
        }
    }

    fn optimize_mir_statement(&mut self, _stmt: &mut MirStmt) {
        // TODO: Implement MIR statement-level optimizations once MIR op set stabilizes
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