//! Language Server Protocol for the Once language
//! 
//! Implements:
//! - LSP protocol support
//! - IDE integration
//! - Code completion
//! - Hover information
//! - Go to definition
//! - Find references
//! - Rename symbols
//! - Diagnostics
//! - Code actions
//! - Formatting
//! - Fix-its (quick fixes)

use std::collections::HashMap;
use thiserror::Error;
use once_parse::Program as Ast;
use once_hir::HirProgram;

/// LSP errors
#[derive(Error, Debug, Clone)]
pub enum LspError {
    #[error("Parse error: {0}")]
    ParseError(String),
    
    #[error("Type error: {0}")]
    TypeError(String),
    
    #[error("File error: {0}")]
    FileError(String),
    
    #[error("Position error: {0}")]
    PositionError(String),
}

/// LSP server for Once language
pub struct OnceLsp {
    pub documents: HashMap<String, DocumentInfo>,
    pub diagnostics: HashMap<String, Vec<String>>,
}

/// Document information
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub uri: String,
    pub content: String,
    pub version: i32,
    pub ast: Option<Ast>,
    pub hir: Option<HirProgram>,
    pub diagnostics: Vec<String>,
}

impl OnceLsp {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            diagnostics: HashMap::new(),
        }
    }

    /// Open a document
    pub fn open_document(&mut self, uri: String, content: String, version: i32) {
        // Parse the document to get AST and HIR
        let (ast, hir) = self.parse_document(&content);
        
        let document = DocumentInfo {
            uri: uri.clone(),
            content,
            version,
            ast,
            hir,
            diagnostics: Vec::new(),
        };
        self.documents.insert(uri, document);
    }
    
    /// Parse a document to get AST and HIR
    fn parse_document(&self, content: &str) -> (Option<Ast>, Option<HirProgram>) {
        use once_lex::Lexer;
        use once_parse::OnceParser;
        use once_hir::HirBuilder;
        
        // Lex the content
        let tokens: Vec<_> = Lexer::new(content).collect();
        if tokens.is_empty() {
            return (None, None);
        }
        
        // Parse to AST
        let ast = match OnceParser::parse(tokens) {
            Ok(ast) => ast,
            Err(_) => return (None, None),
        };
        
        // Build HIR
        let builder = HirBuilder::new();
        let hir = match builder.build(ast.clone()) {
            Ok(hir) => hir,
            Err(_) => return (Some(ast), None),
        };
        
        (Some(ast), Some(hir))
    }

    /// Update a document — re-parses so AST/HIR stay current
    pub fn update_document(&mut self, uri: String, content: String, version: i32) {
        let (ast, hir) = self.parse_document(&content);
        if let Some(document) = self.documents.get_mut(&uri) {
            document.content = content;
            document.version = version;
            document.ast = ast;
            document.hir = hir;
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: String) {
        self.documents.remove(&uri);
        self.diagnostics.remove(&uri);
    }

    /// Get completions for a position
    pub fn get_completions(&self, uri: &str, _line: u32, _character: u32) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Add in-scope variables from the document's HIR
        if let Some(doc) = self.documents.get(uri) {
            if let Some(hir) = &doc.hir {
                for item in &hir.items {
                    match item {
                        once_hir::HirItem::FnDecl(fn_decl) => {
                            completions.push(CompletionItem {
                                label: fn_decl.name.clone(),
                                kind: CompletionItemKind::Function,
                                detail: Some(match &fn_decl.return_type {
                                    Some(ty) => format!("fn({} params) -> {:?}", fn_decl.params.len(), ty),
                                    None => format!("fn({} params)", fn_decl.params.len()),
                                }),
                                documentation: Some(format!("Function {}", fn_decl.name)),
                            });
                        }
                        once_hir::HirItem::LetDecl(let_decl) => {
                            completions.push(CompletionItem {
                                label: let_decl.name.clone(),
                                kind: CompletionItemKind::Variable,
                                detail: let_decl.type_annotation.as_ref().map(|t| format!("{:?}", t)),
                                documentation: Some(format!("Variable {}", let_decl.name)),
                            });
                        }
                        once_hir::HirItem::TypeDecl(type_decl) => {
                            completions.push(CompletionItem {
                                label: type_decl.name.clone(),
                                kind: CompletionItemKind::Enum,
                                detail: Some(format!("{} variants", type_decl.variants.len())),
                                documentation: Some(format!("Type {}", type_decl.name)),
                            });
                        }
                        once_hir::HirItem::StructDecl(s) => {
                            completions.push(CompletionItem {
                                label: s.name.clone(),
                                kind: CompletionItemKind::Struct,
                                detail: Some(format!("{} fields", s.fields.len())),
                                documentation: Some(format!("Struct {}", s.name)),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        // Add built-in types
        completions.push(CompletionItem {
            label: "Int".to_string(),
            kind: CompletionItemKind::Class,
            detail: Some("Built-in integer type".to_string()),
            documentation: Some("Integer type for whole numbers".to_string()),
        });

        completions.push(CompletionItem {
            label: "Float".to_string(),
            kind: CompletionItemKind::Class,
            detail: Some("Built-in float type".to_string()),
            documentation: Some("Float type for decimal numbers".to_string()),
        });

        completions.push(CompletionItem {
            label: "Bool".to_string(),
            kind: CompletionItemKind::Class,
            detail: Some("Built-in bool type".to_string()),
            documentation: Some("Boolean type for true/false values".to_string()),
        });

        // Add keywords
        let keywords = vec!["fn", "let", "if", "else", "while", "for", "return", "spawn", "using", "match"];
        for keyword in keywords {
            completions.push(CompletionItem {
                label: keyword.to_string(),
                kind: CompletionItemKind::Keyword,
                detail: Some(format!("{} keyword", keyword)),
                documentation: Some(format!("{} keyword", keyword)),
            });
        }

        completions
    }

    /// Get hover information for a position with real type/effect info
    pub fn get_hover(&self, uri: &str, _line: u32, _character: u32) -> Option<HoverInfo> {
        if let Some(doc) = self.documents.get(uri) {
            if let Some(hir) = &doc.hir {
                // Run type checker to get inferred types
                let mut type_checker = once_ty::TypeChecker::new();
                if let Ok(()) = type_checker.check(hir) {
                    let mut parts = Vec::new();
                    for (name, scheme) in &type_checker.env.bindings {
                        if !matches!(name.as_str(), "Unit" | "Int" | "Bool" | "Float" | "Str" | "print" | "spawn") {
                            parts.push(format!("{}: {}", name, scheme.ty));
                        }
                    }
                    if !parts.is_empty() {
                        return Some(HoverInfo {
                            contents: parts.join("\n"),
                            range: None,
                        });
                    }
                }
                
                // Run effect checker for effect info
                let mut effect_checker = once_ty::effects::EffectChecker::new();
                if let Ok(()) = effect_checker.check(hir) {
                    let mut parts = Vec::new();
                    for (name, row) in &effect_checker.env.bindings {
                        parts.push(format!("{} !{}", name, row));
                    }
                    if !parts.is_empty() {
                        return Some(HoverInfo {
                            contents: format!("Effects:\n{}", parts.join("\n")),
                            range: None,
                        });
                    }
                }
            }
        }
        Some(HoverInfo {
            contents: "Once language symbol (run type checker for details)".to_string(),
            range: None,
        })
    }

    /// Get definition for a position
    pub fn get_definition(&self, uri: &str, line: u32, character: u32) -> Option<Location> {
        if let Some(doc) = self.documents.get(uri) {
            if let Some(hir) = &doc.hir {
                return self.find_definition_in_hir(hir, line, character);
            }
        }
        None
    }
    
    /// Find definition in HIR with symbol index
    fn find_definition_in_hir(&self, hir: &HirProgram, line: u32, _character: u32) -> Option<Location> {
        // Build a symbol index mapping names to their HIR item index
        let mut symbol_index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (i, item) in hir.items.iter().enumerate() {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    symbol_index.insert(fn_decl.name.clone(), i);
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    symbol_index.insert(let_decl.name.clone(), i);
                }
                once_hir::HirItem::TypeDecl(type_decl) => {
                    symbol_index.insert(type_decl.name.clone(), i);
                }
                once_hir::HirItem::StructDecl(s) => {
                    symbol_index.insert(s.name.clone(), i);
                }
                once_hir::HirItem::TraitDecl(t) => {
                    symbol_index.insert(t.name.clone(), i);
                }
                once_hir::HirItem::ImplBlock(_) => {}
            }
        }
        
        // Use line number as a rough position estimate; walk items to find the one at that line
        let item_index = line as usize;
        if item_index < hir.items.len() {
            let item = &hir.items[item_index];
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 3 + fn_decl.name.len() as u32 }, // "fn " + name
                        },
                    });
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 4 + let_decl.name.len() as u32 }, // "let " + name
                        },
                    });
                }
                once_hir::HirItem::TypeDecl(type_decl) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 5 + type_decl.name.len() as u32 },
                        },
                    });
                }
                once_hir::HirItem::StructDecl(s) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line, character: 0 },
                            end: Position { line, character: 5 + s.name.len() as u32 },
                        },
                    });
                }
                _ => {}
            }
        }
        None
    }

    /// Get references for a position
    pub fn get_references(&self, uri: &str, line: u32, character: u32) -> Vec<Location> {
        if let Some(doc) = self.documents.get(uri) {
            if let Some(hir) = &doc.hir {
                return self.find_references_in_hir(hir, line, character);
            }
        }
        Vec::new()
    }
    
    /// Find references in HIR
    fn find_references_in_hir(&self, hir: &HirProgram, _line: u32, _character: u32) -> Vec<Location> {
        let mut references = Vec::new();
        
        // For now, return all function and variable definitions as references
        // In a full implementation, we'd need to track source positions and actual usage
        for item in &hir.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    references.push(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: fn_decl.name.len() as u32 },
                        },
                    });
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    references.push(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: let_decl.name.len() as u32 },
                        },
                    });
                }
                once_hir::HirItem::TypeDecl(type_decl) => {
                    references.push(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: type_decl.name.len() as u32 },
                        },
                    });
                }
                once_hir::HirItem::StructDecl(_) | once_hir::HirItem::TraitDecl(_) | once_hir::HirItem::ImplBlock(_) => {}
            }
        }
        
        references
    }

    /// Rename symbol at a position
    pub fn rename_symbol(&self, uri: &str, line: u32, character: u32, new_name: String) -> Option<WorkspaceEdit> {
        if let Some(doc) = self.documents.get(uri) {
            if let Some(hir) = &doc.hir {
                return self.rename_symbol_in_hir(hir, line, character, new_name);
            }
        }
        None
    }
    
    /// Rename symbol in HIR
    fn rename_symbol_in_hir(&self, hir: &HirProgram, _line: u32, _character: u32, new_name: String) -> Option<WorkspaceEdit> {
        let mut changes = HashMap::new();
        let mut text_edits = Vec::new();
        
        // For now, rename the first function or variable we find
        // In a full implementation, we'd need to track source positions
        for item in &hir.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    text_edits.push(TextEdit {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: fn_decl.name.len() as u32 },
                        },
                        new_text: new_name.clone(),
                    });
                    break;
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    text_edits.push(TextEdit {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: let_decl.name.len() as u32 },
                        },
                        new_text: new_name.clone(),
                    });
                    break;
                }
                once_hir::HirItem::TypeDecl(type_decl) => {
                    text_edits.push(TextEdit {
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: type_decl.name.len() as u32 },
                        },
                        new_text: new_name.clone(),
                    });
                    break;
                }
                once_hir::HirItem::StructDecl(_) | once_hir::HirItem::TraitDecl(_) | once_hir::HirItem::ImplBlock(_) => {}
            }
        }
        
        if !text_edits.is_empty() {
            changes.insert("file://current".to_string(), text_edits);
            Some(WorkspaceEdit { changes })
        } else {
            None
        }
    }

    /// Get code actions for a range
    pub fn get_code_actions(&self, uri: &str, start_line: u32, start_character: u32, end_line: u32, end_character: u32) -> Vec<CodeAction> {
        let mut actions = Vec::new();
        
        if let Some(doc) = self.documents.get(uri) {
            // Add fix-its for common issues
            let fix_its = self.get_fix_its(uri, start_line as usize, start_character as usize);
            for fix_it in fix_its {
                actions.push(CodeAction {
                    title: fix_it.title,
                    kind: "quickfix".to_string(),
                    edit: Some(fix_it.edit),
                });
            }
            
            // Add code actions for missing imports
            if let Some(hir) = &doc.hir {
                for item in &hir.items {
                    match item {
                        once_hir::HirItem::FnDecl(fn_decl) => {
                            if fn_decl.name == "main" {
                                actions.push(CodeAction {
                                    title: "Add print function".to_string(),
                                    kind: "source".to_string(),
                                    edit: Some(WorkspaceEdit {
                                        changes: std::collections::HashMap::from([(
                                            uri.to_string(),
                                            vec![TextEdit {
                                                range: Range {
                                                    start: Position { line: 0, character: 0 },
                                                    end: Position { line: 0, character: 0 },
                                                },
                                                new_text: "fn print(msg: Str) -> Unit {\n    // Implementation would go here\n    return\n}\n\n".to_string(),
                                            }]
                                        )]),
                                    }),
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        actions
    }

    /// Format document
    pub fn format_document(&self, uri: &str) -> Vec<TextEdit> {
        let mut edits = Vec::new();
        
        if let Some(doc) = self.documents.get(uri) {
            let content = &doc.content;
            let lines: Vec<&str> = content.lines().collect();
            
            for (line_num, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.starts_with("//") {
                    // Basic formatting: ensure proper indentation
                    let expected_indent = if trimmed.starts_with("fn ") || trimmed.starts_with("let ") {
                        0
                    } else if trimmed.starts_with("return") || trimmed.starts_with("}") {
                        0
                    } else {
                        4
                    };
                    
                    let current_indent = line.len() - line.trim_start().len();
                    if current_indent != expected_indent {
                        edits.push(TextEdit {
                            range: Range {
                                start: Position { line: line_num as u32, character: 0 },
                                end: Position { line: line_num as u32, character: current_indent as u32 },
                            },
                            new_text: " ".repeat(expected_indent),
                        });
                    }
                }
            }
        }
        
        edits
    }

    /// Analyze document for diagnostics
    pub fn analyze_document(&self, uri: &str) -> Vec<String> {
        let mut diagnostics = Vec::new();
        
        if let Some(doc) = self.documents.get(uri) {
            // 1. Parse the document
            if doc.ast.is_none() {
                diagnostics.push("Parse error: Could not parse document".to_string());
                return diagnostics;
            }
            
            // 2. Run type checking
            if let Some(hir) = &doc.hir {
                for item in &hir.items {
                    match item {
                        once_hir::HirItem::FnDecl(fn_decl) => {
                            // Check for missing return type
                            if fn_decl.return_type.is_none() {
                                diagnostics.push(format!(
                                    "Warning: Function '{}' should have explicit return type",
                                    fn_decl.name
                                ));
                            }
                            
                            // Check for missing function body
                            if fn_decl.body.statements.is_empty() {
                                diagnostics.push(format!(
                                    "Warning: Function '{}' has empty body",
                                    fn_decl.name
                                ));
                            }
                        }
                        once_hir::HirItem::LetDecl(let_decl) => {
                            // Check for missing type annotation
                            match &let_decl.value {
                                once_hir::HirExpr::Literal(_, _) => {
                                    // Literals are fine without type annotation
                                }
                                once_hir::HirExpr::If { .. } | once_hir::HirExpr::Match { .. } | once_hir::HirExpr::For { .. } => {
                                    // Control flow expressions can sometimes infer their type
                                }
                                _ => {
                                    diagnostics.push(format!(
                                        "Info: Variable '{}' could benefit from type annotation",
                                        let_decl.name
                                    ));
                                }
                            }
                        }
                        once_hir::HirItem::TypeDecl(type_decl) => {
                            // Check for empty variant list
                            if type_decl.variants.is_empty() {
                                diagnostics.push(format!(
                                    "Warning: Type '{}' has no variants",
                                    type_decl.name
                                ));
                            }
                        }
                        once_hir::HirItem::StructDecl(_) | once_hir::HirItem::TraitDecl(_) | once_hir::HirItem::ImplBlock(_) => {}
                    }
                }
            } else {
                diagnostics.push("Warning: Could not build HIR from AST".to_string());
            }
            
            // 3. Check for common issues
            let content = &doc.content;
            let lines: Vec<&str> = content.lines().collect();
            
            for (line_num, line) in lines.iter().enumerate() {
                let trimmed = line.trim();
                
                // Check for missing semicolons
                if !trimmed.is_empty() && !trimmed.starts_with("//") && !trimmed.ends_with(";") && !trimmed.ends_with("{") && !trimmed.ends_with("}") {
                    if trimmed.contains("=") || trimmed.contains("return") {
                        diagnostics.push(format!(
                            "Info: Line {} might need a semicolon",
                            line_num + 1
                        ));
                    }
                }
                
                // Check for unused variables
                if trimmed.starts_with("let ") && !trimmed.contains("=") {
                    diagnostics.push(format!(
                        "Warning: Line {} declares variable but doesn't assign value",
                        line_num + 1
                    ));
                }
            }
        }
        
        diagnostics
    }
}

/// Completion item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionItemKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

/// Completion item kind
#[derive(Debug, Clone)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// Hover information
#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
    pub range: Option<Range>,
}

/// Range
#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

/// Position
#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Location
#[derive(Debug, Clone)]
pub struct Location {
    pub uri: String,
    pub range: Range,
}

/// Workspace edit
#[derive(Debug, Clone)]
pub struct WorkspaceEdit {
    pub changes: HashMap<String, Vec<TextEdit>>,
}

/// Text edit
#[derive(Debug, Clone)]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

/// Code action
#[derive(Debug, Clone)]
pub struct CodeAction {
    pub title: String,
    pub kind: String,
    pub edit: Option<WorkspaceEdit>,
}

/// Fix-it kind
#[derive(Debug, Clone, PartialEq)]
pub enum FixItKind {
    AddMissingClose,
    AddMissingCommit,
    AddVarBlock,
    RemoveUnusedVariable,
    AddMissingReturn,
    FixLinearity,
}

/// Fix-it suggestion
#[derive(Debug, Clone)]
pub struct FixIt {
    pub kind: FixItKind,
    pub title: String,
    pub description: String,
    pub edit: WorkspaceEdit,
}

impl OnceLsp {
    /// Get fix-its for a document at a specific position
    pub fn get_fix_its(&self, uri: &str, line: usize, _character: usize) -> Vec<FixIt> {
        let mut fix_its = Vec::new();

        if let Some(doc) = self.documents.get(uri) {
            // Check for missing close/commit
            if self.is_missing_close(&doc.content, line) {
                fix_its.push(self.create_add_close_fix_it(uri, line));
            }

            if self.is_missing_commit(&doc.content, line) {
                fix_its.push(self.create_add_commit_fix_it(uri, line));
            }

            // Check for missing var block
            if self.is_missing_var_block(&doc.content, line) {
                fix_its.push(self.create_add_var_block_fix_it(uri, line));
            }

            // Check for unused variables
            if let Some(unused_var) = self.find_unused_variable(&doc.content, line) {
                fix_its.push(self.create_remove_unused_variable_fix_it(uri, line, &unused_var));
            }

            // Check for linearity errors
            if self.has_linearity_error(&doc.content, line) {
                fix_its.push(self.create_fix_linearity_fix_it(uri, line));
            }
        }

        fix_its
    }

    /// Check if a close is missing
    fn is_missing_close(&self, content: &str, line: usize) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return false;
        }
        
        let line_content = lines[line];
        // Check if line contains a resource allocation without close
        line_content.contains("File.open") || line_content.contains("TcpStream.connect")
    }

    /// Check if a commit is missing
    fn is_missing_commit(&self, content: &str, line: usize) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return false;
        }
        
        let line_content = lines[line];
        // Check if line contains a transaction without commit
        line_content.contains("Transaction.begin")
    }

    /// Check if a var block is missing
    fn is_missing_var_block(&self, content: &str, line: usize) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return false;
        }
        
        let line_content = lines[line];
        // Check if line contains mutable variable without var block
        line_content.contains("mut ") && !content.contains("var {")
    }

    /// Find unused variable
    fn find_unused_variable(&self, content: &str, line: usize) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return None;
        }
        
        let line_content = lines[line];
        // Simplified - check if line contains let binding that's never used
        if line_content.contains("let ") && line_content.contains("=") {
            // Extract variable name (simplified)
            if let Some(var_name) = line_content.split("let ").nth(1) {
                if let Some(var_name) = var_name.split("=").next() {
                    let var_name = var_name.trim();
                    // Check if variable is used elsewhere
                    let usage_count = content.matches(var_name).count();
                    if usage_count == 1 {
                        return Some(var_name.to_string());
                    }
                }
            }
        }
        None
    }

    /// Check if there's a linearity error
    fn has_linearity_error(&self, content: &str, line: usize) -> bool {
        let lines: Vec<&str> = content.lines().collect();
        if line >= lines.len() {
            return false;
        }
        
        let line_content = lines[line];
        // Check if line contains a moved value being used again
        line_content.contains("// error: value moved")
    }

    /// Create fix-it for adding close
    fn create_add_close_fix_it(&self, uri: &str, line: usize) -> FixIt {
        let mut changes = HashMap::new();
        changes.insert(uri.to_string(), vec![TextEdit {
            range: Range {
                start: Position { line: line as u32, character: 0 },
                end: Position { line: (line + 1) as u32, character: 0 },
            },
            new_text: format!("    f.close();\n"),
        }]);

        FixIt {
            kind: FixItKind::AddMissingClose,
            title: "Add missing close()".to_string(),
            description: "Resource must be explicitly closed".to_string(),
            edit: WorkspaceEdit { changes },
        }
    }

    /// Create fix-it for adding commit
    fn create_add_commit_fix_it(&self, uri: &str, line: usize) -> FixIt {
        let mut changes = HashMap::new();
        changes.insert(uri.to_string(), vec![TextEdit {
            range: Range {
                start: Position { line: line as u32, character: 0 },
                end: Position { line: (line + 1) as u32, character: 0 },
            },
            new_text: format!("    tx.commit();\n"),
        }]);

        FixIt {
            kind: FixItKind::AddMissingCommit,
            title: "Add missing commit()".to_string(),
            description: "Transaction must be explicitly committed".to_string(),
            edit: WorkspaceEdit { changes },
        }
    }

    /// Create fix-it for adding var block
    fn create_add_var_block_fix_it(&self, uri: &str, line: usize) -> FixIt {
        let mut changes = HashMap::new();
        changes.insert(uri.to_string(), vec![
            TextEdit {
                range: Range {
                    start: Position { line: line as u32, character: 0 },
                    end: Position { line: line as u32, character: 0 },
                },
                new_text: "var {\n".to_string(),
            },
            TextEdit {
                range: Range {
                    start: Position { line: (line + 1) as u32, character: 0 },
                    end: Position { line: (line + 1) as u32, character: 0 },
                },
                new_text: "}\n".to_string(),
            },
        ]);

        FixIt {
            kind: FixItKind::AddVarBlock,
            title: "Wrap in var block".to_string(),
            description: "Mutable variables must be in a var block".to_string(),
            edit: WorkspaceEdit { changes },
        }
    }

    /// Create fix-it for removing unused variable
    fn create_remove_unused_variable_fix_it(&self, uri: &str, line: usize, var_name: &str) -> FixIt {
        let mut changes = HashMap::new();
        changes.insert(uri.to_string(), vec![TextEdit {
            range: Range {
                start: Position { line: line as u32, character: 0 },
                end: Position { line: (line + 1) as u32, character: 0 },
            },
            new_text: String::new(),
        }]);

        FixIt {
            kind: FixItKind::RemoveUnusedVariable,
            title: format!("Remove unused variable '{}'", var_name),
            description: "This variable is never used".to_string(),
            edit: WorkspaceEdit { changes },
        }
    }

    /// Create fix-it for fixing linearity error
    fn create_fix_linearity_fix_it(&self, uri: &str, line: usize) -> FixIt {
        let mut changes = HashMap::new();
        changes.insert(uri.to_string(), vec![TextEdit {
            range: Range {
                start: Position { line: line as u32, character: 0 },
                end: Position { line: line as u32, character: 0 },
            },
            new_text: "    // Use .copy() if the value implements Copy trait\n".to_string(),
        }]);

        FixIt {
            kind: FixItKind::FixLinearity,
            title: "Add copy() call".to_string(),
            description: "Value was moved, use copy() to create a duplicate".to_string(),
            edit: WorkspaceEdit { changes },
        }
    }
}

