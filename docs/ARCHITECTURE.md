# Once Language Architecture

This document describes the architecture and design decisions of the Once language compiler and runtime.

## Overview

Once is designed as a modern systems programming language that combines memory safety, performance, and simplicity. The compiler is built as a collection of modular Rust crates, each responsible for a specific aspect of compilation.

## Compiler Architecture

### Frontend

#### Lexical Analysis (`once-lex`)
- **Purpose**: Tokenize source code into tokens
- **Technology**: `logos` crate for efficient lexing
- **Features**: 
  - Keyword recognition
  - Identifier and literal parsing
  - Comment handling
  - Error reporting with source locations

#### Parsing (`once-parse`)
- **Purpose**: Parse tokens into Abstract Syntax Tree (AST)
- **Technology**: Manual recursive descent parser
- **Features**:
  - Expression parsing with operator precedence
  - Statement and declaration parsing
  - Pattern matching support
  - Error recovery and reporting

#### High-Level IR (`once-hir`)
- **Purpose**: Name resolution and desugaring
- **Features**:
  - Symbol table management
  - Import resolution
  - Macro expansion
  - Type annotation inference

### Middle-End

#### Type System (`once-ty`)
- **Purpose**: Hindley-Milner type inference
- **Features**:
  - Type variable management
  - Constraint solving
  - Type scheme generalization
  - Polymorphic type inference

#### Effects System (`once-effects`)
- **Purpose**: Row-polymorphic effect tracking
- **Features**:
  - Effect row construction
  - Effect constraint solving
  - Effect polymorphism
  - Async/await effect tracking

#### Linearity System (`once-linear`)
- **Purpose**: Linear type checking and resource safety
- **Features**:
  - Move/consume checking
  - Resource safety verification
  - Copy trait constraints
  - Closure capture analysis

#### Region Inference (`once-rinf`)
- **Purpose**: Static lifetime inference and escape analysis
- **Features**:
  - Region DAG construction
  - Liveness analysis
  - Escape analysis
  - Cycle detection

### Backend

#### Mid-Level IR (`once-mir`)
- **Purpose**: Lowered IR with explicit operations
- **Features**:
  - Explicit move operations
  - Drop operations
  - Region frees
  - Function calls and returns

#### Code Generation (`once-codegen`)
- **Purpose**: Generate machine code from MIR
- **Technology**: Cranelift backend
- **Features**:
  - Register allocation
  - Instruction selection
  - Object file generation
  - Assembly output

### Runtime

#### Runtime System (`once-runtime`)
- **Purpose**: Runtime support for concurrency and I/O
- **Features**:
  - Deterministic scheduler
  - Channel implementation
  - Actor system
  - Deadlock detection

#### Standard Library (`once-std`)
- **Purpose**: Core types and operations
- **Features**:
  - Linear types (File, TcpStream, etc.)
  - Resource management
  - Memory allocation
  - I/O operations

## Language Design

### Memory Management

Once uses region-based memory management, which provides:

1. **Automatic Memory Management**: No manual memory management required
2. **No Garbage Collection**: Zero runtime overhead
3. **Static Analysis**: Compile-time memory safety guarantees
4. **Escape Analysis**: Automatic detection of memory leaks

### Type System

The type system combines several advanced features:

1. **Hindley-Milner Inference**: Automatic type inference
2. **Linear Types**: Track resource usage and prevent leaks
3. **Row-Polymorphic Effects**: Track computational effects
4. **Region Types**: Track memory lifetimes

### Concurrency Model

Once provides multiple concurrency primitives:

1. **Actors**: Isolated processes with message-passing
2. **Channels**: Type-safe communication
3. **Async/Await**: Structured concurrency
4. **Deterministic Scheduler**: Reproducible execution

## Advanced Features

### Language Server Protocol (`once-lsp`)
- **Purpose**: IDE integration and developer experience
- **Features**:
  - Syntax highlighting
  - Error reporting
  - Code completion
  - Go-to-definition
  - Refactoring support

### Build System (`once-build`)
- **Purpose**: Project management and dependency resolution
- **Features**:
  - Hermetic builds
  - Dependency management
  - Build caching
  - Parallel execution

### Bounds Checking (`once-bounds`)
- **Purpose**: Compile-time bounds checking
- **Features**:
  - Array bounds verification
  - Proof generation
  - Check erasure
  - Optimization opportunities

### Actor Model (`once-actors`)
- **Purpose**: Message-passing concurrency
- **Features**:
  - Actor spawning
  - Message handling
  - Supervisor patterns
  - Fault tolerance

### WebAssembly Support (`once-wasm`)
- **Purpose**: WebAssembly Component Model integration
- **Features**:
  - Component interface generation
  - PCC-lite validation
  - Cross-language interoperability
  - Security guarantees

## Performance Considerations

### Compilation Performance
- **Incremental Compilation**: Only recompile changed modules
- **Parallel Processing**: Utilize multiple cores for compilation
- **Caching**: Cache intermediate results
- **Lazy Evaluation**: Defer expensive operations

### Runtime Performance
- **Zero-cost Abstractions**: No runtime overhead for safety features
- **Native Code Generation**: Compile to optimized machine code
- **Memory Efficiency**: Minimal memory overhead
- **Concurrency**: Efficient scheduling and communication

## Security Features

### Memory Safety
- **No Buffer Overflows**: Bounds checking and region analysis
- **No Use-After-Free**: Region inference and lifetime tracking
- **No Memory Leaks**: Linear types and automatic cleanup
- **No Double-Free**: Move semantics and ownership tracking

### Concurrency Safety
- **No Data Races**: Actor isolation and message-passing
- **No Deadlocks**: Deterministic scheduling and cycle detection
- **No Race Conditions**: Linear types and ownership
- **No Resource Leaks**: Automatic cleanup and resource management

## Future Directions

### Planned Features
1. **Advanced Optimizations**: More sophisticated optimization passes
2. **Standard Library**: Expanded standard library with more modules
3. **Package Manager**: Dependency management and versioning
4. **IDE Integrations**: Better IDE support and tooling
5. **WebAssembly**: Full WebAssembly Component Model support

### Research Areas
1. **Formal Verification**: Mathematical proofs of correctness
2. **Performance Analysis**: Advanced performance profiling
3. **Language Extensions**: New language features and constructs
4. **Tooling**: Better developer tools and debugging support

## Conclusion

Once represents a novel approach to systems programming, combining the safety of modern languages with the performance of traditional systems languages. The modular architecture allows for incremental development and easy extension, while the advanced type system provides strong safety guarantees without sacrificing performance.
