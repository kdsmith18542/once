use once_parse::OnceParser;
use once_lex::Lexer;
use std::fs;

fn main() {
    println!("=== Once Language Parser Verification ===\n");

    // Test 1: Parse linear_resources.onc example file
    println!("1. Testing examples/linear_resources.onc:");
    match fs::read_to_string("G:/BACKUP/once/examples/linear_resources.onc") {
        Ok(source) => {
            let tokens: Vec<_> = Lexer::new(&source).collect();
            match OnceParser::parse(tokens) {
                Ok(program) => {
                    println!("   ✓ Parsing succeeded");
                    println!("   Program items: {}", program.items.len());
                    for (i, item) in program.items.iter().enumerate() {
                        match item {
                            once_parse::Item::FnDecl(fn_decl) => {
                                println!("   Item {}: function '{}'", i+1, fn_decl.name);
                                if let Some(effects) = &fn_decl.effects {
                                    println!("     Effects: {:?}", effects.effects);
                                }
                            }
                            once_parse::Item::LetDecl(let_decl) => {
                                println!("   Item {}: let '{}'", i+1, let_decl.name);
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("   ✗ Parsing failed:");
                    println!("   {}", e);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Failed to read file: {}", e);
        }
    }

    println!();

    // Test 2: Effect annotation with single effect
    println!("2. Testing: fn foo() -> Int !io { 42 }");
    let source2 = "fn foo() -> Int !io { 42 }";
    let tokens2: Vec<_> = Lexer::new(source2).collect();
    match OnceParser::parse(tokens2) {
        Ok(program) => {
            println!("   ✓ Parsing succeeded");
            if let Some(once_parse::Item::FnDecl(fn_decl)) = program.items.first() {
                println!("   Function name: '{}'", fn_decl.name);
                if let Some(effects) = &fn_decl.effects {
                    println!("   Effects: {:?}", effects.effects);
                } else {
                    println!("   No effects parsed");
                }
            }
        }
        Err(e) => {
            println!("   ✗ Parsing failed:");
            println!("   {}", e);
        }
    }

    println!();

    // Test 3: Effect annotation with multiple effects
    println!("3. Testing: fn bar() -> Int ![io, spawn] { 42 }");
    let source3 = "fn bar() -> Int ![io, spawn] { 42 }";
    let tokens3: Vec<_> = Lexer::new(source3).collect();
    match OnceParser::parse(tokens3) {
        Ok(program) => {
            println!("   ✓ Parsing succeeded");
            if let Some(once_parse::Item::FnDecl(fn_decl)) = program.items.first() {
                println!("   Function name: '{}'", fn_decl.name);
                if let Some(effects) = &fn_decl.effects {
                    println!("   Effects: {:?}", effects.effects);
                } else {
                    println!("   No effects parsed");
                }
            }
        }
        Err(e) => {
            println!("   ✗ Parsing failed:");
            println!("   {}", e);
        }
    }

    println!();

    // Test 4: Using statement
    println!("4. Testing 'using' statement from linear_resources.onc:");
    let source4 = "using f = File.open(path) { /* body */ }";
    let tokens4: Vec<_> = Lexer::new(source4).collect();
    match OnceParser::parse(tokens4) {
        Ok(program) => {
            println!("   ✓ Parsing succeeded");
            if let Some(once_parse::Item::LetDecl(let_decl)) = program.items.first() {
                println!("   Let declaration name: '{}'", let_decl.name);
            }
            // The using statement is inside a block, not top-level
            // Let's check the AST for UsingStmt
        }
        Err(e) => {
            println!("   ✗ Parsing failed:");
            println!("   {}", e);
        }
    }

    println!();

    // Test 5: Using statement inside a function
    println!("5. Testing 'using' statement inside a function:");
    let source5 = "fn test() -> Int { using x = 42 { return x } }";
    let tokens5: Vec<_> = Lexer::new(source5).collect();
    match OnceParser::parse(tokens5) {
        Ok(program) => {
            println!("   ✓ Parsing succeeded");
            if let Some(once_parse::Item::FnDecl(fn_decl)) = program.items.first() {
                println!("   Function '{}' has {} statements", fn_decl.name, fn_decl.body.statements.len());
                if let Some(once_parse::Stmt::Using(using_stmt)) = fn_decl.body.statements.first() {
                    println!("   Using statement: '{}'", using_stmt.name);
                } else {
                    println!("   First statement is not a using statement");
                }
            }
        }
        Err(e) => {
            println!("   ✗ Parsing failed:");
            println!("   {}", e);
        }
    }

    println!("\n=== End of verification ===");
}
