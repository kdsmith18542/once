# Once Language: Implementation Reality Check & Recovery Plan

**Status:** Early Prototype (not MVP)  
**Date:** 2026-04-29  
**Based on:** Comprehensive spec vs implementation review

---

## Executive Summary

The Once language project has **ambitious specifications** (ONCE-001 through ONCE-008) but the **implementation is significantly behind** the documented claims. The compiler **does not build** due to Cranelift API errors, and core language features (`using`, effect annotations, linear types) are **not parseable**. This document outlines the gap analysis and provides a realistic recovery plan.

---

## Current State Assessment (Updated: 2026-04-29)

### Build Status: ✅ BUILD WORKS (Codegen Stubbed)

- **Compiler builds successfully** with `cargo build`
- Codegen (`once-codegen/src/real_cranelift.rs`) is stubbed - returns ELF magic bytes placeholder
- No real Cranelift integration yet - needs implementation
- Test suite does not compile due to API mismatches (in progress)

### Test Suite Status: ✅ Compiles (Runtime failures expected)

- Tests now compile successfully
- Fixed API calls: `Lexer::new()`, `OnceParser::parse()`
- Fixed enum variants: `Item` not `AstItem`
- Test results: ~15-27 tests pass (lexer, basic parser)
- Test failures: Due to incomplete parser (expected in Phase 1)
- Runtime failures for: complex expressions, match, for-loops, etc.

### Parser Coverage: ⚠️ 40% Complete

**What parses:**
- Basic function declarations `fn name() -> Type { }`
- Variable declarations `let x = expr;`
- Primitive literals and operators
- `spawn` and `await` as expression-level forms (but without effect tracking)

