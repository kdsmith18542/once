//! Linearity checking for the Once language
//! 
//! Implements move/consume analysis for:
//! - Linear type checking
//! - Resource safety
//! - Copy trait constraints
//! - Closure capture rules

use once_hir::*;
use once_lex::Span;
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Source span for error reporting
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceSpan {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl fmt::Display for SourceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{} ({}..{})", self.line, self.column, self.start, self.end)
    }
}

/// Linearity checking errors with optional source location
#[derive(Error, Debug, Clone)]
pub enum LinearityError {
    #[error("Linear value used multiple times: {name}")]
    LinearValueReused { name: String, span: Option<SourceSpan> },
    
    #[error("Non-linear value in linear context: {name}")]
    NonLinearInLinearContext { name: String, span: Option<SourceSpan> },
    
    #[error("Linear value not consumed: {name}")]
    LinearValueNotConsumed { name: String, span: Option<SourceSpan> },

    #[error("Linear variable usage mismatch in branches: {name}")]
    BranchUsageMismatch { name: String, span: Option<SourceSpan> },
    
    #[error("Copy constraint violated: {0}")]
    CopyConstraintViolated(String),
    
    #[error("Resource not properly consumed: {0}")]
    ResourceNotConsumed(String),
    
    #[error("Closure capture violation: {0}")]
    ClosureCaptureViolation(String),
}

impl LinearityError {
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            LinearityError::LinearValueReused { span, .. } => *span,
            LinearityError::NonLinearInLinearContext { span, .. } => *span,
            LinearityError::LinearValueNotConsumed { span, .. } => *span,
            LinearityError::BranchUsageMismatch { span, .. } => *span,
            _ => None,
        }
    }

    pub fn diagnostic(&self) -> String {
        match self.span() {
            Some(span) => format!("{} at {}", self, span),
            None => self.to_string(),
        }
    }
}

/// Linearity information for variables
#[derive(Debug, Clone, PartialEq)]
pub enum Linearity {
    /// Linear value (must be consumed exactly once)
    Linear,
    /// Affine value (must be consumed at most once)
    Affine,
    /// Non-linear value (can be used multiple times)
    NonLinear,
}

impl fmt::Display for Linearity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Linearity::Linear => write!(f, "linear"),
            Linearity::Affine => write!(f, "affine"),
            Linearity::NonLinear => write!(f, "non-linear"),
        }
    }
}

/// Resource trait for linear types
pub trait Resource {
    fn consume(self) -> ();
}

/// Copy trait for safe copying
pub trait Copy: Clone {
    // Marker trait for safe copying
}

/// Copy constraint for type checking
#[derive(Debug, Clone, PartialEq)]
pub struct CopyConstraint {
    pub type_name: String,
    pub implements_copy: bool,
}

/// Linear usage tracking
#[derive(Debug, Clone)]
pub struct UsageInfo {
    pub variable: String,
    pub linearity: Linearity,
    pub usage_count: usize,
    pub first_use: Option<Span>,
    pub last_use: Option<Span>,
}

/// Closure capture information
#[derive(Debug, Clone)]
pub struct ClosureCapture {
    pub closure_id: String,
    pub captured_vars: Vec<String>,
    pub linear_captures: Vec<String>,
    pub ownership_transfer: bool,
    pub is_linear: bool,
    pub usage_count: usize,
    pub capture_depth: usize,
}

/// Linearity environment
#[derive(Debug, Clone)]
pub struct LinearityEnv {
    /// Variable linearity information
    pub variables: HashMap<String, UsageInfo>,
    /// Copy constraints
    pub copy_constraints: Vec<CopyConstraint>,
    /// Resource traits
    pub resource_traits: HashMap<String, bool>,
    /// Closure captures
    pub closure_captures: Vec<ClosureCapture>,
}

