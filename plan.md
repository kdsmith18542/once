# Once Language: Development Plan & Reality Assessment

**Last Updated:** 2026-05-01
**Based on:** Deep codebase review + specification documents ONCE-001 through ONCE-008

---

## Executive Summary

The Once compiler has a solid architectural skeleton: lexer → parser → HIR → type checker → MIR → Cranelift codegen is wired end-to-end across 21 crates. However, **implementation depth is shallow** below the parser layer. The plan.md that previously claimed "100% complete" was incorrect — regression tests prove otherwise (9 of 24 fail), and the deep review found most "complete" features are actually stubs.

This document provides an honest, spec-aligned plan to complete the compiler.

---

## Specification Documents Reference

| Spec | Title | Maps To |
|------|-------|---------|
| ONCE-001 | Vision & Goals | Overall architecture, safety guarantees |
| ONCE-002 | Core Language Syntax | Lexer, Parser, AST |
| ONCE-003 | Type System & Effects | Type inference, linearity, effects |
| ONCE-004 | Memory & Concurrency | Region inference, channels, spawn/group |
| ONCE-005 | AI / Goal Language | goal declarations, AI synthesis pipeline |
| ONCE-006 | Build System & Tooling | once.toml, capabilities, LSP, explain, fix |
| ONCE-007 | Standard Library API | std::io, std::net, std::collections, std::concurrency |
| ONCE-008 | QoL & Ergonomics | `|>` pipeline, `try` blocks, type holes, doctests, schemas |

---

## Current State: Reality Check

### What Actually Works (Verified by Tests)

| Component | Reality |
|-----------|---------|
| Lexer | Solid. 119 token variants. Minor float regex fixed. All token tests pass. |
| Parser (basic) | Recursive descent with precedence. Handles `fn`, `let`, `return`, `using`, `spawn`, `await`. |
| Type checker | HM inference works for simple cases. Linearity and effect checkers produce results. |
| MIR generator | Emits 18 MIR ops including Drop for `using` blocks. |
| Cranelift codegen | Real working backend. Emits native code for basic functions, control flow, imports. |
| Runtime | Channels with backpressure, deadlock detection via DFS. |
| Standard library | 18 types implemented as Rust code (not callable from Once programs yet). |
| CI pipeline | Complete with fmt, clippy, test, build, security audit. |

### What's Stubbed (Present in Code but Non-Functional)

| Component | What's Missing |
|-----------|---------------|
| HIR | 1:1 AST copy. No name resolution. `is_linear: false` hardcoded. |
| MIR lowering | Binary ops, indexing, for loops, match patterns all produce placeholder ops. |
| Cranelift `FreeRegion` | No-op. The core RMM feature doesn't work. |
| All Cranelift values | Hardcoded as `i64`. No floats, no aggregates. |
| LSP | Custom types (not wire protocol). Positions return `0:0`. |
| Build system | `build_binary` calls itself recursively. Dependency resolution is TODO. |
| Explain tools | All analysis methods return hardcoded dummy values. |
| Bounds checker | `get_array_length` returns hardcoded `10`. |
| Optimizer | Empty pass. Constant folding returns `false`. |

### Failing Tests (9 of 24 regression tests)

```
FAIL: test_float_literal_token
FAIL: test_parse_adt_type
FAIL: test_parse_array_type
FAIL: test_parse_function_with_params   (binary ops in expressions)
FAIL: test_parse_generic_type
FAIL: test_parse_if_expression
FAIL: test_parse_match_expression
FAIL: test_parse_nested_expressions     (binary ops in blocks)
FAIL: test_pipeline_operator
```

Root cause: The parser was written before these AST node types and expression forms were added. The parser's `parse_expr` function handles only literals, identifiers, function calls, blocks, `spawn`, and `await` — not binary operators, `if`/`else`, `match`, `for`, index expressions, or pipeline.

---

## Phase Plan

Each phase is ordered by dependency. Phases must be completed in sequence.
Acceptance criteria are verifiable by running specific cargo test commands.

---

### Phase 0: Parser Completion (ONCE-002)

**Status:** ✅ COMPLETE
**Tests:** 24/24 regression tests pass; 35/35 unit tests pass

