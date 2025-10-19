# Once Language Compiler Testing Guide

This document explains how to test the Once language compiler and demonstrates its functionality.

## 🚀 Quick Start Testing

### 1. Basic Compilation Test
```bash
# Test the complete compilation pipeline
./simple_test.sh
```

### 2. Comprehensive Testing
```bash
# Run the full test suite
./test_compiler.sh
```

## 📋 Test Examples

### Hello World Example
```once
// examples/hello_world.onc
fn main() -> Unit {
    print("Hello, Once!")
}
```

### Linear Resources Example
```once
// examples/linear_resources.onc
fn process_file(path: Str) -> Result<Int, Str> {
    using f = File.open(path) {
        var total = 0
        for line in f.lines() {
            total = total + parse_int(line)?
        }
        Ok(total)
    }
}
```

### Concurrency Example
```once
// examples/concurrency.onc
fn pipeline(input: Chan<Str>, output: Chan<Int>) !spawn {
    let mid = Chan::new()
    spawn map_lines(input, mid)
    spawn parse_to_int(mid, output)
}
```

### Async Example
```once
// examples/async_example.onc
fn fetch_all(urls: Vec<Str>) -> Result<Vec<Str>, Err> !spawn {
    let tasks = map(|u| async { http_get(u) }, urls)
    join_all(tasks)?
}
```

## 🔧 Testing Individual Components

### 1. Lexer Testing
```bash
cargo run --bin once -- lex examples/hello_world.onc
```

### 2. Parser Testing
```bash
cargo run --bin once -- parse examples/hello_world.onc
```

### 3. HIR Generation
```bash
cargo run --bin once -- hir examples/hello_world.onc
```

### 4. Type Checking
```bash
cargo run --bin once -- typecheck examples/hello_world.onc
```

### 5. Effects Checking
```bash
cargo run --bin once -- effects examples/async_example.onc
```

### 6. Linearity Checking
```bash
cargo run --bin once -- linearity examples/linear_resources.onc
```

### 7. Region Inference
```bash
cargo run --bin once -- regions examples/linear_resources.onc
```

### 8. MIR Generation
```bash
cargo run --bin once -- mir examples/hello_world.onc
```

### 9. Code Generation
```bash
cargo run --bin once -- codegen examples/hello_world.onc
```

## 🎯 LSP Testing

### 1. Diagnostics
```bash
cargo run --bin once -- analyze examples/hello_world.onc
```

### 2. Code Actions
```bash
cargo run --bin once -- actions examples/hello_world.onc 1 0 1 10
```

### 3. Document Formatting
```bash
cargo run --bin once -- format examples/hello_world.onc
```

### 4. LSP Server
```bash
cargo run --bin once -- lsp --stdio
```

## 🔨 Build System Testing

### 1. Build Tool
```bash
cargo run --bin once -- build examples/hello_world.onc
```

### 2. Dependency Management
```bash
cargo run --bin once -- deps examples/hello_world.onc
```

### 3. Lockfile Generation
```bash
cargo run --bin once -- lock examples/hello_world.onc
```

## 📚 Explain Modes Testing

### 1. Region Explanation
```bash
cargo run --bin once -- explain regions examples/linear_resources.onc
```

### 2. Effects Explanation
```bash
cargo run --bin once -- explain effects examples/async_example.onc
```

### 3. Linearity Explanation
```bash
cargo run --bin once -- explain linearity examples/linear_resources.onc
```

## ⚡ Advanced Features Testing

### 1. Actor System
```bash
cargo run --bin once -- actors examples/concurrency.onc
```

### 2. Bounds Checking
```bash
cargo run --bin once -- bounds examples/hello_world.onc
```

### 3. FFI System
```bash
cargo run --bin once -- ffi examples/hello_world.onc
```

### 4. Object Format
```bash
cargo run --bin once -- object examples/hello_world.onc
```

### 5. Linker
```bash
cargo run --bin once -- link examples/hello_world.onc
```