impl LinearityEnv {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            copy_constraints: Vec::new(),
            resource_traits: HashMap::new(),
            closure_captures: Vec::new(),
        }
    }

    pub fn add_variable(&mut self, name: String, linearity: Linearity) {
        self.variables.insert(name.clone(), UsageInfo {
            variable: name,
            linearity,
            usage_count: 0,
            first_use: None,
            last_use: None,
        });
    }

    pub fn use_variable(&mut self, name: &str, span: Span) -> Result<(), LinearityError> {
        if let Some(usage) = self.variables.get_mut(name) {
            usage.usage_count += 1;
            if usage.first_use.is_none() {
                usage.first_use = Some(span);
            }
            usage.last_use = Some(span);

            match usage.linearity {
                Linearity::Linear => {
                    if usage.usage_count > 1 {
                        return Err(LinearityError::LinearValueReused {
                            name: name.to_string(),
                            span: Some(SourceSpan { start: span.start, end: span.end, line: span.line, column: span.column }),
                        });
                    }
                }
                Linearity::Affine => {
                    if usage.usage_count > 1 {
                        return Err(LinearityError::LinearValueReused {
                            name: name.to_string(),
                            span: Some(SourceSpan { start: span.start, end: span.end, line: span.line, column: span.column }),
                        });
                    }
                }
                Linearity::NonLinear => {
                    // Non-linear values can be used multiple times
                }
            }
        }
        Ok(())
    }

    pub fn consume_variable(&mut self, name: &str) -> Result<(), LinearityError> {
        if let Some(usage) = self.variables.get_mut(name) {
            match usage.linearity {
                Linearity::Linear | Linearity::Affine => {
                    // Mark as consumed
                    usage.usage_count = 0;
                }
                Linearity::NonLinear => {
                    // Non-linear values don't need explicit consumption
                }
            }
        }
        Ok(())
    }
}

/// Linearity checker for Once programs
pub struct LinearityChecker {
    env: LinearityEnv,
    errors: Vec<LinearityError>,
}

