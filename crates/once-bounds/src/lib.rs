//! Bounds Proofs and Check Erasure for Once Language
//! 
//! Implements:
//! - Compile-time bounds checking for arrays
//! - Proof generation for array access safety
//! - Check erasure when bounds can be proven safe
//! - Size-aware arrays with lightweight bounds proofs

use once_hir::{HirProgram, HirItem, HirExpr, HirStmt, HirType, HirFnDecl, HirBlock, HirLiteral, HirLetStmt, HirReturnStmt};
use once_lex::Span;
use std::collections::HashMap;
use thiserror::Error;

/// Bounds checking errors
#[derive(Error, Debug, Clone)]
pub enum BoundsError {
    #[error("Bounds check failed: {0}")]
    BoundsCheckFailed(String),
    
    #[error("Proof generation failed: {0}")]
    ProofGenerationFailed(String),
    
    #[error("Check erasure failed: {0}")]
    CheckErasureFailed(String),
}

/// Bounds proof for array access
#[derive(Debug, Clone)]
pub struct BoundsProof {
    pub array_var: String,
    pub index_expr: HirExpr,
    pub array_length: usize,
    pub proof_type: ProofType,
    pub constraints: Vec<BoundsConstraint>,
}

/// Type of bounds proof
#[derive(Debug, Clone)]
pub enum ProofType {
    /// Direct constant bounds check
    Constant { index: usize, length: usize },
    /// Variable bounds with constraints
    Variable { constraints: Vec<BoundsConstraint> },
    /// Loop invariant bounds
    LoopInvariant { loop_var: String, bounds: BoundsConstraint },
    /// Function precondition bounds
    Precondition { param: String, bounds: BoundsConstraint },
}

/// Bounds constraint
#[derive(Debug, Clone)]
pub struct BoundsConstraint {
    pub variable: String,
    pub lower_bound: Option<HirExpr>,
    pub upper_bound: Option<HirExpr>,
    pub constraint_type: ConstraintType,
}

/// Type of constraint
#[derive(Debug, Clone)]
pub enum ConstraintType {
    /// 0 <= var < length
    ArrayBounds,
    /// var >= 0
    NonNegative,
    /// var < length
    LessThanLength,
    /// var <= length
    LessThanOrEqualLength,
}

/// Bounds checker
pub struct BoundsChecker {
    pub proofs: HashMap<String, BoundsProof>,
    pub constraints: Vec<BoundsConstraint>,
    pub check_annotations: HashMap<String, CheckAnnotation>,
    known_array_lengths: HashMap<String, usize>,
}

/// Check annotation for bounds checks
#[derive(Debug, Clone)]
pub struct CheckAnnotation {
    pub location: Span,
    pub check_type: CheckType,
    pub can_erase: bool,
    pub proof_id: Option<String>,
}

/// Type of bounds check
#[derive(Debug, Clone)]
pub enum CheckType {
    ArrayAccess { array: String, index: String },
    ArrayLength { array: String },
    SliceBounds { slice: String, start: String, end: String },
}

impl BoundsChecker {
    pub fn new() -> Self {
        Self {
            proofs: HashMap::new(),
            constraints: Vec::new(),
            check_annotations: HashMap::new(),
            known_array_lengths: HashMap::new(),
        }
    }

    /// Check bounds for a HIR program
    pub fn check(&mut self, hir: &HirProgram) -> Result<(), BoundsError> {
        // Analyze all functions for bounds safety
        for item in &hir.items {
            if let HirItem::FnDecl(fn_decl) = item {
                self.check_function(fn_decl)?;
            }
        }

        // Generate proofs for all bounds checks
        self.generate_proofs()?;

        // Erase checks that can be proven safe
        self.erase_safe_checks()?;

        Ok(())
    }

    /// Check bounds for a function
    fn check_function(&mut self, fn_decl: &HirFnDecl) -> Result<(), BoundsError> {
        // Analyze function parameters for array bounds
        for param in &fn_decl.params {
            if let Some(type_annotation) = &param.type_annotation {
                if let HirType::Array(_element_type_hir, length) = type_annotation {
                    self.known_array_lengths.insert(param.name.clone(), *length);
                }
            }
        }

        // Check function body
        self.check_block(&fn_decl.body)?;

        Ok(())
    }

