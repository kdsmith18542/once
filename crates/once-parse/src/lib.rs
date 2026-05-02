//! Parser for the Once language
//! 
//! Converts a stream of tokens into an Abstract Syntax Tree (AST)
//! representing the structure of Once source code.

use once_lex::{Token, TokenWithSpan};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::iter::Peekable;
use std::vec::IntoIter;

/// A parsed Once program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Program {
    pub items: Vec<Item>,
}

/// Span information propagated from the lexer/token stream.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl From<once_lex::Span> for Span {
    fn from(s: once_lex::Span) -> Self {
        Span { start: s.start, end: s.end, line: s.line, column: s.column }
    }
}

/// Top-level items in a Once program
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Item {
    FnDecl(FnDecl),
    LetDecl(LetDecl),
    TypeDecl(TypeDecl),
    StructDecl(StructDecl),
    TraitDecl(TraitDecl),
    ImplBlock(ImplBlock),
    GoalDecl(GoalDecl),
    ImportDecl(ImportDecl),
}

/// Effect row for function effects
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EffectRow {
    pub effects: Vec<String>,
}

/// Function declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnDecl {
    pub name: String,
    pub type_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub effects: Option<EffectRow>,
    pub body: Block,
    pub span: Option<Span>,
}

/// Goal declaration for AI integration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalDecl {
    pub name: String,
    pub type_params: Vec<GenericParam>,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
    pub effects: Option<EffectRow>,
    pub body: Block,
    pub span: Option<Span>,
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub span: Option<Span>,
}

/// Generic parameter with optional bounds
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericParam {
    pub name: String,
    pub bounds: Vec<Type>,
    pub span: Option<Span>,
}

/// Let declaration (module-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetDecl {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub value: Expr,
    pub span: Option<Span>,
}

/// Type declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDecl {
    pub name: String,
    pub type_params: Vec<GenericParam>,
    pub variants: Vec<Variant>,
    pub span: Option<Span>,
}

/// Variant in a type declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<Type>,
}

/// Struct (product type) declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<StructField>,
    pub span: Option<Span>,
}

/// A field in a struct definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
    pub span: Option<Span>,
}

/// Import declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportDecl {
    pub path: Vec<String>,
    pub alias: Option<String>,
    pub items: Vec<String>, // Specific items; empty = import all
    pub span: Option<Span>,
}

/// Trait declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraitDecl {
    pub name: String,
    pub type_params: Vec<GenericParam>,
    pub methods: Vec<FnDecl>,
    pub span: Option<Span>,
}

/// Implementation block
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImplBlock {
    pub trait_name: Option<String>,
    pub target_type: Type,
    pub methods: Vec<FnDecl>,
    pub span: Option<Span>,
}

/// Types in Once
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Ident(String),
    Unit,
    Int,
    Bool,
    Float,
    Str,
    /// Type hole: `_` — compiler infers the type
    Hole,
    /// Linear type: `lin T`
    Linear(Box<Type>),
    /// Affine type: `aff T`
    Affine(Box<Type>),
    /// Array type: `[T; n]`
    Array(Box<Type>, usize),
    /// Generic type: `Option<T>`, `Vec<T>`
    Generic(String, Vec<Type>),
    /// Tuple type: `(A, B, C)`
    Tuple(Vec<Type>),
    /// Function type: `fn(A) -> B`
    Function(Vec<Type>, Box<Type>),
}

/// Block of statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
    pub span: Option<Span>,
}

/// Statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Let(LetStmt),
    Return(ReturnStmt),
    Expr(Expr),
    /// Using statement for linear resource management
    /// `using x = expr { body }` desugars to let + consume at end
    Using(UsingStmt),
    /// Continue to next loop iteration
    Continue,
    /// Break out of current loop
    Break,
}

/// Let statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub value: Expr,
    pub span: Option<Span>,
}

/// Using statement for linear resource management
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsingStmt {
    pub name: String,
    pub init: Expr,
    pub body: Block,
    pub span: Option<Span>,
}

/// Return statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
    pub span: Option<Span>,
}

/// Expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Literal(Literal),
    Ident(String),
    Call { function: String, args: Vec<Expr> },
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Block(Block),
    If {
        condition: Box<Expr>,
        then_branch: Block,
        else_branch: Option<Box<Expr>>,
    },
    Match {
        expr: Box<Expr>,
        arms: Vec<(Pattern, Expr)>,
    },
    For {
        item: String,
        collection: Box<Expr>,
        body: Block,
    },
    /// Array indexing
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// Try/unwrap operator
    Try(Box<Expr>),
    /// While loop
    While {
        condition: Box<Expr>,
        body: Block,
    },
    /// Struct literal: StructName { field: value, ... }
    Struct {
        name: String,
        fields: Vec<(String, Expr)>,
    },
    /// Field access: expr.field
    FieldAccess {
        base: Box<Expr>,
        field: String,
    },
}

/// Patterns for match expressions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Literal(Literal),
    Ident(String),
    Wildcard,
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Assign,
}

/// Literals
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Unit,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Ident(name) => write!(f, "{}", name),
            Type::Unit => write!(f, "Unit"),
            Type::Int => write!(f, "Int"),
            Type::Bool => write!(f, "Bool"),
            Type::Float => write!(f, "Float"),
            Type::Str => write!(f, "Str"),
            Type::Hole => write!(f, "_"),
            Type::Linear(t) => write!(f, "lin {}", t),
            Type::Affine(t) => write!(f, "aff {}", t),
            Type::Array(t, n) => write!(f, "[{}; {}]", t, n),
            Type::Generic(name, args) => {
                write!(f, "{}<", name)?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ">")
            }
            Type::Tuple(types) => {
                write!(f, "(")?;
                for (i, t) in types.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", t)?;
                }
                write!(f, ")")
            }
            Type::Function(args, ret) => {
                write!(f, "fn (")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", arg)?;
                }
                write!(f, ") -> {}", ret)
            }
        }
    }
}

/// Simple parser for Once source code
pub struct OnceParser;

