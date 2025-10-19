# Changelog

All notable changes to the Once language project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial implementation of the Once language compiler
- Lexer with support for all basic tokens
- Parser with recursive descent implementation
- HIR (High-level Intermediate Representation) generation
- Type system with Hindley-Milner inference
- Effects system with row-polymorphic effects
- Linearity system with move/consume checking
- Region inference with static lifetime analysis
- MIR (Mid-level Intermediate Representation) generation
- Code generation with Cranelift backend
- Runtime with actor model and channels
- Standard library with core types and resources
- CLI tool with comprehensive command interface
- LSP (Language Server Protocol) implementation
- Build tool with hermetic builds and dependency management
- Comprehensive test suite with integration, unit, performance, and regression tests
- Documentation including README, architecture guide, language specification, and contributing guidelines
- GitHub repository structure with CI/CD, issue templates, and security policies

### Changed
- N/A

### Deprecated
- N/A

### Removed
- N/A

### Fixed
- N/A

### Security
- N/A

## [0.1.0] - 2024-01-XX

### Added
- Initial release of the Once language compiler
- Complete compiler pipeline from source to object code
- All core language features implemented
- Comprehensive documentation and testing
- GitHub repository with full CI/CD setup

[Unreleased]: https://github.com/once-lang/once/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/once-lang/once/releases/tag/v0.1.0
