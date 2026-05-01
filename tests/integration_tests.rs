//! Integration tests for the Once language compiler
//! 
//! These tests verify the complete compilation pipeline from source code
//! to object files, ensuring all components work together correctly.

use std::process::Command;
use std::fs;
use std::path::Path;

/// Test the complete compilation pipeline
#[test]
fn test_hello_world_compilation() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "build", "--input", "examples/hello_world.onc", "--output", "test_hello_world.o"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Verify object file was created
    assert!(Path::new("test_hello_world.o").exists(), "Object file was not created");
    
    // Clean up
    let _ = fs::remove_file("test_hello_world.o");
}

/// Test compilation with multiple functions
#[test]
fn test_multi_function_compilation() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "build", "--input", "examples/simple_async.onc", "--output", "test_simple_async.o"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Compilation failed: {}", String::from_utf8_lossy(&output.stderr));
    
    // Verify object file was created
    assert!(Path::new("test_simple_async.o").exists(), "Object file was not created");
    
    // Clean up
    let _ = fs::remove_file("test_simple_async.o");
}

/// Test error handling for invalid syntax
#[test]
fn test_syntax_error_handling() {
    // Create a temporary file with syntax errors
    let invalid_code = "fn main() -> Unit {\n    print(\"Hello, World!\"\n    // Missing closing parenthesis\n}";
    fs::write("test_syntax_error.onc", invalid_code).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "build", "--input", "test_syntax_error.onc", "--output", "test_syntax_error.o"])
        .output()
        .expect("Failed to run once compiler");

    // Should fail with syntax error
    assert!(!output.status.success(), "Compilation should have failed");
    
    // Clean up
    let _ = fs::remove_file("test_syntax_error.onc");
    let _ = fs::remove_file("test_syntax_error.o");
}

/// Test type checking
#[test]
fn test_type_checking() {
    // Create a temporary file with type errors
    let type_error_code = "fn main() -> Unit {\n    let x: Int = \"Hello, World!\";\n    print(x)\n}";
    fs::write("test_type_error.onc", type_error_code).expect("Failed to write test file");

    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "build", "--input", "test_type_error.onc", "--output", "test_type_error.o"])
        .output()
        .expect("Failed to run once compiler");

    // Should fail with type error
    assert!(!output.status.success(), "Compilation should have failed");
    
    // Clean up
    let _ = fs::remove_file("test_type_error.onc");
    let _ = fs::remove_file("test_type_error.o");
}

/// Test help command
#[test]
fn test_help_command() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "help"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Help command failed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("Once Language Compiler"), "Help output should contain compiler name");
}

/// Test build command help
#[test]
fn test_build_help() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "build", "--help"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Build help command failed");
    assert!(String::from_utf8_lossy(&output.stdout).contains("--input"), "Build help should contain --input option");
    assert!(String::from_utf8_lossy(&output.stdout).contains("--output"), "Build help should contain --output option");
}

/// Test individual compiler stages
#[test]
fn test_parse_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "parse", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Parse stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_hir_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "hir", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "HIR stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_typecheck_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "typecheck", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Typecheck stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_effects_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "effects", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Effects stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_linearity_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "linearity", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Linearity stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_regions_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "regions", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Regions stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_mir_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "mir", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "MIR stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_codegen_stage() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "codegen", "--input", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Codegen stage failed: {}", String::from_utf8_lossy(&output.stderr));
}

/// Test explain modes
#[test]
fn test_explain_regions() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "explain", "regions", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Explain regions failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_explain_effects() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "explain", "effects", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Explain effects failed: {}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn test_explain_linearity() {
    let output = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "explain", "linearity", "examples/hello_world.onc"])
        .output()
        .expect("Failed to run once compiler");

    assert!(output.status.success(), "Explain linearity failed: {}", String::from_utf8_lossy(&output.stderr));
}

/// Test LSP server startup
#[test]
fn test_lsp_server() {
    let mut child = Command::new("cargo")
        .args(&["run", "-p", "once-cli", "--bin", "once", "--",  "lsp", "--stdio"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start LSP server");

    // Give it a moment to start
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Send initialize request
    let init_request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"processId":null,"rootUri":null,"capabilities":{}}}"#;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        let _ = stdin.write_all(init_request.as_bytes());
    }

    // Give it a moment to respond
    std::thread::sleep(std::time::Duration::from_millis(100));

    // Kill the process
    let _ = child.kill();
}
