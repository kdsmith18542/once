//! Effects system for the Once language
//! 
//! Implements row-polymorphic effects for:
//! - Async/await operations
//! - Channel communication
//! - Spawn operations
//! - Error handling
//! - Resource management

use once_hir::*;
use once_ty::{Type, TypeVar, Region};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Effect system errors
#[derive(Error, Debug, Clone)]
pub enum EffectError {
    #[error("Effect mismatch: expected {expected}, found {found}")]
    EffectMismatch { expected: String, found: String },
    
    #[error("Unhandled effect: {0}")]
    UnhandledEffect(String),
    
    #[error("Effect row constraint unsatisfiable: {0}")]
    UnsatisfiableConstraint(String),
    
    #[error("Effect row unification failed: {0}")]
    UnificationFailed(String),
    
    #[error("Effect row contains duplicate labels: {0}")]
    DuplicateLabels(String),
}

/// Effect labels for different operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EffectLabel {
    /// Async/await operations
    Async,
    /// Channel send/receive
    Channel,
    /// Spawn operations
    Spawn,
    /// Error handling
    Error,
    /// Resource management
    Resource,
    /// Custom effect
    Custom(String),
}

impl fmt::Display for EffectLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectLabel::Async => write!(f, "Async"),
            EffectLabel::Channel => write!(f, "Channel"),
            EffectLabel::Spawn => write!(f, "Spawn"),
            EffectLabel::Error => write!(f, "Error"),
            EffectLabel::Resource => write!(f, "Resource"),
            EffectLabel::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Effect rows for row-polymorphic effects
#[derive(Debug, Clone, PartialEq)]
pub enum EffectRow {
    /// Empty effect row (no effects)
    Empty,
    /// Effect row with a single effect
    Single { label: EffectLabel, ty: Type },
    /// Effect row with multiple effects
    Cons { label: EffectLabel, ty: Type, tail: Box<EffectRow> },
    /// Effect row variable (for row polymorphism)
    Var(EffectRowVar),
    /// Union of effect rows
    Union(Box<EffectRow>, Box<EffectRow>),
    /// Intersection of effect rows
    Intersection(Box<EffectRow>, Box<EffectRow>),
}

/// Effect row variables
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EffectRowVar(pub usize);

impl fmt::Display for EffectRowVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ρ{}", self.0)
    }
}

impl fmt::Display for EffectRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EffectRow::Empty => write!(f, "∅"),
            EffectRow::Single { label, ty } => write!(f, "{}: {}", label, ty),
            EffectRow::Cons { label, ty, tail } => {
                write!(f, "{}: {} | {}", label, ty, tail)
            }
            EffectRow::Var(var) => write!(f, "{}", var),
            EffectRow::Union(left, right) => write!(f, "{} ∪ {}", left, right),
            EffectRow::Intersection(left, right) => write!(f, "{} ∩ {}", left, right),
        }
    }
}

/// Effectful types
#[derive(Debug, Clone, PartialEq)]
pub struct EffectfulType {
    pub base_type: Type,
    pub effects: EffectRow,
}

impl fmt::Display for EffectfulType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.base_type, self.effects)
    }
}

/// Effect constraints
#[derive(Debug, Clone, PartialEq)]
pub enum EffectConstraint {
    /// Effect row equality
    Equal { left: EffectRow, right: EffectRow },
    /// Effect row subsumption (left subsumes right)
    Subsumes { left: EffectRow, right: EffectRow },
    /// Effect row disjointness
    Disjoint { left: EffectRow, right: EffectRow },
    /// Effect row contains specific effect
    Contains { row: EffectRow, effect: EffectLabel },
    /// Effect row does not contain specific effect
    NotContains { row: EffectRow, effect: EffectLabel },
}

/// Effect environment for tracking effects
#[derive(Debug, Clone)]
pub struct EffectEnv {
    /// Effect row bindings
    pub bindings: HashMap<String, EffectRow>,
    /// Effect constraints
    pub constraints: Vec<EffectConstraint>,
    /// Next effect row variable ID
    pub next_row_var_id: usize,
    /// Effect row variables
    pub row_vars: HashSet<EffectRowVar>,
}

impl EffectEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            constraints: Vec::new(),
            next_row_var_id: 0,
            row_vars: HashSet::new(),
        }
    }

    pub fn fresh_row_var(&mut self) -> EffectRowVar {
        let var = EffectRowVar(self.next_row_var_id);
        self.next_row_var_id += 1;
        self.row_vars.insert(var.clone());
        var
    }

    pub fn add_constraint(&mut self, constraint: EffectConstraint) {
        self.constraints.push(constraint);
    }
}

/// Effect checker for Once programs
pub struct EffectChecker {
    env: EffectEnv,
    errors: Vec<EffectError>,
}

