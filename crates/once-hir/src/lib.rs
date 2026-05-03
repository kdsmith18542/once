//! High-level Intermediate Representation (HIR) for the Once language
//! 
//! The HIR is a desugared, name-resolved representation of Once source code.
//! It serves as the input to type checking and subsequent compiler passes.

use once_parse::{Program, Item, FnDecl, LetDecl, GoalDecl, Type, Expr, Stmt, Block, BinaryOp, Literal};
mod import_resolver;
use indexmap::IndexMap;
use import_resolver::ImportResolver;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// A resolved Once program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirProgram {
    pub items: Vec<HirItem>,
    pub imports: Vec<Import>,
}

/// Import statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Import {
    pub path: String,
    pub alias: Option<String>,
    pub items: Vec<String>, // Specific items to import, empty means import all
}

/// Resolved items in a Once program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirItem {
    FnDecl(HirFnDecl),
    LetDecl(HirLetDecl),
    TypeDecl(HirTypeDecl),
    StructDecl(HirStructDecl),
    TraitDecl(HirTraitDecl),
    ImplBlock(HirImplBlock),
}

/// Resolved function declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirFnDecl {
    pub name: String,
    pub type_params: Vec<HirGenericParam>,
    pub params: Vec<HirParam>,
    pub return_type: Option<HirType>,
    pub effects: Option<HirEffectRow>,
    pub body: HirBlock,
    pub is_public: bool,
    pub span: Option<(usize, usize)>,
}

/// Effect row for HIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirEffectRow {
    pub effects: Vec<String>,
}

/// Resolved function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirParam {
    pub name: String,
    pub type_annotation: Option<HirType>,
    pub is_linear: bool,
}

/// Resolved generic parameter with bounds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirGenericParam {
    pub name: String,
    pub bounds: Vec<HirType>,
    pub span: Option<(usize, usize)>,
}

/// Resolved let declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirLetDecl {
    pub name: String,
    pub type_annotation: Option<HirType>,
    pub value: HirExpr,
    pub is_public: bool,
    pub span: Option<(usize, usize)>,
}

/// Resolved type/enum declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirTypeDecl {
    pub name: String,
    pub type_params: Vec<HirGenericParam>,
    pub variants: Vec<HirVariant>,
    pub span: Option<(usize, usize)>,
}

/// Resolved variant in a type declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirVariant {
    pub name: String,
    pub fields: Vec<HirType>,
}

/// Resolved struct (product type) declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirStructDecl {
    pub name: String,
    pub fields: Vec<HirStructField>,
    pub span: Option<(usize, usize)>,
}

/// Resolved struct field
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirStructField {
    pub name: String,
    pub field_type: HirType,
}

/// Resolved trait declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirTraitDecl {
    pub name: String,
    pub type_params: Vec<HirGenericParam>,
    pub methods: Vec<HirFnDecl>,
    pub span: Option<(usize, usize)>,
}

/// Resolved implementation block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirImplBlock {
    pub trait_name: Option<String>,
    pub target_type: HirType,
    pub methods: Vec<HirFnDecl>,
    pub span: Option<(usize, usize)>,
}

/// Resolved types in Once
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirType {
    Ident(String),
    Unit,
    Int,
    Bool,
    Float,
    Str,
    /// Type hole: compiler infers the type
    Hole,
    Linear(Box<HirType>),
    Affine(Box<HirType>),
    Array(Box<HirType>, usize),
    Generic(String, Vec<HirType>),
    Tuple(Vec<HirType>),
    Function(Vec<HirType>, Box<HirType>),
}

/// Resolved block of statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirBlock {
    pub statements: Vec<HirStmt>,
    pub span: Option<(usize, usize)>,
}

/// Resolved statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirStmt {
    Let(HirLetStmt),
    Return(HirReturnStmt),
    Expr(HirExpr),
    /// Using statement for linear resource management
    Using(HirUsingStmt),
    /// Continue to next loop iteration
    Continue,
    /// Break out of current loop
    Break,
}

/// Resolved let statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirLetStmt {
    pub name: String,
    pub type_annotation: Option<HirType>,
    pub value: HirExpr,
    pub is_linear: bool,
    pub span: Option<(usize, usize)>,
}

