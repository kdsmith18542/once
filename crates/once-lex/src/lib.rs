//! Lexer for the Once language
//! 
//! Provides tokenization of Once source code into a stream of tokens
//! with position information for error reporting.

use logos::Logos;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::hash::{Hash, Hasher};

/// A token in the Once language
#[derive(Logos, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[logos(skip r"[ \t\n\r]+")] // Skip whitespace
#[logos(skip r"//[^\n]*")]   // Skip line comments
#[logos(skip r"/\*([^*]|\*[^/])*\*/")] // Skip block comments
pub enum Token {
    // Keywords
    #[token("fn")]
    Fn,
    #[token("let")]
    Let,
    #[token("var")]
    Var,
    #[token("type")]
    Type,
    #[token("trait")]
    Trait,
    #[token("impl")]
    Impl,
    #[token("match")]
    Match,
    #[token("for")]
    For,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("return")]
    Return,
    #[token("using")]
    Using,
    #[token("lin")]
    Lin,
    #[token("aff")]
    Aff,
    #[token("spawn")]
    Spawn,
    #[token("async")]
    Async,
    #[token("await")]
    Await,
    #[token("true")]
    True,
    #[token("false")]
    False,
    #[token("Unit")]
    Unit,

    #[token("Int")]
    Int,
    #[token("Bool")]
    Bool,
    #[token("Float")]
    Float,
    #[token("Str")]
    Str,
    #[token("Vec")]
    Vec,
    #[token("Option")]
    Option,
    #[token("Result")]
    Result,
    #[token("Chan")]
    Chan,
    #[token("Actor")]
    Actor,
    #[token("File")]
    File,
    #[token("Ok")]
    Ok,
    #[token("Err")]
    Err,
    #[token("Some")]
    Some,
    #[token("None")]
    None,

    // Identifiers and literals
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Ident(String),
    
    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    IntLit(i64),
    
    #[regex(r"[0-9]+\\.[0-9]+", |lex| lex.slice().parse::<f64>().unwrap())]
    FloatLit(f64),
    
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    StringLit(String),

    // Operators
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,
    #[token("==")]
    EqEq,
    #[token("!=")]
    Ne,
    #[token("<")]
    Lt,
    #[token("<=")]
    Le,
    #[token(">")]
    Gt,
    #[token(">=")]
    Ge,
    #[token("&&")]
    AndAnd,
    #[token("||")]
    OrOr,
    #[token("!")]
    Bang,
    #[token("&")]
    And,
    #[token("|")]
    Or,
    #[token("^")]
    Caret,
    #[token("<<")]
    Shl,
    #[token(">>")]
    Shr,
    #[token("=")]
    Assign,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,
    #[token("*=")]
    StarEq,
    #[token("/=")]
    SlashEq,
    #[token("%=")]
    PercentEq,
    #[token("&=")]
    AndEq,
    #[token("|=")]
    OrEq,
    #[token("^=")]
    CaretEq,
    #[token("<<=")]
    ShlEq,
    #[token(">>=")]
    ShrEq,
    #[token("?")]
    Question,
    #[token("|>")]
    Pipeline,

    // Delimiters
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token(",")]
    Comma,
    #[token(";")]
    Semicolon,
    #[token(":")]
    Colon,
    #[token("::")]
    ColonColon,
    #[token(".")]
    Dot,
    #[token("..")]
    DotDot,
    #[token("...")]
    DotDotDot,
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("@")]
    At,

    // Error token for invalid input
    Error,
}

impl Eq for Token {}

impl Hash for Token {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Token::Ident(s) => {
                state.write_u8(0);
                s.hash(state);
            }
            Token::IntLit(n) => {
                state.write_u8(1);
                n.hash(state);
            }
            Token::FloatLit(n) => {
                state.write_u8(2);
                n.to_bits().hash(state);
            }
            Token::StringLit(s) => {
                state.write_u8(3);
                s.hash(state);
            }
            _ => {
                // For all other variants, hash based on discriminant
                std::mem::discriminant(self).hash(state);
            }
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Ident(s) => write!(f, "{}", s),
            Token::IntLit(n) => write!(f, "{}", n),
            Token::FloatLit(n) => write!(f, "{}", n),
            Token::StringLit(s) => write!(f, "\"{}\"", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// A token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

/// Position information for a token
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self { start, end, line, column }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }
}

/// Lexer for Once source code
pub struct Lexer<'source> {
    inner: logos::Lexer<'source, Token>,
    line: usize,
    column: usize,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self {
            inner: Token::lexer(source),
            line: 1,
            column: 1,
        }
    }

    fn update_position(&mut self, span: logos::Span) {
        // Count newlines in the token to update line/column
        let source = self.inner.source();
        let token_text = &source[span.start..span.end];
        
        for ch in token_text.chars() {
            if ch == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = TokenWithSpan;

    fn next(&mut self) -> Option<Self::Item> {
        let token = match self.inner.next()? {
            Ok(token) => token,
            Err(_) => return None,
        };
        let span = self.inner.span();
        
        let token_with_span = TokenWithSpan {
            token,
            span: Span::new(span.start, span.end, self.line, self.column),
        };

        self.update_position(span);
        Some(token_with_span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let source = "fn main() -> Unit { return }";
        let mut lexer = Lexer::new(source);
        
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].token, Token::Fn);
        assert_eq!(tokens[1].token, Token::Ident("main".to_string()));
        assert_eq!(tokens[2].token, Token::LParen);
        assert_eq!(tokens[3].token, Token::RParen);
        assert_eq!(tokens[4].token, Token::Arrow);
        assert_eq!(tokens[5].token, Token::Unit);
        assert_eq!(tokens[6].token, Token::LBrace);
        assert_eq!(tokens[7].token, Token::Return);
    }

    #[test]
    fn test_identifiers_and_literals() {
        let source = "let x = 42; let y = \"hello\";";
        let mut lexer = Lexer::new(source);
        
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens[1].token, Token::Ident("x".to_string()));
        assert_eq!(tokens[3].token, Token::IntLit(42));
        assert_eq!(tokens[6].token, Token::Ident("y".to_string()));
        assert_eq!(tokens[8].token, Token::StringLit("hello".to_string()));
    }

    #[test]
    fn test_operators() {
        let source = "x + y * z |> f";
        let mut lexer = Lexer::new(source);
        
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens[1].token, Token::Plus);
        assert_eq!(tokens[3].token, Token::Star);
        assert_eq!(tokens[5].token, Token::Pipeline);
    }
}