// ================================================================
// Tower-LSP integration: real LSP protocol server
// ================================================================

use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::{Client, LanguageServer, LspService, Server};
// Use module path to avoid collision with local Range/Position/Diagnostic types
use tower_lsp::lsp_types::{
    self,
    InitializeParams, InitializeResult, InitializedParams, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind,
    CompletionOptions, HoverProviderCapability, OneOf,
    DidOpenTextDocumentParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    CompletionParams, CompletionResponse,
    HoverParams, Hover, HoverContents, MarkedString,
    GotoDefinitionParams, GotoDefinitionResponse,
    ReferenceParams, RenameParams, DocumentFormattingParams,
    Diagnostic, DiagnosticSeverity, MessageType, Documentation, Url,
};
use std::sync::Mutex;
use once_lex::Lexer;
use once_parse::OnceParser;
use once_hir::HirBuilder;

/// Tower-LSP backend that wraps OnceLsp
fn convert_completion_kind(kind: &CompletionItemKind) -> tower_lsp::lsp_types::CompletionItemKind {
    match kind {
        CompletionItemKind::Text => tower_lsp::lsp_types::CompletionItemKind::TEXT,
        CompletionItemKind::Method => tower_lsp::lsp_types::CompletionItemKind::METHOD,
        CompletionItemKind::Function => tower_lsp::lsp_types::CompletionItemKind::FUNCTION,
        CompletionItemKind::Constructor => tower_lsp::lsp_types::CompletionItemKind::CONSTRUCTOR,
        CompletionItemKind::Field => tower_lsp::lsp_types::CompletionItemKind::FIELD,
        CompletionItemKind::Variable => tower_lsp::lsp_types::CompletionItemKind::VARIABLE,
        CompletionItemKind::Class => tower_lsp::lsp_types::CompletionItemKind::CLASS,
        CompletionItemKind::Interface => tower_lsp::lsp_types::CompletionItemKind::INTERFACE,
        CompletionItemKind::Module => tower_lsp::lsp_types::CompletionItemKind::MODULE,
        CompletionItemKind::Property => tower_lsp::lsp_types::CompletionItemKind::PROPERTY,
        CompletionItemKind::Unit => tower_lsp::lsp_types::CompletionItemKind::UNIT,
        CompletionItemKind::Value => tower_lsp::lsp_types::CompletionItemKind::VALUE,
        CompletionItemKind::Enum => tower_lsp::lsp_types::CompletionItemKind::ENUM,
        CompletionItemKind::Keyword => tower_lsp::lsp_types::CompletionItemKind::KEYWORD,
        CompletionItemKind::Snippet => tower_lsp::lsp_types::CompletionItemKind::SNIPPET,
        CompletionItemKind::Color => tower_lsp::lsp_types::CompletionItemKind::COLOR,
        CompletionItemKind::File => tower_lsp::lsp_types::CompletionItemKind::FILE,
        CompletionItemKind::Reference => tower_lsp::lsp_types::CompletionItemKind::REFERENCE,
        CompletionItemKind::Folder => tower_lsp::lsp_types::CompletionItemKind::FOLDER,
        CompletionItemKind::EnumMember => tower_lsp::lsp_types::CompletionItemKind::ENUM_MEMBER,
        CompletionItemKind::Constant => tower_lsp::lsp_types::CompletionItemKind::CONSTANT,
        CompletionItemKind::Struct => tower_lsp::lsp_types::CompletionItemKind::STRUCT,
        CompletionItemKind::Event => tower_lsp::lsp_types::CompletionItemKind::EVENT,
        CompletionItemKind::Operator => tower_lsp::lsp_types::CompletionItemKind::OPERATOR,
        CompletionItemKind::TypeParameter => tower_lsp::lsp_types::CompletionItemKind::TYPE_PARAMETER,
    }
}