/// Resolved using statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirUsingStmt {
    pub name: String,
    pub init: HirExpr,
    pub body: HirBlock,
    pub is_linear: bool,
    pub span: Option<(usize, usize)>,
}

/// Resolved return statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirReturnStmt {
    pub value: Option<HirExpr>,
    pub span: Option<(usize, usize)>,
}

/// Resolved expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirExpr {
    Literal(HirLiteral),
    Ident(String),
    Call { function: String, args: Vec<HirExpr> },
    Binary { left: Box<HirExpr>, op: HirBinaryOp, right: Box<HirExpr> },
    Block(HirBlock),
    If {
        condition: Box<HirExpr>,
        then_branch: HirBlock,
        else_branch: Option<Box<HirExpr>>,
    },
    Match {
        expr: Box<HirExpr>,
        arms: Vec<(HirPattern, HirExpr)>,
    },
    For {
        item: String,
        collection: Box<HirExpr>,
        body: HirBlock,
    },
    /// Array indexing
    Index {
        base: Box<HirExpr>,
        index: Box<HirExpr>,
    },
    /// Try/unwrap operator
    Try(Box<HirExpr>),
    /// While loop
    While {
        condition: Box<HirExpr>,
        body: HirBlock,
    },
    /// Struct literal: StructName { field: value, ... }
    Struct {
        name: String,
        fields: Vec<(String, HirExpr)>,
    },
    /// Field access: expr.field
    FieldAccess {
        base: Box<HirExpr>,
        field: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirPattern {
    Literal(HirLiteral),
    Ident(String),
    Wildcard,
}

/// Resolved binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirBinaryOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Assign,
}

/// Resolved literals
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirLiteral {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,
}

/// Name resolution context
#[derive(Debug, Clone)]
pub struct NameContext {
    pub symbols: HashMap<String, Symbol>,
    pub imports: IndexMap<String, Import>,
    pub current_module: String,
}

/// Symbol information
#[derive(Debug, Clone)]
pub enum Symbol {
    Function { name: String, params: Vec<HirType>, return_type: HirType },
    Type { name: String, definition: HirType },
    Variable { name: String, type_: HirType, is_linear: bool },
}

/// Errors that can occur during HIR construction
#[derive(Error, Debug)]
pub enum HirError {
    #[error("Undefined symbol: {0}")]
    UndefinedSymbol(String),
    
    #[error("Duplicate symbol: {0}")]
    DuplicateSymbol(String),
    
    #[error("Invalid import: {0}")]
    InvalidImport(String),
    
    #[error("Type error: {0}")]
    TypeError(String),
    
    #[error("Linear value used multiple times: {0}")]
    LinearValueReused(String),
    
    #[error("Non-linear value in linear context: {0}")]
    NonLinearInLinearContext(String),
}

/// HIR builder that converts AST to HIR
pub struct HirBuilder {
    context: NameContext,
    errors: Vec<HirError>,
}

impl HirBuilder {
    pub fn new() -> Self {
        Self {
            context: NameContext {
                symbols: HashMap::new(),
                imports: IndexMap::new(),
                current_module: "main".to_string(),
            },
            errors: Vec::new(),
        }
    }

