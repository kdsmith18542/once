# Once Language Architecture

This document describes the architecture and design decisions of the Once language compiler and runtime.

## Overview

Once is designed as a modern systems programming language that combines memory safety, performance, and simplicity. The compiler is built as a collection of 21 modular Rust crates, each responsible for a specific aspect of compilation.

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
- **Purpose**: Hindley-Milner type inference with row-polymorphic effects
- **Features**:
  - Type variable management
  - Constraint solving
  - Type scheme generalization
  - Polymorphic type inference
  - Effect row construction, constraint solving, and polymorphism (includes effects module at `src/effects.rs`)
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
  - Constraint solving and free placement

#### Bounds Checking (`once-bounds`)
- **Purpose**: Compile-time bounds checking and proof generation
- **Features**:
  - Array bounds verification
  - Proof generation
  - Check erasure
  - Optimization opportunities

### Backend

#### Mid-Level IR (`once-mir`)
- **Purpose**: Lowered IR with explicit operations and control-flow graph
- **Features**:
  - Explicit move and drop operations
  - Region allocation and free operations
  - Bounds check annotations with proof status
  - Function calls, channel operations, spawn/await, group concurrency
  - SSA-like basic blocks with labels, jumps, and branches

#### Optimization (`once-opt`)
- **Purpose**: MIR-level optimization passes
- **Features**:
  - Dead code elimination
  - Move optimization
  - Copy elision
  - Region coalescing

#### Code Generation (`once-codegen`)
- **Purpose**: Generate machine code from MIR
- **Technology**: Cranelift backend
- **Features**:
  - Register allocation
  - Instruction selection
  - Object file generation
  - Assembly output

#### Object Format (`once-onceo`)
- **Purpose**: .onceo object file format with type/effect/region summaries
- **Features**:
  - Module-level metadata
  - Type and effect summaries for link-time checks
  - Region information for inter-module optimization

#### Linker (`once-linker`)
- **Purpose**: Link .onceo object files into executables
- **Features**:
  - Capability ceiling enforcement
  - Version deduplication via namespacing
  - Effect compatibility verification

### Runtime

#### Runtime System (`once-runtime`)
- **Purpose**: Runtime support for concurrency, I/O, and memory management
- **Features**:
  - Deterministic work-stealing scheduler
  - Channel implementation with backpressure policies
  - Actor system with message-passing
  - Deadlock detection (wait-for graph cycle detection)
  - Task groups for structured concurrency
  - Region-based memory management with arena allocation
  - Effect registry for runtime capability tracking

#### Standard Library (`once-std`)
- **Purpose**: Core types and operations
- **Features**:
  - Linear types (File, TcpStream, etc.)
  - Resource management (Resource trait)
  - Memory allocation
  - I/O operations

#### Actor Model (`once-actors`)
- **Purpose**: Message-passing concurrency with actor model
- **Features**:
  - Actor spawning with mailbox channels
  - Message handling loops
  - Supervisor patterns
  - Fault tolerance

### Tooling & Developer Experience

#### CLI (`once-cli`)
- **Purpose**: Command-line interface for all development tasks
- **Features**:
  - `once build`, `once run`, `once test`, `once fmt`, `once lint`
  - `once new` project scaffolding
  - `once explain` diagnostic visualization

#### Build System (`once-build`)
- **Purpose**: Project management and dependency resolution
- **Features**:
  - Hermetic builds
  - Dependency management
  - Build caching
  - Capability enforcement
  - Test compilation and running

#### Lockfile (`once-lockfile`)
- **Purpose**: Content-addressed dependency version locking
- **Features**:
  - Cryptographic hash verification
  - Transitive dependency graph resolution
  - Effect and capability ceiling recording

#### Language Server (`once-lsp`)
- **Purpose**: IDE integration and developer experience
- **Features**:
  - Syntax highlighting
  - Error reporting and diagnostics
  - Code completion
  - Go-to-definition
  - Hover type/effect display

#### Diagnostics & Explain (`once-explain`)
- **Purpose**: Rich diagnostic visualization and fix-its
- **Features**:
  - Region graph visualization
  - Effect derivation tracing
  - Linearity chain debugging
  - Actionable fix-it suggestions

### Interop & Platform

#### WebAssembly Support (`once-wasm`)
- **Purpose**: WebAssembly Component Model integration
- **Features**:
  - Component interface generation
  - PCC-lite validation
  - Cross-language interoperability
  - Security guarantees

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
3. **Row-Polymorphic Effects**: Track computational effects (integrated in `once-ty`)
4. **Region Types**: Track memory lifetimes

### Concurrency Model

Once provides multiple concurrency primitives:

1. **Actors**: Isolated processes with message-passing
2. **Channels**: Type-safe communication
3. **Async/Await**: Structured concurrency
4. **Deterministic Scheduler**: Reproducible execution
5. **Task Groups**: Structured concurrency with guaranteed completion

## Performance Considerations

### Compilation Performance
- **Incremental Compilation**: Only recompile changed modules
- **Parallel Processing**: Utilize multiple cores for compilation
- **Caching**: Cache intermediate results
- **Lazy Evaluation**: Defer expensive operations

### Runtime Performance
- **Zero-cost Abstractions**: No runtime overhead for safety features
- **Native Code Generation**: Compile to optimized machine code
- **Memory Efficiency**: Minimal memory overhead with bulk region deallocation
- **Concurrency**: Work-stealing scheduler with per-worker deques

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

### Supply-Chain Security
- **Capability Enforcement**: Build-time verification of declared effects
- **Lockfile Integrity**: Cryptographic hash verification of all dependencies
- **Reproducible Builds**: Byte-for-byte identical output

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