impl OnceParser {
    pub fn parse(tokens: Vec<TokenWithSpan>) -> Result<Program, String> {
        let mut tokens = tokens.into_iter().peekable();
        let mut items = Vec::new();

        while tokens.peek().is_some() {
            match tokens.peek().unwrap().token {
                Token::Type | Token::Enum => {
                    let type_decl = Self::parse_type_decl(&mut tokens)?;
                    items.push(Item::TypeDecl(type_decl));
                }
                Token::Struct => {
                    let struct_decl = Self::parse_struct_decl(&mut tokens)?;
                    items.push(Item::StructDecl(struct_decl));
                }
                Token::Trait => items.push(Item::TraitDecl(Self::parse_trait_decl(&mut tokens)?)),
                Token::Impl => items.push(Item::ImplBlock(Self::parse_impl_block(&mut tokens)?)),
                Token::Fn => {
                    let fn_decl = Self::parse_fn_decl(&mut tokens)?;
                    items.push(Item::FnDecl(fn_decl));
                }
                Token::Let => {
                    let let_decl = Self::parse_let_decl(&mut tokens)?;
                    items.push(Item::LetDecl(let_decl));
                }
                Token::Goal => {
                    let goal_decl = Self::parse_goal_decl(&mut tokens)?;
                    items.push(Item::GoalDecl(goal_decl));
                }
                Token::Import => {
                    let import_decl = Self::parse_import_decl(&mut tokens)?;
                    items.push(Item::ImportDecl(import_decl));
                }
                _ => return Err(format!("Unexpected token: {:?}", tokens.peek().unwrap().token)),
            }
        }

        Ok(Program { items })
    }

    fn parse_fn_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<FnDecl, String> {
        // fn
        let fn_token = tokens.next().ok_or_else(|| "Expected 'fn' token".to_string())?;
        let start_span = Span::from(fn_token.span);
        
        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected function name".to_string()),
            },
            None => return Err("Expected function name".to_string()),
        };

        // optional type parameters <T: Bound>
        let type_params = Self::parse_generic_params(tokens)?;

        // (
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LParen)) {
            return Err("Expected '('".to_string());
        }

        // params
        let mut params = Vec::new();
        while let Some(t) = tokens.peek() {
            match t.token {
                Token::RParen => break,
                Token::Ident(_) => {
                    let param = Self::parse_param(tokens)?;
                    params.push(param);
                    if let Some(t) = tokens.peek() {
                        if matches!(t.token, Token::Comma) {
                            tokens.next();
                        }
                    }
                }
                _ => return Err("Expected parameter or ')'".to_string()),
            }
        }

        // )
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RParen)) {
            return Err("Expected ')'".to_string());
        }