    pub fn build(mut self, program: Program) -> Result<HirProgram, Vec<HirError>> {
        let mut hir_items = Vec::new();
        let mut imports = Vec::new();

        // First pass: collect imports and declarations
        for item in program.items {
            match item {
                Item::FnDecl(fn_decl) => {
                    let hir_fn = self.resolve_fn_decl(fn_decl);
                    hir_items.push(HirItem::FnDecl(hir_fn));
                }
                Item::LetDecl(let_decl) => {
                    let hir_let = self.resolve_let_decl(let_decl);
                    hir_items.push(HirItem::LetDecl(hir_let));
                }
                Item::TypeDecl(type_decl) => {
                    let hir_type = self.resolve_type_decl(type_decl);
                    hir_items.push(HirItem::TypeDecl(hir_type));
                }
                Item::StructDecl(struct_decl) => {
                    let hir_struct = self.resolve_struct_decl(struct_decl);
                    hir_items.push(HirItem::StructDecl(hir_struct));
                }
                Item::TraitDecl(trait_decl) => {
                    let hir_trait = self.resolve_trait_decl(trait_decl);
                    hir_items.push(HirItem::TraitDecl(hir_trait));
                }
                Item::ImplBlock(impl_block) => {
                    let hir_impl = self.resolve_impl_block(impl_block);
                    hir_items.push(HirItem::ImplBlock(hir_impl));
                }
                Item::ImportDecl(import) => {
                    let import_hir = Import {
                        path: import.path.join("::"),
                        alias: import.alias,
                        items: import.items,
                    };
                    imports.push(import_hir);
                }
                Item::SchemaDecl(_) => {
                    // Schema declarations generate validation code at compile time
                    // Full implementation would produce hydrate functions
                }
                Item::GoalDecl(goal_decl) => {
                    // Goals are lowered to function declarations for the compiler pipeline;
                    // AI solver hooks operate at a higher level.
                    let hir_fn = self.resolve_goal_decl(goal_decl);
                    hir_items.push(HirItem::FnDecl(hir_fn));
                }
            }
        }

        // Resolve imports: load imported modules and extend the item list
        let mut program_hir = HirProgram { items: hir_items, imports };
        let resolver = ImportResolver::new();
        if let Err(e) = resolver.resolve(&mut program_hir) {
            self.errors.push(HirError::InvalidImport(e));
            return Err(self.errors);
        }

        // Register imported symbols in name context for cross-module name resolution
        for item in &program_hir.items {
            match item {
                HirItem::FnDecl(f) => {
                    let params: Vec<HirType> = f.params.iter()
                        .map(|p| p.type_annotation.clone().unwrap_or(HirType::Unit))
                        .collect();
                    let _return_type = f.return_type.clone().unwrap_or(HirType::Unit);
                    self.context.symbols.insert(f.name.clone(), Symbol::Function {
                        name: f.name.clone(),
                        params,
                        return_type: f.return_type.clone().unwrap_or(HirType::Unit),
                    });
                }
                HirItem::LetDecl(l) => {
                    self.context.symbols.insert(l.name.clone(), Symbol::Variable {
                        name: l.name.clone(),
                        type_: l.type_annotation.clone().unwrap_or(HirType::Unit),
                        is_linear: false,
                    });
                }
                HirItem::TypeDecl(t) => {
                    self.context.symbols.insert(t.name.clone(), Symbol::Type {
                        name: t.name.clone(),
                        definition: HirType::Ident(t.name.clone()),
                    });
                }
                HirItem::StructDecl(s) => {
                    self.context.symbols.insert(s.name.clone(), Symbol::Type {
                        name: s.name.clone(),
                        definition: HirType::Ident(s.name.clone()),
                    });
                }
                _ => {}
            }
        }

        if !self.errors.is_empty() {
            return Err(self.errors);
        }
        Ok(program_hir)
    }

    fn resolve_generic_param(&mut self, param: once_parse::GenericParam) -> HirGenericParam {
        HirGenericParam {
            name: param.name,
            bounds: param.bounds.into_iter().map(|t| self.resolve_type(t)).collect(),
            span: param.span.map(|s| (s.start, s.end)),
        }
    }

    pub fn is_linear_type(ty: &HirType) -> bool {
        matches!(ty, HirType::Linear(_) | HirType::Affine(_))
    }

