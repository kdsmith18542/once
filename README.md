# Once

**A modern systems programming language combining memory safety, performance, and simplicity.**

Once is a systems language with region-based memory management — no garbage collector, no borrow checker. It provides automatic memory management through compile-time region inference, linear types for resource safety, and row-polymorphic effects for capability tracking.

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue)](./LICENSE)
[![Rust](https://img.shields.io/badge/rust-edition%202021-orange)](./Cargo.toml)

## Features

- **Region-based memory management** — Automatic allocation/deallocation via static lifetime inference. No GC pauses, no manual `malloc`/`free`.
- **Linear types** — Resources tracked at compile time; use-after-free, double-free, and leaks prevented.
- **Hindley-Milner type inference** — Full type inference with polymorphism. Annotate only where you want to.
- **Row-polymorphic effects** — Track I/O, networking, spawning, and other side effects in the type system.
- **Actor-based concurrency** — Isolated actors communicate via channels. Deterministic work-stealing scheduler with deadlock detection.
- **Cranelift codegen** — Compiles to native machine code via Cranelift. Fast compilation, good runtime performance.
- **LSP server** — IDE support with diagnostics, completion, hover, go-to-definition, rename, and code actions.
- **Capability-aware linker** — Link-time enforcement of effect ceilings and type compatibility.
- **WebAssembly support** — WASM Component Model output with PCC-lite validation.

## Quick Start

```bash
# Build from source (requires Rust)
git clone https://github.com/once-lang/once.git
cd once
cargo build --release

# Compile a program
echo 'fn main() -> Unit { print("Hello, Once!") }' > hello.onc
./target/release/once build --input hello.onc
```

## Language at a Glance

```once
// Functions with type inference
fn greet(name: Str) -> Unit {
    print("Hello, ".concat(name).concat("!"))
}

// Linear resources — must be consumed exactly once
fn read_config() -> File {
    let file = File.open("config.onc")
    // Compiler ensures file is consumed
    return file
}

// Actors with message-passing
actor Counter {
    state: Int

    fn inc() -> Unit {
        state = state + 1
    }

    fn get() -> Int {
        return state
    }
}

fn main() -> Unit {
    let counter = spawn(Counter { 0 })
    counter.send(Counter.inc)
    let value = counter.send(Counter.get)
    print("Count: " + value.to_str())
}
```

More examples in [`examples/`](./examples/).

## Architecture

The compiler is built as 21 modular Rust crates:

```
Source (.onc)
    │
    ▼
┌──────────────┐   ┌──────────┐   ┌──────────┐
│  once-lex    │ → │once-parse│ → │ once-hir │   Frontend
│  (lexer)     │   │ (parser) │   │ (name IR)│
└──────────────┘   └──────────┘   └──────────┘
                                        │
    ┌───────────────────────────────────┤
    ▼                  ▼                ▼
┌──────────┐   ┌──────────────┐  ┌──────────────┐
│ once-ty  │   │ once-linear  │  │  once-rinf   │   Middle-end
│ (types)  │   │ (linearity)  │  │  (regions)   │
└──────────┘   └──────────────┘  └──────────────┘
    │                  │                │
    └──────────────────┼────────────────┘
                       ▼
              ┌──────────────┐   ┌──────────┐
              │   once-mir   │ → │once-opt  │   IR + Optimize
              │   (MIR IR)   │   │(optimize)│
              └──────────────┘   └──────────┘
                       │
                       ▼
           ┌───────────────────────┐
           │    once-codegen       │   Backend
           │    (Cranelift)        │
           └───────────────────────┘
                       │
                       ▼
                  Native .o
```

Read the full architecture doc in [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md).

## Repository Structure

```
once/
├── crates/              # 21 compiler crates
│   ├── once-lex/        # Lexer (logos)
│   ├── once-parse/      # Recursive-descent parser
│   ├── once-hir/        # High-level IR + name resolution
│   ├── once-ty/         # HM type inference + effects
│   ├── once-linear/     # Linear type checker
│   ├── once-rinf/       # Region inference solver
│   ├── once-mir/        # Mid-level IR + verifier
│   ├── once-opt/        # Optimization passes
│   ├── once-codegen/    # Cranelift code generation
│   ├── once-runtime/    # Work-stealing runtime
│   ├── once-actors/     # Actor model
│   ├── once-std/        # Standard library
│   ├── once-cli/        # CLI binary
│   ├── once-build/      # Build system
│   ├── once-lsp/        # Language Server Protocol
│   ├── once-linker/     # Capability-aware linker
│   ├── once-lockfile/   # Content-addressed lockfile
│   ├── once-onceo/      # .onceo object format
│   ├── once-wasm/       # WebAssembly support
│   ├── once-explain/    # Diagnostic visualization
│   └── once-bounds/     # Bounds checking proofs
├── examples/            # Example Once programs
├── tests/               # Test suite
├── docs/                # Specifications & guides
└── .github/             # CI/CD & templates
```

## Building & Testing

```bash
# Build
cargo build --release

# Run all tests
cargo test

# Test a specific crate
cargo test -p once-parse

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt --all -- --check
```

## Documentation

- [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) — Compiler architecture
- [`docs/USER_GUIDE.md`](./docs/USER_GUIDE.md) — Language user guide
- [`docs/`](./docs/) — Language specifications (Parts 1–9)
- [`CONTRIBUTING.md`](./CONTRIBUTING.md) — Contribution guide
- [`CHANGELOG.md`](./CHANGELOG.md) — Release history

## Contributing

Contributions are welcome. See [`CONTRIBUTING.md`](./CONTRIBUTING.md) for the workflow and [`CODE_OF_CONDUCT.md`](./CODE_OF_CONDUCT.md) for community guidelines.

## License

Licensed under either of [MIT License](./LICENSE) or [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0), at your option.