// return type
        let return_type = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Arrow) {
                tokens.next(); // consume ->
                Some(Self::parse_type(tokens)?)
            } else {
                None
            }
        } else {
            None
        };

        // effects: !Effect or ![Effect, Effect, ...]
        let effects = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Bang) {
                tokens.next(); // consume !
                if let Some(t) = tokens.peek() {
                    if matches!(t.token, Token::LBracket) {
                        // ![effect1, effect2, ...]
                        tokens.next(); // consume [
                        let mut effect_list = Vec::new();
                        loop {
                            match tokens.peek() {
                                Some(TokenWithSpan { token: Token::Ident(name), .. }) => {
                                    effect_list.push(name.clone());
                                    tokens.next();
                                }
                                Some(TokenWithSpan { token: Token::RBracket, .. }) => {
                                    tokens.next();
                                    break;
                                }
                                Some(TokenWithSpan { token: Token::Comma, .. }) => {
                                    tokens.next();
                                }
                                _ => break,
                            }
                        }
                        Some(EffectRow { effects: effect_list })
                    } else if let Some(TokenWithSpan { token: Token::Ident(name), .. }) = tokens.peek() {
                        // !effect (single effect)
                        let effect_name = name.clone();
                        tokens.next();
                        Some(EffectRow { effects: vec![effect_name] })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        // {
        let lb = tokens.next().ok_or_else(|| "Expected '{'".to_string())?;
        if lb.token != Token::LBrace {
            return Err("Expected '{'".to_string());
        }
        // body with span from LBrace
        let body = Self::parse_block_with_span(tokens, Span::from(lb.span))?;

        // }
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}'".to_string());
        }

        Ok(FnDecl {
            name,
            type_params,
            params,
            return_type,
            effects,
            body,
            span: Some(start_span),
        })
    }

    fn parse_goal_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<GoalDecl, String> {
        // goal
        let goal_token = tokens.next().ok_or_else(|| "Expected 'goal' token".to_string())?;
        let start_span = Span::from(goal_token.span);

        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected goal name".to_string()),
            },
            None => return Err("Expected goal name".to_string()),
        };

        // optional type parameters <T: Bound>
        let type_params = if matches!(tokens.peek().map(|t| &t.token), Some(Token::Lt)) {
            tokens.next(); // consume <
            let mut params = Vec::new();
            while !matches!(tokens.peek().map(|t| &t.token), Some(Token::Gt)) {
                let param_name = match tokens.next() {
                    Some(t) => match t.token {
                        Token::Ident(n) => n,
                        _ => return Err("Expected type parameter name".to_string()),
                    },
                    None => return Err("Expected type parameter name".to_string()),
                };
                let mut bounds = Vec::new();
                if matches!(tokens.peek().map(|t| &t.token), Some(Token::Colon)) {
                    tokens.next(); // consume :
                    bounds.push(Self::parse_type(tokens)?);
                }
                params.push(GenericParam { name: param_name, bounds, span: Some(start_span) });
                if matches!(tokens.peek().map(|t| &t.token), Some(Token::Comma)) {
                    tokens.next(); // consume ,
                }
            }
            tokens.next(); // consume >
            params
        } else {
            Vec::new()
        };

        // (
        let _lp = tokens.next().ok_or_else(|| "Expected '('".to_string())?;

        // params
        let mut params = Vec::new();
        while !matches!(tokens.peek().map(|t| &t.token), Some(Token::RParen)) {
            let param_name = match tokens.next() {
                Some(t) => match t.token {
                    Token::Ident(name) => name,
                    _ => return Err("Expected parameter name".to_string()),
                },
                None => return Err("Expected parameter name".to_string()),
            };

            let type_annotation = if matches!(tokens.peek().map(|t| &t.token), Some(Token::Colon)) {
                tokens.next(); // consume :
                Some(Self::parse_type(tokens)?)
            } else {
                None
            };

            params.push(Param { name: param_name, type_annotation, span: Some(start_span) });

            if matches!(tokens.peek().map(|t| &t.token), Some(Token::Comma)) {
                tokens.next(); // consume ,
            }
        }
        tokens.next(); // consume )

        // -> return_type
        let return_type = if matches!(tokens.peek().map(|t| &t.token), Some(Token::Arrow)) {
            tokens.next(); // consume ->
            Some(Self::parse_type(tokens)?)
        } else {
            None
        };

        // optional effects !io !spawn
        let effects = if matches!(tokens.peek().map(|t| &t.token), Some(Token::Bang)) {
            let mut effects = Vec::new();
            while matches!(tokens.peek().map(|t| &t.token), Some(Token::Bang)) {
                tokens.next(); // consume !
                let effect_name = match tokens.next() {
                    Some(t) => match t.token {
                        Token::Ident(name) => name,
                        _ => return Err("Expected effect name after !".to_string()),
                    },
                    None => return Err("Expected effect name after !".to_string()),
                };
                effects.push(effect_name);
            }
            Some(EffectRow { effects })
        } else {
            None
        };

        // {
        let lb = tokens.next().ok_or_else(|| "Expected '{'".to_string())?;
        if lb.token != Token::LBrace {
            return Err("Expected '{'".to_string());
        }
        let body = Self::parse_block_with_span(tokens, Span::from(lb.span))?;

        // }
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}'".to_string());
        }

        Ok(GoalDecl {
            name,
            type_params,
            params,
            return_type,
            effects,
            body,
            span: Some(start_span),
        })
    }

    fn parse_import_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<ImportDecl, String> {
        // import
        let import_token = tokens.next().ok_or_else(|| "Expected 'import' token".to_string())?;
        let start_span = Span::from(import_token.span);

        // Parse module path: ident { :: ident }...
        let mut path = Vec::new();
        let first = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected module path".to_string()),
            },
            None => return Err("Expected module path".to_string()),
        };
        path.push(first);

        while let Some(t) = tokens.peek() {
            if matches!(t.token, Token::ColonColon) {
                tokens.next();
                let next = match tokens.next() {
                    Some(t) => match t.token {
                        Token::Ident(name) => name,
                        _ => return Err("Expected identifier after '::'".to_string()),
                    },
                    None => return Err("Expected identifier after '::'".to_string()),
                };
                path.push(next);
            } else {
                break;
            }
        }

        // Optional 'as' alias
        let alias = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::As) {
                tokens.next();
                match tokens.next() {
                    Some(t) => match t.token {
                        Token::Ident(name) => Some(name),
                        _ => return Err("Expected alias name after 'as'".to_string()),
                    },
                    None => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        // Optional '{' item list
        let mut items = Vec::new();
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::LBrace) {
                tokens.next();
                loop {
                    match tokens.peek() {
                        Some(TokenWithSpan { token: Token::RBrace, .. }) => {
                            tokens.next();
                            break;
                        }
                        Some(TokenWithSpan { token: Token::Ident(name), .. }) => {
                            items.push(name.clone());
                            tokens.next();
                        }
                        Some(TokenWithSpan { token: Token::Comma, .. }) => {
                            tokens.next();
                        }
                        _ => break,
                    }
                }
            }
        }

        // Optional ;
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Semicolon) {
                tokens.next();
            }
        }

        Ok(ImportDecl { path, alias, items, span: Some(start_span) })
    }

    fn parse_let_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<LetDecl, String> {
        // let
        let let_token = tokens.next().ok_or_else(|| "Expected 'let' token".to_string())?;
        let start_span = Span::from(let_token.span);
        
        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected variable name".to_string()),
            },
            None => return Err("Expected variable name".to_string()),
        };

        // type annotation
        let type_annotation = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Colon) {
                tokens.next(); // consume :
                Some(Self::parse_type(tokens)?)
            } else {
                None
            }
        } else {
            None
        };

        // =
        if !matches!(tokens.next().map(|t| t.token), Some(Token::Assign)) {
            return Err("Expected '='".to_string());
        }

        // value
        let value = Self::parse_expr(tokens)?;

        // ;
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Semicolon) {
                tokens.next();
            }
        }

        Ok(LetDecl {
            name,
            type_annotation,
            value,
            span: Some(start_span),
        })
    }

    fn parse_type_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<TypeDecl, String> {
        // type or enum
        let type_token = tokens.next().ok_or_else(|| "Expected 'type' or 'enum' token".to_string())?;
        let start_span = Span::from(type_token.span);

        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                Token::Result => "Result".to_string(),
                Token::Option => "Option".to_string(),
                Token::Vec => "Vec".to_string(),
                Token::Chan => "Chan".to_string(),
                Token::Actor => "Actor".to_string(),
                Token::File => "File".to_string(),
                _ => return Err("Expected type name".to_string()),
            },
            None => return Err("Expected type name".to_string()),
        };

        // optional type parameters
        let type_params = Self::parse_generic_params(tokens)?;

        // =
        if !matches!(tokens.next().map(|t| t.token), Some(Token::Assign)) {
            return Err("Expected '=' after type name".to_string());
        }

        // variants separated by |
        let mut variants = Vec::new();
        loop {
            let variant_name = match tokens.next() {
                Some(t) => match t.token {
                    Token::Ident(name) => name,
                    Token::Ok => "Ok".to_string(),
                    Token::Err => "Err".to_string(),
                    Token::Some => "Some".to_string(),
                    Token::None => "None".to_string(),
                    _ => return Err("Expected variant name".to_string()),
                },
                None => return Err("Expected variant name".to_string()),
            };

            let mut fields = Vec::new();
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::LParen) {
                    tokens.next(); // consume (
                    loop {
                        match tokens.peek() {
                            Some(TokenWithSpan { token: Token::RParen, .. }) => {
                                tokens.next();
                                break;
                            }
                            _ => {
                                fields.push(Self::parse_type(tokens)?);
                                if let Some(t) = tokens.peek() {
                                    if matches!(t.token, Token::Comma) {
                                        tokens.next();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            variants.push(Variant {
                name: variant_name,
                fields,
            });

            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::Or) {
                    tokens.next(); // consume |
                    continue;
                }
            }
            break;
        }

        Ok(TypeDecl {
            name,
            type_params,
            variants,
            span: Some(start_span),
        })
    }

    fn parse_struct_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<StructDecl, String> {
        // struct
        let struct_token = tokens.next().ok_or_else(|| "Expected 'struct' token".to_string())?;
        let start_span = Span::from(struct_token.span);

        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected struct name".to_string()),
            },
            None => return Err("Expected struct name".to_string()),
        };

        // {
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after struct name".to_string());
        }

        // fields
        let mut fields = Vec::new();
        while let Some(t) = tokens.peek() {
            if matches!(t.token, Token::RBrace) {
                tokens.next();
                break;
            }
            let field_name = match tokens.next() {
                Some(t) => match t.token {
                    Token::Ident(name) => name,
                    _ => return Err("Expected field name".to_string()),
                },
                None => return Err("Expected field name".to_string()),
            };
            if !matches!(tokens.next().map(|t| t.token), Some(Token::Colon)) {
                return Err("Expected ':' after field name".to_string());
            }
            let field_type = Self::parse_type(tokens)?;
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::Comma) {
                    tokens.next();
                }
            }
            fields.push(StructField {
                name: field_name,
                field_type,
                span: None,
            });
        }

        Ok(StructDecl {
            name,
            fields,
            span: Some(start_span),
        })
    }

    fn parse_trait_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<TraitDecl, String> {
        // trait
        let trait_token = tokens.next().ok_or_else(|| "Expected 'trait' token".to_string())?;
        let start_span = Span::from(trait_token.span);
        
        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected trait name".to_string()),
            },
            None => return Err("Expected trait name".to_string()),
        };

        // optional type parameters
        let type_params = Self::parse_generic_params(tokens)?;

        // {
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after trait name".to_string());
        }

        // methods
        let mut methods = Vec::new();
        while let Some(t) = tokens.peek() {
            match t.token {
                Token::RBrace => break,
                Token::Fn => {
                    methods.push(Self::parse_fn_decl(tokens)?);
                }
                _ => return Err("Expected method or '}' in trait".to_string()),
            }
        }

        // }
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}'".to_string());
        }

        Ok(TraitDecl {
            name,
            type_params,
            methods,
            span: Some(start_span),
        })
    }

    fn parse_impl_block(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<ImplBlock, String> {
        // impl
        let impl_token = tokens.next().ok_or_else(|| "Expected 'impl' token".to_string())?;
        let start_span = Span::from(impl_token.span);

        // check if it's a trait impl: impl Trait for Type
        // or an inherent impl: impl Type
        
        let first_type = Self::parse_type(tokens)?;
        
        let (trait_name, target_type) = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::For) {
                tokens.next(); // consume for
                let trait_name = match first_type {
                    Type::Ident(name) => name,
                    _ => return Err("Expected trait name before 'for'".to_string()),
                };
                let target_type = Self::parse_type(tokens)?;
                (Some(trait_name), target_type)
            } else {
                (None, first_type)
            }
        } else {
            (None, first_type)
        };

        // {
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after impl target".to_string());
        }

        // methods
        let mut methods = Vec::new();
        while let Some(t) = tokens.peek() {
            match t.token {
                Token::RBrace => break,
                Token::Fn => {
                    methods.push(Self::parse_fn_decl(tokens)?);
                }
                _ => return Err("Expected method or '}' in impl".to_string()),
            }
        }

        // }
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}'".to_string());
        }

        Ok(ImplBlock {
            trait_name,
            target_type,
            methods,
            span: Some(start_span),
        })
    }