impl LinearityChecker {
    pub fn new() -> Self {
        Self {
            env: LinearityEnv::new(),
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, hir: &HirProgram) -> Result<(), Vec<LinearityError>> {
        // Add built-in resource types
        self.add_builtin_resources();
        
        // Check linearity for all items
        for item in &hir.items {
            self.check_item(item)?;
        }

        // Check for unconsumed linear values
        self.check_unconsumed_linear_values()?;

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn add_builtin_resources(&mut self) {
        // Add built-in resource types
        self.env.resource_traits.insert("File".to_string(), true);
        self.env.resource_traits.insert("TcpStream".to_string(), true);
        self.env.resource_traits.insert("Deadline".to_string(), true);
        self.env.resource_traits.insert("Task".to_string(), true);
        
        // Add built-in copy types
        self.env.copy_constraints.push(CopyConstraint {
            type_name: "Int".to_string(),
            implements_copy: true,
        });
        self.env.copy_constraints.push(CopyConstraint {
            type_name: "Bool".to_string(),
            implements_copy: true,
        });
        self.env.copy_constraints.push(CopyConstraint {
            type_name: "Float".to_string(),
            implements_copy: true,
        });
        self.env.copy_constraints.push(CopyConstraint {
            type_name: "Unit".to_string(),
            implements_copy: true,
        });
    }

    fn check_item(&mut self, item: &HirItem) -> Result<(), Vec<LinearityError>> {
        match item {
            HirItem::FnDecl(fn_decl) => self.check_fn_decl(fn_decl),
            HirItem::LetDecl(let_decl) => self.check_let_decl(let_decl),
            HirItem::TypeDecl(_) => Ok(()),
            HirItem::StructDecl(_) => Ok(()),
            HirItem::TraitDecl(_) => Ok(()),
            HirItem::ImplBlock(_) => Ok(()),
        }
    }

    fn check_fn_decl(&mut self, fn_decl: &HirFnDecl) -> Result<(), Vec<LinearityError>> {
        // Create new environment for function
        let mut fn_env = self.env.clone();
        
        // Add parameters to environment
        for param in &fn_decl.params {
            let linearity = if param.is_linear {
                Linearity::Linear
            } else {
                Linearity::NonLinear
            };
            
            fn_env.add_variable(param.name.clone(), linearity);
        }

        // Check function body
        self.check_block(&fn_decl.body, &mut fn_env)?;

        // Check for unconsumed linear values in function scope
        for (name, usage) in &fn_env.variables {
            if usage.linearity == Linearity::Linear && usage.usage_count == 0 && usage.first_use.is_none() {
                        self.errors.push(LinearityError::LinearValueNotConsumed {
                            name: name.clone(),
                            span: None,
                        });
            }
        }

        // Merge back to main environment
        self.env.variables.extend(fn_env.variables);
        self.env.copy_constraints.extend(fn_env.copy_constraints);
        self.env.resource_traits.extend(fn_env.resource_traits);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn check_let_decl(&mut self, let_decl: &HirLetDecl) -> Result<(), Vec<LinearityError>> {
        // Check the value expression
        self.check_expr(&let_decl.value)?;
        
        // Determine linearity based on type annotation
        let linearity = if let Some(ty) = &let_decl.type_annotation {
            match ty {
                HirType::Linear(_) => Linearity::Linear,
                HirType::Affine(_) => Linearity::Affine,
                _ => Linearity::NonLinear,
            }
        } else {
            Linearity::NonLinear
        };

        // Add variable to environment
        self.env.add_variable(let_decl.name.clone(), linearity);

        Ok(())
    }

    fn check_block(&mut self, block: &HirBlock, env: &mut LinearityEnv) -> Result<(), Vec<LinearityError>> {
        for stmt in &block.statements {
            self.check_stmt(stmt, env)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &HirStmt, env: &mut LinearityEnv) -> Result<(), Vec<LinearityError>> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                // Check the value expression
                self.check_expr_with_env(&let_stmt.value, env)?;
                
                // Determine linearity
                let linearity = if let Some(ty) = &let_stmt.type_annotation {
                    match ty {
                        HirType::Linear(_) => Linearity::Linear,
                        HirType::Affine(_) => Linearity::Affine,
                        _ => Linearity::NonLinear,
                    }
                } else {
                    Linearity::NonLinear
                };

                // Add variable to environment
                env.add_variable(let_stmt.name.clone(), linearity);
            }
            HirStmt::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.check_expr_with_env(expr, env)?;
                }
            }
            HirStmt::Expr(expr) => {
                self.check_expr_with_env(expr, env)?;
            }
            HirStmt::Using(using_stmt) => {
                // Check init expression
                self.check_expr_with_env(&using_stmt.init, env)?;
                // The using variable is always linear
                env.add_variable(using_stmt.name.clone(), Linearity::Linear);
                // Check body statements
                for stmt in &using_stmt.body.statements {
                    self.check_stmt(stmt, env)?;
                }
                // Variable goes out of scope at end of using block - consume it
                // (linearity enforcement happens at end of block)
            }
        }
        Ok(())
    }

    fn check_expr(&mut self, expr: &HirExpr) -> Result<(), Vec<LinearityError>> {
        let mut env = self.env.clone();
        let result = self.check_expr_with_env(expr, &mut env);
        self.env = env;
        result
    }

    fn check_expr_with_env(&mut self, expr: &HirExpr, env: &mut LinearityEnv) -> Result<(), Vec<LinearityError>> {
        match expr {
            HirExpr::Literal(_) => Ok(()),
            HirExpr::Ident(name) => {
                // Check variable usage
                if let Err(e) = env.use_variable(name, Span::new(0, 0, 0, 0)) {
                    self.errors.push(e);
                    return Err(self.errors.clone());
                }
                Ok(())
            }
            HirExpr::Call { function, args } => {
                // Check arguments
                for arg in args {
                    self.check_expr_with_env(arg, env)?;
                }
                
                // Check if this is a resource-consuming operation
                match function.as_str() {
                    "consume" | "close" | "drop" => {
                        // These operations consume their arguments
                        for arg in args {
                            if let HirExpr::Ident(name) = arg {
                                if let Err(e) = env.consume_variable(name) {
                                    self.errors.push(e);
                                    return Err(self.errors.clone());
                                }
                            }
                        }
                    }
                    _ => {}
                }
                Ok(())
            }
            HirExpr::Binary { left, op: _, right } => {
                self.check_expr_with_env(left, env)?;
                self.check_expr_with_env(right, env)?;
                Ok(())
            }
            HirExpr::Block(block) => {
                self.check_block(block, env)?;
                Ok(())
            }
            HirExpr::If { condition, then_branch, else_branch } => {
                self.check_expr_with_env(condition, env)?;
                
                let mut then_env = env.clone();
                self.check_block(then_branch, &mut then_env)?;
                
                if let Some(else_expr) = else_branch {
                    let mut else_env = env.clone();
                    self.check_expr_with_env(else_expr, &mut else_env)?;
                    
                    for (name, then_usage) in &then_env.variables {
                        if let Some(else_usage) = else_env.variables.get(name) {
                            if then_usage.linearity == Linearity::Linear || then_usage.linearity == Linearity::Affine {
                                if then_usage.usage_count != else_usage.usage_count {
                                    self.errors.push(LinearityError::BranchUsageMismatch {
                                        name: name.clone(),
                                        span: None,
                                    });
                                }
                            }

                            if let Some(env_usage) = env.variables.get_mut(name) {
                                env_usage.usage_count = then_usage.usage_count.max(else_usage.usage_count);
                            }
                        }
                    }
                } else {
                    for (name, then_usage) in &then_env.variables {
                        if let Some(env_usage) = env.variables.get_mut(name) {
                             if then_usage.linearity == Linearity::Linear {
                                if then_usage.usage_count != env_usage.usage_count {
                                     self.errors.push(LinearityError::BranchUsageMismatch {
                                        name: name.clone(),
                                        span: None,
                                    });
                                }
                            }
                            env_usage.usage_count = env_usage.usage_count.max(then_usage.usage_count);
                        }
                    }
                }
                Ok(())
            }
            HirExpr::Match { expr, arms } => {
                self.check_expr_with_env(expr, env)?;
                
                let mut max_usages = std::collections::HashMap::new();
                let mut first_arm_usage = None;

                for (_, arm_expr) in arms {
                    let mut arm_env = env.clone();
                    self.check_expr_with_env(arm_expr, &mut arm_env)?;
                    
                    if first_arm_usage.is_none() {
                        first_arm_usage = Some(arm_env.variables.clone());
                    } else {
                        // Compare with first arm to ensure consistency for linear variables
                        for (name, arm_usage) in &arm_env.variables {
                            if let Some(first_usage) = first_arm_usage.as_ref().unwrap().get(name) {
                                if arm_usage.linearity == Linearity::Linear || arm_usage.linearity == Linearity::Affine {
                                    if arm_usage.usage_count != first_usage.usage_count {
                                        self.errors.push(LinearityError::BranchUsageMismatch {
                                            name: name.clone(),
                                            span: None,
                                        });
                                    }
                                }
                            }
                        }
                    }

                    for (name, arm_usage) in &arm_env.variables {
                        let current_max = max_usages.get(name).cloned().unwrap_or(0);
                        max_usages.insert(name.clone(), current_max.max(arm_usage.usage_count));
                    }
                }
                
                for (name, max_usage) in max_usages {
                    if let Some(env_usage) = env.variables.get_mut(&name) {
                        env_usage.usage_count = env_usage.usage_count.max(max_usage);
                    }
                }
                
                Ok(())
            }
            HirExpr::For { item, collection, body } => {
                self.check_expr_with_env(collection, env)?;
                
                let mut body_env = env.clone();
                self.check_block(body, &mut body_env)?;
                
                for (name, body_usage) in &body_env.variables {
                    if let Some(env_usage) = env.variables.get_mut(name) {
                        if body_usage.usage_count > env_usage.usage_count && (env_usage.linearity == Linearity::Linear || env_usage.linearity == Linearity::Affine) {
                            self.errors.push(LinearityError::ClosureCaptureViolation(
                                format!("Linear variable '{}' cannot be used inside a for loop", name)
                            ));
                        }
                        env_usage.usage_count = env_usage.usage_count.max(body_usage.usage_count);
                    }
                }
                
                Ok(())
            }
            HirExpr::Index { base, index } => {
                self.check_expr_with_env(base, env)?;
                self.check_expr_with_env(index, env)?;
                Ok(())
            }
            HirExpr::Try(inner) => {
                self.check_expr_with_env(inner, env)
            }
            HirExpr::Struct { name: _, fields } => {
                for (_, val) in fields {
                    self.check_expr_with_env(val, env)?;
                }
                Ok(())
            }
            HirExpr::FieldAccess { base, field: _ } => {
                self.check_expr_with_env(base, env)
            }
        }
    }

    fn check_unconsumed_linear_values(&mut self) -> Result<(), Vec<LinearityError>> {
        for (name, usage) in &self.env.variables {
            match usage.linearity {
                Linearity::Linear => {
                    if usage.usage_count == 0 && usage.first_use.is_none() {
                self.errors.push(LinearityError::LinearValueNotConsumed {
                    name: name.clone(),
                    span: None,
                });
                    }
                }
                Linearity::Affine => {
                }
                Linearity::NonLinear => {
                }
            }
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn analyze_closure_capture(&mut self, closure_id: &str, captured_vars: Vec<String>) -> Result<ClosureCapture, Vec<LinearityError>> {
        let mut linear_captures = Vec::new();
        let mut ownership_transfer = false;
        let mut capture_depth = 0;
        
        for var_name in &captured_vars {
            if let Some(usage_info) = self.env.variables.get(var_name) {
                match usage_info.linearity {
                    Linearity::Linear => {
                        linear_captures.push(var_name.clone());
                        ownership_transfer = true;
                        capture_depth = capture_depth.max(usage_info.usage_count);
                    }
                    Linearity::Affine => {
                        linear_captures.push(var_name.clone());
                        ownership_transfer = true;
                    }
                    Linearity::NonLinear => {
                    }
                }
            }
        }
        
        let is_linear = !linear_captures.is_empty();
        let usage_count = if is_linear { 1 } else { 0 };
        
        let capture = ClosureCapture {
            closure_id: closure_id.to_string(),
            captured_vars,
            linear_captures,
            ownership_transfer,
            is_linear,
            usage_count,
            capture_depth,
        };
        
        self.validate_closure_capture(&capture)?;
        
        Ok(capture)
    }

    fn validate_closure_capture(&self, capture: &ClosureCapture) -> Result<(), Vec<LinearityError>> {
        let mut errors = Vec::new();
        
        for linear_var in &capture.linear_captures {
            if let Some(usage_info) = self.env.variables.get(linear_var) {
                if usage_info.usage_count > 1 {
                    errors.push(LinearityError::ClosureCaptureViolation(
                        format!("Linear variable '{}' used {} times before closure capture", 
                                linear_var, usage_info.usage_count)
                    ));
                }
            }
        }
        
        if capture.is_linear && capture.usage_count > 1 {
            errors.push(LinearityError::ClosureCaptureViolation(
                format!("Linear closure '{}' used {} times", 
                        capture.closure_id, capture.usage_count)
            ));
        }
        
        if capture.capture_depth > 1 && !capture.ownership_transfer {
            errors.push(LinearityError::ClosureCaptureViolation(
                format!("Nested linear closure '{}' must transfer ownership", 
                        capture.closure_id)
            ));
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn check_closure_usage(&mut self, closure_id: &str) -> Result<(), Vec<LinearityError>> {
        if let Some(capture) = self.env.closure_captures.iter().find(|c| c.closure_id == closure_id) {
            if capture.is_linear {
                if capture.usage_count > 0 {
                    return Err(vec![LinearityError::ClosureCaptureViolation(
                        format!("Linear closure '{}' already used", closure_id)
                    )]);
                }
                
                if let Some(capture_mut) = self.env.closure_captures.iter_mut().find(|c| c.closure_id == closure_id) {
                    capture_mut.usage_count += 1;
                }
            }
        }
        
        Ok(())
    }

    pub fn get_closure_capture(&self, closure_id: &str) -> Option<&ClosureCapture> {
        self.env.closure_captures.iter().find(|c| c.closure_id == closure_id)
    }

    pub fn check_unconsumed_captures(&self) -> Result<(), Vec<LinearityError>> {
        let mut errors = Vec::new();
        
        for capture in &self.env.closure_captures {
            if capture.is_linear && capture.usage_count == 0 {
                errors.push(LinearityError::ClosureCaptureViolation(
                    format!("Linear closure '{}' with unconsumed captures", capture.closure_id)
                ));
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[derive(Debug, Clone)]
pub struct LinearityChain {
    pub first_use: Span,
    pub consumption_point: Option<Span>,
    pub violation: Span,
    pub variable: String,
}

impl LinearityChain {
    pub fn new(variable: String, first_use: Span, violation: Span) -> Self {
        Self {
            first_use,
            consumption_point: None,
            violation,
            variable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral};

    #[test]
    fn test_linear_variable_usage() {
        let mut checker = LinearityChecker::new();
        
        let mut env = LinearityEnv::new();
        env.add_variable("x".to_string(), Linearity::Linear);
        env.use_variable("x", Span::new(0, 0, 0, 0)).unwrap();
        env.consume_variable("x").unwrap();
        
        assert_eq!(env.variables.get("x").unwrap().usage_count, 0);
    }

    #[test]
    fn test_linear_variable_reuse() {
        let mut checker = LinearityChecker::new();
        
        let mut env = LinearityEnv::new();
        env.add_variable("x".to_string(), Linearity::Linear);
        env.use_variable("x", Span::new(0, 0, 0, 0)).unwrap();
        let result = env.use_variable("x", Span::new(1, 1, 1, 1));
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LinearityError::LinearValueReused { .. }));
    }
}