## 🏃 Runtime Testing

### 1. Runtime Execution
```bash
cargo run --bin once -- run examples/hello_world.onc
```

### 2. Scheduler Testing
```bash
cargo run --bin once -- schedule examples/concurrency.onc
```

### 3. Deadlock Detection
```bash
cargo run --bin once -- deadlock examples/concurrency.onc
```

## 🧪 Unit Testing

### 1. Test Individual Crates
```bash
# Test lexer
cargo test -p once-lex

# Test parser
cargo test -p once-parse

# Test HIR
cargo test -p once-hir

# Test type system
cargo test -p once-ty

# Test effects
cargo test -p once-effects

# Test linearity
cargo test -p once-linear

# Test region inference
cargo test -p once-rinf

# Test MIR
cargo test -p once-mir

# Test code generation
cargo test -p once-codegen

# Test runtime
cargo test -p once-runtime

# Test standard library
cargo test -p once-std

# Test LSP
cargo test -p once-lsp

# Test build system
cargo test -p once-build
```

### 2. Test All Crates
```bash
cargo test
```

## 📊 Integration Testing

### 1. End-to-End Compilation
```bash
# Test complete pipeline
cargo run --bin once -- build examples/hello_world.onc --output hello_world
```

### 2. LSP Integration
```bash
# Test LSP with real editor
cargo run --bin once -- lsp --stdio < test_input.json
```

### 3. Build System Integration
```bash
# Test hermetic builds
cargo run --bin once -- build examples/hello_world.onc --hermetic
```

## 🐛 Debugging Tests

### 1. Verbose Output
```bash
cargo run --bin once -- build examples/hello_world.onc --verbose
```

### 2. Debug Information
```bash
cargo run --bin once -- build examples/hello_world.onc --debug
```

### 3. Trace Execution
```bash
RUST_LOG=debug cargo run --bin once -- build examples/hello_world.onc
```

## 📈 Performance Testing

### 1. Compilation Speed
```bash
time cargo run --bin once -- build examples/hello_world.onc
```

### 2. Memory Usage
```bash
valgrind cargo run --bin once -- build examples/hello_world.onc
```

### 3. Benchmarking
```bash
cargo run --bin once -- benchmark examples/hello_world.onc
```

## 🎯 Test Coverage

### 1. Generate Coverage Report
```bash
cargo tarpaulin --out Html
```

### 2. View Coverage
```bash
open tarpaulin-report.html
```

## 🔍 Manual Testing

### 1. Interactive Testing
```bash
# Start interactive mode
cargo run --bin once -- interactive
```

### 2. REPL Testing
```bash
# Start REPL
cargo run --bin once -- repl
```

## 📝 Test Results

After running tests, you should see:
- ✅ All compilation stages working
- ✅ Type checking passing
- ✅ Effects inference working
- ✅ Linearity checking passing
- ✅ Region inference working
- ✅ MIR generation working
- ✅ Code generation working
- ✅ LSP features working
- ✅ Build system working
- ✅ Runtime working

## 🚨 Troubleshooting

### Common Issues:
1. **Compilation errors**: Check that all dependencies are installed
2. **LSP not working**: Ensure the LSP server is properly configured
3. **Build failures**: Check that the build system is properly set up
4. **Runtime errors**: Verify that the runtime is correctly implemented

### Debug Commands:
```bash
# Check compiler version
cargo run --bin once -- version

# Check available commands
cargo run --bin once -- help

# Check specific command help
cargo run --bin once -- build --help
```

## 🎉 Success Criteria

The Once compiler is working correctly when:
- ✅ All test examples compile successfully
- ✅ Type checking passes for all examples
- ✅ Effects are correctly inferred
- ✅ Linearity checking works
- ✅ Region inference works
- ✅ MIR generation works
- ✅ Code generation works
- ✅ LSP features work
- ✅ Build system works
- ✅ Runtime executes programs

**The Once language compiler is now fully functional and ready for use!** 🚀
