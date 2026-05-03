use std::fs;
use std::path::{Path, PathBuf};
use super::{HirProgram, HirItem, Import};

/// Module file extensions to try when resolving imports
const MODULE_EXTENSIONS: &[&str] = &[".onc", ".onco"];

pub struct ImportResolver {
    project_root: PathBuf,
}

impl ImportResolver {
    pub fn new() -> Self {
        Self { project_root: PathBuf::from(".") }
    }

    pub fn new_with_root(root: PathBuf) -> Self {
        Self { project_root: root }
    }

    pub fn resolve(&self, program: &mut HirProgram) -> Result<(), String> {
        for imp in &mut program.imports {
            // Normalize relative paths
            Self::normalize_path(imp);
            // Add default items if none specified
            if imp.items.is_empty() {
                imp.items.push("*".to_string());
            }
            // For std/core add prelude
            if (imp.path == "std" || imp.path == "core") && imp.items == vec!["*".to_string()] {
                imp.items.push("prelude".to_string());
            }
        }

        // Second pass: try to resolve file-backed imports
        let mut resolved_modules = Vec::new();
        let imports = std::mem::take(&mut program.imports);

        for imp in imports {
            let path = &imp.path;
            // Skip built-in modules that are handled at compile time
            if path == "std" || path == "core" || path == "prelude" {
                // Built-in modules are resolved via the type checker's built-in types
                resolved_modules.push(imp);
                continue;
            }

            // Try to resolve as file - try each extension
            let mut found = false;
            for ext in MODULE_EXTENSIONS {
                let file_path = self.resolve_module_path(path, ext);
                if file_path.exists() {
                    match self.load_module_file(&file_path, &imp.items) {
                        Ok(module_items) => {
                            program.items.extend(module_items);
                            found = true;
                            break;
                        }
                        Err(e) => {
                            eprintln!("Warning: Could not load module '{}': {}", path, e);
                        }
                    }
                }
            }

            if !found {
                // Module not found on disk - keep import for error reporting
                resolved_modules.push(imp);
            }
        }

        program.imports = resolved_modules;
        Ok(())
    }

    fn normalize_path(imp: &mut Import) {
        loop {
            if imp.path.starts_with("./") {
                imp.path = imp.path.trim_start_matches("./").to_string();
            } else if imp.path.starts_with("../") {
                imp.path = imp.path.trim_start_matches("../").to_string();
            } else {
                break;
            }
        }
        // Normalize Windows-like separators to '/'
        imp.path = imp.path.replace('\\', "/");
        // Collapse .. segments
        if imp.path.contains('/') {
            let mut stack: Vec<&str> = Vec::new();
            for part in imp.path.split('/') {
                if part.is_empty() || part == "." {
                    continue;
                } else if part == ".." {
                    stack.pop();
                } else {
                    stack.push(part);
                }
            }
            imp.path = stack.join("/");
        }
    }

    fn resolve_module_path(&self, module_path: &str, ext: &str) -> PathBuf {
        let rel = module_path.replace("::", "/");
        let mut candidate = self.project_root.join(&rel);
        candidate.set_extension(ext.trim_start_matches('.'));
        
        // Also try as directory with mod.onc
        if !candidate.exists() {
            let dir_candidate = self.project_root.join(&rel).join("mod.onc");
            if dir_candidate.exists() {
                return dir_candidate;
            }
        }
        
        candidate
    }

    fn load_module_file(&self, path: &Path, items: &[String]) -> Result<Vec<HirItem>, String> {
        let source = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;

        // For .onco (binary) files, delegate to once-onceo loader
        if path.extension().map_or(false, |e| e == "onco") {
            return self.load_compiled_module(&source, items);
        }

        // Parse the source file
        let lexer = once_lex::Lexer::new(&source);
        let tokens: Vec<_> = lexer.collect();

        let ast = once_parse::OnceParser::parse(tokens)
            .map_err(|e| format!("Parse error in {}: {}", path.display(), e))?;

        // Build HIR from the parsed AST
        let mut builder = super::HirBuilder::new();
        let module_hir = builder.build(ast)
            .map_err(|e| format!("HIR error in {}: {:?}", path.display(), e))?;

        // Filter items based on import list
        if items.is_empty() || items == &["*".to_string()] {
            Ok(module_hir.items)
        } else {
            Ok(module_hir.items.into_iter()
                .filter(|item| Self::item_matches_import(item, items))
                .collect())
        }
    }

    fn item_matches_import(item: &HirItem, items: &[String]) -> bool {
        let name = match item {
            HirItem::FnDecl(f) => &f.name,
            HirItem::LetDecl(l) => &l.name,
            HirItem::TypeDecl(t) => &t.name,
            HirItem::StructDecl(s) => &s.name,
            HirItem::TraitDecl(t) => &t.name,
            HirItem::ImplBlock(_) => return true, // impls are always included
        };
        items.contains(&"*".to_string()) || items.contains(name)
    }

    fn load_compiled_module(&self, _data: &str, _items: &[String]) -> Result<Vec<HirItem>, String> {
        // .onco modules contain serialized type summaries
        // For now, return empty - full implementation loads via once-onceo deserializer
        Ok(Vec::new())
    }
}
