# Once Language - User Guide

## Introduction

Welcome to Once! This guide will help you get started with the Once programming language, a modern systems programming language designed for safety, simplicity, and performance.

## Installation

### From Source

```bash
git clone https://github.com/once-lang/once.git
cd once
cargo build --release
```

The compiler will be available at `./target/release/once`.

### Quick Test

```bash
# Create a simple test program
echo 'fn main() -> Unit { print("Hello, Once!") }' > hello.onc

# Compile it
./target/release/once build --input hello.onc

# Check that it produced output
ls -la main.o
```

## Language Basics

### Functions

Functions in Once are defined with the `fn` keyword:

```once
fn greet(name: Str) -> Unit {
    print("Hello, ".concat(name).concat("!"))
}

fn main() -> Unit {
    greet("World")
}
```

### Types

Once has a rich type system with automatic inference:

```once
// Primitive types
let x: Int = 42
let y: Float = 3.14
let z: Bool = true
let s: Str = "Hello"

// Unit type (no value)
let u: Unit = ()

// Function types
let f: fn(Int, Int) -> Int = add
```

### Variables and Assignment

Variables are immutable by default:

```once
let x = 42  // Immutable
let mut y = 0  // Mutable
y = y + 1  // Allowed
```

## Memory Management

### Region-Based Memory Management

Once uses region-based memory management for automatic memory safety:

```once
fn process_data() -> Unit {
    let data = allocate_data()  // Data allocated in current region
    
    // Process data...
    
    // Memory automatically freed when function returns
}
```

### Linear Types

Linear types ensure resources are used exactly once:

```once
fn write_file(path: Str, content: Str) -> Unit !io {
    using file = File::create(path) {  // Linear resource
        file.write(content)  // Must use the file
    }  // File automatically closed
}
```

## Concurrency

### Channels

Type-safe communication between concurrent processes:

```once
fn producer(output: Chan<Str>) -> Unit {
    output.send("Hello from producer")
}

fn consumer(input: Chan<Str>) -> Unit {
    let msg = input.recv()
    print(msg)
}

fn main() -> Unit !spawn {
    let (tx, rx) = Chan::new()
    
    spawn producer(tx)
    spawn consumer(rx)
}
```

### Async/Await

Structured concurrency with async/await:

```once
async fn fetch_data(url: Str) -> Str !io {
    // Asynchronous network request
    let response = http_get(url).await
    response.body
}

fn main() -> Unit !spawn, io {
    let data = fetch_data("https://api.example.com/data").await
    print(data)
}
```

## Effects System

Once tracks computational effects at the type level:

```once
// Pure function (no effects)
fn add(x: Int, y: Int) -> Int {
    x + y
}

// I/O effect
fn read_file(path: Str) -> Str !io {
    // File operations
}

// Spawn effect
fn launch_worker() -> Unit !spawn {
    spawn worker_task()
}

// Multiple effects
fn complex_operation() -> Unit !io, spawn {
    let config = read_file("config.txt")
    spawn process_data(config)
}
```

## Error Handling

### Option Types

Handle optional values:

```once
fn find_user(id: Int) -> Option<User> {
    // Return Some(user) or None
}

fn main() -> Unit {
    match find_user(42) {
        Some(user) => print("Found: ".concat(user.name)),
        None => print("User not found")
    }
}
```

### Result Types

Handle operations that can fail:

```once
fn parse_number(s: Str) -> Result<Int, ParseError> {
    // Try to parse, return Ok(value) or Err(error)
}

fn main() -> Unit {
    match parse_number("123") {
        Ok(n) => print("Parsed: ".concat(n.to_string())),
        Err(e) => print("Parse error: ".concat(e.message))
    }
}
```

## Advanced Features

### Pattern Matching

Powerful pattern matching:

```once
fn process_value(value: Value) -> Unit {
    match value {
        Value::Number(n) => print("Number: ".concat(n.to_string())),
        Value::Text(s) => print("Text: ".concat(s)),
        Value::List(items) => {
            print("List with ".concat(items.len().to_string()).concat(" items"))
        }
    }
}
```

### Generic Functions

Type-parametric functions:

```once
fn identity<T>(x: T) -> T {
    x
}

fn map<A, B>(f: fn(A) -> B, list: List<A>) -> List<B> {
    // Implementation
}
```

### Traits and Polymorphism

```once
trait Printable {
    fn print(self) -> Unit
}

impl Printable for Int {
    fn print(self) -> Unit {
        // Print integer
    }
}

impl Printable for Str {
    fn print(self) -> Unit {
        // Print string
    }
}
```

## Compiler Commands

### Building Programs

```bash
# Build a single file
once build --input program.onc

# Build with custom output
once build --input program.onc --output myprogram.o

# Build a project
once build-project --project myproject/
```

### Development Tools

```bash
# Check syntax and types
once typecheck program.onc

# Show intermediate representations
once mir program.onc     # Show MIR
once codegen program.onc # Show generated code

# Language server for IDEs
once lsp --stdio
```

