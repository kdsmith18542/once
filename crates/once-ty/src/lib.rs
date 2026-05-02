//! Type system for the Once language
//! 
//! Implements Hindley-Milner type inference with support for:
//! - Linear and affine types
//! - Region-based memory management
//! - Row-polymorphic effects
//! - Type unification and constraint solving

pub mod effects;

use once_hir::*;
use once_linear::{LinearityChecker, LinearityError};
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

impl From<once_lex::Span> for SourceSpan {
    fn from(s: once_lex::Span) -> Self {
        Self { start: s.start, end: s.end, line: s.line, column: s.column }
    }
}

/// Type inference errors with optional source location
#[derive(Error, Debug, Clone)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String, span: Option<SourceSpan> },

    #[error("Undefined variable: {name}")]
    UndefinedVariable { name: String, span: Option<SourceSpan> },

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

    #[error("Trait bound not satisfied: {type_name} does not implement {trait_name}")]
    TraitBoundNotSatisfied { type_name: String, trait_name: String, span: Option<SourceSpan> },

    #[error("Effect error: {0}")]
    Effect(String),
}

impl TypeError {
    /// Get the source span associated with this error, if any
    pub fn span(&self) -> Option<SourceSpan> {
        match self {
            TypeError::TypeMismatch { span, .. } => *span,
            TypeError::UndefinedVariable { span, .. } => *span,
            TypeError::TraitBoundNotSatisfied { span, .. } => *span,
            TypeError::LinearValueReused(_) => None, // TODO: add span to this variant
            TypeError::NonLinearInLinearContext(_) => None,
            TypeError::Effect(_) => None,
            _ => None,
        }
    }

    /// Format a full diagnostic message with source location
    pub fn diagnostic(&self) -> String {
        match self.span() {
            Some(span) => format!("{} at {}", self, span),
            None => self.to_string(),
        }
    }
}

impl From<LinearityError> for TypeError {
    fn from(e: LinearityError) -> Self {
        match e {
            LinearityError::LinearValueReused { name, .. } => TypeError::LinearValueReused(name),
            LinearityError::NonLinearInLinearContext { name, .. } => TypeError::NonLinearInLinearContext(name),
            LinearityError::LinearValueNotConsumed { name, .. } => TypeError::LinearValueReused(format!("{} (not consumed)", name)),
            _ => TypeError::LinearValueReused(e.to_string()),
        }
    }
}

impl From<effects::EffectError> for TypeError {
    fn from(e: effects::EffectError) -> Self {
        match e {
            effects::EffectError::UnhandledEffect { name, .. } => TypeError::Effect(name),
            _ => TypeError::Effect(e.to_string()),
        }
    }
}

/// Type variables for unification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeVar(pub usize);

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "α{}", self.0)
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
    /// Trait constraint (type must implement trait)
    Trait { ty: Type, trait_name: String, args: Vec<Type> },
}

impl Constraint {
    pub fn apply_subst(&self, subst: &Substitution) -> Self {
        match self {
            Constraint::Equal { left, right } => Constraint::Equal {
                left: left.apply_subst(subst),
                right: right.apply_subst(subst),
            },
            Constraint::Linear { ty } => Constraint::Linear { ty: ty.apply_subst(subst) },
            Constraint::Affine { ty } => Constraint::Affine { ty: ty.apply_subst(subst) },
            Constraint::RegionOutlives { longer, shorter } => Constraint::RegionOutlives {
                longer: longer.clone(),
                shorter: shorter.clone(),
            },
            Constraint::Effect { ty, effects } => Constraint::Effect {
                ty: ty.apply_subst(subst),
                effects: effects.clone(),
            },
            Constraint::Trait { ty, trait_name, args } => Constraint::Trait {
                ty: ty.apply_subst(subst),
                trait_name: trait_name.clone(),
                args: args.iter().map(|a| a.apply_subst(subst)).collect(),
            },
        }
    }

    pub fn free_vars(&self) -> HashSet<TypeVar> {
        match self {
            Constraint::Equal { left, right } => {
                let mut vars = left.free_vars();
                vars.extend(right.free_vars());
                vars
            }
            Constraint::Linear { ty } => ty.free_vars(),
            Constraint::Affine { ty } => ty.free_vars(),
            Constraint::RegionOutlives { .. } => HashSet::new(),
            Constraint::Effect { ty, .. } => ty.free_vars(),
            Constraint::Trait { ty, args, .. } => {
                let mut vars = ty.free_vars();
                for arg in args {
                    vars.extend(arg.free_vars());
                }
                vars
            }
        }
    }
}

/// Type schemes (quantified types)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeScheme {
    pub vars: Vec<TypeVar>,
    pub ty: Type,
    pub constraints: Vec<Constraint>,
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


