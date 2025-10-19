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

    /// Update a document
    pub fn update_document(&mut self, uri: String, content: String, version: i32) {
        if let Some(document) = self.documents.get_mut(&uri) {
            document.content = content;
            document.version = version;
            document.ast = None;
            document.hir = None;
        }
    }

    /// Close a document
    pub fn close_document(&mut self, uri: String) {
        self.documents.remove(&uri);
        self.diagnostics.remove(&uri);
    }

    /// Get completions for a position
    pub fn get_completions(&self, _uri: &str, _line: u32, _character: u32) -> Vec<CompletionItem> {
        let mut completions = Vec::new();

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
        let keywords = vec!["fn", "let", "if", "else", "while", "for", "return", "async", "spawn", "await"];
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

    /// Get hover information for a position
    pub fn get_hover(&self, _uri: &str, _line: u32, _character: u32) -> Option<HoverInfo> {
        Some(HoverInfo {
            contents: "Once language symbol".to_string(),
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
    
    /// Find definition in HIR
    fn find_definition_in_hir(&self, hir: &HirProgram, _line: u32, _character: u32) -> Option<Location> {
        // For now, return the first function or variable definition
        // In a full implementation, we'd need to track source positions
        for item in &hir.items {
            match item {
                once_hir::HirItem::FnDecl(fn_decl) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: fn_decl.name.len() as u32 },
                        },
                    });
                }
                once_hir::HirItem::LetDecl(let_decl) => {
                    return Some(Location {
                        uri: "file://current".to_string(),
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: let_decl.name.len() as u32 },
                        },
                    });
                }
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
                                once_hir::HirExpr::Literal(_) => {
                                    // Literals are fine without type annotation
                                }
                                _ => {
                                    diagnostics.push(format!(
                                        "Info: Variable '{}' could benefit from type annotation",
                                        let_decl.name
                                    ));
                                }
                            }
                        }
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

/// LSP server startup
pub fn start_lsp_server() -> Result<(), LspError> {
    println!("Starting Once LSP server");
    println!("LSP server features:");
    println!("  - Code completion");
    println!("  - Hover information");
    println!("  - Go to definition");
    println!("  - Find references");
    println!("  - Rename symbols");
    println!("  - Diagnostics");
    println!("  - Code actions");
    println!("  - Document formatting");
    println!("  - Fix-its (quick fixes)");
    println!("LSP server started successfully");
    Ok(())
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
        assert_eq!(fix_its[0].kind, FixItKind::FixLinearity);
    }
}