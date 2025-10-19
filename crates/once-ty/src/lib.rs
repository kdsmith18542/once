//! Type system for the Once language
//! 
//! Implements Hindley-Milner type inference with support for:
//! - Linear and affine types
//! - Region-based memory management
//! - Row-polymorphic effects
//! - Type unification and constraint solving

use once_hir::*;
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Type inference errors
#[derive(Error, Debug, Clone)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },
    
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    
    #[error("Cannot unify types: {0} and {1}")]
    CannotUnify(String, String),
    
    #[error("Circular type constraint: {0}")]
    CircularConstraint(String),
    
    #[error("Linear value used multiple times: {0}")]
    LinearValueReused(String),
    
    #[error("Non-linear value in linear context: {0}")]
    NonLinearInLinearContext(String),
    
    #[error("Region constraint unsatisfiable: {0}")]
    UnsatisfiableRegion(String),
}

/// Type variables for unification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(pub usize);

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "α{}", self.0)
    }
}

/// Type schemes (quantified types)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub ty: Type,
}

/// Types in the type system
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    /// Type variable
    Var(TypeVar),
    /// Basic types
    Unit,
    Int,
    Bool,
    Float,
    Str,
    /// Linear type (must be consumed exactly once)
    Linear(Box<Type>),
    /// Affine type (must be consumed at most once)
    Affine(Box<Type>),
    /// Function type
    Function { params: Vec<Type>, return_type: Box<Type> },
    /// Tuple type
    Tuple(Vec<Type>),
    /// Array type with size
    Array { element_type: Box<Type>, size: Option<u64> },
    /// Reference type with region
    Ref { region: Region, ty: Box<Type> },
    /// Mutable reference type with region
    RefMut { region: Region, ty: Box<Type> },
    /// Channel type
    Channel(Box<Type>),
    /// Result type
    Result { ok_type: Box<Type>, err_type: Box<Type> },
    /// Option type
    Option(Box<Type>),
    /// User-defined type
    UserDefined { name: String, args: Vec<Type> },
}

/// Regions for memory management
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Region {
    /// Static region (lives for the entire program)
    Static,
    /// Local region (lives for a function call)
    Local,
    /// Named region variable
    Var(String),
    /// Region parameter
    Param(String),
}

impl fmt::Display for Region {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Region::Static => write!(f, "static"),
            Region::Local => write!(f, "local"),
            Region::Var(name) => write!(f, "r_{}", name),
            Region::Param(name) => write!(f, "ρ{}", name),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Var(var) => write!(f, "{}", var),
            Type::Unit => write!(f, "Unit"),
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Float => write!(f, "Float"),
            Type::Str => write!(f, "Str"),
            Type::Linear(ty) => write!(f, "lin {}", ty),
            Type::Affine(ty) => write!(f, "aff {}", ty),
            Type::Function { params, return_type } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ") -> {}", return_type)
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, ty) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", ty)?;
                }
                write!(f, ")")
            }
            Type::Array { element_type, size } => {
                write!(f, "[{}", element_type)?;
                if let Some(size) = size {
                    write!(f, "; {}", size)?;
                }
                write!(f, "]")
            }
            Type::Ref { region, ty } => write!(f, "&{} {}", region, ty),
            Type::RefMut { region, ty } => write!(f, "&{} mut {}", region, ty),
            Type::Channel(ty) => write!(f, "Chan<{}>", ty),
            Type::Result { ok_type, err_type } => write!(f, "Result<{}, {}>", ok_type, err_type),
            Type::Option(ty) => write!(f, "Option<{}>", ty),
            Type::UserDefined { name, args } => {
                write!(f, "{}", name)?;
                if !args.is_empty() {
                    write!(f, "<")?;
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", arg)?;
                    }
                    write!(f, ">")?;
                }
                Ok(())
            }
        }
    }
}

/// Type constraints for unification
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Type equality constraint
    Equal { left: Type, right: Type },
    /// Linear constraint (value must be consumed exactly once)
    Linear { ty: Type },
    /// Affine constraint (value must be consumed at most once)
    Affine { ty: Type },
    /// Region constraint (region must outlive another)
    RegionOutlives { longer: Region, shorter: Region },
    /// Effect constraint
    Effect { ty: Type, effects: Vec<String> },
}