/// Substitution mapping from type variables to types
#[derive(Debug, Clone, PartialEq)]
pub struct Substitution {
    pub mapping: HashMap<TypeVar, Type>,
}

impl Substitution {
    pub fn empty() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }

    pub fn single(var: TypeVar, ty: Type) -> Self {
        let mut mapping = HashMap::new();
        mapping.insert(var, ty);
        Self { mapping }
    }

    /// Compose two substitutions: self after other
    /// (self ∘ other)(x) = self(other(x))
    pub fn compose(self, other: Self) -> Self {
        let mut mapping = other.mapping.clone();
        for (var, ty) in self.mapping {
            mapping.insert(var, ty.apply_subst(&other));
        }
        Self { mapping }
    }
}

impl Type {
    /// Apply a substitution to this type
    pub fn apply_subst(&self, subst: &Substitution) -> Type {
        match self {
            Type::Var(var) => {
                if let Some(ty) = subst.mapping.get(var) {
                    ty.apply_subst(subst)
                } else {
                    Type::Var(var.clone())
                }
            }
            Type::Linear(ty) => Type::Linear(Box::new(ty.apply_subst(subst))),
            Type::Affine(ty) => Type::Affine(Box::new(ty.apply_subst(subst))),
            Type::Function { params, return_type } => Type::Function {
                params: params.iter().map(|p| p.apply_subst(subst)).collect(),
                return_type: Box::new(return_type.apply_subst(subst)),
            },
            Type::Tuple(types) => Type::Tuple(types.iter().map(|t| t.apply_subst(subst)).collect()),
            Type::Array { element_type, size } => Type::Array {
                element_type: Box::new(element_type.apply_subst(subst)),
                size: *size,
            },
            Type::Ref { region, ty } => Type::Ref {
                region: region.clone(),
                ty: Box::new(ty.apply_subst(subst)),
            },
            Type::RefMut { region, ty } => Type::RefMut {
                region: region.clone(),
                ty: Box::new(ty.apply_subst(subst)),
            },
            Type::Channel(ty) => Type::Channel(Box::new(ty.apply_subst(subst))),
            Type::Result { ok_type, err_type } => Type::Result {
                ok_type: Box::new(ok_type.apply_subst(subst)),
                err_type: Box::new(err_type.apply_subst(subst)),
            },
            Type::Option(ty) => Type::Option(Box::new(ty.apply_subst(subst))),
            Type::UserDefined { name, args } => Type::UserDefined {
                name: name.clone(),
                args: args.iter().map(|a| a.apply_subst(subst)).collect(),
            },
            // Base types don't contain variables
            _ => self.clone(),
        }
    }

    /// Compute free type variables in this type
    pub fn free_vars(&self) -> HashSet<TypeVar> {
        let mut vars = HashSet::new();
        self.free_vars_into(&mut vars);
        vars
    }

    fn free_vars_into(&self, vars: &mut HashSet<TypeVar>) {
        match self {
            Type::Var(var) => { vars.insert(var.clone()); }
            Type::Linear(ty) | Type::Affine(ty) => ty.free_vars_into(vars),
            Type::Function { params, return_type } => {
                for p in params { p.free_vars_into(vars); }
                return_type.free_vars_into(vars);
            }
            Type::Tuple(types) => {
                for t in types { t.free_vars_into(vars); }
            }
            Type::Array { element_type, .. } => element_type.free_vars_into(vars),
            Type::Ref { ty, .. } | Type::RefMut { ty, .. } => ty.free_vars_into(vars),
            Type::Channel(ty) => ty.free_vars_into(vars),
            Type::Result { ok_type, err_type } => {
                ok_type.free_vars_into(vars);
                err_type.free_vars_into(vars);
            }
            Type::Option(ty) => ty.free_vars_into(vars),
            Type::UserDefined { args, .. } => {
                for a in args { a.free_vars_into(vars); }
            }
            _ => {}
        }
    }
}

impl TypeScheme {
    /// Apply a substitution to this scheme (avoiding bound variables)
    pub fn apply_subst(&self, subst: &Substitution) -> TypeScheme {
        let mut filtered = subst.mapping.clone();
        for var in &self.vars {
            filtered.remove(var);
        }
        TypeScheme {
            vars: self.vars.clone(),
            ty: self.ty.apply_subst(&Substitution { mapping: filtered.clone() }),
            constraints: self.constraints.iter().map(|c| c.apply_subst(&Substitution { mapping: filtered.clone() })).collect(),
        }
    }

