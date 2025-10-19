//! Linearity checking for the Once language
//! 
//! Implements move/consume analysis for:
//! - Linear type checking
//! - Resource safety
//! - Copy trait constraints
//! - Closure capture rules

use once_hir::*;
use once_ty::{Type, TypeVar};
use once_lex::Span;
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Linearity checking errors
#[derive(Error, Debug, Clone)]
pub enum LinearityError {
    #[error("Linear value used multiple times: {0}")]
    LinearValueReused(String),
    
    #[error("Non-linear value in linear context: {0}")]
    NonLinearInLinearContext(String),
    
    #[error("Linear value not consumed: {0}")]
    LinearValueNotConsumed(String),
    
    #[error("Copy constraint violated: {0}")]
    CopyConstraintViolated(String),
    
    #[error("Resource not properly consumed: {0}")]
    ResourceNotConsumed(String),
    
    #[error("Closure capture violation: {0}")]
    ClosureCaptureViolation(String),
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
                        return Err(LinearityError::LinearValueReused(name.to_string()));
                    }
                }
                Linearity::Affine => {
                    if usage.usage_count > 1 {
                        return Err(LinearityError::LinearValueReused(name.to_string()));
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

        // Merge back to main environment
        self.env.variables.extend(fn_env.variables);
        self.env.copy_constraints.extend(fn_env.copy_constraints);
        self.env.resource_traits.extend(fn_env.resource_traits);

        Ok(())
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
        }
    }

    fn check_unconsumed_linear_values(&mut self) -> Result<(), Vec<LinearityError>> {
        for (name, usage) in &self.env.variables {
            match usage.linearity {
                Linearity::Linear => {
                    if usage.usage_count > 0 {
                        self.errors.push(LinearityError::LinearValueNotConsumed(name.clone()));
                    }
                }
                Linearity::Affine => {
                    // Affine values can be unconsumed
                }
                Linearity::NonLinear => {
                    // Non-linear values don't need consumption
                }
            }
        }
        
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Analyze closure capture
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
                        // Affine types can be captured but must be consumed
                        linear_captures.push(var_name.clone());
                        ownership_transfer = true;
                    }
                    Linearity::NonLinear => {
                        // Non-linear types can be captured freely
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
        
        // Validate closure capture rules
        self.validate_closure_capture(&capture)?;
        
        Ok(capture)
    }

    /// Validate closure capture rules
    fn validate_closure_capture(&self, capture: &ClosureCapture) -> Result<(), Vec<LinearityError>> {
        let mut errors = Vec::new();
        
        // Rule 1: Linear values must be moved into the closure
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
        
        // Rule 2: Linear closures can only be called once
        if capture.is_linear && capture.usage_count > 1 {
            errors.push(LinearityError::ClosureCaptureViolation(
                format!("Linear closure '{}' used {} times", 
                        capture.closure_id, capture.usage_count)
            ));
        }
        
        // Rule 3: Nested linear closures must transfer ownership
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

    /// Check closure usage
    pub fn check_closure_usage(&mut self, closure_id: &str) -> Result<(), Vec<LinearityError>> {
        if let Some(capture) = self.env.closure_captures.iter().find(|c| c.closure_id == closure_id) {
            if capture.is_linear {
                // Linear closures can only be used once
                if capture.usage_count > 0 {
                    return Err(vec![LinearityError::ClosureCaptureViolation(
                        format!("Linear closure '{}' already used", closure_id)
                    )]);
                }
                
                // Mark as used
                if let Some(capture_mut) = self.env.closure_captures.iter_mut().find(|c| c.closure_id == closure_id) {
                    capture_mut.usage_count += 1;
                }
            }
        }
        
        Ok(())
    }

    /// Get closure capture information
    pub fn get_closure_capture(&self, closure_id: &str) -> Option<&ClosureCapture> {
        self.env.closure_captures.iter().find(|c| c.closure_id == closure_id)
    }

    /// Check for unconsumed linear captures
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

/// Linearity chain for debugging
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
        
        // Test linear variable that's used once
        let mut env = LinearityEnv::new();
        env.add_variable("x".to_string(), Linearity::Linear);
        env.use_variable("x", Span::new(0, 0, 0, 0)).unwrap();
        env.consume_variable("x").unwrap();
        
        assert_eq!(env.variables.get("x").unwrap().usage_count, 0);
    }

    #[test]
    fn test_linear_variable_reuse() {
        let mut checker = LinearityChecker::new();
        
        // Test linear variable that's used twice (should fail)
        let mut env = LinearityEnv::new();
        env.add_variable("x".to_string(), Linearity::Linear);
        env.use_variable("x", Span::new(0, 0, 0, 0)).unwrap();
        let result = env.use_variable("x", Span::new(1, 1, 1, 1));
        
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LinearityError::LinearValueReused(_)));
    }
}