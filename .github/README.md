# Once Language - GitHub Repository

This repository contains the complete implementation of the Once programming language compiler and associated tools.

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/once-lang/once.git
cd once

# Build the compiler
cargo build --release

# Run a simple program
echo 'fn main() -> Unit { print("Hello, Once!") }' > hello.onc
cargo run --bin once build --input hello.onc --output hello.o
```

## 📁 Repository Structure

```
once/
├── crates/                 # Rust crates for the compiler
│   ├── once-lex/          # Lexer
│   ├── once-parse/        # Parser
│   ├── once-hir/          # High-level IR
│   ├── once-ty/           # Type system
│   ├── once-effects/      # Effects system
│   ├── once-linear/       # Linearity system
│   ├── once-rinf/         # Region inference
│   ├── once-mir/          # Mid-level IR
│   ├── once-codegen/      # Code generation
│   ├── once-runtime/      # Runtime
│   ├── once-std/          # Standard library
│   ├── once-cli/          # CLI tool
│   ├── once-build/        # Build tool
│   ├── once-lsp/          # Language Server Protocol
│   └── ...                # Additional crates
├── examples/              # Example programs
├── tests/                 # Test suite
├── docs/                  # Documentation
└── .github/              # GitHub configuration
```

## 🧪 Testing

The repository includes a comprehensive test suite:

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test --test integration_tests
cargo test --test unit_tests
cargo test --test performance_tests
cargo test --test regression_tests
cargo test --test benchmark_tests
```

## 📚 Documentation

- [README.md](../README.md) - Project overview and getting started
- [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) - Compiler architecture
- [docs/LANGUAGE_SPEC.md](../docs/LANGUAGE_SPEC.md) - Language specification
- [docs/CONTRIBUTING.md](../docs/CONTRIBUTING.md) - Contributing guidelines
- [tests/README.md](../tests/README.md) - Test suite documentation

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests for new functionality
5. Ensure all tests pass
6. Submit a pull request

### Code Review Process

- All changes require review from maintainers
- Code must pass all tests and CI checks
- Documentation must be updated for new features
- Performance impact must be considered

## 🔒 Security

Security vulnerabilities should be reported privately to security@once-lang.org.

See our [Security Policy](SECURITY.md) for more information.

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🏷️ Releases

Releases are automatically created from Git tags. See [CHANGELOG.md](CHANGELOG.md) for release notes.

## 📊 Project Status

- **Compiler**: ✅ Complete
- **Runtime**: ✅ Complete
- **Standard Library**: ✅ Complete
- **Tools**: ✅ Complete
- **Documentation**: ✅ Complete
- **Tests**: ✅ Complete

## 🎯 Roadmap

- [ ] Performance optimizations
- [ ] Additional language features
- [ ] Enhanced tooling
- [ ] Community packages
- [ ] IDE integrations

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/once-lang/once/issues)
- **Discussions**: [GitHub Discussions](https://github.com/once-lang/once/discussions)
- **Email**: support@once-lang.org

## 🌟 Acknowledgments

Thanks to all contributors and the Rust community for the excellent tooling and ecosystem.