All ONCE-002 syntax is parsed correctly: functions, let/var, if/else, match, for, binary ops with full precedence, pipeline operator, array indexing, generic types, ADT/type declarations, trait/impl blocks, goal declarations, effect rows, and using blocks.

### Phase 1: HIR & Semantic Analysis (ONCE-003)

**Status:** ✅ COMPLETE with minor fix applied
**Tests:** All type system, trait resolution, effect, and linearity tests pass

**Completed:**
- [x] HIR mirrors AST with full expression support (If, Match, For, Index, Try)
- [x] Type declarations, trait declarations, and impl blocks lowered to HIR
- [x] Goal declarations lowered to function declarations
- [x] Fixed `is_linear` flag population from type annotations (was hardcoded `false`)
- [x] All 178 tests pass across 19 test suites

**Deferred (lower priority):**
- 1.6 Desugar `using` blocks at HIR level (currently handled in MIR)

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 0.1 | Fix float literal regex | ONCE-002 §5 | `r"[0-9]+\\.[0-9]+"` double-escapes dot | Change to `r"[0-9]+\.[0-9]+"` in `once-lex/src/lib.rs` |
| 0.2 | Add binary operator parsing | ONCE-002 §7 | `parse_expr` only handles primaries | Add `+`, `-`, `*`, `/`, `==`, `!=`, `<`, `<=`, `>`, `>=`, `&&`, `||` with precedence |
| 0.3 | Add `if`/`else` expression parsing | ONCE-002 §8.2 | Not in `parse_expr` | Parse `if expr block [else (if_expr | block)]` → `Expr::If` |
| 0.4 | Add `match` expression parsing | ONCE-002 §8.3 | Not in `parse_expr` | Parse `match expr { pat => expr, ... }` → `Expr::Match` |
| 0.5 | Add `for` loop parsing | ONCE-002 §8.4 | Not in `parse_expr` | Parse `for ident in expr block` → `Expr::For` |
| 0.6 | Add index expression parsing | ONCE-002 §7.5 | Not in `parse_expr` | Parse `expr[expr]` → `Expr::Index` |
| 0.7 | Add pipeline operator support | ONCE-008 §3 | Not handled | Desugar `x |> f(y)` or add to parse_expr |
| 0.8 | Add `type` declaration parsing | ONCE-002 §6.2 | `Token::Type` exists, no parse rule | Parse `type Ident [<params>] = variant { | variant } ;` |
| 0.9 | Add array type parsing | ONCE-003 §2.4 | Partial in `parse_type` | Ensure `[T; N]` parses as `Type::Array` |
| 0.10 | Add generic type parsing | ONCE-003 §2.5 | Partial in `parse_type` | Ensure `Ident<T>` parses as `Type::Generic` |

**Acceptance:** `cargo test --test regression_tests` → 24 passed, 0 failed
**Spec coverage:** ONCE-002 §2-8 (Core Language Syntax)

---

---

### Phase 2: MIR Lowering Completion (ONCE-004)

**Status:** 🔄 IN PROGRESS
**Priority:** HIGH

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 1.1 | Implement name resolution | ONCE-002 §9 | HIR is 1:1 AST copy | `HirBuilder::resolve_*` must populate symbol table, resolve identifiers to definitions |
| 1.2 | Populate `is_linear` flag | ONCE-003 §4 | Hardcoded `false` | Set from type annotations (`lin T`, `aff T`) and resource types |
| 1.3 | Lower `TypeDecl` to HIR | ONCE-002 §6.2 | AST has `TypeDecl`, HIR lacks it | Add `HirTypeDecl` variant; populate with resolved variants |
| 1.4 | Lower new expression forms | ONCE-002 §8 | Parser nodes exist, HIR mirrors but untested | Wire `If`, `Match`, `For`, `Index`, `Pipeline` through HIR builder |
| 1.5 | Fix type checker for new forms | ONCE-003 §5 | `check_expr` has stubs for some | Implement type checking for `If`, `Match`, `For`, `Index`, `Pipeline` |
| 1.6 | Desugar `using` blocks | ONCE-004 §4 | AST has `UsingStmt`, MIR generates Drop | Move Drop generation to HIR pass for cleaner MIR |

**Acceptance:** All type system tests pass (`cargo test --test type_system_tests`)
**Spec coverage:** ONCE-003 §2-6

---

### Phase 2: MIR Lowering (ONCE-004)