fn parse_param(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Param, String> {
        // name with span
        let name_token = tokens.next().ok_or_else(|| "Expected parameter name".to_string())?;
        let name = match name_token.token {
            Token::Ident(n) => n,
            _ => return Err("Expected parameter name".to_string()),
        };
        let span = Some(Span::from(name_token.span));

        let type_annotation = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Colon) {
                tokens.next(); // consume :
                Some(Self::parse_type(tokens)?)
            } else {
                None
            }
        } else {
            None
        };

        Ok(Param {
            name,
            type_annotation,
            span,
        })
    }

    fn parse_type(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Type, String> {
        match tokens.next() {
            Some(t) => match t.token {
                Token::Unit => Ok(Type::Unit),
                Token::Int => Ok(Type::Int),
                Token::Bool => Ok(Type::Bool),
                Token::Float => Ok(Type::Float),
                Token::Str => Ok(Type::Str),
                Token::Lin => {
                    let inner = Self::parse_type(tokens)?;
                    Ok(Type::Linear(Box::new(inner)))
                }
                Token::Aff => {
                    let inner = Self::parse_type(tokens)?;
                    Ok(Type::Affine(Box::new(inner)))
                }
                Token::LBracket => {
                    let elem_type = Self::parse_type(tokens)?;
                    tokens.next(); // consume ;
                    let size = match tokens.next() {
                        Some(TokenWithSpan { token: Token::IntLit(n), .. }) => n as usize,
                        _ => return Err("Expected array size".to_string()),
                    };
                    match tokens.next() {
                        Some(TokenWithSpan { token: Token::RBracket, .. }) => Ok(Type::Array(Box::new(elem_type), size)),
                        _ => Err("Expected ]".to_string()),
                    }
                }
                Token::Ident(name) => {
                    if name == "_" {
                        Ok(Type::Hole)
                    } else {
                        Self::parse_generic_or_ident_type(tokens, name)
                    }
                }
                Token::Vec => Self::parse_generic_or_ident_type(tokens, "Vec".to_string()),
                Token::Option => Self::parse_generic_or_ident_type(tokens, "Option".to_string()),
                Token::Result => Self::parse_generic_or_ident_type(tokens, "Result".to_string()),
                Token::Chan => Self::parse_generic_or_ident_type(tokens, "Chan".to_string()),
                Token::Actor => Self::parse_generic_or_ident_type(tokens, "Actor".to_string()),
                Token::File => Self::parse_generic_or_ident_type(tokens, "File".to_string()),
                Token::Ok => Self::parse_generic_or_ident_type(tokens, "Ok".to_string()),
                Token::Err => Self::parse_generic_or_ident_type(tokens, "Err".to_string()),
                Token::Some => Self::parse_generic_or_ident_type(tokens, "Some".to_string()),
                Token::None => Self::parse_generic_or_ident_type(tokens, "None".to_string()),
                Token::LParen => {
                    let mut args = Vec::new();
                    loop {
                        match tokens.peek() {
                            Some(TokenWithSpan { token: Token::RParen, .. }) => {
                                tokens.next();
                                break;
                            }
                            _ => {
                                args.push(Self::parse_type(tokens)?);
                                match tokens.peek() {
                                    Some(TokenWithSpan { token: Token::Comma, .. }) => {
                                        tokens.next();
                                    }
                                    Some(TokenWithSpan { token: Token::RParen, .. }) => {
                                        // will be consumed in next iteration
                                    }
                                    _ => break,
                                }
                            }
                        }
                    }
                    Ok(Type::Tuple(args))
                }
                _ => Err("Expected type".to_string()),
            },
            None => Err("Expected type".to_string()),
        }
    }

    fn parse_generic_or_ident_type(
        tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>,
        name: String,
    ) -> Result<Type, String> {
        if let Some(TokenWithSpan { token: Token::Lt, .. }) = tokens.peek() {
            tokens.next(); // consume <
            let mut args = Vec::new();
            loop {
                args.push(Self::parse_type(tokens)?);
                match tokens.peek() {
                    Some(TokenWithSpan { token: Token::Gt, .. }) => {
                        tokens.next();
                        break;
                    }
                    Some(TokenWithSpan { token: Token::Comma, .. }) => {
                        tokens.next();
                    }
                    _ => break,
                }
            }
            Ok(Type::Generic(name, args))
        } else {
            Ok(Type::Ident(name))
        }
    }

    fn parse_block(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Block, String> {
        let mut statements = Vec::new();

        while let Some(t) = tokens.peek() {
            match t.token {
                Token::RBrace => break,
                Token::Let => {
                    let stmt = Self::parse_let_stmt(tokens)?;
                    statements.push(Stmt::Let(stmt));
                }
                Token::Return => {
                    let stmt = Self::parse_return_stmt(tokens)?;
                    statements.push(Stmt::Return(stmt));
                }
                Token::Using => {
                    let stmt = Self::parse_using_stmt(tokens)?;
                    statements.push(Stmt::Using(stmt));
                }
                Token::Continue => {
                    tokens.next();
                    if let Some(t) = tokens.peek() {
                        if matches!(t.token, Token::Semicolon) { tokens.next(); }
                    }
                    statements.push(Stmt::Continue);
                }
                Token::Break => {
                    tokens.next();
                    if let Some(t) = tokens.peek() {
                        if matches!(t.token, Token::Semicolon) { tokens.next(); }
                    }
                    statements.push(Stmt::Break);
                }
                _ => {
                    let expr = Self::parse_expr(tokens)?;
                    statements.push(Stmt::Expr(expr));
                }
            }
        }

        Ok(Block { statements, span: None })
    }

    fn parse_block_with_span(
        tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>,
        start_span: Span,
    ) -> Result<Block, String> {
        let mut block = Self::parse_block(tokens)?;
        block.span = Some(start_span);
        Ok(block)
    }

    fn parse_let_stmt(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<LetStmt, String> {
    // let
    let let_token = tokens.next().ok_or_else(|| "Expected 'let' token".to_string())?;
    let start_span = Span::from(let_token.span);
        
        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected variable name".to_string()),
            },
            None => return Err("Expected variable name".to_string()),
        };

        // type annotation
        let type_annotation = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Colon) {
                tokens.next(); // consume :
                Some(Self::parse_type(tokens)?)
            } else {
                None
            }
        } else {
            None
        };

        // =
        if !matches!(tokens.next().map(|t| t.token), Some(Token::Assign)) {
            return Err("Expected '='".to_string());
        }

        // value
        let value = Self::parse_expr(tokens)?;

        // ;
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Semicolon) {
                tokens.next();
            }
        }

        Ok(LetStmt {
            name,
            type_annotation,
            value,
            span: Some(start_span),
        })
    }

    fn parse_return_stmt(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<ReturnStmt, String> {
        // return
        let return_token = tokens.next().ok_or_else(|| "Expected 'return' token".to_string())?;
        let start_span = Span::from(return_token.span);

        let value = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Semicolon) || matches!(t.token, Token::RBrace) {
                None
            } else {
                Some(Self::parse_expr(tokens)?)
            }
        } else {
            None
        };

        // ;
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Semicolon) {
                tokens.next();
            }
        }

