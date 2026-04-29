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
}

/// Function declaration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<Type>,
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

/// Let declaration (module-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetDecl {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub value: Expr,
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
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add, Sub, Mul, Div,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
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
                Token::Fn => {
                    let fn_decl = Self::parse_fn_decl(&mut tokens)?;
                    items.push(Item::FnDecl(fn_decl));
                }
                Token::Let => {
                    let let_decl = Self::parse_let_decl(&mut tokens)?;
                    items.push(Item::LetDecl(let_decl));
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
            params,
            return_type,
            body,
            span: Some(start_span),
        })
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
                    
                    // Check if this is a function call
                    if let Some(t) = tokens.peek() {
                        if matches!(t.token, Token::LParen) {
                            tokens.next(); // consume (
                            let mut args = Vec::new();
                            
                            while let Some(t) = tokens.peek() {
                                match t.token {
                                    Token::RParen => break,
                                    _ => {
                                        let arg = Self::parse_expr(tokens)?;
                                        args.push(arg);
                                        if let Some(t) = tokens.peek() {
                                            if matches!(t.token, Token::Comma) {
                                                tokens.next();
                                            }
                                        }
                                    }
                                }
                            }
                            
                            if !matches!(tokens.next().map(|t| t.token), Some(Token::RParen)) {
                                return Err("Expected ')'".to_string());
                            }
                            
                            Ok(Expr::Call { function: name, args })
                        } else {
                            Ok(Expr::Ident(name))
                        }
                    } else {
                        Ok(Expr::Ident(name))
                    }
                }
                        Token::LBrace => {
                            // consume '{' and capture its span
                            let lb = tokens.next().ok_or_else(|| "Expected '{'".to_string())?;
                            if lb.token != Token::LBrace {
                                return Err("Expected '{'".to_string());
                            }
                            let block = Self::parse_block_with_span(tokens, Span::from(lb.span))?;
                            if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
                                return Err("Expected '}'".to_string());
                            }
                            Ok(Expr::Block(block))
                        }
                       Token::Spawn => {
                           tokens.next(); // consume spawn
                           Self::expect_token(tokens, Token::LParen)?;
                           let expr = Self::parse_expr(tokens)?;
                           Self::expect_token(tokens, Token::RParen)?;
                           Ok(Expr::Call { function: "spawn".to_string(), args: vec![expr] })
                       }
                       Token::Await => {
                           tokens.next(); // consume await
                           let expr = Self::parse_expr(tokens)?;
                           Ok(Expr::Call { function: "await".to_string(), args: vec![expr] })
                       }
                _ => Err(format!("Unexpected token: {:?}", t.token)),
            },
            None => Err("Unexpected end of input".to_string()),
        }
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