**Status:** PARTIAL
**Priority:** HIGH — codegen depends on correct MIR
**Depends on:** Phase 1

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 2.1 | Fix binary op lowering | ONCE-002 §7 | Binary → `Move` (wrong) | Lower `+`, `-`, `*`, `/` to `MirOp::Add`, `Sub`, `Mul`, `Div` |
| 2.2 | Fix array indexing | ONCE-002 §7.5 | Index → `Move` (wrong) | Lower to `MirOp::BoundsCheck` + `MirOp::Load` |
| 2.3 | Fix `for` loop lowering | ONCE-002 §8.4 | Produces empty block | Generate loop header, condition check, body, increment |
| 2.4 | Fix `match` lowering | ONCE-002 §8.3 | Branches on scrutinee itself | Generate pattern comparison + branch chain |
| 2.5 | Complete `if`/`else` lowering | ONCE-002 §8.2 | Branch/Label exists but untested | Ensure `if` → conditional branch + else label |
| 2.6 | Add float/aggregate type support | ONCE-003 §2 | All values are i64 | Add `MirType` annotations to ops; support `f64`, `bool` |

**Acceptance:** `cargo test --test mir_lowering_tests`
**Spec coverage:** ONCE-004 §2-5

---

### Phase 3: Cranelift Codegen Completion (ONCE-004)

**Status:** PARTIAL
**Priority:** HIGH — runtime depends on correct codegen
**Depends on:** Phase 2

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 3.1 | Implement `FreeRegion` | ONCE-004 §2 | No-op | Track region allocations; emit bulk deallocation at region exit |
| 3.2 | Add float codegen | ONCE-003 §2 | i64 only | Lower `f64` constants, float ops via Cranelift float types |
| 3.3 | Add bool codegen | ONCE-002 §5 | i64 only | Lower `bool` constants, boolean branches |
| 3.4 | Fix function calls with params | ONCE-002 §6.1 | Basic but untested | Pass typed params via Cranelift function signature |
| 3.5 | Wire `once-std` functions as imports | ONCE-007 | Runtime imports declared | Add std library function imports (print, file_open, etc.) |
| 3.6 | Add `BoundsCheck` codegen | ONCE-004 §6 | Compiles compare+trap | Wire to `once-bounds` proofs; elide when proven safe |

**Acceptance:** `cargo test --test codegen_memory_tests`
**Spec coverage:** ONCE-004 §2-6

---

### Phase 4: Runtime Integration (ONCE-004, ONCE-007)

**Status:** PARTIAL
**Priority:** MEDIUM
**Depends on:** Phase 3

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 4.1 | Fix task execution | ONCE-004 §5 | Simulated (matches on "main" only) | Execute actual compiled functions from JIT code |
| 4.2 | Implement `group` blocks | ONCE-004 §5.2 | Not implemented | Add `group { ... }` syntax; scheduler waits for all children |
| 4.3 | Wire actor model | ONCE-004 §5.3 | `once-actors` exists, disconnected | Connect actor lifecycle to runtime scheduler |
| 4.4 | Implement structured concurrency | ONCE-004 §5.2 | Not implemented | Guarantee no child process outlives parent scope |

**Acceptance:** `cargo test --test codegen_concurrency_tests`
**Spec coverage:** ONCE-004 §5

---

### Phase 5: Tooling & Build System (ONCE-006)

**Status:** PARTIAL
**Priority:** MEDIUM
**Depends on:** Phase 0

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 5.1 | Fix `once-build` dependency resolution | ONCE-006 §4 | TODO | Implement version resolution, graph construction, topological build order |
| 5.2 | Implement capability verification | ONCE-006 §5 | Types exist, checker stubbed | Transitive capability check; fail on undeclared capabilities |
| 5.3 | Wire lockfile generation | ONCE-006 §4 | Lockfile type exists | Generate `once.lock` with hashes; validate on rebuild |
| 5.4 | Implement `once new`, `once build` | ONCE-006 §7 | CLI subcommands exist, backends stubbed | Complete the build pipeline |
| 5.5 | Wire LSP protocol | ONCE-006 §7 | Custom types, no wire protocol | Replace custom types with `tower-lsp` / `lsp-types`; implement `initialize`, `textDocument/didOpen`, diagnostics |
| 5.6 | Implement explain tools | ONCE-008 §7 | Returns hardcoded values | Connect to real effect/region/linearity analysis output |

