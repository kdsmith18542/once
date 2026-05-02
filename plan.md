# Once Language: Path to 100% Completion

**Updated:** 2026-05-01 | **Tests:** 0 failures across all suites  
**Specs:** ONCE-001 through ONCE-008

---

## Current Status

| Area | Done | Remaining |
|------|------|-----------|
| Lexer + Parser | 95% | while, const, module/import parsing |
| Type System | 85% | Type holes `_`, struct type-check depth, trait solving edge cases |
| Memory & Concurrency | 80% | group enforcement, actor wiring, Mutable XOR Shared |
| Codegen (Cranelift) | 85% | Float aggregate types, typed function params |
| Runtime | 75% | JIT function loading (currently handler registry) |
| Std Library | 85% | FFI bridge to Once programs |
| Build System | 80% | Capability enforcement, once fmt, once lint |
| AI / Goal | 70% | Real LLM integration, example verification |
| QoL / Ergonomics | 50% | try lowering, type holes, doctests, schemas |

---

## Remaining Work — Ordered by Impact

### 1. Language Completeness (ONCE-002, 003)

These are what users see and touch.

| # | Task | Spec | Effort | Why |
|---|------|------|--------|-----|
| 1.1 | `while` loop parsing | ONCE-002 §8.4 | Small | Token exists, just needs parser rule + MIR lowering |
| 1.2 | Type holes `_` — report inferred type | ONCE-003 §5.6 | Small | Parse `_` as type, hook type checker to report |
| 1.3 | `const` / `continue` / `break` parsing | ONCE-002 §8 | Small | Tokenized, no grammar |
| 1.4 | Module `import` parsing | ONCE-002 §9 | Medium | Parse import statements, wire to name resolution |
| 1.5 | Struct type-check depth | ONCE-003 §2 | Medium | Resolve struct field types from declarations |

### 2. Runtime & Concurrency (ONCE-004)

These make the language usable for real programs.

| # | Task | Spec | Effort | Why |
|---|------|------|--------|-----|
| 2.1 | Enforce `group` block — parent waits for children | ONCE-004 §5.2 | Medium | TaskGroup exists, needs runtime enforcement |
| 2.2 | Wire `once-actors` to scheduler | ONCE-004 §5.3 | Medium | Actor crate exists (611 lines), disconnected |
| 2.3 | Host compiled Once functions from Cranelift output | ONCE-004 §5 | Large | Load JIT code into memory, call from runtime |

### 3. Build System & Tooling (ONCE-006)

These make the compiler a real toolchain.

| # | Task | Spec | Effort | Why |
|---|------|------|--------|-----|
| 3.1 | `once fmt` — canonical formatting | ONCE-006 §7 | Medium | Pretty-print AST |
| 3.2 | `once lint` — static analysis | ONCE-006 §7 | Medium | Dead code, unused imports, style checks |
| 3.3 | Capability security enforcement | ONCE-006 §5 | Medium | Types exist, needs transitive check at build time |
| 3.4 | LSP: wire diagnostics from type checker | ONCE-006 §7 | Medium | tower-lsp wired, diagnostics stubbed |

### 4. QoL Features (ONCE-008)

These make the developer experience great.

| # | Task | Spec | Effort | Why |
|---|------|------|--------|-----|
| 4.1 | `try` block lowering — auto-capture error context | ONCE-008 §4 | Medium | Expr::Try exists, needs MIR + codegen |
| 4.2 | Doctests — compile `///` ```once``` blocks as tests | ONCE-008 §5 | Medium | Extract code blocks, compile, run |
| 4.3 | `once fix --consumes` — insert using blocks | ONCE-008 §7 | Medium | CLI exists, needs AST manipulation |
| 4.4 | Wire `once explain` find_* methods | ONCE-006 §7 | Medium | Returns hardcoded values, needs span traversal |
| 4.5 | Schema hydration (JSON → typed struct) | ONCE-008 §2 | Large | New language feature |
| 4.6 | Test-time effect overrides | ONCE-008 §6 | Large | Mock framework for effects |

### 5. AI Integration (ONCE-005)

These fulfill the vision of AI as first-class partner.

| # | Task | Spec | Effort | Why |
|---|------|------|--------|-----|
| 5.1 | Real LLM/API integration for goal synthesis | ONCE-005 §4 | Large | Replace StubAiSolver with API call |
| 5.2 | Example-based verification — run & compare | ONCE-005 §4 | Medium | Compile generated code, run examples, fail on mismatch |

---

## Milestone Plan

### M1: Syntax Complete (~2 weeks)
**Goal:** Every ONCE-002 syntax element parses and lowers to HIR

- [ ] while loop parsing + MIR lowering
- [ ] Type holes `_` reporting
- [ ] `const` / `continue` / `break`
- [ ] Module `import` parsing
- [ ] Struct field type resolution
- **Acceptance:** Parse + HIR for all spec syntax, 0 test failures

### M2: Runtime Solid (~3 weeks)
**Goal:** Real programs run with correct memory and concurrency

- [ ] group block enforcement
- [ ] Actor model wired to scheduler
- [ ] try block MIR + codegen lowering
- [ ] JIT function loading (or robust handler registry)
- **Acceptance:** Example programs in `examples/` compile and run

### M3: Toolchain Ready (~2 weeks)
**Goal:** once is a usable compiler with tooling

- [ ] `once fmt` canonical formatter
- [ ] `once lint` static analysis
- [ ] Capability security enforcement
- [ ] LSP diagnostics from type checker
- [ ] Doctests
- [ ] `once explain` wired to real data
- **Acceptance:** `once build`, `once fmt`, `once lint`, `once explain` all functional

### M4: AI & Polish (~2 weeks)
**Goal:** Goal language produces real code; developer UX polished

- [ ] Real LLM integration for goal synthesis
- [ ] Example-based verification
- [ ] `once fix --consumes` auto-insertion
- [ ] Schema hydration
- [ ] Effect overrides in tests
- **Acceptance:** Goal declarations synthesize verifiable code

---

## Immediate Next Actions (Today)

```
1. while loop parsing + MIR lowering          (1.1 — 30 min)
2. Type holes _ reporting                      (1.2 — 30 min)
3. import parsing                               (1.4 — 1 hour)
4. try block MIR lowering + codegen             (4.1 — 1 hour)
5. once fmt implementation                      (3.1 — 2 hours)
6. LSP diagnostics from type checker            (3.4 — 2 hours)
```

---

## Dependency Order

```
M1 (Syntax) ──────┬──► M2 (Runtime) ──► M4 (AI/Polish)
                  │
                  └──► M3 (Toolchain) ──► M4 (AI/Polish)
```

M1 and M3 can proceed in parallel. M2 depends on M1. M4 depends on M2 and M3.

---

**Document Status:** Updated to reflect post-session 2026-05-01 reality. All 17 test suites pass with 0 failures.
