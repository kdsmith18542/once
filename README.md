# Once Language

A simple-to-learn systems programming language with region-based memory management, linear types, and advanced concurrency features.

## Overview

Once is designed to be a modern systems programming language that combines the safety of Rust with the simplicity of Go, while introducing novel concepts like region-based memory management and linear types for resource safety.

## Key Features

### 🛡️ Memory Safety
- **Region-based Memory Management**: Automatic memory management without garbage collection
- **Linear Types**: Prevent resource leaks and ensure proper cleanup
- **Static Analysis**: Compile-time guarantees for memory safety

### ⚡ Concurrency
- **Actor Model**: Message-passing concurrency with actors
- **Channels**: Type-safe communication between concurrent processes
- **Async/Await**: Structured concurrency with async/await syntax
- **Deterministic Scheduler**: Reproducible concurrency for testing

### 🔧 Type System
- **Hindley-Milner Type Inference**: Automatic type inference
- **Linear Types**: Track resource usage and prevent leaks
- **Row-Polymorphic Effects**: Track computational effects
- **Region Inference**: Static lifetime analysis

### 🚀 Performance
- **Zero-cost Abstractions**: No runtime overhead for safety features
- **Cranelift Backend**: Fast compilation with LLVM-quality code generation
- **Native Performance**: Compiles to native machine code

## Quick Start

### Installation

```bash
# Clone the repository
git clone https://github.com/once-lang/once.git
cd once

# Build the compiler
cargo build --release

# The compiler will be available at ./target/release/once
```

### Hello World

Create a file `hello.onc`:

```once
fn main() -> Unit {
    print("Hello, World!")
}
```

Compile and run:

```bash
./target/release/once build --input hello.onc --output hello.o
```

### Linear Resources

```once
use once_std::io::File;

fn write_log(path: Str, line: Str) -> Unit !io {
    using f = File::create(path) {
        f.write(line)
    }
}

fn main() -> Unit !io {
    write_log("log.txt", "This is a log entry.")
}
```

### Concurrency

```once
use once_runtime::channel::Chan;

fn producer(output: Chan<Str>) -> Unit !spawn {
    output.send("Hello from producer")
}

fn consumer(input: Chan<Str>) -> Unit !io {
    let msg = input.recv();
    print(msg);
}

fn main() -> Unit !spawn, io {
    let (tx, rx) = Chan::new();
    
    spawn producer(tx);
    spawn consumer(rx);
}
```

## Language Features

### Type System

Once features a sophisticated type system that provides safety without sacrificing performance:

- **Primitive Types**: `Int`, `Float`, `Bool`, `Str`, `Unit`
- **Linear Types**: `File`, `TcpStream`, `Channel` - must be consumed exactly once
- **Effect Types**: Track computational effects like `!io`, `!spawn`, `!async`
- **Region Types**: Track memory lifetimes and prevent use-after-free

### Memory Management

Once uses region-based memory management, which provides:

- **Automatic Memory Management**: No manual memory management required
- **No Garbage Collection**: Zero runtime overhead
- **Static Analysis**: Compile-time memory safety guarantees
- **Escape Analysis**: Automatic detection of memory leaks

### Concurrency Model

Once provides multiple concurrency primitives:

- **Actors**: Isolated processes with message-passing
- **Channels**: Type-safe communication
- **Async/Await**: Structured concurrency
- **Deterministic Scheduler**: Reproducible execution for testing

## Compiler Architecture

The Once compiler is built as a collection of modular Rust crates:

### Core Components

- **`once-lex`**: Lexical analysis and tokenization
- **`once-parse`**: Parser and AST generation
- **`once-hir`**: High-level IR with name resolution
- **`once-ty`**: Type system and inference
- **`once-effects`**: Effect system and tracking
- **`once-linear`**: Linear type checking
- **`once-rinf`**: Region inference and analysis
- **`once-mir`**: Mid-level IR with explicit moves
- **`once-codegen`**: Code generation with Cranelift
- **`once-runtime`**: Runtime system and scheduler

### Advanced Features

- **`once-lsp`**: Language Server Protocol support
- **`once-build`**: Build system and dependency management
- **`once-bounds`**: Bounds checking and proof generation
- **`once-actors`**: Actor model implementation
- **`once-wasm`**: WebAssembly Component Model support
- **`once-explain`**: Detailed error explanations

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/once-lang/once.git
cd once

# Build all crates
cargo build --workspace

# Run tests
cargo test --workspace

# Build release version
cargo build --release --workspace
```

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p once-lex

# Run integration tests
./test_compiler.sh
```

### Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

## Examples

### Basic Syntax

```once
// Function definition
fn add(x: Int, y: Int) -> Int {
    x + y
}

// Pattern matching
fn factorial(n: Int) -> Int {
    match n {
        0 => 1,
        n => n * factorial(n - 1)
    }
}

// Linear resources
fn process_file(path: Str) -> Unit !io {
    using file = File::open(path) {
        let content = file.read_all();
        print(content)
    }
}
```

### Concurrency

```once
// Actor system
fn main() -> Unit !spawn {
    let system = ActorSystem::new();
    let actor = system.spawn("worker", worker_behavior);
    actor.send("Hello, Actor!");
}

fn worker_behavior(msg: Str) -> Unit {
    print("Received: ".concat(msg))
}
```

### Advanced Types

```once
// Generic functions
fn map<A, B>(f: fn(A) -> B, xs: Vec<A>) -> Vec<B> {
    // Implementation
}

// Effect polymorphism
fn read_file(path: Str) -> Str !io {
    // File I/O operations
}

// Linear types
fn consume_resource(resource: LinearResource) -> Unit {
    // Resource is consumed here
}
```

## Performance

Once is designed for high performance:

- **Fast Compilation**: Incremental compilation with caching
- **Native Performance**: Compiles to optimized machine code
- **Zero Runtime Overhead**: No garbage collection or runtime checks
- **Memory Efficient**: Region-based management with minimal overhead

## Safety Features

Once provides comprehensive safety guarantees:

- **Memory Safety**: No buffer overflows, use-after-free, or memory leaks
- **Type Safety**: Static type checking prevents runtime errors
- **Concurrency Safety**: No data races or deadlocks
- **Resource Safety**: Automatic cleanup of linear resources

## Roadmap

### Current Status
- ✅ Core language features
- ✅ Type system and inference
- ✅ Memory management
- ✅ Concurrency primitives
- ✅ Cranelift backend
- ✅ LSP support

### Upcoming Features
- 🔄 WebAssembly Component Model
- 🔄 Advanced optimizations
- 🔄 Standard library expansion
- 🔄 IDE integrations
- 🔄 Package manager

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- **Cranelift**: For the excellent code generation backend
- **Rust**: For the inspiration on memory safety
- **Go**: For the simplicity and concurrency model
- **Haskell**: For the type system design

## Community

- **Discord**: [Join our Discord server](https://discord.gg/once-lang)
- **GitHub**: [Report issues and contribute](https://github.com/once-lang/once)
- **Documentation**: [Read the full documentation](https://docs.once-lang.org)

---

**Once Language** - Simple, Safe, Fast Systems Programming