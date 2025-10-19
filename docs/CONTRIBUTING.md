# Contributing to Once Language

Thank you for your interest in contributing to the Once language project! This guide will help you get started with contributing to the project.

## Table of Contents

1. [Getting Started](#getting-started)
2. [Development Setup](#development-setup)
3. [Contributing Guidelines](#contributing-guidelines)
4. [Code Style](#code-style)
5. [Testing](#testing)
6. [Pull Request Process](#pull-request-process)
7. [Issue Reporting](#issue-reporting)

## Getting Started

### Prerequisites

- Rust 1.70+ (latest stable recommended)
- Git
- Basic understanding of compiler design
- Familiarity with systems programming concepts

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/your-username/once.git
   cd once
   ```

2. **Build the Project**
   ```bash
   cargo build --workspace
   ```

3. **Run Tests**
   ```bash
   cargo test --workspace
   ```

4. **Run Example Programs**
   ```bash
   ./target/debug/once build --input examples/hello_world.onc --output hello_world.o
   ```

## Contributing Guidelines

### Types of Contributions

We welcome various types of contributions:

- **Bug Fixes**: Fix issues in the compiler or runtime
- **Feature Implementation**: Implement new language features
- **Documentation**: Improve documentation and examples
- **Testing**: Add tests and improve test coverage
- **Performance**: Optimize compilation or runtime performance
- **Tooling**: Improve developer tools and IDE support

### Areas for Contribution

#### High Priority
- **Parser Improvements**: Better error messages and recovery
- **Type System**: Enhanced type inference and error reporting
- **Standard Library**: More modules and functionality
- **Documentation**: Tutorials and language guides
- **Testing**: More comprehensive test suites

#### Medium Priority
- **Optimizations**: Compiler optimizations and performance
- **LSP Features**: Better IDE integration
- **WebAssembly**: Full WebAssembly Component Model support
- **Package Manager**: Dependency management system

#### Low Priority
- **Advanced Features**: Experimental language features
- **Tooling**: Additional developer tools
- **Examples**: More example programs
- **Benchmarks**: Performance benchmarking suite

## Code Style

### Rust Code Style

We follow standard Rust conventions:

- Use `rustfmt` for code formatting
- Use `clippy` for linting
- Follow Rust naming conventions
- Write comprehensive documentation

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --workspace

# Run clippy with all warnings
cargo clippy --workspace -- -W clippy::all
```

### Documentation

- All public APIs must be documented
- Use `///` for doc comments
- Include examples in documentation
- Document error conditions

## Testing

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run tests for specific crate
cargo test -p once-lex

# Run tests with output
cargo test --workspace -- --nocapture
```

### Test Categories

1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test crate interactions
3. **End-to-End Tests**: Test complete compilation pipeline
4. **Property Tests**: Test language properties and invariants

### Adding Tests

When adding new features, include:

- **Unit tests** for individual functions
- **Integration tests** for feature interactions
- **Example programs** demonstrating the feature
- **Error case tests** for failure scenarios

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

## Pull Request Process

### Before Submitting

1. **Fork the repository** and create a feature branch
2. **Make your changes** following the code style guidelines
3. **Add tests** for your changes
4. **Update documentation** if necessary
5. **Run all tests** to ensure nothing is broken
6. **Commit your changes** with clear commit messages

### Commit Messages

Use clear, descriptive commit messages:

```
feat: add support for async/await syntax
fix: resolve parser error with nested expressions
docs: update language specification
test: add tests for linear type checking
```

### Pull Request Template

When creating a pull request, include:

1. **Description**: What changes were made and why
2. **Testing**: How the changes were tested
3. **Documentation**: Any documentation updates needed
4. **Breaking Changes**: Any breaking changes and migration path
5. **Related Issues**: Link to related issues

### Review Process

1. **Automated Checks**: CI/CD pipeline runs tests and checks
2. **Code Review**: Maintainers review the code
3. **Feedback**: Address any feedback or requested changes
4. **Approval**: Once approved, changes are merged

## Issue Reporting

### Bug Reports

When reporting bugs, include:

1. **Description**: Clear description of the issue
2. **Reproduction**: Steps to reproduce the issue
3. **Expected Behavior**: What should happen
4. **Actual Behavior**: What actually happens
5. **Environment**: OS, Rust version, etc.
6. **Code Example**: Minimal code example if applicable

### Feature Requests

When requesting features, include:

1. **Description**: Clear description of the feature
2. **Use Case**: Why the feature is needed
3. **Proposed Solution**: How the feature should work
4. **Alternatives**: Other approaches considered
5. **Implementation**: Any implementation ideas

### Issue Labels

We use labels to categorize issues:

- `bug`: Something isn't working
- `enhancement`: New feature or request
- `documentation`: Improvements to documentation
- `good first issue`: Good for newcomers
- `help wanted`: Extra attention is needed
- `priority: high`: High priority issue
- `priority: medium`: Medium priority issue
- `priority: low`: Low priority issue

## Development Workflow

### Branch Naming

Use descriptive branch names:

```
feature/async-await-support
fix/parser-error-messages
docs/language-specification
test/linear-type-checking
```

### Development Process

1. **Create Issue**: Discuss the change in an issue first
2. **Fork Repository**: Create your own fork
3. **Create Branch**: Create a feature branch
4. **Make Changes**: Implement your changes
5. **Add Tests**: Write tests for your changes
6. **Update Docs**: Update documentation if needed
7. **Submit PR**: Create a pull request
8. **Address Feedback**: Respond to review feedback
9. **Merge**: Once approved, changes are merged

## Code of Conduct

### Our Pledge

We are committed to providing a welcoming and inclusive environment for all contributors, regardless of:

- Age, body size, disability, ethnicity
- Gender identity and expression
- Level of experience, education
- Nationality, personal appearance
- Race, religion, sexual orientation

### Expected Behavior

- Use welcoming and inclusive language
- Be respectful of differing viewpoints
- Accept constructive criticism gracefully
- Focus on what is best for the community
- Show empathy towards other community members

### Unacceptable Behavior

- Harassment, trolling, or inflammatory comments
- Public or private harassment
- Publishing private information without permission
- Other unprofessional conduct

## Getting Help

### Resources

- **Documentation**: Check the docs/ directory
- **Examples**: Look at examples/ directory
- **Issues**: Search existing issues
- **Discussions**: Use GitHub Discussions
- **Discord**: Join our Discord server

### Questions

If you have questions:

1. **Check Documentation**: Look for existing documentation
2. **Search Issues**: Check if your question was asked before
3. **Ask in Discussions**: Use GitHub Discussions for questions
4. **Join Discord**: Chat with the community
5. **Create Issue**: If it's a bug or feature request

## Recognition

We appreciate all contributions, no matter how small! Contributors are recognized in:

- **README**: Listed as contributors
- **Release Notes**: Mentioned in release notes
- **Documentation**: Credited in documentation
- **Community**: Recognized in community discussions

## Conclusion

Thank you for contributing to Once! Your contributions help make Once a better language for everyone. If you have any questions or need help, don't hesitate to ask.

Happy coding! 🚀
