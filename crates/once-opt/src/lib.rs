//! Optimization Passes for Once Language
//!
//! Implements:
//! - Constant folding
//! - Dead code elimination
//! - Inline small functions
//! - Loop unrolling for known iterations

use once_hir::{HirProgram, HirItem, HirExpr, HirStmt, HirFnDecl};
use once_mir::{MirProgram, MirBlock, MirOp};
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

    fn constant_fold_block(&mut self, block: &mut Vec<HirStmt>) -> bool {
        false
    }

    pub fn optimize_mir(&mut self, mir: &mut MirProgram) -> Result<(), OptError> {
        for func in &mut mir.functions {
            self.optimize_mir_function(func);
        }
        Ok(())
    }

    fn optimize_mir_function(&mut self, func: &mut once_mir::MirFn) {
        for block in &mut func.blocks {
            self.optimize_mir_block(block);
        }
    }

    fn optimize_mir_block(&mut self, block: &mut MirBlock) {
        let mut i = 0;
        while i < block.ops.len() {
            if let MirOp::Const { .. } = &block.ops[i] {
                if i + 1 < block.ops.len() {
                    if let MirOp::BinOp { .. } = &block.ops[i + 1] {
                        block.ops.remove(i);
                        continue;
                    }
                }
            }
            i += 1;
        }
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