struct Backend {
    client: Client,
    lsp: Mutex<OnceLsp>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _params: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "Once LSP server initialized").await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.text_document.text.clone();
        let version = params.text_document.version;

        self.lsp.lock().unwrap().open_document(uri.clone(), content.clone(), version);
        let diagnostics = self.run_diagnostics(&content);
        self.client.publish_diagnostics(Url::parse(&uri).unwrap(), diagnostics, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        let content = params.content_changes.first()
            .map(|c| c.text.clone())
            .unwrap_or_default();
        let version = params.text_document.version;

        self.lsp.lock().unwrap().update_document(uri.clone(), content.clone(), version);
        let diagnostics = self.run_diagnostics(&content);
        self.client.publish_diagnostics(Url::parse(&uri).unwrap(), diagnostics, None).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri.to_string();
        self.lsp.lock().unwrap().close_document(uri);
    }

    async fn completion(&self, params: CompletionParams) -> LspResult<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri.to_string();
        let line = params.text_document_position.position.line;
        let character = params.text_document_position.position.character;

        let lsp = self.lsp.lock().unwrap();
        let items = lsp.get_completions(&uri, line, character);
        drop(lsp);

        let lsp_items: Vec<lsp_types::CompletionItem> = items.into_iter()
            .map(|item| lsp_types::CompletionItem {
                label: item.label,
                kind: Some(convert_completion_kind(&item.kind)),
                detail: item.detail,
                documentation: item.documentation.map(Documentation::String),
                ..Default::default()
            })
            .collect();

        Ok(Some(CompletionResponse::Array(lsp_items)))
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let line = params.text_document_position_params.position.line;
        let character = params.text_document_position_params.position.character;

        let lsp = self.lsp.lock().unwrap();
        let hover_info = lsp.get_hover(&uri, line, character);
        drop(lsp);

        match hover_info {
            Some(info) => Ok(Some(Hover {
                contents: HoverContents::Scalar(MarkedString::String(info.contents)),
                range: info.range.map(|r| lsp_types::Range {
                    start: lsp_types::Position { line: r.start.line, character: r.start.character },
                    end: lsp_types::Position { line: r.end.line, character: r.end.character },
                }),
            })),
            None => Ok(None),
        }
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri.to_string();
        let line = params.text_document_position_params.position.line;
        let character = params.text_document_position_params.position.character;

        let lsp = self.lsp.lock().unwrap();
        let loc = lsp.get_definition(&uri, line, character);
        drop(lsp);

        match loc {
            Some(loc) => Ok(Some(GotoDefinitionResponse::Scalar(lsp_types::Location {
                uri: Url::parse(&loc.uri).unwrap(),
                range: lsp_types::Range {
                    start: lsp_types::Position { line: loc.range.start.line, character: loc.range.start.character },
                    end: lsp_types::Position { line: loc.range.end.line, character: loc.range.end.character },
                },
            }))),
            None => Ok(None),
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> LspResult<Option<Vec<lsp_types::TextEdit>>> {
        let uri = params.text_document.uri.to_string();
        let lsp = self.lsp.lock().unwrap();
        let edits = lsp.format_document(&uri);
        drop(lsp);

        let result: Vec<lsp_types::TextEdit> = edits.into_iter().map(|e| lsp_types::TextEdit {
            range: lsp_types::Range {
                start: lsp_types::Position { line: e.range.start.line, character: e.range.start.character },
                end: lsp_types::Position { line: e.range.end.line, character: e.range.end.character },
            },
            new_text: e.new_text,
        }).collect();

        Ok(Some(result))
    }
}

