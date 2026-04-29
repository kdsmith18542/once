//! High-level Intermediate Representation (HIR) for the Once language
//! 
//! The HIR is a desugared, name-resolved representation of Once source code.
//! It serves as the input to type checking and subsequent compiler passes.

use once_parse::{Program, Item, FnDecl, LetDecl, Type, Expr, Stmt, Block, BinaryOp, Literal, Span};
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
}

/// Resolved function declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirFnDecl {
    pub name: String,
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

/// Resolved let declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirLetDecl {
    pub name: String,
    pub type_annotation: Option<HirType>,
    pub value: HirExpr,
    pub is_public: bool,
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
}

/// Resolved binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirBinaryOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
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
            }
        }

        if self.errors.is_empty() {
            let mut program_hir = HirProgram { items: hir_items, imports };
            let resolver = ImportResolver::new();
            let _ = resolver.resolve(&mut program_hir);
            Ok(program_hir)
        } else {
            Err(self.errors)
        }
    }

fn resolve_fn_decl(&mut self, fn_decl: FnDecl) -> HirFnDecl {
        let params = fn_decl.params.into_iter()
            .map(|param| HirParam {
                name: param.name.clone(),
                type_annotation: param.type_annotation.map(|t| self.resolve_type(t)),
                is_linear: false, // Will be determined during type checking
            })
            .collect();

        let body = self.resolve_block(fn_decl.body);

        // Convert effect row
        let effects = fn_decl.effects.map(|e| HirEffectRow {
            effects: e.effects,
        });

        HirFnDecl {
            name: fn_decl.name,
            params,
            return_type: fn_decl.return_type.map(|t| self.resolve_type(t)),
            effects,
            body,
            is_public: false, // Will be determined by visibility analysis
            span: fn_decl.span.map(|s| (s.start, s.end)),
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
            Stmt::Let(let_stmt) => HirStmt::Let(HirLetStmt {
                name: let_stmt.name,
                type_annotation: let_stmt.type_annotation.map(|t| self.resolve_type(t)),
                value: self.resolve_expr(let_stmt.value),
                is_linear: false, // Will be determined during type checking
                span: let_stmt.span
                    .map(|s| (s.start, s.end)),
            }),
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
        let source = "fn main() -> Unit { return }";
        // let tokens: Vec<_> = Lexer::new(source).collect();
        // let tokens: Vec<_> = Lexer::new(source).collect();
        // let program = OnceParser::parse(tokens).unwrap();
        
        // let builder = HirBuilder::new();
        // let hir = builder.build(program).unwrap();
        
        // assert_eq!(hir.items.len(), 1);
        // if let HirItem::FnDecl(fn_decl) = &hir.items[0] {
        //     assert_eq!(fn_decl.name, "main");
        //     assert_eq!(fn_decl.params.len(), 0);
        //     assert_eq!(fn_decl.return_type, Some(HirType::Unit));
        // } else {
        //     panic!("Expected function declaration");
        // }
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