Ok(ReturnStmt { value, span: Some(start_span) })
    }

    fn parse_using_stmt(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<UsingStmt, String> {
        // using
        let using_token = tokens.next().ok_or_else(|| "Expected 'using' token".to_string())?;
        let start_span = Span::from(using_token.span);

        // name
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected variable name after 'using'".to_string()),
            },
            None => return Err("Expected variable name after 'using'".to_string()),
        };

        // =
        match tokens.next() {
            Some(TokenWithSpan { token: Token::Assign, .. }) => {}
            _ => return Err("Expected '=' after variable name".to_string()),
        }

        // init expression
        let init = Self::parse_expr(tokens)?;

        // { block }
        let body = Self::parse_block_with_span(tokens, start_span.clone())?;

        Ok(UsingStmt { name, init, body, span: Some(start_span) })
    }

    fn parse_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        Self::parse_pipeline_expr(tokens)
    }

    // Precedence level 1: Pipeline |>
    fn parse_pipeline_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_or_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::Pipeline) {
                    tokens.next(); // consume |>
                    let right = Self::parse_or_expr(tokens)?;
                    // Pipeline: x |> f(y) desugars to f(x, y) or f(y)(x)
                    // For simplicity, we represent it as a call with the piped value as first arg
                    if let Expr::Call { function, mut args } = right {
                        args.insert(0, left);
                        left = Expr::Call { function, args };
                    } else if let Expr::Ident(function) = right {
                        left = Expr::Call { function, args: vec![left] };
                    } else {
                        return Err("Expected function call after pipeline operator".to_string());
                    }
                    continue;
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 2: Logical OR ||
    fn parse_or_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_and_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::OrOr) {
                    tokens.next();
                    let right = Self::parse_and_expr(tokens)?;
                    left = Expr::Binary { left: Box::new(left), op: BinaryOp::Or, right: Box::new(right) };
                    continue;
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 3: Logical AND &&
    fn parse_and_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_eq_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::AndAnd) {
                    tokens.next();
                    let right = Self::parse_eq_expr(tokens)?;
                    left = Expr::Binary { left: Box::new(left), op: BinaryOp::And, right: Box::new(right) };
                    continue;
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 4: Equality ==, !=
    fn parse_eq_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_cmp_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                match t.token {
                    Token::EqEq => {
                        tokens.next();
                        let right = Self::parse_cmp_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Eq, right: Box::new(right) };
                        continue;
                    }
                    Token::Ne => {
                        tokens.next();
                        let right = Self::parse_cmp_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Ne, right: Box::new(right) };
                        continue;
                    }
                    _ => {}
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 5: Comparison <, <=, >, >=
    fn parse_cmp_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_add_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                match t.token {
                    Token::Lt => {
                        tokens.next();
                        let right = Self::parse_add_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Lt, right: Box::new(right) };
                        continue;
                    }
                    Token::Le => {
                        tokens.next();
                        let right = Self::parse_add_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Le, right: Box::new(right) };
                        continue;
                    }
                    Token::Gt => {
                        tokens.next();
                        let right = Self::parse_add_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Gt, right: Box::new(right) };
                        continue;
                    }
                    Token::Ge => {
                        tokens.next();
                        let right = Self::parse_add_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Ge, right: Box::new(right) };
                        continue;
                    }
                    _ => {}
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 6: Additive +, -
    fn parse_add_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_mul_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                match t.token {
                    Token::Plus => {
                        tokens.next();
                        let right = Self::parse_mul_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Add, right: Box::new(right) };
                        continue;
                    }
                    Token::Minus => {
                        tokens.next();
                        let right = Self::parse_mul_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Sub, right: Box::new(right) };
                        continue;
                    }
                    _ => {}
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 7: Multiplicative *, /, %
    fn parse_mul_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut left = Self::parse_prefix_expr(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                match t.token {
                    Token::Star => {
                        tokens.next();
                        let right = Self::parse_prefix_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Mul, right: Box::new(right) };
                        continue;
                    }
                    Token::Slash => {
                        tokens.next();
                        let right = Self::parse_prefix_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Div, right: Box::new(right) };
                        continue;
                    }
                    Token::Percent => {
                        tokens.next();
                        let right = Self::parse_prefix_expr(tokens)?;
                        left = Expr::Binary { left: Box::new(left), op: BinaryOp::Div, right: Box::new(right) };
                        continue;
                    }
                    _ => {}
                }
            }
            break;
        }
        Ok(left)
    }

    // Precedence level 8: Prefix -, !, await, try
    fn parse_prefix_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        if let Some(t) = tokens.peek() {
            match t.token {
                Token::Minus => {
                    tokens.next();
                    let expr = Self::parse_prefix_expr(tokens)?;
                    // Represent negation as 0 - expr
                    Ok(Expr::Binary {
                        left: Box::new(Expr::Literal(Literal::Int(0))),
                        op: BinaryOp::Sub,
                        right: Box::new(expr),
                    })
                }
                Token::Bang => {
                    tokens.next();
                    let expr = Self::parse_prefix_expr(tokens)?;
                    // Represent !expr as expr == false
                    Ok(Expr::Binary {
                        left: Box::new(expr),
                        op: BinaryOp::Eq,
                        right: Box::new(Expr::Literal(Literal::Bool(false))),
                    })
                }
                Token::Await => {
                    tokens.next();
                    let expr = Self::parse_prefix_expr(tokens)?;
                    Ok(Expr::Call { function: "await".to_string(), args: vec![expr] })
                }
                Token::Try => {
                    tokens.next();
                    let expr = Self::parse_prefix_expr(tokens)?;
                    Ok(Expr::Try(Box::new(expr)))
                }
                _ => Self::parse_postfix_expr(tokens),
            }
        } else {
            Err("Unexpected end of input".to_string())
        }
    }

    // Precedence level 9: Postfix function calls, array indexing
    fn parse_postfix_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        let mut expr = Self::parse_primary(tokens)?;
        loop {
            if let Some(t) = tokens.peek() {
                match t.token {
                    Token::LParen => {
                        // Function call
                        tokens.next(); // consume (
                        let mut args = Vec::new();
                        while let Some(t) = tokens.peek() {
                            if matches!(t.token, Token::RParen) {
                                break;
                            }
                            let arg = Self::parse_expr(tokens)?;
                            args.push(arg);
                            if let Some(t) = tokens.peek() {
                                if matches!(t.token, Token::Comma) {
                                    tokens.next();
                                }
                            }
                        }
                        if !matches!(tokens.next().map(|t| t.token), Some(Token::RParen)) {
                            return Err("Expected ')'".to_string());
                        }
                        if let Expr::Ident(name) = expr {
                            expr = Expr::Call { function: name, args };
                        } else {
                            // For now, only support direct function calls
                            return Err("Expected function name before '('".to_string());
                        }
                        continue;
                    }
                    Token::LBracket => {
                        // Array indexing
                        tokens.next(); // consume [
                        let index = Self::parse_expr(tokens)?;
                        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBracket)) {
                            return Err("Expected ']'".to_string());
                        }
                        expr = Expr::Index { base: Box::new(expr), index: Box::new(index) };
                        continue;
                    }
                    Token::Dot => {
                        // Field access: expr.field
                        tokens.next(); // consume .
                        let field = match tokens.next() {
                            Some(t) => match t.token {
                                Token::Ident(name) => name,
                                _ => return Err("Expected field name after '.'".to_string()),
                            },
                            None => return Err("Expected field name after '.'".to_string()),
                        };
                        expr = Expr::FieldAccess { base: Box::new(expr), field };
                        continue;
                    }
                    Token::LBrace => {
                        // Struct literal: only if the ident starts with uppercase (Once convention)
                        if let Expr::Ident(name) = &expr {
                            if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                                let name = name.clone();
                                tokens.next(); // consume {
                                let mut fields = Vec::new();
                                while let Some(t) = tokens.peek() {
                                    if matches!(t.token, Token::RBrace) {
                                        tokens.next();
                                        break;
                                    }
                                    let field_name = match tokens.next() {
                                        Some(t) => match t.token {
                                            Token::Ident(name) => name,
                                            _ => return Err("Expected field name".to_string()),
                                        },
                                        None => return Err("Expected field name".to_string()),
                                    };
                                    if !matches!(tokens.next().map(|t| t.token), Some(Token::Colon)) {
                                        return Err("Expected ':' after field name".to_string());
                                    }
                                    let value = Self::parse_expr(tokens)?;
                                    fields.push((field_name, value));
                                    if let Some(t) = tokens.peek() {
                                        if matches!(t.token, Token::Comma) {
                                            tokens.next();
                                        }
                                    }
                                }
                                expr = Expr::Struct { name, fields };
                                continue;
                            }
                        }
                        break;
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    // Precedence level 10: Primary expressions
    fn parse_primary(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        match tokens.peek() {
            Some(t) => match &t.token {
                Token::IntLit(n) => {
                    let n = *n;
                    tokens.next();
                    Ok(Expr::Literal(Literal::Int(n)))
                }
                Token::FloatLit(n) => {
                    let n = *n;
                    tokens.next();
                    Ok(Expr::Literal(Literal::Float(n)))
                }
                Token::StringLit(s) => {
                    let s = s.clone();
                    tokens.next();
                    Ok(Expr::Literal(Literal::String(s)))
                }
                Token::True => {
                    tokens.next();
                    Ok(Expr::Literal(Literal::Bool(true)))
                }
                Token::False => {
                    tokens.next();
                    Ok(Expr::Literal(Literal::Bool(false)))
                }
                Token::Unit => {
                    tokens.next();
                    Ok(Expr::Literal(Literal::Unit))
                }
                Token::Ident(name) => {
                    let name = name.clone();
                    tokens.next();
                    Ok(Expr::Ident(name))
                }
                Token::Vec | Token::Option | Token::Result | Token::Chan | Token::Actor | Token::File |
                Token::Ok | Token::Err | Token::Some | Token::None => {
                    let name = format!("{:?}", t.token);
                    tokens.next();
                    Ok(Expr::Ident(name))
                }
                Token::LParen => {
                    tokens.next(); // consume (
                    let expr = Self::parse_expr(tokens)?;
                    if !matches!(tokens.next().map(|t| t.token), Some(Token::RParen)) {
                        return Err("Expected ')'".to_string());
                    }
                    Ok(expr)
                }
                Token::LBrace => {
                    let lb = tokens.next().ok_or_else(|| "Expected '{'".to_string())?;
                    let block = Self::parse_block_with_span(tokens, Span::from(lb.span))?;
                    if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
                        return Err("Expected '}'".to_string());
                    }
                    Ok(Expr::Block(block))
                }
                Token::If => Self::parse_if_expr(tokens),
                Token::Match => Self::parse_match_expr(tokens),
                Token::For => Self::parse_for_expr(tokens),
                Token::While => Self::parse_while_expr(tokens),
                Token::Spawn => {
                    tokens.next(); // consume spawn
                    Self::expect_token(tokens, Token::LParen)?;
                    let expr = Self::parse_expr(tokens)?;
                    Self::expect_token(tokens, Token::RParen)?;
                    Ok(Expr::Call { function: "spawn".to_string(), args: vec![expr] })
                }
                _ => Err(format!("Unexpected token: {:?}", t.token)),
            },
            None => Err("Unexpected end of input".to_string()),
        }
    }

    fn parse_if_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        tokens.next(); // consume if
        let condition = Box::new(Self::parse_expr(tokens)?);

        // Then branch: expect a block or expression
        let then_branch = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::LBrace) {
                let lb = tokens.next().unwrap();
                let block = Self::parse_block_with_span(tokens, Span::from(lb.span))?;
                if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
                    return Err("Expected '}' after if branch".to_string());
                }
                block
            } else {
                // Single expression then branch
                let expr = Self::parse_expr(tokens)?;
                Block {
                    statements: vec![Stmt::Expr(expr)],
                    span: None,
                }
            }
        } else {
            return Err("Expected then branch after if condition".to_string());
        };

        // Else branch
        let else_branch = if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Else) {
                tokens.next(); // consume else
                if let Some(t) = tokens.peek() {
                    if matches!(t.token, Token::LBrace) {
                        let lb = tokens.next().unwrap();
                        let block = Self::parse_block_with_span(tokens, Span::from(lb.span))?;
                        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
                            return Err("Expected '}' after else branch".to_string());
                        }
                        Some(Box::new(Expr::Block(block)))
                    } else if matches!(t.token, Token::If) {
                        Some(Box::new(Self::parse_if_expr(tokens)?))
                    } else {
                        Some(Box::new(Self::parse_expr(tokens)?))
                    }
                } else {
                    return Err("Expected expression after else".to_string());
                }
            } else {
                None
            }
        } else {
            None
        };

        Ok(Expr::If { condition, then_branch, else_branch })
    }

    fn parse_match_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        tokens.next(); // consume match
        // Use parse_primary for the scrutinee to avoid struct-literal/postfix conflicts with '{'
        let expr = Box::new(Self::parse_primary(tokens)?);

        // Expect {
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after match expression".to_string());
        }

        let mut arms = Vec::new();
        loop {
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::RBrace) {
                    tokens.next();
                    break;
                }
            }

            let pattern = Self::parse_pattern(tokens)?;

            // Expect =>
            if !matches!(tokens.next().map(|t| t.token), Some(Token::FatArrow)) {
                return Err("Expected '=>' after match pattern".to_string());
            }

            let arm_expr = Self::parse_expr(tokens)?;
            arms.push((pattern, arm_expr));

            // Optional comma between arms
            if let Some(t) = tokens.peek() {
                if matches!(t.token, Token::Comma) {
                    tokens.next();
                }
            }
        }

        Ok(Expr::Match { expr, arms })
    }

    fn parse_for_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        tokens.next(); // consume for
        let item = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected identifier after 'for'".to_string()),
            },
            None => return Err("Expected identifier after 'for'".to_string()),
        };

        // Expect in
        if !matches!(tokens.next().map(|t| t.token), Some(Token::In)) {
            return Err("Expected 'in' after for item".to_string());
        }

        let collection = Box::new(Self::parse_expr(tokens)?);

        // Expect body block
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after for collection".to_string());
        }
        let body = Self::parse_block(tokens)?;
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}' after for body".to_string());
        }

        Ok(Expr::For { item, collection, body })
    }

    fn parse_while_expr(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Expr, String> {
        tokens.next(); // consume while
        let condition = Box::new(Self::parse_expr(tokens)?);

        // Expect body block
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{' after while condition".to_string());
        }
        let body = Self::parse_block(tokens)?;
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}' after while body".to_string());
        }

        Ok(Expr::While { condition, body })
    }

    fn parse_pattern(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Pattern, String> {
        match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => {
                    if name == "_" {
                        Ok(Pattern::Wildcard)
                    } else {
                        Ok(Pattern::Ident(name))
                    }
                }
                Token::IntLit(n) => Ok(Pattern::Literal(Literal::Int(n))),
                Token::FloatLit(n) => Ok(Pattern::Literal(Literal::Float(n))),
                Token::StringLit(s) => Ok(Pattern::Literal(Literal::String(s))),
                Token::True => Ok(Pattern::Literal(Literal::Bool(true))),
                Token::False => Ok(Pattern::Literal(Literal::Bool(false))),
                Token::Unit => Ok(Pattern::Literal(Literal::Unit)),
                _ => Err("Expected pattern".to_string()),
            },
            None => Err("Expected pattern".to_string()),
        }
    }

    fn parse_generic_params(tokens: &mut Peekable<IntoIter<TokenWithSpan>>) -> Result<Vec<GenericParam>, String> {
        let mut params = Vec::new();
        if let Some(t) = tokens.peek() {
            if matches!(t.token, Token::Lt) {
                tokens.next(); // consume <
                loop {
                    let param_token = tokens.next().ok_or_else(|| "Expected identifier in generic parameters".to_string())?;
                    let name = match param_token.token {
                        Token::Ident(name) => name,
                        _ => return Err("Expected identifier in generic parameters".to_string()),
                    };
                    let span = Some(Span::from(param_token.span));
                    
                    let mut bounds = Vec::new();
                    if let Some(t) = tokens.peek() {
                        if matches!(t.token, Token::Colon) {
                            tokens.next(); // consume :
                            loop {
                                bounds.push(Self::parse_type(tokens)?);
                                if let Some(t) = tokens.peek() {
                                    if matches!(t.token, Token::Plus) {
                                        tokens.next(); // consume +
                                        continue;
                                    }
                                }
                                break;
                            }
                        }
                    }
                    
                    params.push(GenericParam { name, bounds, span });
                    
                    match tokens.peek() {
                        Some(TokenWithSpan { token: Token::Comma, .. }) => { tokens.next(); }
                        Some(TokenWithSpan { token: Token::Gt, .. }) => { tokens.next(); break; }
                        _ => return Err("Expected ',' or '>' in generic parameters".to_string()),
                    }
                }
            }
        }
        Ok(params)
    }

    fn expect_token(tokens: &mut Peekable<IntoIter<TokenWithSpan>>, expected: Token) -> Result<(), String> {
        if let Some(token_with_span) = tokens.next() {
            if token_with_span.token == expected {
                Ok(())
            } else {
                Err(format!(
                    "Expected token {:?}, found {:?}",
                    expected, token_with_span.token
                ))
            }
        } else {
            Err(format!("Expected token {:?}, found end of input", expected))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_lex::Lexer;

    #[test]
    fn test_parse_simple_function() {
        let source = "fn main() -> Unit { return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let result = OnceParser::parse(tokens);
        
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);
        
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert_eq!(fn_decl.name, "main");
            assert_eq!(fn_decl.params.len(), 0);
            assert_eq!(fn_decl.return_type, Some(Type::Unit));
        } else {
            panic!("Expected function declaration");
        }
    }

    #[test]
    fn test_parse_goal_declaration() {
        let source = "goal shortest_path(graph: Graph, start: Node, end: Node) -> Path { return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let result = OnceParser::parse(tokens);

        assert!(result.is_ok(), "Parse failed: {:?}", result.err());
        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);

        if let Item::GoalDecl(goal_decl) = &program.items[0] {
            assert_eq!(goal_decl.name, "shortest_path");
            assert_eq!(goal_decl.params.len(), 3);
            assert_eq!(goal_decl.return_type, Some(Type::Ident("Path".to_string())));
        } else {
            panic!("Expected goal declaration, got {:?}", program.items[0]);
        }
    }

    #[test]
    fn test_parse_let_statement() {
        let source = "let x = 42;";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let result = OnceParser::parse(tokens);
        
        assert!(result.is_ok());
        let program = result.unwrap();
        assert_eq!(program.items.len(), 1);
        
        if let Item::LetDecl(let_decl) = &program.items[0] {
            assert_eq!(let_decl.name, "x");
            assert_eq!(let_decl.value, Expr::Literal(Literal::Int(42)));
        } else {
            panic!("Expected let declaration");
        }
    }

    #[test]
    fn test_span_propagation() {
        let source = "fn main() -> Unit { return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let result = OnceParser::parse(tokens).unwrap();
        // The top-level program should contain a single function declaration
        assert_eq!(result.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &result.items[0] {
            // Span should be populated for the function declaration
            assert!(fn_decl.span.is_some());
            // Basic sanity: start should be <= end
            if let Some(span) = fn_decl.span {
                assert!(span.start <= span.end);
            }
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_param() {
        let source = "fn add(a: Int) -> Int { return a }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let result = OnceParser::parse(tokens).unwrap();
        assert_eq!(result.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &result.items[0] {
            assert!(fn_decl.span.is_some());
            assert_eq!(fn_decl.params.len(), 1);
            let param = &fn_decl.params[0];
            assert!(param.span.is_some());
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_return() {
        let source = "fn main() -> Unit { return 1 }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert!(fn_decl.span.is_some());
            if let Some(first_stmt) = fn_decl.body.statements.get(0) {
                if let Stmt::Return(ret) = first_stmt {
                    assert!(ret.span.is_some());
                } else {
                    panic!("Expected Return statement in function body");
                }
            } else {
                panic!("Function body is empty");
            }
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_block() {
        let source = "fn main() -> Unit { let x = 42; return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert!(fn_decl.body.span.is_some());
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_block_multiline() {
        let source = "fn main() -> Unit {\n  let a = 1;\n  let b = 2;\n  return\n}";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert!(fn_decl.body.span.is_some());
            if let Some(span) = fn_decl.body.span {
                assert!(span.start <= span.end);
            }
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_inner_block_span() {
        // Ensure an inner block used as an expression has a span
        let source = "fn main() -> Unit { let x = { 1; }; }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            // first statement should be a Let with a Block as its value
            if let Some(Stmt::Let(let_stmt)) = fn_decl.body.statements.get(0) {
                if let Expr::Block(inner_block) = &let_stmt.value {
                    assert!(inner_block.span.is_some());
                } else {
                    panic!("Expected inner Block as value of Let statement");
                }
            } else {
                panic!("Expected Let statement in function body");
            }
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_nested_block() {
        // Nested blocks: { { 1 } }
        let source = "fn main() -> Unit { let x = { { 1 } }; return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert!(fn_decl.body.span.is_some());
            if let Some(Stmt::Let(let_decl)) = fn_decl.body.statements.get(0) {
                if let Expr::Block(outer_block) = &let_decl.value {
                    assert!(outer_block.span.is_some());
                    if let Some(stmt) = outer_block.statements.get(0) {
                        if let Stmt::Expr(Expr::Block(inner_block)) = stmt {
                            assert!(inner_block.span.is_some());
                        }
                    }
                }
            }
        } else {
            panic!("Expected FnDecl");
        }
    }

    #[test]
    fn test_span_propagation_deep_nested_block() {
        // Deep nested blocks: 3 levels
        let source = "fn main() -> Unit { let x = { { { 1 } } }; return }";
        let tokens: Vec<_> = Lexer::new(source).collect();
        let program = OnceParser::parse(tokens).unwrap();
        assert_eq!(program.items.len(), 1);
        if let Item::FnDecl(fn_decl) = &program.items[0] {
            assert!(fn_decl.body.span.is_some());
            if let Some(Stmt::Let(let_decl)) = fn_decl.body.statements.get(0) {
                if let Expr::Block(outer_block) = &let_decl.value {
                    assert!(outer_block.span.is_some());
                    if let Some(stmt) = outer_block.statements.get(0) {
                        if let Stmt::Expr(Expr::Block(mid_block)) = stmt {
                            assert!(mid_block.span.is_some());
                            if let Some(inner_stmt) = mid_block.statements.get(0) {
                                if let Stmt::Expr(Expr::Block(inner_block)) = inner_stmt {
                                    assert!(inner_block.span.is_some());
                                }
                            }
                        }
                    }
                }
            }
        } else {
            panic!("Expected FnDecl");
        }
    }
}