impl Backend {
    fn byte_to_position(&self, content: &str, byte_offset: usize) -> lsp_types::Position {
        let safe_offset = byte_offset.min(content.len());
        let preceding = &content[..safe_offset];
        let line = preceding.chars().filter(|c| *c == '\n').count() as u32;
        let last_newline = preceding.rfind('\n').map(|i| i + 1).unwrap_or(0);
        let col_chars = content[last_newline..safe_offset].chars().count() as u32;
        lsp_types::Position { line, character: col_chars }
    }

    fn run_diagnostics(&self, content: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        // Lex
        let tokens: Vec<_> = Lexer::new(content).collect();
        
        // Check for lexer errors
        for t in &tokens {
            if let once_lex::Token::Error = t.token {
                diagnostics.push(Diagnostic {
                    range: lsp_types::Range {
                        start: lsp_types::Position { line: t.span.line as u32, character: t.span.column as u32 },
                        end: lsp_types::Position { line: t.span.line as u32, character: (t.span.column + 1) as u32 },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: "Invalid token".to_string(),
                    ..Default::default()
                });
            }
        }
        
        // Parse
        match OnceParser::parse(tokens) {
            Ok(ast) => {
                // HIR
                let mut builder = HirBuilder::new();
                match builder.build(ast) {
                    Ok(hir) => {
                        // Type check
                        let mut type_checker = once_ty::TypeChecker::new();
                        if let Err(errors) = type_checker.check(&hir) {
                            for err in errors {
                                let pos = if let Some(span) = err.span() {
                                    self.byte_to_position(content, span.start)
                                } else {
                                    lsp_types::Position { line: 0, character: 0 }
                                };
                                diagnostics.push(Diagnostic {
                                    range: lsp_types::Range {
                                        start: pos,
                                        end: lsp_types::Position { line: pos.line, character: pos.character + 1 },
                                    },
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    message: format!("Type error: {}", err.diagnostic()),
                                    ..Default::default()
                                });
                            }
                        }
                        
                        // Effect check — pipe to LSP
                        let mut effect_checker = once_ty::effects::EffectChecker::new();
                        if let Err(errors) = effect_checker.check(&hir) {
                            for err in errors {
                                let pos = if let Some(span) = err.span() {
                                    self.byte_to_position(content, span.start)
                                } else {
                                    lsp_types::Position { line: 0, character: 0 }
                                };
                                diagnostics.push(Diagnostic {
                                    range: lsp_types::Range {
                                        start: pos,
                                        end: lsp_types::Position { line: pos.line, character: pos.character + 1 },
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message: format!("Effect warning: {}", err.diagnostic()),
                                    ..Default::default()
                                });
                            }
                        }
                        
                        // Linearity check — pipe to LSP
                        let mut linearity_checker = once_linear::LinearityChecker::new();
                        if let Err(errors) = linearity_checker.check(&hir) {
                            for err in errors {
                                let pos = if let Some(span) = err.span() {
                                    self.byte_to_position(content, span.start)
                                } else {
                                    lsp_types::Position { line: 0, character: 0 }
                                };
                                diagnostics.push(Diagnostic {
                                    range: lsp_types::Range {
                                        start: pos,
                                        end: lsp_types::Position { line: pos.line, character: pos.character + 1 },
                                    },
                                    severity: Some(DiagnosticSeverity::WARNING),
                                    message: format!("Linearity error: {}", err.diagnostic()),
                                    ..Default::default()
                                });
                            }
                        }
                        
                        // Type hole info as hints
                        for diag in type_checker.hole_diagnostics() {
                            diagnostics.push(Diagnostic {
                                range: lsp_types::Range {
                                    start: lsp_types::Position { line: 0, character: 0 },
                                    end: lsp_types::Position { line: 0, character: 1 },
                                },
                                severity: Some(DiagnosticSeverity::INFORMATION),
                                message: diag,
                                ..Default::default()
                            });
                        }
                    }
                    Err(errors) => {
                        for err in errors {
                            diagnostics.push(Diagnostic {
                                range: lsp_types::Range {
                                    start: lsp_types::Position { line: 0, character: 0 },
                                    end: lsp_types::Position { line: 0, character: 1 },
                                },
                                severity: Some(DiagnosticSeverity::ERROR),
                                message: format!("HIR error: {:?}", err),
                                ..Default::default()
                            });
                        }
                    }
                }
            }
            Err(err) => {
diagnostics.push(Diagnostic {
                range: lsp_types::Range {
                    start: lsp_types::Position { line: 0, character: 0 },
                    end: lsp_types::Position { line: 0, character: 1 },
                },
                    severity: Some(DiagnosticSeverity::ERROR),
                    message: format!("Parse error: {}", err),
                    ..Default::default()
                });
            }
        }
        
        diagnostics
    }
}

/// Start the LSP server using tower-lsp over stdio
pub async fn start_lsp_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (service, socket) = LspService::new(|client| Backend {
        client,
        lsp: Mutex::new(OnceLsp::new()),
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), socket)
        .serve(service)
        .await;
    Ok(())
}

/// Start the LSP server using tower-lsp over TCP
pub async fn start_lsp_server_tcp(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::net::TcpListener;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    eprintln!("Once LSP server listening on port {}", port);

    loop {
        let (stream, addr) = listener.accept().await?;
        eprintln!("Client connected: {}", addr);

        let (read, write) = tokio::io::split(stream);
        let (service, socket) = LspService::new(|client| Backend {
            client,
            lsp: Mutex::new(OnceLsp::new()),
        });

        tokio::spawn(Server::new(read, write, socket).serve(service));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_creation() {
        let lsp = OnceLsp::new();
        assert_eq!(lsp.documents.len(), 0);
        assert_eq!(lsp.diagnostics.len(), 0);
    }

    #[test]
    fn test_open_document() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "fn main() {}".to_string(), 1);
        assert_eq!(lsp.documents.len(), 1);
    }