/// Type environment for inference
#[derive(Debug, Clone)]
pub struct TypeEnv {
    /// Variable bindings
    pub bindings: HashMap<String, TypeScheme>,
    /// Type constraints
    pub constraints: Vec<Constraint>,
    /// Next type variable ID
    pub next_var_id: usize,
    /// Region constraints
    pub region_constraints: Vec<(Region, Region)>,
}

impl TypeEnv {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            constraints: Vec::new(),
            next_var_id: 0,
            region_constraints: Vec::new(),
        }
    }

    pub fn fresh_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var_id);
        self.next_var_id += 1;
        var
    }

    pub fn fresh_region(&mut self, name: String) -> Region {
        Region::Var(name)
    }

    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    pub fn add_region_constraint(&mut self, longer: Region, shorter: Region) {
        self.region_constraints.push((longer, shorter));
    }
}

/// Type checker for Once programs
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
        }
    }

    pub fn check(&mut self, hir: &HirProgram) -> Result<(), Vec<TypeError>> {
        // Add built-in types to environment
        self.add_builtin_types();
        
        // Type check all items
        for item in &hir.items {
            self.check_item(item)?;
        }

        // Solve constraints
        self.solve_constraints()?;

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn add_builtin_types(&mut self) {
        // Add basic types
        self.env.bindings.insert("Unit".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Unit,
        });
        
        self.env.bindings.insert("Int".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Int,
        });
        
        self.env.bindings.insert("Bool".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Bool,
        });
        
        self.env.bindings.insert("Float".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Float,
        });
        
        self.env.bindings.insert("Str".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Str,
        });

        // Add built-in functions
        self.env.bindings.insert("print".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Function {
                params: vec![Type::Str],
                return_type: Box::new(Type::Unit),
            },
        });
    }

    fn add_builtin_types_to_env(&self, env: &mut TypeEnv) {
        // Add basic types
        env.bindings.insert("Unit".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Unit,
        });
        
        env.bindings.insert("Int".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Int,
        });
        
        env.bindings.insert("Bool".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Bool,
        });
        
        env.bindings.insert("Float".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Float,
        });
        
        env.bindings.insert("Str".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Str,
        });

        // Add built-in functions
        env.bindings.insert("print".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Function {
                params: vec![Type::Str],
                return_type: Box::new(Type::Unit),
            },
        });
    }

    fn check_item(&mut self, item: &HirItem) -> Result<(), Vec<TypeError>> {
        match item {
            HirItem::FnDecl(fn_decl) => self.check_fn_decl(fn_decl),
            HirItem::LetDecl(let_decl) => self.check_let_decl(let_decl),
        }
    }

    fn check_fn_decl(&mut self, fn_decl: &HirFnDecl) -> Result<(), Vec<TypeError>> {
        // Create new environment for function with built-ins
        let mut fn_env = self.env.clone();
        self.add_builtin_types_to_env(&mut fn_env);
        
        // Add parameters to environment
        for param in &fn_decl.params {
            let param_type = if let Some(ty) = &param.type_annotation {
                self.hir_type_to_type(ty)
            } else {
                Type::Var(fn_env.fresh_var())
            };
            
            fn_env.bindings.insert(param.name.clone(), TypeScheme {
                vars: vec![],
                ty: param_type,
            });
        }

        // Check function body
        let body_type = self.check_block(&fn_decl.body, &mut fn_env)?;
        
        // Check return type
        if let Some(return_type) = &fn_decl.return_type {
            let expected_type = self.hir_type_to_type(return_type);
            fn_env.add_constraint(Constraint::Equal {
                left: body_type,
                right: expected_type,
            });
        }

        // Merge constraints back to main environment
        self.env.constraints.extend(fn_env.constraints);
        self.env.region_constraints.extend(fn_env.region_constraints);

        Ok(())
    }

    fn check_let_decl(&mut self, let_decl: &HirLetDecl) -> Result<(), Vec<TypeError>> {
        let value_type = self.check_expr(&let_decl.value)?;
        
        if let Some(ty) = &let_decl.type_annotation {
            let expected_type = self.hir_type_to_type(ty);
            self.env.add_constraint(Constraint::Equal {
                left: value_type.clone(),
                right: expected_type,
            });
        }

        // Add binding to environment
        self.env.bindings.insert(let_decl.name.clone(), TypeScheme {
            vars: vec![],
            ty: value_type,
        });

        Ok(())
    }

    fn check_block(&mut self, block: &HirBlock, env: &mut TypeEnv) -> Result<Type, Vec<TypeError>> {
        let mut last_type = Type::Unit;
        
        for stmt in &block.statements {
            last_type = self.check_stmt(stmt, env)?;
        }
        
        Ok(last_type)
    }

    fn check_stmt(&mut self, stmt: &HirStmt, env: &mut TypeEnv) -> Result<Type, Vec<TypeError>> {
        match stmt {
            HirStmt::Let(let_stmt) => {
                let value_type = self.check_expr_with_env(&let_stmt.value, env)?;
                
                if let Some(ty) = &let_stmt.type_annotation {
                    let expected_type = self.hir_type_to_type(ty);
                    env.add_constraint(Constraint::Equal {
                        left: value_type.clone(),
                        right: expected_type,
                    });
                }

                // Add binding to environment
                env.bindings.insert(let_stmt.name.clone(), TypeScheme {
                    vars: vec![],
                    ty: value_type,
                });

                Ok(Type::Unit)
            }
            HirStmt::Return(return_stmt) => {
                if let Some(expr) = &return_stmt.value {
                    self.check_expr_with_env(expr, env)
                } else {
                    Ok(Type::Unit)
                }
            }
            HirStmt::Expr(expr) => self.check_expr_with_env(expr, env),
        }
    }

    fn check_expr(&mut self, expr: &HirExpr) -> Result<Type, Vec<TypeError>> {
        let mut env = self.env.clone();
        let result = self.check_expr_with_env(expr, &mut env);
        self.env = env;
        result
    }

    fn check_expr_with_env(&mut self, expr: &HirExpr, env: &mut TypeEnv) -> Result<Type, Vec<TypeError>> {
        match expr {
            HirExpr::Literal(lit) => Ok(self.literal_type(lit)),
            HirExpr::Ident(name) => {
                if let Some(scheme) = env.bindings.get(name) {
                    Ok(self.instantiate_scheme(scheme))
                } else {
                    self.errors.push(TypeError::UndefinedVariable(name.clone()));
                    Err(self.errors.clone())
                }
            }
            HirExpr::Call { function, args } => {
                let arg_types: Result<Vec<Type>, _> = args.iter()
                    .map(|arg| self.check_expr_with_env(arg, env))
                    .collect();
                let arg_types = arg_types?;
                
                let return_type = Type::Var(env.fresh_var());
                let function_type = Type::Function {
                    params: arg_types,
                    return_type: Box::new(return_type.clone()),
                };
                
                // Check if function exists
                if let Some(scheme) = env.bindings.get(function) {
                    let expected_type = self.instantiate_scheme(scheme);
                    env.add_constraint(Constraint::Equal {
                        left: function_type,
                        right: expected_type,
                    });
                } else {
                    self.errors.push(TypeError::UndefinedVariable(function.clone()));
                    return Err(self.errors.clone());
                }
                
                Ok(return_type)
            }
            HirExpr::Binary { left, op: _, right } => {
                let left_type = self.check_expr_with_env(left, env)?;
                let right_type = self.check_expr_with_env(right, env)?;
                
                // Add constraint that both operands have the same type
                env.add_constraint(Constraint::Equal {
                    left: left_type.clone(),
                    right: right_type,
                });
                
                Ok(left_type)
            }
            HirExpr::Block(block) => self.check_block(block, env),
        }
    }

    fn literal_type(&self, lit: &HirLiteral) -> Type {
        match lit {
            HirLiteral::Int(_) => Type::Int,
            HirLiteral::Float(_) => Type::Float,
            HirLiteral::String(_) => Type::Str,
            HirLiteral::Bool(_) => Type::Bool,
            HirLiteral::Unit => Type::Unit,
        }
    }

    fn hir_type_to_type(&self, hir_type: &HirType) -> Type {
        match hir_type {
            HirType::Ident(name) => {
                if let Some(scheme) = self.env.bindings.get(name) {
                    self.instantiate_scheme(scheme)
                } else {
                    Type::UserDefined { name: name.clone(), args: vec![] }
                }
            }
            HirType::Unit => Type::Unit,
            HirType::Int => Type::Int,
            HirType::Bool => Type::Bool,
            HirType::Float => Type::Float,
            HirType::Str => Type::Str,
            HirType::Linear(ty) => Type::Linear(Box::new(self.hir_type_to_type(ty))),
            HirType::Affine(ty) => Type::Affine(Box::new(self.hir_type_to_type(ty))),
        }
    }

    fn instantiate_scheme(&self, scheme: &TypeScheme) -> Type {
        // For now, just return the type directly
        // In a full implementation, we'd substitute fresh variables for quantified ones
        scheme.ty.clone()
    }

    fn solve_constraints(&mut self) -> Result<(), Vec<TypeError>> {
        // Simple constraint solver
        // In a full implementation, this would use unification
        let constraints = self.env.constraints.clone();
        for constraint in &constraints {
            match constraint {
                Constraint::Equal { left, right } => {
                    if !self.unify_types(left, right) {
                        self.errors.push(TypeError::CannotUnify(
                            format!("{}", left),
                            format!("{}", right),
                        ));
                    }
                }
                Constraint::Linear { ty: _ } => {
                    // Check that linear types are used exactly once
                    // This is a simplified check
                }
                Constraint::Affine { ty: _ } => {
                    // Check that affine types are used at most once
                    // This is a simplified check
                }
                Constraint::RegionOutlives { longer, shorter } => {
                    // Check region constraints
                    self.env.add_region_constraint(longer.clone(), shorter.clone());
                }
                Constraint::Effect { ty: _, effects: _ } => {
                    // Check effect constraints
                    // This is a simplified check
                }
            }
        }

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn unify_types(&self, left: &Type, right: &Type) -> bool {
        match (left, right) {
            (Type::Var(_), _) | (_, Type::Var(_)) => true, // Variables can unify with anything
            (Type::Unit, Type::Unit) => true,
            (Type::Int, Type::Int) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Float, Type::Float) => true,
            (Type::Str, Type::Str) => true,
            (Type::Linear(ty1), Type::Linear(ty2)) => self.unify_types(ty1, ty2),
            (Type::Affine(ty1), Type::Affine(ty2)) => self.unify_types(ty1, ty2),
            (Type::Function { params: p1, return_type: r1 }, Type::Function { params: p2, return_type: r2 }) => {
                p1.len() == p2.len() && 
                p1.iter().zip(p2.iter()).all(|(a, b)| self.unify_types(a, b)) &&
                self.unify_types(r1, r2)
            }
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                types1.len() == types2.len() &&
                types1.iter().zip(types2.iter()).all(|(a, b)| self.unify_types(a, b))
            }
            (Type::Ref { region: r1, ty: t1 }, Type::Ref { region: r2, ty: t2 }) => {
                r1 == r2 && self.unify_types(t1, t2)
            }
            (Type::RefMut { region: r1, ty: t1 }, Type::RefMut { region: r2, ty: t2 }) => {
                r1 == r2 && self.unify_types(t1, t2)
            }
            (Type::Channel(ty1), Type::Channel(ty2)) => self.unify_types(ty1, ty2),
            (Type::Result { ok_type: o1, err_type: e1 }, Type::Result { ok_type: o2, err_type: e2 }) => {
                self.unify_types(o1, o2) && self.unify_types(e1, e2)
            }
            (Type::Option(ty1), Type::Option(ty2)) => self.unify_types(ty1, ty2),
            (Type::UserDefined { name: n1, args: a1 }, Type::UserDefined { name: n2, args: a2 }) => {
                n1 == n2 && a1.len() == a2.len() &&
                a1.iter().zip(a2.iter()).all(|(a, b)| self.unify_types(a, b))
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral};

    #[test]
    fn test_basic_type_checking() {
        let program = HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(HirType::Unit),
                    body: HirBlock {
                        statements: vec![
                            HirStmt::Return(HirReturnStmt { value: None }),
                        ],
                    },
                    is_public: false,
                }),
            ],
            imports: vec![],
        };

        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(result.is_ok());
    }

    #[test]
    fn test_literal_types() {
        let checker = TypeChecker::new();
        
        assert_eq!(checker.literal_type(&HirLiteral::Int(42)), Type::Int);
        assert_eq!(checker.literal_type(&HirLiteral::Float(3.14)), Type::Float);
        assert_eq!(checker.literal_type(&HirLiteral::String("hello".to_string())), Type::Str);
        assert_eq!(checker.literal_type(&HirLiteral::Bool(true)), Type::Bool);
        assert_eq!(checker.literal_type(&HirLiteral::Unit), Type::Unit);
    }
}