    fn resolve_fn_decl(&mut self, fn_decl: FnDecl) -> HirFnDecl {
        let type_params = fn_decl.type_params.into_iter()
            .map(|p| self.resolve_generic_param(p))
            .collect();

        let params = fn_decl.params.into_iter()
            .map(|param| {
                let hir_ty = param.type_annotation.map(|t| self.resolve_type(t));
                let is_linear = hir_ty.as_ref().map_or(false, |t| HirBuilder::is_linear_type(t));
                HirParam {
                    name: param.name.clone(),
                    type_annotation: hir_ty,
                    is_linear,
                }
            })
            .collect();

        let body = self.resolve_block(fn_decl.body);

        // Convert effect row
        let effects = fn_decl.effects.map(|e| HirEffectRow {
            effects: e.effects,
        });

        HirFnDecl {
            name: fn_decl.name,
            type_params,
            params,
            return_type: fn_decl.return_type.map(|t| self.resolve_type(t)),
            effects,
            body,
            is_public: false, // Will be determined by visibility analysis
            span: fn_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_goal_decl(&mut self, goal_decl: GoalDecl) -> HirFnDecl {
        let type_params = goal_decl.type_params.into_iter()
            .map(|p| self.resolve_generic_param(p))
            .collect();

        let params = goal_decl.params.into_iter()
            .map(|param| {
                let hir_ty = param.type_annotation.map(|t| self.resolve_type(t));
                let is_linear = hir_ty.as_ref().map_or(false, |t| HirBuilder::is_linear_type(t));
                HirParam {
                    name: param.name.clone(),
                    type_annotation: hir_ty,
                    is_linear,
                }
            })
            .collect();

        let body = self.resolve_block(goal_decl.body);

        let effects = goal_decl.effects.map(|e| HirEffectRow {
            effects: e.effects,
        });

        HirFnDecl {
            name: goal_decl.name,
            type_params,
            params,
            return_type: goal_decl.return_type.map(|t| self.resolve_type(t)),
            effects,
            body,
            is_public: false,
            span: goal_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_let_decl(&mut self, let_decl: LetDecl) -> HirLetDecl {
        HirLetDecl {
            name: let_decl.name,
            type_annotation: let_decl.type_annotation.map(|t| self.resolve_type(t)),
            value: self.resolve_expr(let_decl.value),
            is_public: false, // Will be determined by visibility analysis
            span: let_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_type_decl(&mut self, type_decl: once_parse::TypeDecl) -> HirTypeDecl {
        HirTypeDecl {
            name: type_decl.name,
            type_params: type_decl.type_params.into_iter()
                .map(|p| self.resolve_generic_param(p))
                .collect(),
            variants: type_decl.variants.into_iter().map(|v| HirVariant {
                name: v.name,
                fields: v.fields.into_iter().map(|t| self.resolve_type(t)).collect(),
            }).collect(),
            span: type_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_struct_decl(&mut self, struct_decl: once_parse::StructDecl) -> HirStructDecl {
        HirStructDecl {
            name: struct_decl.name,
            fields: struct_decl.fields.into_iter().map(|f| HirStructField {
                name: f.name,
                field_type: self.resolve_type(f.field_type),
            }).collect(),
            span: struct_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_trait_decl(&mut self, trait_decl: once_parse::TraitDecl) -> HirTraitDecl {
        HirTraitDecl {
            name: trait_decl.name,
            type_params: trait_decl.type_params.into_iter()
                .map(|p| self.resolve_generic_param(p))
                .collect(),
            methods: trait_decl.methods.into_iter().map(|m| self.resolve_fn_decl(m)).collect(),
            span: trait_decl.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_impl_block(&mut self, impl_block: once_parse::ImplBlock) -> HirImplBlock {
        HirImplBlock {
            trait_name: impl_block.trait_name,
            target_type: self.resolve_type(impl_block.target_type),
            methods: impl_block.methods.into_iter().map(|m| self.resolve_fn_decl(m)).collect(),
            span: impl_block.span.map(|s| (s.start, s.end)),
        }
    }

    fn resolve_block(&mut self, block: Block) -> HirBlock {
        let span = block.span.map(|s| (s.start, s.end));
        HirBlock {
            statements: block.statements.into_iter()
                .map(|stmt| self.resolve_stmt(stmt))
                .collect(),
            span,
        }
    }

fn resolve_stmt(&mut self, stmt: Stmt) -> HirStmt {
        match stmt {
            Stmt::Let(let_stmt) => {
                let hir_ty = let_stmt.type_annotation.map(|t| self.resolve_type(t));
                let is_linear = hir_ty.as_ref().map_or(false, |t| HirBuilder::is_linear_type(t));
                HirStmt::Let(HirLetStmt {
                    name: let_stmt.name,
                    type_annotation: hir_ty,
                    value: self.resolve_expr(let_stmt.value),
                    is_linear,
                    span: let_stmt.span
                        .map(|s| (s.start, s.end)),
                })
            },
            Stmt::Return(return_stmt) => HirStmt::Return(HirReturnStmt {
                value: return_stmt.value.map(|e| self.resolve_expr(e)),
                span: return_stmt.span.map(|s| (s.start, s.end)),
            }),
            Stmt::Expr(expr) => HirStmt::Expr(self.resolve_expr(expr)),
            Stmt::Using(using_stmt) => HirStmt::Using(HirUsingStmt {
                name: using_stmt.name,
                init: self.resolve_expr(using_stmt.init),
                body: self.resolve_block(using_stmt.body),
                is_linear: true, // Using statements always involve linear resources
                span: using_stmt.span.map(|s| (s.start, s.end)),
            }),
            Stmt::Continue => HirStmt::Continue,
            Stmt::Break => HirStmt::Break,
        }
    }

    fn resolve_expr(&mut self, expr: Expr) -> HirExpr {
        match expr {
            Expr::Literal(lit) => HirExpr::Literal(self.resolve_literal(lit)),
            Expr::Ident(name) => HirExpr::Ident(name),
            Expr::Call { function, args } => HirExpr::Call {
                function,
                args: args.into_iter().map(|e| self.resolve_expr(e)).collect(),
            },
            Expr::Binary { left, op, right } => HirExpr::Binary {
                left: Box::new(self.resolve_expr(*left)),
                op: self.resolve_binary_op(op),
                right: Box::new(self.resolve_expr(*right)),
            },
            Expr::Block(block) => HirExpr::Block(self.resolve_block(block)),
            Expr::If { condition, then_branch, else_branch } => HirExpr::If {
                condition: Box::new(self.resolve_expr(*condition)),
                then_branch: self.resolve_block(then_branch),
                else_branch: else_branch.map(|b| Box::new(self.resolve_expr(*b))),
            },
            Expr::Match { expr, arms } => HirExpr::Match {
                expr: Box::new(self.resolve_expr(*expr)),
                arms: arms.into_iter().map(|(p, e)| (self.resolve_pattern(p), self.resolve_expr(e))).collect(),
            },
            Expr::For { item, collection, body } => HirExpr::For {
                item,
                collection: Box::new(self.resolve_expr(*collection)),
                body: self.resolve_block(body),
            },
            Expr::While { condition, body } => HirExpr::While {
                condition: Box::new(self.resolve_expr(*condition)),
                body: self.resolve_block(body),
            },
            Expr::Index { base, index } => HirExpr::Index {
                base: Box::new(self.resolve_expr(*base)),
                index: Box::new(self.resolve_expr(*index)),
            },
            Expr::Try(inner) => HirExpr::Try(Box::new(self.resolve_expr(*inner))),
            Expr::Struct { name, fields } => HirExpr::Struct {
                name,
                fields: fields.into_iter().map(|(n, e)| (n, self.resolve_expr(e))).collect(),
            },
            Expr::FieldAccess { base, field } => HirExpr::FieldAccess {
                base: Box::new(self.resolve_expr(*base)),
                field,
            },
        }
    }

    fn resolve_pattern(&mut self, pat: once_parse::Pattern) -> HirPattern {
        match pat {
            once_parse::Pattern::Literal(lit) => HirPattern::Literal(self.resolve_literal(lit)),
            once_parse::Pattern::Ident(name) => HirPattern::Ident(name),
            once_parse::Pattern::Wildcard => HirPattern::Wildcard,
        }
    }

    fn resolve_type(&mut self, ty: Type) -> HirType {
        match ty {
            Type::Ident(name) => HirType::Ident(name),
            Type::Unit => HirType::Unit,
            Type::Int => HirType::Int,
            Type::Bool => HirType::Bool,
            Type::Float => HirType::Float,
            Type::Str => HirType::Str,
            Type::Hole => HirType::Hole,
            Type::Linear(t) => HirType::Linear(Box::new(self.resolve_type(*t))),
            Type::Affine(t) => HirType::Affine(Box::new(self.resolve_type(*t))),
            Type::Array(t, n) => HirType::Array(Box::new(self.resolve_type(*t)), n),
            Type::Generic(name, args) => HirType::Generic(name, args.into_iter().map(|t| self.resolve_type(t)).collect()),
            Type::Tuple(types) => HirType::Tuple(types.into_iter().map(|t| self.resolve_type(t)).collect()),
            Type::Function(args, ret) => HirType::Function(
                args.into_iter().map(|t| self.resolve_type(t)).collect(),
                Box::new(self.resolve_type(*ret)),
            ),
        }
    }

    fn resolve_literal(&mut self, lit: Literal) -> HirLiteral {
        match lit {
            Literal::Int(n) => HirLiteral::Int(n),
            Literal::Float(n) => HirLiteral::Float(n),
            Literal::String(s) => HirLiteral::String(s),
            Literal::Bool(b) => HirLiteral::Bool(b),
            Literal::Unit => HirLiteral::Unit,
        }
    }

    fn resolve_binary_op(&mut self, op: BinaryOp) -> HirBinaryOp {
        match op {
            BinaryOp::Add => HirBinaryOp::Add,
            BinaryOp::Sub => HirBinaryOp::Sub,
            BinaryOp::Mul => HirBinaryOp::Mul,
            BinaryOp::Div => HirBinaryOp::Div,
            BinaryOp::Eq => HirBinaryOp::Eq,
            BinaryOp::Ne => HirBinaryOp::Ne,
            BinaryOp::Lt => HirBinaryOp::Lt,
            BinaryOp::Le => HirBinaryOp::Le,
            BinaryOp::Gt => HirBinaryOp::Gt,
            BinaryOp::Ge => HirBinaryOp::Ge,
            BinaryOp::And => HirBinaryOp::And,
            BinaryOp::Or => HirBinaryOp::Or,
            BinaryOp::Assign => HirBinaryOp::Assign,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_parse::{OnceParser, Program};
    // use the Hir ImportResolver in tests as a basic exec-path check
    // use once_lex::Lexer;

    #[test]
    fn test_hir_construction() {
        let fn_decl = HirFnDecl {
            name: "main".to_string(),
            type_params: vec![],
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
            span: None,
        };
        
        let mut prog = HirProgram { items: vec![HirItem::FnDecl(fn_decl)], imports: vec![] };
        let resolver = ImportResolver::new();
        assert!(resolver.resolve(&mut prog).is_ok());
        assert_eq!(prog.items.len(), 1);
        if let HirItem::FnDecl(ref fn_decl) = prog.items[0] {
            assert_eq!(fn_decl.name, "main");
            assert_eq!(fn_decl.params.len(), 0);
            assert_eq!(fn_decl.return_type, Some(HirType::Unit));
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_import_resolver_noop() {
        // Minimal sanity check that ImportResolver can be invoked without error
        let mut prog = HirProgram { items: Vec::new(), imports: Vec::new() };
        let resolver = ImportResolver::new();
        assert!(resolver.resolve(&mut prog).is_ok());
    }

    #[test]
    fn test_import_resolver_basic() {
        use crate::Import;
        // Minimal scenario: a single import path with no items should be expanded to a placeholder
        let mut prog = HirProgram { items: Vec::new(), imports: vec![Import { path: "std".to_string(), alias: None, items: Vec::new() }] };
        let resolver = ImportResolver::new();
        assert!(resolver.resolve(&mut prog).is_ok());
        assert_eq!(prog.imports[0].items, vec!["*".to_string(), "prelude".to_string()]);
    }

    #[test]
    fn test_import_resolver_relative() {
        use crate::Import;
        let mut prog = HirProgram { items: Vec::new(), imports: vec![Import { path: "./utils".to_string(), alias: Some("U".to_string()), items: Vec::new() }] };
        let resolver = ImportResolver::new();
        assert!(resolver.resolve(&mut prog).is_ok());
        let imp = &prog.imports[0];
        // Relative paths are normalized to remove leading './'
        assert_eq!(imp.path, "utils");
        assert_eq!(imp.alias.as_ref().unwrap(), "U");
        assert_eq!(imp.items, vec!["*".to_string()]);
    }
}

#[test]
fn test_import_resolver_named_imports() {
    use crate::Import;
    // Named imports should be preserved by the resolver
    let mut prog = HirProgram {
        items: Vec::new(),
        imports: vec![Import { path: "pkg::utils".to_string(), alias: None, items: vec!["Foo".to_string(), "Bar".to_string()] }],
    };
    let resolver = ImportResolver::new();
    assert!(resolver.resolve(&mut prog).is_ok());
    assert_eq!(prog.imports[0].items, vec!["Foo".to_string(), "Bar".to_string()]);
}