    /// Check bounds for a block
    fn check_block(&mut self, block: &HirBlock) -> Result<(), BoundsError> {
        for stmt in &block.statements {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    /// Check bounds for a statement
    fn check_stmt(&mut self, stmt: &HirStmt) -> Result<(), BoundsError> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                self.check_expr(&let_stmt.value)?;
            }
            HirStmt::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.check_expr(expr)?;
                }
            }
            HirStmt::Expr(expr) => {
                self.check_expr(expr)?;
            }
            HirStmt::Continue(_) | HirStmt::Break(_) => {}
            HirStmt::Using(using_stmt) => {
                self.check_expr(&using_stmt.init)?;
                self.check_block(&using_stmt.body)?;
            }
        }
        Ok(())
    }

    /// Check bounds for an expression
    fn check_expr(&mut self, expr: &HirExpr) -> Result<(), BoundsError> {
        match expr {
            HirExpr::Binary { left, op: _, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
            }
            HirExpr::Call { function: _, args, .. } => {
                for arg in args {
                    self.check_expr(arg)?;
                }
            }
            HirExpr::If { condition, then_branch, else_branch, .. } => {
                self.check_expr(condition)?;
                self.check_block(then_branch)?;
                if let Some(else_expr) = else_branch {
                    self.check_expr(else_expr)?;
                }
            }
            HirExpr::Match { expr, arms, .. } => {
                self.check_expr(expr)?;
                for arm in arms {
                    self.check_expr(&arm.body)?;
                }
            }
            HirExpr::For { collection, body, .. } => {
                self.check_expr(collection)?;
                self.check_block(body)?;
            }
            HirExpr::Block(block, _) => {
                self.check_block(block)?;
            }
            _ => {} // Other expressions don't need bounds checking yet
        }
        Ok(())
    }

    /// Check array access bounds
    fn check_array_access(&mut self, array: &HirExpr, index: &HirExpr) -> Result<(), BoundsError> {
        let check_id = format!("array_access_{}", self.check_annotations.len());
        
        let span = self.extract_span_from_expr(array);
        
        let annotation = CheckAnnotation {
            location: span,
            check_type: CheckType::ArrayAccess {
                array: self.extract_variable_name(array),
                index: self.extract_variable_name(index),
            },
            can_erase: false,
            proof_id: None,
        };

        self.check_annotations.insert(check_id.clone(), annotation);

        // Try to generate proof for this access
        if let Some(proof) = self.generate_array_access_proof(array, index)? {
            self.proofs.insert(check_id, proof);
        }

        Ok(())
    }

    /// Check array slice bounds
    fn check_array_slice(&mut self, array: &HirExpr, start: &HirExpr, end: &HirExpr) -> Result<(), BoundsError> {
        let check_id = format!("array_slice_{}", self.check_annotations.len());
        
        let annotation = CheckAnnotation {
            location: Span { start: 0, end: 0, line: 0, column: 0 },
            check_type: CheckType::SliceBounds {
                slice: self.extract_variable_name(array),
                start: self.extract_variable_name(start),
                end: self.extract_variable_name(end),
            },
            can_erase: false,
            proof_id: None,
        };

        self.check_annotations.insert(check_id, annotation);
        Ok(())
    }

    /// Check array length access
    fn check_array_length(&mut self, array: &HirExpr) -> Result<(), BoundsError> {
        let check_id = format!("array_length_{}", self.check_annotations.len());
        
        let annotation = CheckAnnotation {
            location: Span { start: 0, end: 0, line: 0, column: 0 },
            check_type: CheckType::ArrayLength {
                array: self.extract_variable_name(array),
            },
            can_erase: true, // Array length access is always safe
            proof_id: None,
        };

        self.check_annotations.insert(check_id, annotation);
        Ok(())
    }

    /// Generate proof for array access
    fn generate_array_access_proof(&self, array: &HirExpr, index: &HirExpr) -> Result<Option<BoundsProof>, BoundsError> {
        // Try to determine array length from type information
        let array_length = self.get_array_length(array)?;
        
        // Try to determine if index is constant
        if let Some(constant_index) = self.get_constant_value(index) {
            if array_length > 0 && constant_index < array_length {
                return Ok(Some(BoundsProof {
                    array_var: self.extract_variable_name(array),
                    index_expr: index.clone(),
                    array_length,
                    proof_type: ProofType::Constant {
                        index: constant_index,
                        length: array_length,
                    },
                    constraints: Vec::new(),
                }));
            } else if array_length > 0 {
                return Err(BoundsError::BoundsCheckFailed(
                    format!("Index {} out of bounds for array of length {}", constant_index, array_length)
                ));
            }
            // If array_length is 0 (unknown), fall through to variable bounds proof
        }

        // Generate variable bounds proof
        let constraints = self.generate_variable_constraints(array, index)?;
        
        Ok(Some(BoundsProof {
            array_var: self.extract_variable_name(array),
            index_expr: index.clone(),
            array_length,
            proof_type: ProofType::Variable { constraints: constraints.clone() },
            constraints,
        }))
    }

    /// Generate constraints for variable bounds
    fn generate_variable_constraints(&self, array: &HirExpr, index: &HirExpr) -> Result<Vec<BoundsConstraint>, BoundsError> {
        let mut constraints = Vec::new();
        
        // Add non-negative constraint
        constraints.push(BoundsConstraint {
            variable: self.extract_variable_name(index),
                        lower_bound: Some(HirExpr::Literal(HirLiteral::Int(0), None)),
            upper_bound: None,
            constraint_type: ConstraintType::NonNegative,
        });

        // Add less than length constraint
        constraints.push(BoundsConstraint {
            variable: self.extract_variable_name(index),
            lower_bound: None,
            upper_bound: Some(HirExpr::Call {
                function: "len".to_string(),
                args: vec![array.clone()],
                span: None,
            }),
            constraint_type: ConstraintType::LessThanLength,
        });

        Ok(constraints)
    }

    /// Get array length from type information
    fn get_array_length(&self, array: &HirExpr) -> Result<usize, BoundsError> {
        let var_name = self.extract_variable_name(array);
        if let Some(&len) = self.known_array_lengths.get(&var_name) {
            Ok(len)
        } else {
            // Array length not known statically; return 0 to indicate unknown
            Ok(0)
        }
    }

    /// Get constant value from expression
    fn get_constant_value(&self, expr: &HirExpr) -> Option<usize> {
        match expr {
            HirExpr::Literal(HirLiteral::Int(value), _) => Some(*value as usize),
            _ => None,
        }
    }

    /// Extract variable name from expression
    fn extract_variable_name(&self, expr: &HirExpr) -> String {
        match expr {
            HirExpr::Ident(name, _) => name.clone(),
            _ => "unknown".to_string(),
        }
    }

    fn extract_span_from_expr(&self, expr: &HirExpr) -> Span {
        match expr {
            HirExpr::Block(HirBlock { span: Some(hs), .. }, _) => Span { start: hs.start, end: hs.end, line: hs.line, column: hs.column },
            _ => Span { start: 0, end: 0, line: 0, column: 0 },
        }
    }

    /// Generate all proofs
    fn generate_proofs(&mut self) -> Result<(), BoundsError> {
        // This would analyze the constraint system and generate proofs
        // For now, mark all constant bounds checks as safe
        for (check_id, annotation) in &mut self.check_annotations {
            if let CheckType::ArrayAccess { .. } = &annotation.check_type {
                if let Some(proof) = self.proofs.get(check_id) {
                    if matches!(proof.proof_type, ProofType::Constant { .. }) {
                        annotation.can_erase = true;
                        annotation.proof_id = Some(check_id.clone());
                    }
                }
            }
        }
        Ok(())
    }

    /// Erase safe checks
    fn erase_safe_checks(&mut self) -> Result<(), BoundsError> {
        // Remove bounds checks that can be proven safe
        self.check_annotations.retain(|_, annotation| !annotation.can_erase);
        Ok(())
    }

    /// Get remaining bounds checks that need runtime verification
    pub fn get_runtime_checks(&self) -> Vec<&CheckAnnotation> {
        self.check_annotations.values().collect()
    }

    /// Get bounds proofs
    pub fn get_proofs(&self) -> &HashMap<String, BoundsProof> {
        &self.proofs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_checker_creation() {
        let checker = BoundsChecker::new();
        assert!(checker.proofs.is_empty());
        assert!(checker.constraints.is_empty());
    }

    #[test]
    fn test_constant_bounds_proof() {
        let mut checker = BoundsChecker::new();
        
        // Register array length so constant bounds check can succeed
        checker.known_array_lengths.insert("arr".to_string(), 10);
        
        // Test constant array access
        let array = HirExpr::Ident("arr".to_string(), None);
        let index = HirExpr::Literal(HirLiteral::Int(5), None);
        
        let proof = checker.generate_array_access_proof(&array, &index).unwrap();
        assert!(proof.is_some());
        
        if let Some(proof) = proof {
            assert_eq!(proof.array_var, "arr");
            assert!(matches!(proof.proof_type, ProofType::Constant { .. }));
        }
    }
}
