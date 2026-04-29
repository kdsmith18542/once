pub struct ImportResolver;

impl ImportResolver {
    pub fn new() -> Self { Self {} }
    pub fn resolve(&self, program: &mut super::HirProgram) -> Result<(), String> {
        // Minimal real-ish behavior:
        // 1) Normalize relative paths (strip leading ./ or ../ iteratively)
        // 2) If an import has no items, populate with a wildcard
        // 3) If an import path looks like a standard lib (e.g. "std", "core"), and
        //    there are no items yet, add a minimal default item like "prelude".
        for imp in &mut program.imports {
            // Normalize relative paths
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
            if imp.path.contains('\\') {
                imp.path = imp.path.replace('\\', "/");
            }
            // Collapse simple relative segments like a/b/../c -> a/c
            if imp.path.contains('/') {
                let mut stack: Vec<&str> = Vec::new();
                for part in imp.path.split('/') {
                    if part.is_empty() || part == "." {
                        continue;
                    } else if part == ".." {
                        if !stack.is_empty() {
                            stack.pop();
                        }
                    } else {
                        stack.push(part);
                    }
                }
                imp.path = stack.join("/");
            }
            if imp.items.is_empty() {
                imp.items.push("*".to_string());
            }
            if (imp.path == "std" || imp.path == "core") && imp.items == vec!["*".to_string()] {
                imp.items.push("prelude".to_string());
            }
        }
        Ok(())
    }
}