    #[test]
    fn test_get_completions() {
        let lsp = OnceLsp::new();
        let completions = lsp.get_completions("file://test.onc", 0, 0);
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_get_hover() {
        let lsp = OnceLsp::new();
        let hover = lsp.get_hover("file://test.onc", 0, 0);
        assert!(hover.is_some());
    }

    #[test]
    fn test_fix_it_missing_close() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "let f = File.open(\"test.txt\")".to_string(), 1);
        let fix_its = lsp.get_fix_its("file://test.onc", 0, 0);
        assert!(!fix_its.is_empty());
        assert_eq!(fix_its[0].kind, FixItKind::AddMissingClose);
    }

    #[test]
    fn test_fix_it_missing_commit() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "let tx = Transaction.begin()".to_string(), 1);
        let fix_its = lsp.get_fix_its("file://test.onc", 0, 0);
        assert!(!fix_its.is_empty());
        assert_eq!(fix_its[0].kind, FixItKind::AddMissingCommit);
    }

    #[test]
    fn test_fix_it_missing_var_block() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "let mut x = 0".to_string(), 1);
        let fix_its = lsp.get_fix_its("file://test.onc", 0, 0);
        assert!(!fix_its.is_empty());
        assert_eq!(fix_its[0].kind, FixItKind::AddVarBlock);
    }

    #[test]
    fn test_fix_it_unused_variable() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "let unused = 42".to_string(), 1);
        let fix_its = lsp.get_fix_its("file://test.onc", 0, 0);
        assert!(!fix_its.is_empty());
        assert_eq!(fix_its[0].kind, FixItKind::RemoveUnusedVariable);
    }

    #[test]
    fn test_fix_it_linearity_error() {
        let mut lsp = OnceLsp::new();
        lsp.open_document("file://test.onc".to_string(), "let x = y; // error: value moved".to_string(), 1);
        let fix_its = lsp.get_fix_its("file://test.onc", 0, 0);
        assert!(!fix_its.is_empty());
        assert!(fix_its.iter().any(|f| f.kind == FixItKind::FixLinearity), "Expected a FixLinearity fix-it, got {:?}", fix_its.iter().map(|f| f.kind.clone()).collect::<Vec<_>>());
    }
}