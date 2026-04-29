## Project Status

**Current Phase**: Stabilization Phase (Phase 0)

The Once language compiler has successfully stabilized its foundation with working code generation, completing Phase 0 of the recovery plan. The compiler can now build end-to-end for basic examples.

### Development Status

- ✅ **Working**: Runtime actors/channels, region solver algorithm, basic lexer/parser, **real Cranelift code generation**
- ✅ **Partial**: Type system infrastructure (HM inference scaffold present)
- ✅ **Parser Support**: Basic syntax including functions, let bindings, expressions
- ✅ **Build System**: Project compiles successfully and produces working object files

### What's Working

- ✅ **Runtime System**: Actor model and channel implementation (`once-runtime`, `once-actors`)
- ✅ **Region Inference Solver**: Region-based memory management algorithm (`once-rinf`)
- ✅ **Basic Lexer & Parser**: Subset of language syntax (functions, `let` bindings, expressions)
- ✅ **Design Specifications**: Comprehensive language design documents (architecture, semantics)
- ✅ **Real Cranelift Code Generation**: End-to-end compilation of basic examples to object files (`once-codegen`)

### Known Issues

1. **Parser Incomplete**: Specification example programs using advanced features are rejected; advanced syntax not tokenized/parsed (`using`, effect annotations, `lin`/`aff` types)
2. **No Full Standard Library**: Limited standard library functionality
3. **Missing Features**: Advanced type system features (effect constraints, linearity enforcement) not yet connected to codegen

### Current Limitations

- ✅ **Build Status**: Project compiles successfully (`cargo build`)
- ✅ **Codegen**: Working, produces valid object files for supported syntax
- ⚠️ **Type Checking**: Type inference structures exist but advanced features (effects, linearity) not fully enforced
- ⚠️ **Test Suite**: Compiles but many tests fail at runtime due to incomplete parser
- ⚠️ **Language Server**: LSP support planned but not yet implemented

### Next Steps

1. Extend parser to handle `using`, effects, and linear/affine type annotations (Phase 1.1-1.3 from plan.md)
2. Implement HIR-to-MIR lowering pass for new syntax features
3. Wire working runtime into generated code
4. Implement effect and linearity checking in type system
5. Establish CI with build status reporting

For a detailed roadmap with milestones and timelines, see [plan.md](./plan.md).