impl EffectChecker {
    pub fn new() -> Self {
        Self {
            env: EffectEnv::new(),
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, hir: &HirProgram) -> Result<(), Vec<EffectError>> {
        // Check effects for all items
        for item in &hir.items {
            self.check_item(item)?;
        }

        // Solve effect constraints
        self.solve_constraints()?;

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_item(&mut self, item: &HirItem) -> Result<(), Vec<EffectError>> {
        match item {
            HirItem::FnDecl(fn_decl) => self.check_fn_decl(fn_decl),
            HirItem::LetDecl(let_decl) => self.check_let_decl(let_decl),
        }
    }

    fn check_fn_decl(&mut self, fn_decl: &HirFnDecl) -> Result<(), Vec<EffectError>> {
        // Create new environment for function
        let mut fn_env = self.env.clone();
        
        // Check function body for effects
        let _body_effects = self.check_block(&fn_decl.body, &mut fn_env)?;
        
        // Add effect constraints
        self.env.constraints.extend(fn_env.constraints);
        self.env.row_vars.extend(fn_env.row_vars);

        Ok(())
    }

    fn check_let_decl(&mut self, let_decl: &HirLetDecl) -> Result<(), Vec<EffectError>> {
        // Check the value expression for effects
        let value_effects = self.check_expr(&let_decl.value)?;
        
        // Add binding to environment
        self.env.bindings.insert(let_decl.name.clone(), value_effects);

        Ok(())
    }

    fn check_block(&mut self, block: &HirBlock, env: &mut EffectEnv) -> Result<EffectRow, Vec<EffectError>> {
        let mut combined_effects = EffectRow::Empty;
        
        for stmt in &block.statements {
            let stmt_effects = self.check_stmt(stmt, env)?;
            combined_effects = self.union_effect_rows(combined_effects, stmt_effects);
        }
        
        Ok(combined_effects)
    }

    fn check_stmt(&mut self, stmt: &HirStmt, env: &mut EffectEnv) -> Result<EffectRow, Vec<EffectError>> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                let value_effects = self.check_expr(&let_stmt.value)?;
                
                // Add binding to environment
                env.bindings.insert(let_stmt.name.clone(), value_effects.clone());
                
                Ok(value_effects)
            }
            HirStmt::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.check_expr(expr)
                } else {
                    Ok(EffectRow::Empty)
                }
            }
            HirStmt::Expr(expr) => self.check_expr(expr),
        }
    }

    fn check_expr(&mut self, expr: &HirExpr) -> Result<EffectRow, Vec<EffectError>> {
        match expr {
            HirExpr::Literal(_) => Ok(EffectRow::Empty),
            HirExpr::Ident(name) => {
                if let Some(effects) = self.env.bindings.get(name) {
                    Ok(effects.clone())
                } else {
                    Ok(EffectRow::Empty)
                }
            }
            HirExpr::Call { function, args } => {
                // Check arguments for effects
                let mut arg_effects = EffectRow::Empty;
                for arg in args {
                    let arg_effect = self.check_expr(arg)?;
                    arg_effects = self.union_effect_rows(arg_effects, arg_effect);
                }
                
                // Function calls may have effects
                let call_effects = match function.as_str() {
                    "spawn" => EffectRow::Single {
                        label: EffectLabel::Spawn,
                        ty: Type::Unit,
                    },
                    "await" => EffectRow::Single {
                        label: EffectLabel::Async,
                        ty: Type::Unit,
                    },
                    "send" | "recv" => EffectRow::Single {
                        label: EffectLabel::Channel,
                        ty: Type::Unit,
                    },
                    _ => EffectRow::Empty,
                };
                
                Ok(self.union_effect_rows(arg_effects, call_effects))
            }
            HirExpr::Binary { left, op: _, right } => {
                let left_effects = self.check_expr(left)?;
                let right_effects = self.check_expr(right)?;
                Ok(self.union_effect_rows(left_effects, right_effects))
            }
            HirExpr::Block(block) => self.check_block(block, &mut self.env.clone()),
        }
    }

    fn union_effect_rows(&self, left: EffectRow, right: EffectRow) -> EffectRow {
        match (left, right) {
            (EffectRow::Empty, right) => right,
            (left, EffectRow::Empty) => left,
            (left, right) => EffectRow::Union(Box::new(left), Box::new(right)),
        }
    }

    fn solve_constraints(&mut self) -> Result<(), Vec<EffectError>> {
        // Simple constraint solver for effects
        // In a full implementation, this would use sophisticated unification
        for constraint in &self.env.constraints {
            match constraint {
                EffectConstraint::Equal { left, right } => {
                    if !self.unify_effect_rows(left, right) {
                        self.errors.push(EffectError::UnificationFailed(
                            format!("Cannot unify {} and {}", left, right)
                        ));
                    }
                }
                EffectConstraint::Subsumes { left, right } => {
                    if !self.subsumes_effect_rows(left, right) {
                        self.errors.push(EffectError::UnsatisfiableConstraint(
                            format!("{} does not subsume {}", left, right)
                        ));
                    }
                }
                EffectConstraint::Disjoint { left, right } => {
                    if !self.disjoint_effect_rows(left, right) {
                        self.errors.push(EffectError::UnsatisfiableConstraint(
                            format!("{} and {} are not disjoint", left, right)
                        ));
                    }
                }
                EffectConstraint::Contains { row, effect } => {
                    if !self.contains_effect(row, effect) {
                        self.errors.push(EffectError::UnhandledEffect(
                            format!("Effect row {} does not contain {}", row, effect)
                        ));
                    }
                }
                EffectConstraint::NotContains { row, effect } => {
                    if self.contains_effect(row, effect) {
                        self.errors.push(EffectError::UnhandledEffect(
                            format!("Effect row {} contains {}", row, effect)
                        ));
                    }
                }
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn unify_effect_rows(&self, left: &EffectRow, right: &EffectRow) -> bool {
        match (left, right) {
            (EffectRow::Empty, EffectRow::Empty) => true,
            (EffectRow::Var(_), _) | (_, EffectRow::Var(_)) => true, // Variables can unify with anything
            (EffectRow::Single { label: l1, ty: t1 }, EffectRow::Single { label: l2, ty: t2 }) => {
                l1 == l2 && self.unify_types(t1, t2)
            }
            (EffectRow::Cons { label: l1, ty: t1, tail: tail1 }, EffectRow::Cons { label: l2, ty: t2, tail: tail2 }) => {
                l1 == l2 && self.unify_types(t1, t2) && self.unify_effect_rows(tail1, tail2)
            }
            (EffectRow::Union(l1, r1), EffectRow::Union(l2, r2)) => {
                self.unify_effect_rows(l1, l2) && self.unify_effect_rows(r1, r2)
            }
            _ => false,
        }
    }

    fn subsumes_effect_rows(&self, left: &EffectRow, right: &EffectRow) -> bool {
        // Simplified subsumption check
        // In a full implementation, this would be more sophisticated
        match (left, right) {
            (EffectRow::Empty, _) => true,
            (_, EffectRow::Empty) => true,
            (EffectRow::Var(_), _) | (_, EffectRow::Var(_)) => true,
            _ => self.unify_effect_rows(left, right),
        }
    }

    fn disjoint_effect_rows(&self, left: &EffectRow, right: &EffectRow) -> bool {
        // Simplified disjointness check
        match (left, right) {
            (EffectRow::Empty, _) | (_, EffectRow::Empty) => true,
            (EffectRow::Var(_), _) | (_, EffectRow::Var(_)) => true,
            _ => !self.unify_effect_rows(left, right),
        }
    }

    fn contains_effect(&self, row: &EffectRow, effect: &EffectLabel) -> bool {
        match row {
            EffectRow::Empty => false,
            EffectRow::Single { label, .. } => label == effect,
            EffectRow::Cons { label, tail, .. } => label == effect || self.contains_effect(tail, effect),
            EffectRow::Var(_) => true, // Variables may contain any effect
            EffectRow::Union(left, right) => self.contains_effect(left, effect) || self.contains_effect(right, effect),
            EffectRow::Intersection(left, right) => self.contains_effect(left, effect) && self.contains_effect(right, effect),
        }
    }

    fn unify_types(&self, left: &Type, right: &Type) -> bool {
        match (left, right) {
            (Type::Var(_), _) | (_, Type::Var(_)) => true,
            (Type::Unit, Type::Unit) => true,
            (Type::Int, Type::Int) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Float, Type::Float) => true,
            (Type::Str, Type::Str) => true,
            (Type::Function { params: p1, return_type: r1 }, Type::Function { params: p2, return_type: r2 }) => {
                p1.len() == p2.len() && 
                p1.iter().zip(p2.iter()).all(|(a, b)| self.unify_types(a, b)) &&
                self.unify_types(r1, r2)
            }
            _ => false,
        }
    }
}

/// Effect inference for Once programs
pub struct EffectInferencer {
    checker: EffectChecker,
}

impl EffectInferencer {
    pub fn new() -> Self {
        Self {
            checker: EffectChecker::new(),
        }
    }

    pub fn infer(&mut self, hir: &HirProgram) -> Result<EffectfulType, Vec<EffectError>> {
        // Infer effects for the main function
        let main_effects = self.checker.check(hir)?;
        
        // For now, return a simple effectful type
        // In a full implementation, this would infer the actual effects
        Ok(EffectfulType {
            base_type: Type::Unit,
            effects: EffectRow::Empty,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral};

    #[test]
    fn test_effect_row_union() {
        let checker = EffectChecker::new();
        
        let left = EffectRow::Single {
            label: EffectLabel::Async,
            ty: Type::Unit,
        };
        let right = EffectRow::Single {
            label: EffectLabel::Channel,
            ty: Type::Unit,
        };
        
        let union = checker.union_effect_rows(left, right);
        assert!(matches!(union, EffectRow::Union(_, _)));
    }

    #[test]
    fn test_effect_contains() {
        let checker = EffectChecker::new();
        
        let row = EffectRow::Single {
            label: EffectLabel::Async,
            ty: Type::Unit,
        };
        
        assert!(checker.contains_effect(&row, &EffectLabel::Async));
        assert!(!checker.contains_effect(&row, &EffectLabel::Channel));
    }

    #[test]
    fn test_empty_effect_row() {
        let checker = EffectChecker::new();
        
        let row = EffectRow::Empty;
        
        assert!(!checker.contains_effect(&row, &EffectLabel::Async));
        assert!(!checker.contains_effect(&row, &EffectLabel::Channel));
    }
}