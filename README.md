## Project Status

**Current Phase**: Early Prototype

The Once language compiler is in an **early prototype phase** with foundational components implemented but significant work remaining before a functional compiler is achieved. The codebase contains experimental implementations of key algorithms and infrastructure, but the pipeline is not yet end-to-end functional.

### Development Status

- ✅ **Working**: Runtime actors/channels, region solver algorithm, basic lexer/parser
- 🟡 **Partial**: Type system infrastructure (HM inference scaffold present, not enforced in codegen)
- ❌ **Broken**: Code generation (Cranelift integration has API mismatch errors)
- ❌ **Missing**: Parser support for `using` statement, effect annotations, linear/affine types (`lin`/`aff`)

### What's Working

- ✅ **Runtime System**: Actor model and channel implementation (`once-runtime`, `once-actors`)
- ✅ **Region Inference Solver**: Region-based memory management algorithm (`once-rinf`)
- ✅ **Basic Lexer & Parser**: Subset of language syntax (functions, `let` bindings, expressions)
- ✅ **Design Specifications**: Comprehensive language design documents (architecture, semantics)

### Known Issues (Major Blockers)

1. **Codegen Stubbed**: Cranelift integration returns placeholder ELF bytes - needs real implementation
2. **Parser Incomplete**: Specification example programs are rejected; advanced syntax not tokenized/parsed
3. **No End-to-End Pipeline**: Source code cannot be compiled to executable output; integration gaps between stages

### Current Limitations

- ✅ **Build Status**: Project compiles successfully (`cargo build`)
- ⚠️ **Codegen**: Stubbed, returns ELF magic bytes placeholder
- ⚠️ **Type Checking**: Type inference structures exist but are not connected to code generation
- ⚠️ **Language Server**: LSP support planned but not yet implemented
- ⚠️ **Test Suite**: Compiles but many tests fail at runtime due to incomplete parser

### What's Missing

- `using` statement for resource management
- Effect annotation syntax and checking
- Linear (`lin`) and affine (`aff`) type qualifiers
- Full HIR → MIR lowering
- Object file emission and linking
- Standard library
- Test suite execution (infrastructure present but broken)

### Next Steps

1. Fix Cranelift compilation errors in `once-cranelift` module
2. Extend parser to handle `using`, effects, and linear/affine type annotations
3. Implement HIR-to-MIR lowering pass
4. Wire working runtime into generated code
5. Establish CI with build status reporting

For a detailed roadmap with milestones and timelines, see [plan.md](./plan.md).