### Debugging and Analysis

```bash
# Explain compiler decisions
once explain regions program.onc
once explain effects program.onc
once explain linearity program.onc

# Run with runtime
once run program.onc
```

## Project Structure

### Creating a New Project

```bash
# Create project directory
mkdir myproject
cd myproject

# Create main file
cat > main.onc << 'EOF'
fn main() -> Unit {
    print("Hello from my project!")
}
EOF

# Create project configuration
cat > once.toml << 'EOF'
[package]
name = "myproject"
version = "0.1.0"

[capabilities]
io = true
EOF
```

### Project Layout

```
myproject/
├── main.onc          # Main source file
├── once.toml         # Project configuration
├── src/              # Additional source files
│   ├── utils.onc
│   └── types.onc
└── tests/            # Test files
    └── utils_test.onc
```

## Standard Library

### Core Modules

```once
use once_std::io::File
use once_std::net::TcpListener
use once_std::collections::HashMap
use once_std::time::Duration
```

### Common Operations

```once
// File I/O
using file = File::open("data.txt") {
    let content = file.read_all()
    print(content)
}

// Networking
using listener = TcpListener::bind("127.0.0.1:8080") {
    let (stream, addr) = listener.accept().await
    // Handle connection
}

// Collections
let mut map = HashMap::new()
map.insert("key", "value")
let value = map.get("key")
```

## Best Practices

### Code Organization

```once
// Group related functions together
mod math {
    fn add(x: Int, y: Int) -> Int { x + y }
    fn multiply(x: Int, y: Int) -> Int { x * y }
}

// Use clear naming
fn calculate_total_price(items: List<Item>) -> Float {
    // Implementation
}

// Handle errors appropriately
fn process_user_input(input: Str) -> Result<UserData, ValidationError> {
    // Validate and process
}
```

### Memory Management

```once
// Use linear types for resources
fn process_file(path: Str) -> Result<Unit, IOError> !io {
    using file = File::open(path)? {
        // File is guaranteed to be closed
        let content = file.read_all()?
        process_content(content)
    }
}

// Avoid unnecessary allocations
fn sum_slice(slice: &[Int]) -> Int {
    let mut total = 0
    for &item in slice {
        total = total + item
    }
    total
}
```

### Concurrency

```once
// Prefer channels over shared state
fn worker(id: Int, input: Chan<WorkItem>, output: Chan<Result>) -> Unit {
    loop {
        let item = input.recv()
        let result = process_item(item)
        output.send(result)
    }
}

// Use async/await for I/O
async fn fetch_user_data(user_id: Int) -> UserData !io {
    let url = "https://api.example.com/users/".concat(user_id.to_string())
    let response = http_get(url).await
    parse_user_data(response.body)
}
```

## Troubleshooting

### Common Errors

```once
// Linear type not consumed
fn bad_example(file: File) -> Unit {
    // Error: linear resource 'file' not used
}

// Effect not declared
fn io_operation() -> Unit {  // Missing !io effect
    print("Hello")  // This performs I/O
}
```

### Debugging Tips

```bash
# Get detailed error information
once explain effects program.onc
once explain regions program.onc

# Check intermediate representations
once mir program.onc
once hir program.onc

# Run with verbose output
RUST_LOG=debug once build program.onc
```

## Performance Tips

### Memory Efficiency

```once
// Reuse allocations where possible
fn process_items(items: Vec<Item>) -> Vec<Result> {
    items.map(process_item)  // Avoids extra allocations
}

// Use stack allocation for small data
fn small_computation() -> Unit {
    let buffer: [u8; 1024] = [0; 1024]  // Stack allocated
    // Process buffer
}
```

### Concurrency Optimization

```once
// Use bounded channels to prevent memory bloat
let (tx, rx) = Chan::bounded(100)  // Limit queue size

// Prefer work-stealing for CPU-bound tasks
let pool = ThreadPool::work_stealing(4)
pool.spawn(|| expensive_computation())
```

## Migration from Other Languages

### From Rust

```rust
// Rust code
fn process_data(data: Vec<u8>) -> Result<String, Error> {
    // Implementation
}
```

```once
// Equivalent Once code
fn process_data(data: Vec<u8>) -> Result<Str, Error> !io {
    // Implementation - effects tracked automatically
}
```

### From Go

```go
// Go code
func processData(data []byte) (string, error) {
    // Implementation
}
```

```once
// Equivalent Once code
fn process_data(data: Vec<u8>) -> Result<Str, Error> !io {
    // Implementation - memory managed automatically
}
```

## Contributing

We welcome contributions! Please:

1. Read the [Contributing Guide](CONTRIBUTING.md)
2. Follow the code style guidelines
3. Add tests for new features
4. Update documentation

## Getting Help

- **Documentation**: [Full API Reference](https://docs.once-lang.org)
- **Community**: [Discord Server](https://discord.gg/once-lang)
- **Issues**: [GitHub Issues](https://github.com/once-lang/once/issues)

---

Happy coding with Once! 🎉