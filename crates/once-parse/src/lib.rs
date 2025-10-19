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
}

/// Function parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_annotation: Option<Type>,
}

/// Let declaration (module-level)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetDecl {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub value: Expr,
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
}

/// Block of statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub statements: Vec<Stmt>,
}

/// Statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Stmt {
    Let(LetStmt),
    Return(ReturnStmt),
    Expr(Expr),
}

/// Let statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LetStmt {
    pub name: String,
    pub type_annotation: Option<Type>,
    pub value: Expr,
}

/// Return statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReturnStmt {
    pub value: Option<Expr>,
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
        tokens.next();
        
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
        if !matches!(tokens.next().map(|t| t.token), Some(Token::LBrace)) {
            return Err("Expected '{'".to_string());
        }

        // body
        let body = Self::parse_block(tokens)?;

        // }
        if !matches!(tokens.next().map(|t| t.token), Some(Token::RBrace)) {
            return Err("Expected '}'".to_string());
        }

        Ok(FnDecl {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_let_decl(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<LetDecl, String> {
        // let
        tokens.next();
        
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
        })
    }

    fn parse_param(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<Param, String> {
        let name = match tokens.next() {
            Some(t) => match t.token {
                Token::Ident(name) => name,
                _ => return Err("Expected parameter name".to_string()),
            },
            None => return Err("Expected parameter name".to_string()),
        };

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
                Token::Ident(name) => Ok(Type::Ident(name)),
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
                _ => {
                    let expr = Self::parse_expr(tokens)?;
                    statements.push(Stmt::Expr(expr));
                }
            }
        }

        Ok(Block { statements })
    }

    fn parse_let_stmt(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<LetStmt, String> {
        // let
        tokens.next();
        
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
        })
    }

    fn parse_return_stmt(tokens: &mut std::iter::Peekable<std::vec::IntoIter<TokenWithSpan>>) -> Result<ReturnStmt, String> {
        // return
        tokens.next();

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

        Ok(ReturnStmt { value })
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
                           tokens.next(); // consume {
                           let block = Self::parse_block(tokens)?;
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
}