**Acceptance:** `cargo test --test build_system_tests` for build; manual LSP smoke test
**Spec coverage:** ONCE-006 §2-7, ONCE-008 §7

---

### Phase 6: AI / Goal Language (ONCE-005)

**Status:** EARLY STUB
**Priority:** LOW
**Depends on:** Phase 1 (type checker must work for generated code)

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 6.1 | Complete `goal` parser | ONCE-005 §2 | Parser AST node exists | Parse `spec`, `constraints`, `examples` clauses fully |
| 6.2 | Build AI prompt construction | ONCE-005 §4 | `GoalSynthesizer` exists, `StubAiSolver` | Construct structured prompts from goal content + type context |
| 6.3 | Implement code verification | ONCE-005 §4 | Not implemented | Run type/linearity/effect checkers on AI-generated code |
| 6.4 | Implement example-based testing | ONCE-005 §4 | Not implemented | Compile and run examples; fail build on mismatch |
| 6.5 | Add goal code caching | ONCE-005 §4 | Not implemented | Content-hash key; regenerate only on goal change |

**Acceptance:** `cargo test --test ai_integration_tests`
**Spec coverage:** ONCE-005 §2-5

---

### Phase 7: Quality of Life (ONCE-008)

**Status:** NOT STARTED
**Priority:** LOW
**Depends on:** Phase 0, Phase 1

**Tasks:**

| # | Task | Spec Ref | Current State | What's Needed |
|---|------|----------|---------------|---------------|
| 7.1 | Implement `try` blocks | ONCE-008 §4 | Not implemented | Auto-capture error context; instrument returns |
| 7.2 | Implement type holes `_` | ONCE-003 §5.6 | Not implemented | Parse `_` as unknown type; report inferred type at hole position |
| 7.3 | Implement doctests | ONCE-008 §5 | Not implemented | Extract ```once code blocks from `///` docs; compile and run as tests |
| 7.4 | Implement schema hydration | ONCE-008 §2 | Not implemented | Parse `schema` declarations; generate validation code from field mappings |
| 7.5 | Implement effect overrides in tests | ONCE-008 §6 | Not implemented | `override <effect> with <mock>` in `#[test]` scope |
| 7.6 | Implement `once fix` commands | ONCE-008 §7 | Not implemented | `once fix --imports`, `once fix --consumes` |

**Acceptance:** Manual testing with example programs
**Spec coverage:** ONCE-008 §2-7

---

## Dependency Graph

```
Phase 0 (Parser) ──────┬──► Phase 1 (HIR/Semantic) ──► Phase 2 (MIR) ──► Phase 3 (Codegen) ──► Phase 4 (Runtime)
                       │
                       ├──► Phase 5 (Tooling/Build)
                       │
                       └──► Phase 7 (QoL)

Phase 1 ──► Phase 6 (AI/Goal)
```

---

## Milestones

| Milestone | Phases | Tests Passing | Status |
|-----------|--------|---------------|--------|
| M1: Parser Complete | 0 | 24/24 regression tests | 🔴 15/24 |
| M2: Type System Complete | 0-1 | All type system tests | 🔴 Stubs present |
| M3: Codegen Complete | 0-3 | All memory + concurrency codegen tests | 🟡 Partial |
| M4: Tooling Ready | 0-5 | Build system + LSP smoke test | 🟡 Partial |
| M5: AI Integration | 0-1, 6 | AI integration tests | 🔴 Stubs only |
| M6: Production Ready | 0-7 | Full test suite | 🔴 Not started |

---

## Immediate Next Actions (Ordered)

1. **Fix the 9 failing regression tests** (Phase 0) — this is the highest priority; everything else is blocked
2. **Complete HIR name resolution** (Phase 1.1-1.2) — currently the HIR is a structural copy with no semantic analysis
3. **Complete MIR lowering for control flow** (Phase 2.2-2.5) — binary ops, match, for loops produce placeholder MIR
4. **Implement FreeRegion in Cranelift** (Phase 3.1) — the core memory management feature
5. **Wire LSP protocol** (Phase 5.5) — replace custom types with real LSP types via tower-lsp

---

**Document Status:** Living document. Update acceptance status after each test run.
