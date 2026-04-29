# Once Language - Contributing Guide

Thank you for your interest in contributing to the Once programming language! This guide will help you get started with contributing to the project.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Environment](#development-environment)
3. [Project Structure](#project-structure)
4. [Coding Standards](#coding-standards)
5. [Testing](#testing)
6. [Submitting Changes](#submitting-changes)
7. [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites

- **Rust**: Version 1.70 or later
- **Cargo**: Latest stable version
- **Git**: For version control

### Clone and Setup

```bash
# Clone the repository
git clone https://github.com/once-lang/once.git
cd once

# Build the project
cargo build

# Run tests to ensure everything works
cargo test
```

### First Contribution

If you're new to the project, look for issues labeled "good first issue" or "help wanted". These are good starting points for new contributors.

## Development Environment

### Recommended Tools

- **VS Code** with Rust extension for IDE support
- **rust-analyzer** for advanced language features
- **rustfmt** for code formatting
- **clippy** for linting

### Editor Configuration

For VS Code, use this `.vscode/settings.json`:

```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

## Project Structure

```
once/
├── crates/                 # All Rust crates
│   ├── once-lex/          # Lexer
│   ├── once-parse/        # Parser
│   ├── once-hir/          # High-level IR
│   ├── once-ty/           # Type system
│   ├── once-effects/      # Effect system
│   ├── once-linear/       # Linear type checking
│   ├── once-rinf/         # Region inference
│   ├── once-mir/          # Mid-level IR
│   ├── once-codegen/      # Code generation
│   ├── once-runtime/      # Runtime system
│   ├── once-std/          # Standard library
│   └── once-cli/          # Command-line interface
├── examples/              # Example programs
├── tests/                 # Integration tests
├── docs/                  # Documentation
├── scripts/               # Build and utility scripts
└── Cargo.toml             # Workspace configuration
```

### Key Crates

- **`once-lex`**: Tokenizes source code
- **`once-parse`**: Builds AST from tokens
- **`once-hir`**: Performs name resolution and desugaring
- **`once-ty`**: Type inference and checking
- **`once-codegen`**: Generates machine code via Cranelift
- **`once-cli`**: Command-line interface and orchestration

## Coding Standards

### Rust Style Guidelines

We follow the official Rust style guidelines:

```rust
// Good: Clear naming, proper indentation
fn calculate_total(items: &[Item]) -> f64 {
    let mut total = 0.0;
    for item in items {
        total += item.price * item.quantity as f64;
    }
    total
}

// Avoid: Non-idiomatic code
fn calc_total(i: &Vec<Item>) -> f64 {
    let mut t = 0f64;
    for x in i { t = t + x.price * (x.quantity as f64); }
    t
}
```

### Naming Conventions

- **Functions**: `snake_case` (e.g., `parse_expression`)
- **Types**: `PascalCase` (e.g., `Expression`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_STACK_SIZE`)
- **Fields**: `snake_case` (e.g., `field_name`)

### Documentation

All public APIs must be documented:

```rust
/// Parses a Once source file into an AST.
///
/// This function performs lexical analysis followed by parsing
/// to produce an abstract syntax tree.
///
/// # Arguments
/// * `source` - The source code as a string
///
/// # Returns
/// A `Result` containing the parsed AST or a parse error
///
/// # Examples
/// ```
/// let ast = parse_once_file("fn main() { print(\"Hello\") }")?;
/// ```
pub fn parse_once_file(source: &str) -> Result<Ast, ParseError> {
    // Implementation...
}
```

### Error Handling

Use appropriate error types and provide helpful messages:

```rust
// Good: Specific error type with context
#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Undefined variable '{name}' at {span}")]
    UndefinedVariable { name: String, span: Span },
    
    #[error("Type mismatch: expected {expected}, got {actual} at {span}")]
    TypeMismatch { 
        expected: Type, 
        actual: Type, 
        span: Span 
    },
}

// Avoid: Generic error messages
return Err("Something went wrong".to_string());
```

## Testing

### Unit Tests

Place unit tests in the same file as the code they test:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_addition() {
        assert_eq!(add(2, 3), 5);
    }
    
    #[test]
    fn test_overflow() {
        assert!(add(i64::MAX, 1).is_err());
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory:

```rust
// tests/compiler_tests.rs
#[test]
fn test_hello_world_compilation() {
    let source = r#"
        fn main() -> Unit {
            print("Hello, World!")
        }
    "#;
    
    let result = compile_once_code(source);
    assert!(result.is_ok());
    
    let output = result.unwrap();
    assert!(output.contains("Hello, World!"));
}
```

### Property-Based Testing

Use proptest for complex test cases:

```rust
proptest! {
    #[test]
    fn doesnt_crash_on_any_input(s in "\\PC*") {
        let result = parse_once_code(&s);
        // Just ensure it doesn't panic
        let _ = result;
    }
}
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_hello_world

# Run tests for specific crate
cargo test -p once-lex

# Run with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out Html
```

## Submitting Changes

### Git Workflow

1. **Fork** the repository on GitHub
2. **Clone** your fork locally
3. **Create** a feature branch
4. **Make** your changes
5. **Test** thoroughly
6. **Commit** with clear messages
7. **Push** to your fork
8. **Create** a Pull Request

### Branch Naming

Use descriptive branch names:

```bash
# Good
git checkout -b feature/add-pattern-matching
git checkout -b fix/parser-crash-on-empty-files
git checkout -b refactor/simplify-type-inference

# Avoid
git checkout -b my-branch
git checkout -b fix-stuff
```

### Commit Messages

Follow conventional commit format:

```bash
# Good
feat: add support for tuple destructuring
fix: prevent crash on malformed UTF-8 input
docs: update API reference for new functions
refactor: simplify error handling in parser

# Avoid
fixed bug
updated code
changes
```

### Pull Request Guidelines

**Title**: Clear, descriptive summary of changes

**Description**: 
- What problem does this solve?
- How was it implemented?
- Any breaking changes?
- Tests included?

**Checklist**:
- [ ] Tests pass (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Clippy warnings fixed (`cargo clippy`)
- [ ] Documentation updated
- [ ] No breaking changes (or documented)

## Code Review Process

### Review Checklist

**For Reviewers**:
- [ ] Code compiles without warnings
- [ ] Tests pass and coverage maintained
- [ ] Documentation is accurate and complete
- [ ] Error messages are helpful
- [ ] Performance implications considered
- [ ] Security implications reviewed
- [ ] API design follows project conventions

**For Contributors**:
- [ ] Address all review feedback
- [ ] Update tests if behavior changed
- [ ] Update documentation
- [ ] Rebase on latest main branch
- [ ] Squash commits if requested

### Common Issues

**Performance**: Always consider algorithmic complexity

**Memory Safety**: Rust guarantees must be maintained

**API Stability**: Public APIs should be stable

**Error Handling**: Errors should be actionable

## Community Guidelines

### Code of Conduct

We follow a code of conduct to ensure a welcoming environment:

- **Be respectful**: Treat others with kindness and respect
- **Be collaborative**: Work together to achieve common goals  
- **Be inclusive**: Welcome people from all backgrounds
- **Be patient**: Remember that everyone is learning

### Communication

- **GitHub Issues**: For bugs, features, and general discussion
- **GitHub Discussions**: For longer-form discussions
- **Discord**: For real-time chat and quick questions
- **Email**: For private matters (maintainers@once-lang.org)

### Getting Help

- **Documentation**: Check the [User Guide](USER_GUIDE.md) first
- **Issues**: Search existing issues before creating new ones
- **Community**: Ask questions on Discord or GitHub Discussions
- **Mentorship**: New contributors can request a mentor

## Advanced Topics

### Compiler Architecture

The Once compiler follows a multi-stage pipeline:

1. **Lexing** (`once-lex`): Convert source to tokens
2. **Parsing** (`once-parse`): Build AST from tokens  
3. **HIR** (`once-hir`): Name resolution and desugaring
4. **Type Checking** (`once-ty`): Infer and check types
5. **Effects** (`once-effects`): Check computational effects
6. **Linearity** (`once-linear`): Verify linear resource usage
7. **Regions** (`once-rinf`): Infer memory regions
8. **MIR** (`once-mir`): Lower to mid-level IR
9. **Code Generation** (`once-codegen`): Generate machine code

### Adding New Features

When adding major features:

1. **Discuss** the design in an issue first
2. **Prototype** the implementation
3. **Write tests** before implementation
4. **Document** the feature thoroughly
5. **Consider backwards compatibility**

### Performance Optimization

- **Profile** before optimizing
- **Benchmark** improvements
- **Consider** algorithmic complexity
- **Test** edge cases
- **Document** performance characteristics

### Security Considerations

- **Input validation**: Validate all inputs
- **Resource limits**: Prevent resource exhaustion
- **Memory safety**: Rely on Rust's guarantees
- **Cryptography**: Use vetted implementations
- **Supply chain**: Pin dependency versions

## Recognition

Contributors are recognized in:

- **CHANGELOG.md**: For significant changes
- **AUTHORS.md**: For substantial contributions
- **GitHub**: Through contributor statistics
- **Community**: Through shoutouts and mentions

## License

By contributing to Once, you agree that your contributions will be licensed under the same MIT license as the project.

---

Thank you for contributing to Once! Your efforts help make systems programming safer, simpler, and more productive for everyone. 🚀