**What does NOT parse (all spec'd features):**
| Feature | Spec Doc | Example | Status |
|---------|----------|---------|--------|
| `using` statement | ONCE-004 §3.2 | `using f = File.open(path) { }` | ❌ Token exists, no grammar rule |
| Effect annotations | ONCE-003 §5.1 | `fn f() -> T !io { }` | ❌ `!` after return type unsupported |
| Linear type qualifiers | ONCE-003 §4.1 | `param: lin File` | ❌ `lin`/`aff` keywords not in lexer |
| Affine type qualifiers | ONCE-003 §4.1 | `param: aff T` | ❌ Same |
| `goal` declarations | ONCE-005 §2 | `goal factorial(n: Int) -> Int { ... }` | ❌ Entirely missing |

**Impact:** Example files in `examples/` directory would all fail to parse:
- `examples/linear_resources.onc` (uses `using`)
- `examples/concurrency.onc` (uses `!spawn`)
- `examples/async_example.onc` (uses `!spawn`)

### Type System: ⚠️ Half-Baked

| Component | Status | Notes |
|-----------|--------|-------|
| Hindley-Milner inference | 🟡 Stubbed | Data structures exist (`once-ty`), but constraint solving incomplete |
| Linear/Affine types | ❌ Not enforced | HIR types present but never constructed; checker has stub logic |
| Effect constraints | ❌ Dead code | `Constraint::Effect` never generated; solver stub |
| Region inference | 🟡 Partial | `once-rinf` solves region DAG but output not used by codegen |
| Type unification | 🟡 Basic | Simple equality unification works; row polymorphism unused |

**Critical gap:** No constraint generation connects syntax to semantics. The type checker does not emit `Linear`/`Affine`/`Effect` constraints from AST/HIR, so the solver has nothing to enforce.

### Effect System: 🟡 Mismatched Design

**Spec-defined effect kinds (ONCE-003 §5.3):**
```
io, net, spawn, time, ffi, nondet
```

**Implementation effect labels (`once-effects/src/lib.rs:37-50`):**
```rust
Async, Channel, Spawn, Error, Resource, Custom(String)
```

**Discrepancy:**
- `io` (spec) ↔ `Resource`? (unclear mapping)
- `net`, `time`, `ffi`, `nondet` → **no equivalents**
- `Async`, `Channel`, `Error` → **extra labels not in spec**

**Effect checker behavior:**
- Infers effects from operations like `spawn`, `await`, `send`, `recv`
- Does **not** validate against function signature annotations (because those don't exist)
- Reports aggregate effect row but plays no role in type checking or compilation failure

**Result:** Effect system is a standalone analysis pass with no compile-time enforcement mechanism.

### Linearity & Resource Management: ❌ Non-Functional

**Intended flow (spec ONCE-004 §3):**
```
using f = File.open(path) { ... }
   ↓ desugars to
{
  let f = File.open(path);
  ... body ...
  f.consume();  // auto-inserted at block exit
}
```

**Actual implementation:**
- `Token::Using` exists in lexer
- Parser would reject `using` as unexpected token
- No `Stmt::Using` in AST
- No HIR desugaring pass
- Linearity checker (`once-linear`) tracks usage counts but only for variables marked `is_linear = true` — which never happens because HIR builder hardcodes `false`
- **Zero linearity errors are ever produced** in practice

### Concurrency Runtime: ✅ Implemented

** implemented in:**
- `once-runtime` (842 lines): `Channel<T>` with backpressure, deadlock detection via DFS cycle detection, task handles
- `once-actors` (611 lines): actor lifecycle, mailboxes, supervision, message passing

**Status:** Runtime code appears complete and independent. However:
- Not integrated with region-based memory management
- No compile-time guarantees about "mutable XOR shared" (type system doesn't enforce)
- Effect annotations for `spawn` not checked, so no capability tracking

### Standard Library: ⚠️ Wrapper-Focused

**`once-std` (1917 lines) provides:**
- `Resource` trait with `consume()` method
- `FileHandle`, linear I/O types wrapping `std::fs::File`
- `HashMap`/`HashSet` wrappers
- `TcpStream`/`TcpListener` wrappers

**Gaps:**
- Not a true standalone stdlib; depends heavily on Rust's `std`
- No integration with effect system (file operations don't emit `io` effect)
- No linear type enforcement at compile time
- Designed as runtime support, not compile-time-checked API

### Code Generation: ❌ Broken

**Backend:** Cranelift-based (`once-codegen/src/real_cranelift.rs`)
**Errors:**
1. `Module<ObjectBuilder>` — trait used as type (should be `cranelift_module::Module<ObjectBuilder>` concrete type from module)
2. `call()` returns `Inst` but variable typed as `Value` — API changed in Cranelift 0.105
3. Unused imports and similar style issues

**Status:** No working codegen means no object files, no linking, no executable output. The entire compiler pipeline halts at this stage.

### Build System: ❌ Not Executable

`once-build` contains data structures for:
- Build targets and dependencies
- Cache entries
- FFI security config

But **no actual build orchestration code** observed (no `build()` method that walks DAG, invokes compiler, links objects). The CLI's `BuildProject` command is defined but implementation not examined — likely stubbed.

### Test Suite: ❌ Non-Compilable

**Evidence:**
- `tests/unit_tests.rs` calls `tokenize()` and `parse()` — **these functions don't exist** (grep found no definitions)
- Uses outdated enum variants: `AstItem::FnDecl` (actual is `Item::FnDecl`)
- References `HirType::Unit` (actual is `once_parse::Type::Unit`)
- Would fail to compile even if main build were fixed

**Conclusion:** Test suite is placeholder/out of sync with current codebase. No validated test coverage exists.

### Documentation vs Reality: 🚨 Overstated

**Claims in `docs/WEEK2_PROGRESS.md`:**
> "Compiler Pipeline: Complete source-to-object compilation" — **False** (codegen broken)
> "All core language features implemented" — **False** (`using`, effects, `lin`/`aff` missing)
> "Zero compilation errors" — **False** (4 errors in codegen)
> "Working compiler pipeline" — **False** (parser rejects spec examples)
> "MVP Complete ✅" — **False** (definition of MVP should include working compiler + core syntax)

**The project documentation describes a vision that is not yet realized.** The specs are well-written design documents, but implementation is in early stages.

---

## Root Causes

1. **Premature celebration:** Progress reports written before verifying compiler actually builds and parses its own examples
2. **API drift:** Cranelift dependency updated without adapting wrapper code (lack of maintenance)
3. **Scope creep / incomplete sprints:** Many crates created (24) but frontend grammar work left unfinished
4. **Testing gap:** Test suite not kept in sync with API changes; no CI catching basic compile failures
5. **Spec-implementation misalignment:** Effect label sets diverged; linear syntax reserved but not tokenized

---

## Recovery Plan (Phased)

### Phase 0: Stabilize the Foundation (Weeks 1-2)

**Goal:** Get the compiler to a state where it builds and runs basic "Hello World" without syntax errors.

#### 0.1 Fix Cranelift Integration (P0 - Critical)
- **Owner:** Backend engineer
- **Tasks:**
  - Update `real_cranelift.rs` to use correct Cranelift 0.105 API
  - Replace `Module<ObjectBuilder>` trait usage with concrete `cranelift_module::Module`
  - Fix `call()` return type mismatch (`Inst` → extract `Value` via `func_ref` or adjust return type tracking)
  - Verify object file emission works end-to-end
- **Acceptance:** `cargo build --release` succeeds with 0 errors; can emit `.o` file for trivial function
- **Validation:** Run existing unit tests that exercise codegen (if any); otherwise create minimal test

#### 0.2 Make Test Suite Compilable (P0 - Critical)
- **Owner:** QA / Tooling
- **Tasks:**
  - Replace undefined `tokenize()` calls with `Lexer::new(...).collect()`
  - Update enum variants to match current AST (`Item`, `Type`, etc.)
  - Fix or remove broken tests; ensure `cargo test --lib` compiles and runs
- **Acceptance:** All unit tests compile; at least 50% pass (even if functionality incomplete)
- **Validation:** CI green on compile + basic test run

#### 0.3 Establish Baseline Compiler (P1 - High)
- **Owner:** Compiler lead
- ** Tasks:**
  - Document currently supported syntax subset in README
  - Create "known failures" list for features not yet implemented
  - Pin Cranelift version or add compatibility layer to prevent future drift
- **Acceptance:** Clear, honest documentation of what works today

---

### Phase 1: Complete Frontend Syntax (Weeks 3-6)

**Goal:** Parser accepts all syntax defined in ONCE-002, ONCE-003, ONCE-004 core specs.

#### 1.1 Add `lin` / `aff` Keywords (P1 - High) ✅ DONE
- **Files:** `once-lex/src/lib.rs`, `once-parse/src/lib.rs`, `once-hir/src/lib.rs`, `once-ty/src/lib.rs`
- **Completed:**
  1. ✅ Added `Lin` and `Aff` variants to `Token` enum in lexer
  2. ✅ Extended AST `Type` enum with `Linear`, `Affine`, `Array`, `Generic`, `Tuple`, `Function` variants
  3. ✅ Updated `parse_type` to handle `Token::Lin`/`Token::Aff` and new type forms
  4. ✅ Extended HIR `HirType` with new variants
  5. ✅ Updated `once-ty` and `once-codegen` to handle new types
- **Acceptance:** `lin File` and `aff Vec<T>` now parse into AST/HIR correctly
- **Validation:** Parser tests for linear/affine types pass

#### 1.2 Implement `using` Statement (P1 - High) ✅ DONE
- **Files:** `once-parse/src/lib.rs`, `once-hir/src/lib.rs`, `once-ty/src/lib.rs`, `once-mir/src/lib.rs`, `once-rinf/src/lib.rs`, `once-effects/src/lib.rs`, `once-linear/src/lib.rs`
- **Completed:**
  1. ✅ Added `Stmt::Using` variant to AST with `UsingStmt` struct
  2. ✅ Parser rule: recognize `Token::Using` followed by identifier, `=`, expression, `{` block `}`
  3. ✅ HIR: Added `HirStmt::Using` and `HirUsingStmt` to handle desugaring
  4. ✅ Type checker (`once-ty`) handles using statements with linear type wrapping
  5. ✅ MIR generation (`once-mir`) generates Move/Drop operations for using blocks
  6. ✅ Region inference (`once-rinf`) processes using statement constraints
  7. ✅ Effects checking (`once-effects`) checks using statement effects
  8. ✅ Linearity checking (`once-linear`) marks using variables as Linear
- **Acceptance:** `using x = expr { body }` parses and generates correct IR
- **Validation:** Parser accepts using syntax, HIR builder processes it, MIR generates drop
- **Files:** `once-parse/src/lib.rs`, `once-hir/src/lib.rs`, maybe `once-linear` for desugaring
- **Tasks:**
  1. Add `Stmt::Using` variant to AST with fields: `var_name: String`, `init_expr: Expr`, `body: Block`
  2. Parser rule: recognize `Token::Using` followed by identifier, `=`, expression, `{` block `}`
  3. HIR: Add `HirStmt::Using` (or desugar immediately in `HirBuilder`)
  4. Desugaring strategy: Transform `using x = init { body }` into:
     ```rust
     {
       let x = init;
       body
       consume(x);  // insert at all exit points (normal return + early returns with ?)
     }
     ```
  5. Handle `?` operator: if body contains `?`, ensure `consume()` runs in both success and error paths (transform to `defer`-like pattern)
  6. MIR: Ensure `consume()` call becomes `Drop` operation on linear value
- **Acceptance:** `using` examples from `examples/linear_resources.onc` parse and typecheck
- **Validation:** Add integration test that verifies `consume()` is inserted and called

#### 1.3 Implement Effect Annotations `![]` (P1 - High)
- **Files:** `once-parse/src/lib.rs`, `once-hir/src/lib.rs`, `once-ty/src/lib.rs`, `once-effects/src/lib.rs`
- **Tasks:**
  1. Extend lexer if needed: `Token::Bang` already exists; need identifier parsing for effect names after `!`
  2. Parser: After return type in `parse_fn_decl`, check for `Token::Bang`. If present, parse:
     - `!` → single bare effect identifier (e.g., `!io`)
     - `![` effect-list `]` → row of effects (e.g., `![io, spawn]`)
  3. AST `FnDecl`: add field `effects: Option<Vec<String>>` (or structured `EffectRow`)
  4. HIR `HirFnDecl`: add `effects: Option<HirEffectRow>` (store parsed effect names)
  5. HIR builder: copy AST effects to HIR
  6. Type system: Embed effect row into function type. Modify `Type::Function` to include `effects: EffectRow`
  7. Constraint generation: When checking function body, collect inferred effects (from `once-effects` checker) and unify with declared effects
  8. Effect checker integration: Connect `once-effects::EffectChecker` output to type constraint solver; generate `Constraint::Effect` that declared ⊇ inferred
  9. MIR: Propagate effect row for linkage metadata
- **Acceptance:** Functions with `!io`, `!spawn`, `![io, net]` parse, typecheck, and mismatches produce errors
- **Validation:** Add positive and negative tests: correct effects OK; missing effect declaration fails

#### 1.4 Align Effect Label Set with Spec (P2 - Medium)
- **Decision point:** Adhere to ONCE-003 labels (`io`, `net`, `spawn`, `time`, `ffi`, `nondet`) or revise spec
- **If adhering:** 
  1. Rename `EffectLabel` variants to match spec
  2. Map parser effect identifiers to these variants
  3. Update effect inference in `once-effects` to recognize `spawn`, `await` as `spawn` effect (not `Async`), channel ops as `io` (or keep `Channel`? needs decision)
  4. Remove `Async`, `Channel`, `Error`, `Resource` as separate labels; fold into `io`/`spawn`/`ffi` as appropriate
- **If revising spec:** Document deviation and update ONCE-003 accordingly
- **Acceptance:** Consistent label set across spec and implementation; `once-effects` produces rows using canonical labels
- **Validation:** Test effect inference matches expected sets

#### 1.5 Enforce Linear/Affine Usage (P1 - High)
- **Files:** `once-ty/src/lib.rs`, `once-linear/src/lib.rs`
- **Tasks:**
  1. Type checker: When processing `let` binding or param with linear/affine type, emit `Constraint::Linear(ty)` or `Constraint::Affine(ty)` with reference to that variable
  2. Constraint solver: Actually implement checks (not stubs):
     - Track each linear/affine variable's usage count in `LinearityEnv`
     - For linear: error if `use_count != 1` at end of scope
     - For affine: error if `use_count > 1`
     - Ensure `consume()` call or return counts as single use
  3. Hook into MIR generation: ensure `Drop` emitted at appropriate points
- **Acceptance:** Attempting to use a linear value twice produces a compile error; linear value not consumed produces error
- **Validation:** Negative tests: double-use of `lin File` fails; positive test: `using` or explicit `consume()` succeeds

#### 1.6 Full Parser Integration (P2 - Medium)
- Ensure all syntax elements interact correctly:
  - `using` inside functions with effect annotations
  - Linear parameters in functions with effects
  - Nested `using` blocks, early returns, `?` operator
- Acceptance: Parse all examples in `docs/` and `examples/` without errors
- Validation: Create `parser_integration_tests.rs` with all constructs

---

### Phase 2: Type System & Semantic Analysis (Weeks 7-10)

#### 2.1 Complete Hindley-Milner Inference (P1 - High)
- **Files:** `once-ty/src/lib.rs`
- **Tasks:**
  - Ensure all expression forms generate correct type constraints
  - Verify generic function type specialization works
  - Test type generalization for let-bindings
- **Acceptance:** Polymorphic functions like `identity<T>(x: T) -> T` infer and instantiate correctly
- **Validation:** Existing type inference tests (if any) pass; add new generic tests

#### 2.2 Wire Region Inference to Codegen (P1 - High)
- **Files:** `once-rinf`, `once-mir`, `once-codegen`
- **Tasks:**
  - Ensure `RegionDag` from region checker flows into MIR generator
  - MIR should emit `Allocate(region)` and `FreeRegion(region)` ops
  - Codegen translates region ops to `alloc`/`free` calls or stack slot management
- **Acceptance:** Generated code includes region-based memory management calls
- **Validation:** Inspect assembly/object for allocation/free patterns; run with memory checker (Valgrind/ASAN)

#### 2.3 Implement Effect Constraint Solving (P1 - High)
- **Files:** `once-ty/src/lib.rs`, `once-effects/src/lib.rs`
- **Tasks:**
  - Modify `Type::Function` to store `effects: EffectRow`
  - During type checking, for each function:
    1. Collect declared effects from signature (or `∅` if none)
    2. Run effect inference on body to get `inferred_effects`
    3. Generate constraint: `declared_effects ⊇ inferred_effects` (subsumption)
  - Solve effect constraints alongside type constraints
- **Acceptance:** Function body effects must be subset of signature; missing `!io` on I/O function fails compilation
- **Validation:** Negative test: `fn f() { print("") }` fails (missing `!io`); positive: `fn f() !io { print("") }` passes

#### 2.4 Integrate Linearity with Type System (P1 - High)
- **Files:** `once-ty`, `once-linear`
- **Tasks:**
  - Merge linearity checking into main type checking pass (or keep separate but coordinated)
  - Ensure `Constraint::Linear/Affine` constraints are generated for all linear/affine bindings
  - Solve constraints before MIR generation
  - Propagate linearity information to MIR (so `Drop` knows to call `consume()`)
- **Acceptance:** Compiler rejects code that misuses linear resources
- **Validation:** Test cases from spec (using file without consume, using linear value twice)

#### 2.5 Row-Polymorphic Effects (P2 - Medium)
- Implement effect row polymorphism if spec'd (ONCE-003 mentions it)
- Allow functions to quantify over effect rows
- **Acceptance:** Higher-rank effect polymorphism typechecks
- **Validation:** Advanced generic effect polymorphism examples

---

### Phase 3: Codegen & Runtime Integration (Weeks 11-14)

#### 3.1 Stabilize Cranelift Backend (P0 - Critical)
- Already started in Phase 0, but may need iterative fixes
- Ensure register allocation, instruction selection work
- Test with variety of functions (arithmetic, conditionals, calls)
- **Acceptance:** `once build` produces working object files for simple programs

#### 3.2 Region-Aware Codegen (P1 - High)
- Implement region-based allocation in generated code:
  - Primary region: stack-allocated, freed on function return
  - Subregions: stack pointer adjustment
  - Escaping values: heap-allocate with box/rc or spill to parent region
- **Acceptance:** Generated code correctly manages lifetimes without GC
- **Validation:** Run under memory sanitizers; no leaks/use-after-free

#### 3.3 Runtime Linking (P1 - High)
- Linker (`once-linker`) should combine object files with runtime lib
- Ensure runtime functions (channel send/recv, actor spawn, scheduler) are available
- **Acceptance:** `once run` executes a program that can spawn actors and communicate
- **Validation:** Concurrency examples run correctly

#### 3.4 Linear Drop Glue (P1 - High)
- Generate calls to `consume()`/drop methods for linear types at region exit or explicit drop
- **Acceptance:** `File` auto-closes when `using` block exits
- **Validation:** Check file descriptor released; resource leak detectors clean

---

### Phase 4: Standard Library & Examples (Weeks 15-17)

#### 4.1 Build Real Standard Library (P2 - Medium)
- Replace `once-std` wrappers around `std` with:
  - Linear I/O types that actually call OS via syscalls (not `std::fs`)
  - Pure-Once collections with ownership semantics
  - Effect-tagged APIs: `File::read()` should require `!io` effect
- **Acceptance:** Standard library not merely re-exporting Rust std
- **Validation:** Compile and run with `--no_std` eventually? Or at least minimal dependencies

#### 4.2 Capability-Based FFI (P2 - Medium)
- Implement `once.toml` capabilities (`io`, `net`, `spawn`)
- Enforce at compile time: function requiring `!io` can only be called if crate has `io = true` capability
- **Acceptance:** Building without capability fails
- **Validation:** Capability error messages clear and actionable

#### 4.3 Example Programs (Verification)
- All examples in `examples/` should parse, typecheck, codegen, run
- Add more: HTTP server, pipeline processing, file utility
- **Acceptance:** `once run examples/concurrency.onc` executes and produces correct output
- **Validation:** Manual + automated tests for each example

---

### Phase 5: Tooling & Polish (Weeks 18-20)

#### 5.1 Language Server (P2 - Medium)
- Complete LSP features: diagnostics on parse/type errors, completions, go-to-definition
- Hook into all compiler passes to provide real-time feedback
- **Acceptance:** `once lsp` works with VS Code extension

#### 5.2 Explain & Debug Tools (P2 - Medium)
- `once explain effects`, `once explain regions`, `once explain linearity` from spec ONCE-008
- Visualize region DAG, effect rows, move/consume sites
- **Acceptance:** Helpful output that aids understanding of complex programs

#### 5.3 Build System Completion (P1 - High)
- `once-build` should actually compile projects with dependencies
- Hermetic builds: sandboxed execution, content-addressed cache
- Lockfile generation and enforcement
- **Acceptance:** Multi-crate project builds reproducibly
- **Validation:** Clean build on fresh machine (simulate)

#### 5.4 CI/CD & Quality Gates (P0 - Critical)
- CI runs on every push:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --all-targets`
  - `cargo build --release` for all examples
  - Generate coverage report
- **Acceptance:** All checks green before merge
- **Validation:** GitHub Actions passes

---

### Phase 6: AI Integration & Advanced Features (Weeks 21-24+)

#### 6.1 Goal Language (ONCE-005) (P3 - Low)
- `goal` declarations with `spec`, `constraints`, `examples` clauses
- Integration with LLM for code synthesis
- Test generation from examples
- **Prerequisite:** Full core language stable

#### 6.2 WASM Component Model (ONCE-006) (P3 - Low)
- `once-wasm` integration complete
- PCC-lite validation
- Cross-language component interop

#### 6.3 Bounds Checking & Optimization (P2 - Medium)
- `once-bounds`: compile-time array bounds proofs
- Proof erasure when verified
- Performance optimization passes

---

## Immediate Next Actions (This Week)

1. **Fix Cranelift build** — highest priority, blocking everything
2. **Get `cargo test` to run** — fix broken tests
3. **Document actual syntax support** in README (remove false claims)
4. **Add parser unit tests** for `using` and `![]` as TODOs (expected failure)
5. **Set up CI** to catch build breaks early (if not already)

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Cranelift API continues to drift | Medium | High | Pin to exact version; add compatibility layer; consider switching to LLVM if unsustainible |
| Spec-implementation gap grows | High | High | Freeze spec changes until implementation catches up; document deltas |
| Contributor burnout from technical debt | Medium | Medium | Prioritize reducible tasks; celebrate small wins; realistic roadmap |
| Linear/affine design too complex for inference | Low | High | Consider simplifying to explicit `unsafe` blocks for linear ops if inference proves undecidable |
| Effect system performance issues | Medium | Medium | Profile constraint solver; optimize data structures |

---

## Success Metrics

- **Week 2:** Compiler builds; test suite compiles; 70% of ONCE-002 syntax parses
- **Week 6:** Full ONCE-002, ONCE-003, ONCE-004 syntax parsed; all examples parse without errors
- **Week 10:** Type checker enforces linearity and effects; region inference passes all tests
- **Week 14:** Codegen produces working executables for all examples; runtime integrated
- **Week 20:** Standard library complete; build system hermetic; LSP functional; CI green
- **Week 24:** AI goal layer prototype; WASM target experimental; ready for beta users

---

## Conclusion

The Once project is **not MVP-complete** as previously claimed. It is in **early prototype stage** with significant gaps between specification and implementation. The recovery plan above provides a realistic, phased approach to bridge that gap over ~6 months with focused effort on:

1. **Stabilizing the build** (Cranelift)
2. **Completing the frontend grammar** (`using`, effects, linear types)
3. **Wiring semantic checks** (linearity, effects, regions) into the type system
4. **Delivering working codegen** and runtime integration
5. **Catching up documentation** to match reality

By following this plan, the project can achieve its vision without further overstating progress.

---

**Document Ownership:**  
Maintained by the core team. Updated monthly or after major milestone completions.