    /// Free variables in the scheme (not bound by vars)
    pub fn free_vars(&self) -> HashSet<TypeVar> {
        let mut vars = self.ty.free_vars();
        for constraint in &self.constraints {
            vars.extend(constraint.free_vars());
        }
        for var in &self.vars {
            vars.remove(var);
        }
        vars
    }
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

/// Trait implementation record
#[derive(Debug, Clone)]
pub struct TraitImpl {
    pub trait_name: String,
    pub type_name: String,
    pub methods: Vec<(String, TypeScheme)>,
}

/// Trait definition
#[derive(Debug, Clone)]
pub struct TraitDef {
    pub name: String,
    pub type_params: Vec<String>,
    pub methods: Vec<(String, TypeScheme)>,
}

/// Type checker for Once programs
pub struct TypeChecker {
    env: TypeEnv,
    errors: Vec<TypeError>,
    /// Registered trait definitions
    pub traits: HashMap<String, TraitDef>,
    /// Registered trait implementations
    pub trait_impls: Vec<TraitImpl>,
    /// Current source span for error reporting
    current_span: Option<SourceSpan>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            errors: Vec::new(),
            traits: HashMap::new(),
            trait_impls: Vec::new(),
            current_span: None,
        }
    }

    fn with_span<T>(&mut self, span: Option<SourceSpan>, f: impl FnOnce(&mut Self) -> T) -> T {
        let old = self.current_span;
        self.current_span = span;
        let result = f(self);
        self.current_span = old;
        result
    }

    fn push_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    /// Register a trait definition
    pub fn register_trait(&mut self, trait_def: TraitDef) {
        self.traits.insert(trait_def.name.clone(), trait_def);
    }

    /// Register a trait implementation
    pub fn register_trait_impl(&mut self, impl_: TraitImpl) {
        self.trait_impls.push(impl_);
    }

    /// Look up whether a type implements a trait
    pub fn resolve_trait(&self, trait_name: &str, ty: &Type) -> Option<&TraitImpl> {
        let type_name = match ty {
            Type::UserDefined { name, .. } => name.as_str(),
            Type::Int => "Int",
            Type::Bool => "Bool",
            Type::Float => "Float",
            Type::Str => "Str",
            Type::Unit => "Unit",
            _ => return None,
        };
        self.trait_impls.iter()
            .find(|imp| imp.trait_name == trait_name && imp.type_name == type_name)
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

        // Run linearity checker
        let mut linear_checker = LinearityChecker::new();
        if let Err(linear_errors) = linear_checker.check(hir) {
            for e in linear_errors {
                self.errors.push(TypeError::from(e));
            }
        }

        // Run effect checker
        let mut effect_checker = effects::EffectChecker::new();
        if let Err(effect_errors) = effect_checker.check(hir) {
            for e in effect_errors {
                self.errors.push(TypeError::from(e));
            }
        }

        // Generalize top-level let bindings
        let top_level_names: Vec<String> = self.env.bindings.keys()
            .filter(|k| !matches!(k.as_str(), "Unit" | "Int" | "Bool" | "Float" | "Str" | "print"))
            .cloned()
            .collect();
        for name in top_level_names {
            if let Some(scheme) = self.env.bindings.get(&name) {
                let generalized = self.generalize(&self.env.clone(), &scheme.ty);
                self.env.bindings.insert(name, generalized);
            }
        }

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
            constraints: vec![],
        });
        
        self.env.bindings.insert("Int".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Int,
            constraints: vec![],
        });
        
        self.env.bindings.insert("Bool".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Bool,
            constraints: vec![],
        });
        
        self.env.bindings.insert("Float".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Float,
            constraints: vec![],
        });
        
        self.env.bindings.insert("Str".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Str,
            constraints: vec![],
        });

        // Add built-in functions
        self.env.bindings.insert("print".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Function {
                params: vec![Type::Str],
                return_type: Box::new(Type::Unit),
            },
            constraints: vec![],
        });
    }

    fn add_builtin_types_to_env(&self, env: &mut TypeEnv) {
        // Add basic types
        env.bindings.insert("Unit".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Unit,
            constraints: vec![],
        });
        
        env.bindings.insert("Int".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Int,
            constraints: vec![],
        });
        
        env.bindings.insert("Bool".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Bool,
            constraints: vec![],
        });
        
        env.bindings.insert("Float".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Float,
            constraints: vec![],
        });
        
        env.bindings.insert("Str".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Str,
            constraints: vec![],
        });

        // Add built-in functions
        env.bindings.insert("print".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Function {
                params: vec![Type::Str],
                return_type: Box::new(Type::Unit),
            },
            constraints: vec![],
        });

        // Add spawn built-in
        env.bindings.insert("spawn".to_string(), TypeScheme {
            vars: vec![],
            ty: Type::Function {
                params: vec![],
                return_type: Box::new(Type::Unit),
            },
            constraints: vec![],
        });
    }

    fn check_item(&mut self, item: &HirItem) -> Result<(), Vec<TypeError>> {
        match item {
            HirItem::FnDecl(fn_decl) => self.check_fn_decl(fn_decl),
            HirItem::LetDecl(let_decl) => self.check_let_decl(let_decl),
            HirItem::TypeDecl(_) => Ok(()), // Type declarations are checked separately
            HirItem::StructDecl(_) => Ok(()), // Struct declarations are checked separately
            HirItem::TraitDecl(trait_decl) => self.check_trait_decl(trait_decl),
            HirItem::ImplBlock(impl_block) => self.check_impl_block(impl_block),
        }
    }

    fn check_trait_decl(&mut self, trait_decl: &HirTraitDecl) -> Result<(), Vec<TypeError>> {
        let trait_def = TraitDef {
            name: trait_decl.name.clone(),
            type_params: trait_decl.type_params.iter().map(|p| p.name.clone()).collect(),
            methods: trait_decl.methods.iter().map(|m| {
                (m.name.clone(), TypeScheme {
                    vars: vec![], // TODO: Handle type parameters properly
                    ty: self.hir_type_to_type(&m.return_type.clone().unwrap_or(HirType::Unit)),
                    constraints: vec![],
                })
            }).collect(),
        };
        self.register_trait(trait_def);
        Ok(())
    }

    fn check_impl_block(&mut self, impl_block: &HirImplBlock) -> Result<(), Vec<TypeError>> {
        if let Some(trait_name) = &impl_block.trait_name {
            let target_type = self.hir_type_to_type(&impl_block.target_type);
            let type_name = match target_type {
                Type::UserDefined { name, .. } => name,
                Type::Int => "Int".to_string(),
                Type::Bool => "Bool".to_string(),
                Type::Float => "Float".to_string(),
                Type::Str => "Str".to_string(),
                Type::Unit => "Unit".to_string(),
                _ => "Unknown".to_string(),
            };
            
            let trait_impl = TraitImpl {
                trait_name: trait_name.clone(),
                type_name,
                methods: impl_block.methods.iter().map(|m| {
                    (m.name.clone(), TypeScheme {
                        vars: vec![],
                        ty: self.hir_type_to_type(&m.return_type.clone().unwrap_or(HirType::Unit)),
                        constraints: vec![],
                    })
                }).collect(),
            };
            self.register_trait_impl(trait_impl);
        }
        Ok(())
    }

    fn check_fn_decl(&mut self, fn_decl: &HirFnDecl) -> Result<(), Vec<TypeError>> {
        // Create new environment for function with built-ins
        let mut fn_env = self.env.clone();
        self.add_builtin_types_to_env(&mut fn_env);
        
        // Handle type parameters
        for param in &fn_decl.type_params {
            let var = fn_env.fresh_var();
            let mut bounds = Vec::new();
            for bound in &param.bounds {
                let bound_type = self.hir_type_to_type(bound);
                if let Type::UserDefined { name, .. } = bound_type {
                    bounds.push(Constraint::Trait {
                        ty: Type::Var(var.clone()),
                        trait_name: name,
                        args: vec![],
                    });
                }
            }
            fn_env.bindings.insert(param.name.clone(), TypeScheme {
                vars: vec![], // Fixed within function body
                ty: Type::Var(var),
                constraints: bounds,
            });
        }

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
                constraints: vec![],
            });
        }

        // Check body
        let body_type = self.check_block(&fn_decl.body, &mut fn_env)?;
        
        // Check return type
        if let Some(ret_ty) = &fn_decl.return_type {
            let expected_ret = self.hir_type_to_type(ret_ty);
            fn_env.add_constraint(Constraint::Equal {
                left: body_type,
                right: expected_ret,
            });
        }
        
        // Add all collected constraints to main checker to be solved
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

        // Store type without generalizing; generalization happens after solving
        self.env.bindings.insert(let_decl.name.clone(), TypeScheme {
            vars: vec![],
            ty: value_type,
            constraints: vec![],
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

                // Generalize local let bindings for HM polymorphism
                let scheme = self.generalize(env, &value_type);
                env.bindings.insert(let_stmt.name.clone(), scheme);

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
            HirStmt::Continue | HirStmt::Break => Ok(Type::Unit),
            HirStmt::Using(using_stmt) => {
                // Check the init expression
                let init_type = self.check_expr_with_env(&using_stmt.init, env)?;
                
                // Add binding to environment for the body (mark as linear)
                // Linear types are represented with Linear wrapper in our type system
                let linear_type = Type::Linear(Box::new(init_type.clone()));
                env.bindings.insert(using_stmt.name.clone(), TypeScheme {
                    vars: vec![],
                    ty: linear_type,
                    constraints: vec![],
                });
                
                // Check body statements (all should return Unit)
                for stmt in &using_stmt.body.statements {
                    self.check_stmt(stmt, env)?;
                }
                
                Ok(Type::Unit)
            }
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
                    self.errors.push(TypeError::UndefinedVariable {
                        name: name.clone(),
                        span: self.current_span,
                    });
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
                    self.errors.push(TypeError::UndefinedVariable {
                        name: function.clone(),
                        span: self.current_span,
                    });
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
            HirExpr::If { condition, then_branch, else_branch } => {
                let cond_type = self.check_expr_with_env(condition, env)?;
                env.add_constraint(Constraint::Equal {
                    left: cond_type,
                    right: Type::Bool,
                });
                
                let then_type = self.check_block(then_branch, env)?;
                
                if let Some(else_expr) = else_branch {
                    let else_type = self.check_expr_with_env(else_expr, env)?;
                    env.add_constraint(Constraint::Equal {
                        left: then_type.clone(),
                        right: else_type,
                    });
                    Ok(then_type)
                } else {
                    // An if without an else must return Unit
                    env.add_constraint(Constraint::Equal {
                        left: then_type,
                        right: Type::Unit,
                    });
                    Ok(Type::Unit)
                }
            }
            HirExpr::Match { expr, arms } => {
                let expr_type = self.check_expr_with_env(expr, env)?;
                let return_type = Type::Var(env.fresh_var());
                
                for (pattern, arm_expr) in arms {
                    let mut arm_env = env.clone();
                    let pattern_type = self.check_pattern(pattern, &mut arm_env)?;
                    arm_env.add_constraint(Constraint::Equal {
                        left: pattern_type,
                        right: expr_type.clone(),
                    });
                    
                    let arm_type = self.check_expr_with_env(arm_expr, &mut arm_env)?;
                    arm_env.add_constraint(Constraint::Equal {
                        left: arm_type,
                        right: return_type.clone(),
                    });
                    
                    env.constraints.extend(arm_env.constraints);
                    env.region_constraints.extend(arm_env.region_constraints);
                }
                
                Ok(return_type)
            }
            HirExpr::For { item, collection, body } => {
                let coll_type = self.check_expr_with_env(collection, env)?;
                let item_type = Type::Var(env.fresh_var());
                
                // Collection must be an Array of item_type. In the future, this should support any iterable.
                env.add_constraint(Constraint::Equal {
                    left: coll_type,
                    right: Type::Array { element_type: Box::new(item_type.clone()), size: None },
                });
                
                let mut body_env = env.clone();
                body_env.bindings.insert(item.clone(), TypeScheme {
                    vars: vec![],
                    ty: item_type,
                    constraints: vec![],
                });
                
                let body_type = self.check_block(body, &mut body_env)?;
                body_env.add_constraint(Constraint::Equal {
                    left: body_type,
                    right: Type::Unit,
                });
                
                env.constraints.extend(body_env.constraints);
                env.region_constraints.extend(body_env.region_constraints);
                
                Ok(Type::Unit)
            }
            HirExpr::Index { base, index } => {
                let base_type = self.check_expr_with_env(base, env)?;
                let index_type = self.check_expr_with_env(index, env)?;
                let elem_type = Type::Var(env.fresh_var());
                
                // Base must be an array
                env.add_constraint(Constraint::Equal {
                    left: base_type,
                    right: Type::Array { element_type: Box::new(elem_type.clone()), size: None },
                });
                
                // Index must be an integer
                env.add_constraint(Constraint::Equal {
                    left: index_type,
                    right: Type::Int,
                });
                
                Ok(elem_type)
            }
            HirExpr::Try(inner) => {
                // For now, just check the inner expression and return its type
                // In a full implementation, this would unwrap Result types
                self.check_expr_with_env(inner, env)
            }
            HirExpr::Struct { name: _, fields } => {
                // Each field's expression is checked; the struct type determined from declarations
                for (_, val) in fields {
                    self.check_expr_with_env(val, env)?;
                }
                // Return a placeholder struct type
                Ok(Type::UserDefined { name: "struct".to_string(), args: vec![] })
            }
            HirExpr::FieldAccess { base, field: _ } => {
                // Check the base expression and return the field type
                self.check_expr_with_env(base, env)
            }
            HirExpr::While { condition, body } => {
                let cond_type = self.check_expr_with_env(condition, env)?;
                env.add_constraint(Constraint::Equal {
                    left: cond_type,
                    right: Type::Bool,
                });
                let body_type = self.check_block(body, env)?;
                env.add_constraint(Constraint::Equal {
                    left: body_type,
                    right: Type::Unit,
                });
                Ok(Type::Unit)
            }
        }
    }

    fn check_pattern(&mut self, pattern: &HirPattern, env: &mut TypeEnv) -> Result<Type, Vec<TypeError>> {
        match pattern {
            HirPattern::Literal(lit) => Ok(self.literal_type(lit)),
            HirPattern::Ident(name) => {
                let var_type = Type::Var(env.fresh_var());
                env.bindings.insert(name.clone(), TypeScheme {
                    vars: vec![],
                    ty: var_type.clone(),
                    constraints: vec![],
                });
                Ok(var_type)
            }
            HirPattern::Wildcard => Ok(Type::Var(env.fresh_var())),
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

    fn hir_type_to_type(&mut self, hir_type: &HirType) -> Type {
        match hir_type {
            HirType::Ident(name) => {
                if let Some(scheme) = self.env.bindings.get(name).cloned() {
                    self.instantiate_scheme(&scheme)
                } else {
                    Type::UserDefined { name: name.clone(), args: vec![] }
                }
            }
            HirType::Unit => Type::Unit,
            HirType::Int => Type::Int,
            HirType::Bool => Type::Bool,
            HirType::Float => Type::Float,
            HirType::Str => Type::Str,
            HirType::Hole => {
                let fresh = Type::Var(self.env.fresh_var());
                println!("  [type hole] _ inferred as type variable — will be resolved during unification");
                fresh
            }
            HirType::Linear(ty) => Type::Linear(Box::new(self.hir_type_to_type(ty))),
            HirType::Affine(ty) => Type::Affine(Box::new(self.hir_type_to_type(ty))),
            HirType::Array(ty, n) => Type::Array { 
                element_type: Box::new(self.hir_type_to_type(ty)), 
                size: Some(*n as u64) 
            },
            HirType::Generic(name, args) => Type::UserDefined { 
                name: name.clone(), 
                args: args.iter().map(|t| self.hir_type_to_type(t)).collect() 
            },
            HirType::Tuple(types) => Type::Tuple(types.iter().map(|t| self.hir_type_to_type(t)).collect()),
            HirType::Function(args, ret) => Type::Function { 
                params: args.iter().map(|t| self.hir_type_to_type(t)).collect(), 
                return_type: Box::new(self.hir_type_to_type(ret)) 
            },
        }
    }

    /// Instantiate a type scheme by replacing bound variables with fresh type variables
    fn instantiate_scheme(&mut self, scheme: &TypeScheme) -> Type {
        let mut subst = Substitution::empty();
        for var in &scheme.vars {
            let fresh = Type::Var(self.env.fresh_var());
            subst.mapping.insert(var.clone(), fresh);
        }
        // Instantiate constraints too
        for constraint in &scheme.constraints {
            let instantiated = constraint.apply_subst(&subst);
            self.env.add_constraint(instantiated);
        }
        scheme.ty.apply_subst(&subst)
    }

    /// Generalize a type into a scheme by abstracting over variables not free in the environment
    fn generalize(&self, env: &TypeEnv, ty: &Type) -> TypeScheme {
        let env_free: HashSet<TypeVar> = env.bindings.values()
            .flat_map(|scheme| scheme.free_vars())
            .collect();
        let ty_free = ty.free_vars();
        let vars: Vec<TypeVar> = ty_free.difference(&env_free).cloned().collect();
        TypeScheme {
            vars,
            ty: ty.clone(),
            constraints: vec![], // TODO: Collect constraints on these variables
        }
    }

    fn solve_constraints(&mut self) -> Result<(), Vec<TypeError>> {
        let mut subst = Substitution::empty();
        let constraints = std::mem::take(&mut self.env.constraints);
        let mut trait_constraints = Vec::new();

        for constraint in constraints {
            let constraint = constraint.apply_subst(&subst);
            match constraint {
                Constraint::Equal { left, right } => {
                    match self.unify(&left, &right) {
                        Ok(s) => {
                            subst = s.compose(subst);
                        }
                        Err(e) => {
                            self.errors.push(e);
                        }
                    }
                }
                Constraint::Linear { ty } => {
                    let ty = ty.apply_subst(&subst);
                    match ty {
                        Type::Linear(_) => {}
                        _ => {
                            self.errors.push(TypeError::NonLinearInLinearContext(
                                format!("{}", ty)
                            ));
                        }
                    }
                }
                Constraint::Affine { ty } => {
                    let ty = ty.apply_subst(&subst);
                    match ty {
                        Type::Affine(_) | Type::Linear(_) => {}
                        _ => {
                            self.errors.push(TypeError::NonLinearInLinearContext(
                                format!("{}", ty)
                            ));
                        }
                    }
                }
                Constraint::RegionOutlives { longer, shorter } => {
                    self.env.add_region_constraint(longer, shorter);
                }
                Constraint::Effect { ty: _, effects: _ } => {
                    // Effect constraints are handled by the effect checker
                }
                Constraint::Trait { .. } => {
                    trait_constraints.push(constraint);
                }
            }
        }

        // Solve trait constraints after type variables have been unified
        for constraint in trait_constraints {
            let constraint = constraint.apply_subst(&subst);
            if let Constraint::Trait { ty, trait_name, args: _ } = constraint {
                // For now, if it's still a variable, we just ignore it (assuming it will be generalized)
                // In a real compiler, we would check if any implementation exists.
                if !matches!(ty, Type::Var(_)) {
                    if self.resolve_trait(&trait_name, &ty).is_none() {
                        self.errors.push(TypeError::TraitBoundNotSatisfied {
                            type_name: format!("{}", ty),
                            trait_name,
                            span: self.current_span,
                        });
                    }
                }
            }
        }

        // Apply final substitution to all bindings
        let mut new_bindings = HashMap::new();
        for (name, scheme) in &self.env.bindings {
            new_bindings.insert(name.clone(), scheme.apply_subst(&subst));
        }
        self.env.bindings = new_bindings;

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    /// Hindley-Milner unification: returns a substitution that makes two types equal
    fn unify(&self, left: &Type, right: &Type) -> Result<Substitution, TypeError> {
        match (left, right) {
            (Type::Var(v), t) => self.bind_var(v.clone(), t),
            (t, Type::Var(v)) => self.bind_var(v.clone(), t),
            (Type::Unit, Type::Unit) => Ok(Substitution::empty()),
            (Type::Int, Type::Int) => Ok(Substitution::empty()),
            (Type::Bool, Type::Bool) => Ok(Substitution::empty()),
            (Type::Float, Type::Float) => Ok(Substitution::empty()),
            (Type::Str, Type::Str) => Ok(Substitution::empty()),
            (Type::Linear(ty1), Type::Linear(ty2)) => self.unify(ty1, ty2),
            (Type::Affine(ty1), Type::Affine(ty2)) => self.unify(ty1, ty2),
            (Type::Function { params: p1, return_type: r1 }, Type::Function { params: p2, return_type: r2 }) => {
                if p1.len() != p2.len() {
                    return Err(TypeError::CannotUnify(
                        format!("{}", left),
                        format!("{}", right),
                    ));
                }
                let mut subst = Substitution::empty();
                for (a, b) in p1.iter().zip(p2.iter()) {
                    let s = self.unify(&a.apply_subst(&subst), &b.apply_subst(&subst))?;
                    subst = s.compose(subst);
                }
                let s = self.unify(&r1.apply_subst(&subst), &r2.apply_subst(&subst))?;
                subst = s.compose(subst);
                Ok(subst)
            }
            (Type::Tuple(types1), Type::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return Err(TypeError::CannotUnify(
                        format!("{}", left),
                        format!("{}", right),
                    ));
                }
                let mut subst = Substitution::empty();
                for (a, b) in types1.iter().zip(types2.iter()) {
                    let s = self.unify(&a.apply_subst(&subst), &b.apply_subst(&subst))?;
                    subst = s.compose(subst);
                }
                Ok(subst)
            }
            (Type::Array { element_type: e1, size: _ }, Type::Array { element_type: e2, size: _ }) => {
                // Array types unify if their element types unify; size is checked separately
                self.unify(e1, e2)
            }
            (Type::Ref { region: r1, ty: t1 }, Type::Ref { region: r2, ty: t2 }) => {
                if r1 != r2 {
                    return Err(TypeError::CannotUnify(
                        format!("{}", left),
                        format!("{}", right),
                    ));
                }
                self.unify(t1, t2)
            }
            (Type::RefMut { region: r1, ty: t1 }, Type::RefMut { region: r2, ty: t2 }) => {
                if r1 != r2 {
                    return Err(TypeError::CannotUnify(
                        format!("{}", left),
                        format!("{}", right),
                    ));
                }
                self.unify(t1, t2)
            }
            (Type::Channel(ty1), Type::Channel(ty2)) => self.unify(ty1, ty2),
            (Type::Result { ok_type: o1, err_type: e1 }, Type::Result { ok_type: o2, err_type: e2 }) => {
                let mut subst = self.unify(o1, o2)?;
                let s = self.unify(&e1.apply_subst(&subst), &e2.apply_subst(&subst))?;
                subst = s.compose(subst);
                Ok(subst)
            }
            (Type::Option(ty1), Type::Option(ty2)) => self.unify(ty1, ty2),
            (Type::UserDefined { name: n1, args: a1 }, Type::UserDefined { name: n2, args: a2 }) => {
                if n1 != n2 || a1.len() != a2.len() {
                    return Err(TypeError::CannotUnify(
                        format!("{}", left),
                        format!("{}", right),
                    ));
                }
                let mut subst = Substitution::empty();
                for (a, b) in a1.iter().zip(a2.iter()) {
                    let s = self.unify(&a.apply_subst(&subst), &b.apply_subst(&subst))?;
                    subst = s.compose(subst);
                }
                Ok(subst)
            }
            _ => Err(TypeError::CannotUnify(
                format!("{}", left),
                format!("{}", right),
            )),
        }
    }

    /// Bind a type variable to a type, checking for occurs (circular reference)
    fn bind_var(&self, var: TypeVar, ty: &Type) -> Result<Substitution, TypeError> {
        if let Type::Var(v2) = ty {
            if var == *v2 {
                return Ok(Substitution::empty());
            }
        }
        if ty.free_vars().contains(&var) {
            return Err(TypeError::CircularConstraint(
                format!("{} occurs in {}", var, ty)
            ));
        }
        Ok(Substitution::single(var, ty.clone()))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use once_hir::{HirProgram, HirItem, HirFnDecl, HirBlock, HirStmt, HirExpr, HirLiteral, HirReturnStmt, HirEffectRow};

    #[test]
    fn test_basic_type_checking() {
        let program = HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(HirType::Unit),
                    effects: None,
                    body: HirBlock {
                        statements: vec![
                            HirStmt::Return(HirReturnStmt { value: None, span: None }),
                        ],
                        span: None,
                    },
                    is_public: false,
                    type_params: vec![],
                    span: None,
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

    #[test]
    fn test_if_expr_type_checking() {
        let program = HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(HirType::Int),
                    effects: None,
                    body: HirBlock {
                        statements: vec![
                            HirStmt::Return(HirReturnStmt { 
                                value: Some(HirExpr::If {
                                    condition: Box::new(HirExpr::Literal(HirLiteral::Bool(true))),
                                    then_branch: HirBlock {
                                        statements: vec![HirStmt::Expr(HirExpr::Literal(HirLiteral::Int(1)))],
                                        span: None,
                                    },
                                    else_branch: Some(Box::new(HirExpr::Block(HirBlock {
                                        statements: vec![HirStmt::Expr(HirExpr::Literal(HirLiteral::Int(2)))],
                                        span: None,
                                    }))),
                                }),
                                span: None,
                            }),
                        ],
                        span: None,
                    },
                    is_public: false,
                    type_params: vec![],
                    span: None,
                }),
            ],
            imports: vec![],
        };

        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(result.is_ok(), "If expression branches matching should pass type checking");
    }

    #[test]
    fn test_linearity_integration() {
        let program = HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    params: vec![
                        HirParam {
                            name: "f".to_string(),
                            type_annotation: Some(HirType::Linear(Box::new(HirType::Ident("File".to_string())))),
                            is_linear: true,
                        },
                    ],
                    return_type: Some(HirType::Unit),
                    effects: None,
                    body: HirBlock {
                        statements: vec![
                            HirStmt::Expr(HirExpr::Ident("f".to_string())),
                            HirStmt::Expr(HirExpr::Ident("f".to_string())), // Double use!
                            HirStmt::Return(HirReturnStmt { value: None, span: None }),
                        ],
                        span: None,
                    },
                    is_public: false,
                    type_params: vec![],
                    span: None,
                }),
            ],
            imports: vec![],
        };

        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, TypeError::LinearValueReused(_))));
    }

    #[test]
    fn test_effect_validation() {
        let program = HirProgram {
            items: vec![
                HirItem::FnDecl(HirFnDecl {
                    name: "main".to_string(),
                    params: vec![],
                    return_type: Some(HirType::Unit),
                    effects: Some(HirEffectRow { effects: vec!["pure".to_string()] }), // Declared as Pure
                    body: HirBlock {
                        statements: vec![
                            HirStmt::Expr(HirExpr::Call {
                                function: "spawn".to_string(), // But calls spawn!
                                args: vec![],
                            }),
                            HirStmt::Return(HirReturnStmt { value: None, span: None }),
                        ],
                        span: None,
                    },
                    is_public: false,
                    type_params: vec![],
                    span: None,
                }),
            ],
            imports: vec![],
        };

        let mut checker = TypeChecker::new();
        let result = checker.check(&program);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        if !errors.iter().any(|e| matches!(e, TypeError::Effect(_))) {
            for e in &errors {
                println!("Error: {:?}", e);
            }
            panic!("Expected Effect error, but got others");
        }
    }
}