# Once Compiler: Path to 100% Production Readiness

**Status:** ~82% Spec Compliant | **Goal:** 100% Production Ready
**Audit Basis:** ONCE-001 through ONCE-008 (all 8 specification documents)
**Last Updated:** 2026-05-02

## Executive Summary

The Once compiler pipeline is architecturally complete — lexing, parsing, HIR lowering, type-checking, linearity checking, effect checking, region inference, MIR generation, and Cranelift-based codegen all function end-to-end with 200+ test suites passing. Recent work closed 17 task gaps across all four original phases.

However, a deep audit against the full text of all 8 specification documents reveals that the compiler has **structural completeness** but not **semantic completeness**. The pipeline exists for every feature, but many stages contain placeholder logic that produces incorrect or trivial results at runtime. The gaps center on three areas: **true OS-level concurrency**, **the AI goal-synthesis layer**, and **comprehensive developer tooling**.

This document is the definitive, long-term roadmap organized by specification coverage and dependency order.

---

## Specification Coverage Matrix

| Spec | Topic | Status | Key Gaps |
|------|-------|--------|----------|
| ONCE-001 | Vision & Goals | ✅ Aligned | — |
| ONCE-002 | Core Language Syntax | ⚠️ 85% | Module path resolution edge cases; `export`/`pub` visibility not enforced |
| ONCE-003 | Type System & Effects | ⚠️ 88% | Top-level signatures not forced-annotated; effect propagation incomplete at boundaries; `_` hole reporting partial |
| ONCE-004 | Memory & Concurrency | 🔴 45% | Runtime is single-threaded; actors disconnected; deadlock detection no-op; `group` not runtime-enforced |
| ONCE-005 | AI Integration | 🟡 65% | AI solver defaults to stub; prompt construction naive; example verification structural-only (no runtime eval) |
| ONCE-006 | Build System & Tooling | 🟡 70% | No `once test` command; builds are sequential; capability ceiling not enforced; `once.lock` exists but hashes not validated |
| ONCE-007 | Standard Library | 🟡 75% | `Deadline`/`Duration` semantics placeholder; `DnsResolver` stubbed; linear lifecycle for net types incomplete |
| ONCE-008 | Developer Ergonomics | 🟡 72% | Schema hydration generates code but doesn't compile through pipeline; `once explain` returns hardcoded region data; `try` error capture not instrumented |

---

## Phase 1: Runtime Reality — True Concurrency (Weeks 1–4)
**Specs:** ONCE-004 (Memory & Concurrency), ONCE-007 (§5 std::concurrency)
**Prerequisite for:** Phase 3, Phase 4

The runtime is currently a single-threaded polling loop. ONCE-004 specifies cooperative process scheduling, actor-based concurrency, channel communication, structured concurrency via `group`, and the Mutable XOR Shared invariant. None of this functions at runtime.

### 1.1 Threaded Task Scheduler
- [ ] **Replace sequential `execute_pending_tasks` loop with a thread pool.**
  - *Current state:* `for task_id in pending_tasks { ... }` — one task at a time, same OS thread.
  - *Target:* `std::thread` per task (or rayon thread pool). Each task runs on its own OS thread.
  - *Acceptance:* Two spawned tasks run concurrently (wall-clock overlap measurable).
- [ ] **Make `Scheduler` `Send + Sync`.**
  - *Current state:* `Scheduler` uses `&mut self` throughout; impossible to share across threads.
  - *Target:* Wrap internal state in `Arc<Mutex<...>>` or `Arc<RwLock<...>>`. Channel operations become lock-free where possible.
  - *Acceptance:* Scheduler can be shared across `std::thread::spawn` boundaries without `RefCell` panics.
- [ ] **Implement cooperative scheduling with backpressure.**
  - *Current state:* No scheduling policy — just runs everything in order.
  - *Target:* `Scheduler` maintains a ready queue, a blocked queue (waiting on channel/task), and a fair work-stealing dispatcher.
  - *Acceptance:* 1000 tasks spawned; all complete; no starvation observable.

### 1.2 Channel Backpressure & Correctness
- [ ] **Implement all three `BackpressurePolicy` variants at runtime.**
  - *Current state:* `Blocking` works correctly (condvar), `Dropping` works correctly.
  - *Target:* Test `Erroring` in a multi-thread context. Verify channel closes propagate to waiting receivers.
  - *Acceptance:* `Erroring` channel returns `BackpressureError` when full; `Blocking` channels wake senders on recv.
- [ ] **Add channel close/broadcast semantics.**
  - *Current state:* No mechanism to close a channel. Receivers block forever if no more senders.
  - *Target:* `Channel::close()` wakes all waiting receivers with a `ChannelClosed` error. `Channel::broadcast()` notifies all blocked senders.
  - *Acceptance:* Closing a channel unblocks all waiting receivers within 100ms.

### 1.3 Actor System Integration
- [ ] **Wire `once-actors::ActorSystem` into `once-runtime::Runtime`.**
  - *Current state:* `once-actors` is a standalone crate with zero integration. `Runtime::spawn_actor()` creates a plain `Task`, not an actor mailbox loop.
  - *Target:* `Runtime` holds an `ActorSystem`. `spawn_actor(name, behavior)` creates an `Actor` with a `Channel` mailbox. `actor_ref.send(msg)` delivers to the mailbox channel. Messages are processed by the behavior function on the actor's dedicated OS thread.
  - *Acceptance:* Echo actor receives a message and echoes it back to the sender. Counter actor increments state across 1000 sends without race conditions.
- [ ] **Implement actor supervision with restart logic.**
  - *Current state:* `supervisor_behavior` exists as code but never runs the restart loop.
  - *Target:* Supervisor actor monitors child actors via heartbeat. On crash (3 attempts max), restarts child with fresh state. After 3 failures, stops child permanently.
  - *Acceptance:* An actor that crashes every 3rd message survives 6 messages (2 restarts), then permanently stops.

### 1.4 Structured Concurrency (`group`)
- [ ] **Implement `group` block runtime enforcement.**
  - *Current state:* `TaskGroup` exists with condvar, but tasks are never actually spawned as group children or awaited via the group.
  - *Target:* MIR `SpawnTask` within a `group` region records the task ID in the active group. At group scope exit, the parent thread blocks on `group.await()` until all children complete. If any child panics/errors, remaining siblings are cancelled.
  - *Acceptance:* Spawn 3 tasks in a group. Parent blocks until all 3 finish. Spawn 3 tasks where the 2nd fails — 3rd is cancelled, error propagated to parent.

### 1.5 Deadlock Detection
- [ ] **Populate the wait-for graph with real relationships.**
  - *Current state:* `DeadlockDetector::detect_deadlock()` inserts empty `Vec::new()` for every node — DFS never finds cycles.
  - *Target:* When a task blocks on a channel recv, add `(task_id, channel_id)` to the wait graph. When a channel has no senders and has waiting receivers, detect channel deadlock. When tasks form a cycle (`A waits on B`, `B waits on A`), detect task deadlock.
  - *Acceptance:* Two tasks each waiting on the other's channel produce `DeadlockError` within the detection interval. A task waiting on a channel with no senders produces `DeadlockError`.

---

## Phase 2: Type System & Semantics Completion (Weeks 3–5)
**Specs:** ONCE-002 (§4 grammar), ONCE-003 (Type System & Effects), ONCE-008 (§3 try)
**Prerequisite for:** Phase 4

### 2.1 Mandatory Top-Level Annotations
- [ ] **Enforce type annotations on all top-level functions.**
  - *Current state:* `TypeChecker` allows `fn foo(x) { ... }` without explicit `-> Type`.
  - *Target:* Top-level (non-closure) functions without return type annotation produce `TypeError::MissingReturnAnnotation` (error, not warning). Parameter types may be inferred, but the return type must be explicit.
  - *Acceptance:* `let x = fn(y) { y + 1 };` compiles (closure). `fn add_one(y) { y + 1 }` fails (top-level).
- [ ] **Enforce explicit effect annotations on all exported functions.**
  - *Current state:* No visibility system. No check for exported vs. private effect inference.
  - *Target:* Implement `pub`/`export` visibility. `pub fn foo() !io { ... }` compiles. `pub fn bar() { call_io() }` fails — must annotate `!io` because exported. Private functions may infer effects.
  - *Acceptance:* Exported function calling `spawn` without `!spawn` annotation produces `EffectError`.

### 2.2 Full Pattern Matching & Exhaustiveness
- [ ] **Lower full enum pattern matching to MIR.**
  - *Current state:* MIR generator comment: "Full pattern matching (enum variants, bindings, guards) is future work." Match arms use a flat branch on scrutinee value.
  - *Target:* For each `match` arm, generate tag-check for the enum variant, bind pattern variables, test guard conditions. After all arms, emit a fallback that reports non-exhaustive match (compile-time check first, runtime trap for safety).
  - *Acceptance:* `match Some(42) { Some(x) => x, None => 0 }` produces correct MIR with variant-tag branching.

### 2.3 Iterator Protocol for `for` Loops
- [ ] **Implement `IntoIterator` trait and MIR lowering.**
  - *Current state:* `for` loop body always executes; full iterator protocol is marked "future work."
  - *Target:* Define `trait IntoIterator { type Item; fn into_iter(self) -> Iterator<Self::Item>; }`. MIR lowers `for item in collection` to `let mut iter = collection.into_iter(); loop { match iter.next() { Some(item) => body, None => break } }`.
  - *Acceptance:* `for x in [1, 2, 3] { print(x) }` iterates exactly 3 times with correct values.

### 2.4 `try` Block Error Context Instrumentation
- [ ] **Implement `try` error context capture in both codegen backends.**
  - *Current state:* `MirOp::TryBlock` is a no-op in both fallback and Cranelift backends.
  - *Target:* When `try { expr }` is reached, the codegen inserts instrumentation that: (1) captures the current source location, (2) captures any linear variable names in scope, (3) wraps any propagated `Err` with this context. The generated error type is `Error { message, location, context }`.
  - *Acceptance:* A `try` block that calls a function returning `Err("not found")` produces an error containing both "not found" and the file:line location of the `try` expression.

---

## Phase 3: Developer Tooling — `once test`, LSP, and CLI (Weeks 4–7)
**Specs:** ONCE-006 (Build & Tooling), ONCE-008 (Ergonomics)
**Prerequisite for:** Phase 4 (test infrastructure needed for AI verification)

### 3.1 `once test` Command
- [ ] **Implement test discovery and runner.**
  - *Current state:* No `Test` subcommand exists in the CLI. No test framework.
  - *Target:* `once test` scans `tests/` directory for `*.onc` files. Functions annotated `#[test]` (or starting with `test_`) are discovered, compiled, and executed. Each test function must return `Result<(), Error>`. Assertions via `assert_eq!(a, b)`, `assert!(cond)` built-ins.
  - *Acceptance:* `once test` discovers 3 test files, runs 7 test functions, reports `7 passed, 0 failed`.
- [ ] **Implement test-time effect overrides.**
  - *Current state:* `EffectRegistry` exists in runtime but no CLI integration.
  - *Target:* `#[test]` functions can declare `override std::effects::net with mock_net { ... }`. The mock handler replaces the real effect for the duration of that test only. Mock returns predefined data without making real network calls.
  - *Acceptance:* A test that calls `TcpStream::connect("google.com:80")` with `net` overridden passes instantly returning mock data.

### 3.2 `once explain` Real Diagnostics
- [ ] **Replace hardcoded region data with real DAG queries.**
  - *Current state:* `find_region_at_span` returns `Region { id: 1, name: "r1" }` for every query.
  - *Target:* Traverse the `RegionDag` produced by `once-rinf`. For a given source span, find the enclosing region node, its allocation count, its free point, and any escape edges. Visualize as an ASCII tree in the terminal.
  - *Acceptance:* `once explain --regions` for a function with nested scopes shows a hierarchy: `fn_main (alloc=2) -> r1 (alloc=1) -> r2 (alloc=1)`.
- [ ] **Replace hardcoded effect data with real effect graphs.**
  - *Current state:* `find_effect_at_span` returns the last effect in the checker's list, or `Empty`.
  - *Target:* Build a call-graph from the HIR. For each function, report the transitive closure of effects. Show which call introduces which effect. Walk the graph from the queried span outward.
  - *Acceptance:* `once explain --effects` for function `foo` shows: `foo !io, !spawn` because `foo -> bar (!spawn)` and `foo -> write_file (!io)`.
- [ ] **Replace hardcoded linearity data with real usage chains.**
  - *Current state:* Returns first variable in the linearity env or dummy data.
  - *Target:* Walk the `LinearityEnv` variable map. For a given variable name, report first use location, last use, whether it's consumed, and the ownership chain.
  - *Acceptance:* `once explain --linearity` for variable `file` shows: "linear File `file` — created at main.onc:12, consumed at main.onc:18, usage count: 1".

### 3.3 LSP Server Completeness
- [ ] **Implement go-to-definition via span-to-symbol mapping.**
  - *Current state:* `goto_definition` returns `Ok(None)`.
  - *Target:* Build a symbol index mapping source positions to definitions. When the cursor is on an identifier, find the corresponding `FnDecl`/`LetDecl`/`TypeDecl` and return its location.
  - *Acceptance:* Cursor on `foo` in `foo()` jumps to `fn foo() { ... }` definition.
- [ ] **Implement hover with type/effect info.**
  - *Current state:* No hover implementation.
  - *Target:* On hover over an identifier, show the inferred type (from `TypeChecker`), effect signature (from `EffectChecker`), and any constraints.
  - *Acceptance:* Hovering over `x` in `let x = 42` shows `x: Int`.
- [ ] **Implement code completion.**
  - *Current state:* No completion implementation.
  - *Target:* For completions at cursor position: suggest in-scope variables (from `NameContext`), function names (from HIR items), struct fields (from `StructDecl`), and trait methods.
  - *Acceptance:* Typing `file.` after `using file = File::open(...)` suggests `read_to_string`, `write`, `consume`.
- [ ] **Implement TCP mode.**
  - *Current state:* Message says "not yet implemented."
  - *Target:* Accept `--port` flag, listen on TCP socket, serve LSP over TCP.
  - *Acceptance:* LSP client connects to `localhost:9001` and receives diagnostics.

### 3.4 `once lint` Production Rules
- [ ] **Add capability ceiling lint.**
  - *Target:* If a dependency uses an effect not declared in the root `once.toml` `[capabilities]`, lint warns.
- [ ] **Add `box`/`rc` escape-hatch warning.**
  - *Target:* Warnings when `box T` or `rc T` is used in performance-sensitive contexts (from ONCE-004 §2.4).
- [ ] **Add unused effect lint.**
  - *Target:* If a function declares `!io` but never calls any `!io` function, warn about unnecessary effect annotation.

---

## Phase 4: AI Integration (Weeks 6–9)
**Specs:** ONCE-005 (AI-Integration & Goal Syntax)
**Prerequisite:** Phase 2 (type/effect enforcement), Phase 3.1 (test runner)

### 4.1 Robust LLM Client
- [ ] **Replace `curl` subprocess with a native HTTP client.**
  - *Current state:* `HttpAiSolver::call_api` spawns `curl` as a subprocess.
  - *Target:* Use `reqwest` (blocking or async) for API calls. Handle retries (exponential backoff, 3 attempts), timeouts (30s per request), and streaming responses.
  - *Acceptance:* API call succeeds without spawning external processes. Timeout after 30s produces `BuildError`.
- [ ] **Implement structured prompt construction.**
  - *Current state:* Prompt is a single concatenated string with no structured context.
  - *Target:* Prompt includes: (a) full type signatures of all functions/types in scope, (b) the goal's `spec` clause, (c) structured `constraints`, (d) `examples` clause as few-shot format, (e) system prompt establishing Once syntax rules and linear type semantics.
  - *Acceptance:* Prompt for a goal with 2 examples and 3 constraints is well-formed JSON with all fields.
- [ ] **Implement prompt caching keyed on content hash.**
  - *Current state:* `GoalSynthesizer` uses content hashes but the cache is in-memory only.
  - *Target:* Cache synthesized goals to `target/ai-cache/` by content hash. Check cache before calling LLM. Invalidate on source change.
  - *Acceptance:* Second `once build` of unchanged project hits cache, skips LLM call.

### 4.2 Code Generation from Goal Declarations
- [ ] **Parse `goal` decl `spec`, `constraints`, and `examples` clauses.**
  - *Current state:* Parser parses `goal` as a function-like declaration with a body block, not with goal-specific clauses.
  - *Target:* Extend `GoalDecl` AST to carry `spec: Option<String>`, `constraints: Vec<String>`, `examples: Vec<(Vec<String>, String)>`. Parse `spec "..."`, `constraints [...]`, and `examples [in(...) -> out(...)]` from the `goal` body.
  - *Acceptance:* `goal sum(a: Int, b: Int) -> Int { spec "add two numbers"; examples [in(1, 2) -> out(3)] }` parses with all clauses populated.
- [ ] **Implement AI code synthesis from goal clauses.**
  - *Current state:* `StubAiSolver` returns `{ 0 }` or `{ "" }` for everything.
  - *Target:* `GoalSynthesizer` invokes `HttpAiSolver` with structured prompt. Response is parsed as Once source. If parsing fails, retry with error feedback (up to 3 correction attempts). If type-checking fails, retry with type error feedback.
  - *Acceptance:* `goal max(a: Int, b: Int) -> Int { ... }` synthesizes a function returning `if a > b { a } else { b }`.

### 4.3 Example-Based Verification at Runtime
- [ ] **Implement runtime sandbox for example evaluation.**
  - *Current state:* `verify_goal` only does structural type-checking of examples, no runtime execution.
  - *Target:* For each example `in(A, B) -> out(C)`, JIT-compile a test harness that calls the synthesized function with inputs and `assert_eq!` on the output. Run inside a sandboxed `EffectRegistry` with all effects overridden to prevent side effects during verification.
  - *Acceptance:* Synthesized `sum(1, 2)` returns `3` — passes. Synthesized `sum(1, 2)` returns `4` — fails, goal rejected.
- [ ] **Implement the full AI-augmented compilation pipeline.**
  - *Target:* `once build` on a project with `goal` declarations: (1) parse goals, (2) check AI cache, (3) synthesize if needed, (4) parse + type-check + effect-check generated code, (5) run example verification, (6) if all pass, substitute synthesized `fn` into the compilation, (7) proceed to MIR/codegen as normal. If any step fails, fail the build with clear error pointing to the goal and the failure reason.
  - *Acceptance:* A project with 3 goals builds end-to-end, synthesizing valid implementations that pass all verification stages.

### 4.4 Goal Ejection
- [ ] **Implement `once goal eject <goal_name>`.**
  - *Target:* Replace the `goal` declaration in the source file with the synthesized `fn` (as a regular function). The AI-generated code becomes checked-in source, decoupling the project from AI at that point. Preserve the original `goal` block as a doc comment for reference.
  - *Acceptance:* After `once goal eject shortest_path`, the `.onc` file contains `fn shortest_path(...) { ... }` with the AI-generated body, and the original goal declaration is a `///` comment above it.

---

## Phase 5: Standard Library & FFI Completeness (Weeks 7–10)
**Specs:** ONCE-007 (Standard Library)

### 5.1 Linear Lifecycle Completeness
- [ ] **Implement full `Resource` trait enforcement for all stdlib types.**
  - *Current state:* `FileHandle`, `TcpStream`, `TcpListener` have `is_linear: bool` field but the `Resource` trait methods (`consume`) are not implemented or enforced at the type level.
  - *Target:* Every stdlib type holding an OS resource implements `Resource`. The `consume` method closes/flushes the resource and returns `Result<(), Error>`. The type checker enforces that all such types are consumed before leaving scope.
  - *Acceptance:* Forgetting to `consume()` a `FileHandle` produces a compile-time `LinearityError::LinearValueNotConsumed`.

### 5.2 Time & Deadline
- [ ] **Implement correct `Deadline` semantics.**
  - *Current state:* `Deadline::from_now()` stores "now" as the deadline — it's always expired. `extend()` is a comment placeholder.
  - *Target:* `Deadline::from_now(duration)` = `Instant::now() + duration`. `Deadline::has_passed()` = `Instant::now() > deadline`. `Deadline::extend(duration)` = `self.deadline += duration`. Channel recv with deadline returns `Err(Timeout)` if deadline passes before a message arrives.
  - *Acceptance:* `let d = Deadline::from_now(5s); sleep(1s); assert!(!d.has_passed()); sleep(5s); assert!(d.has_passed());`

### 5.3 DNS Resolution
- [ ] **Implement real `DnsResolver`.**
  - *Current state:* `DnsResolver::resolve("localhost")` returns `[127, 0, 0, 1]` — hardcoded.
  - *Target:* Call `std::net::lookup_host` or system resolver. Return `Vec<IpAddr>`. Handle resolution failures with `Result`.
  - *Acceptance:* `DnsResolver::resolve("example.com")` returns at least one real IP address.

### 5.4 Capability Enforcement
- [ ] **Implement capability ceiling validation.**
  - *Current state:* `once.toml` `[capabilities]` is parsed but never enforced.
  - *Target:* During `once build`, after resolving all dependencies, compute the union of all effect requirements across the dependency graph. If any dependency requires `net` but the root manifest has `net = false` (or absent), fail the build with `BuildError::CapabilityViolation`.
  - *Acceptance:* A project declaring `net = false` that depends on a library requiring `!net` fails to build.

---

## Phase 6: Optimization & Performance (Weeks 9–12)
**Specs:** ONCE-006 (§3 hermetic builds)

### 6.1 MIR Optimizer
- [ ] **Implement constant folding.**
  - *Target:* Walk MIR for `BinOp(Add, LoadLiteral(1), LoadLiteral(2))` → replace with `LoadLiteral(3)`. Handle all arithmetic and comparison ops.
  - *Acceptance:* `let x = 60 * 60 * 24` produces a single `LoadLiteral(86400)` in MIR.
- [ ] **Implement dead code elimination.**
  - *Target:* Eliminate MIR statements whose result is never used (no downstream `Move`/`Call`/`Return` references the temp). Remove unreachable basic blocks (no `Jump`/`Branch` target).
  - *Acceptance:* `let unused = expensive_call()` is eliminated from MIR if `unused` never appears in a consuming context.
- [ ] **Implement function inlining with heuristics.**
  - *Target:* Inline small functions (≤ 50 MIR ops, called ≤ 3 times) at their call sites. Adjust caller temps/locals. Preserve function for recursive/external calls.
  - *Acceptance:* `fn square(x: Int) -> Int { x * x }` called in `let y = square(5)` is inlined to `let y = 5 * 5`.

### 6.2 Parallel Builds
- [ ] **Implement true parallel build execution.**
  - *Current state:* `execute_builds` loops sequentially. `parallel_jobs` field is dead code.
  - *Target:* Build a topological DAG of targets. Execute independent targets in parallel using a thread pool of size `parallel_jobs`. Each target compiles in its own thread.
  - *Acceptance:* A project with 4 independent crates builds in wall-clock time of 1 crate (not 4x), with `parallel_jobs = 4`.

### 6.3 Incremental Compilation
- [ ] **Implement content-addressed cache with hash validation.**
  - *Current state:* Cache methods exist but are never called.
  - *Target:* Before compiling a target, hash its source + all transitive dependency hashes. If the hash matches a cached artifact, skip compilation and reuse the `.o` file. Validate `once.lock` hashes against cached artifacts on every build.
  - *Acceptance:* `once build` on unchanged source prints "using cached artifact for crate X" and produces object file in <10ms.

---

## Phase 7: Polish & Hardening (Ongoing)
**Specs:** ONCE-008 (§5 learning/comprehension), general quality

### 7.1 Error Message Quality
- [ ] **Add source excerpts to all compiler errors.**
  - *Target:* Every `TypeError`, `LinearityError`, `EffectError`, and `MirError` includes a source context excerpt (3 lines of surrounding code with a `^~~~` underline at the error span).
  - *Acceptance:* A type mismatch error shows the offending line and underlines the mismatched expression.
- [ ] **Add `--explain <error-code>` to the CLI.**
  - *Target:* Every error has a unique code (e.g., `E001` for type mismatch). `once explain E001` prints a detailed guide with examples of the error and common fixes.
  - *Acceptance:* `once explain E004` prints "Linear value used multiple times — linear values must be consumed exactly once. Use `using` blocks or explicit `.consume()`. See also: ONCE-003 §4."

### 7.2 Security Hardening
- [ ] **FFI safety audit.**
  - *Target:* All `extern "C"` exports in `once-runtime` validated for: no buffer overflows (bounds-check all `from_raw_parts`), no use-after-free (ensure `Box::from_raw` happens exactly once), no null-pointer derefs (check all `ptr == 0` before use).
  - *Acceptance:* Valgrind/ASAN clean on all FFI paths.
- [ ] **Capability sandboxing at runtime.**
  - *Target:* When `once.toml` declares `net = false`, the runtime refuses to execute `once_net_*` FFI calls (returns error instead of making the syscall). Enforced via a capability bitmask in the runtime.

### 7.3 Documentation
- [ ] **Write the Once Book.**
  - *Target:* A `docs/book/` directory with chapters: Getting Started, Core Language, Types & Effects, Memory & Concurrency, Standard Library, AI Integration, Tooling. Each chapter uses real, compilable Once code examples.
- [ ] **Generate API docs from doc comments.**
  - *Target:* Extract `///` doc comments from stdlib types and functions. Render as HTML with cross-references between types. Host on `docs.once-lang.org`.

---

## Dependency Graph

```
Phase 1 (Runtime Threads) ────┬──► Phase 3 (Tooling) ────► Phase 4 (AI)
                              │                                    │
Phase 2 (Type System) ────────┴────────────────────────────────────┤
                                                                    │
Phase 5 (Stdlib) ──────────────────────────────────────────────────┤
                                                                    │
Phase 6 (Optimizations) ◄──────────────────────────────────────────┘
                                                                    │
Phase 7 (Polish) ◄─────────────────────────────────────────────────┘
```

- **Phase 1 blocks Phase 3 and Phase 4** — AI verification needs a real test runner; test runner needs real concurrency.
- **Phase 2 blocks Phase 4** — AI synthesis needs correct type/effect enforcement to validate generated code.
- **Phase 5 is independent** — can run in parallel with Phase 3.
- **Phase 6 depends on Phase 1** — parallel builds require a thread-safe scheduler.
- **Phase 7 is ongoing** — starts immediately, continues through all phases.

---

## Effort Estimates

| Phase | Description | Engineer-Weeks |
|-------|-------------|----------------|
| Phase 1 | Runtime Concurrency | 6–8 |
| Phase 2 | Type System Completion | 4–6 |
| Phase 3 | Developer Tooling | 6–8 |
| Phase 4 | AI Integration | 5–7 |
| Phase 5 | Standard Library | 3–5 |
| Phase 6 | Optimizations | 4–6 |
| Phase 7 | Polish & Hardening | 4–6 |
| **Total** | | **32–46 engineer-weeks** |

---

## Success Criteria for 100% Production Readiness

1. **All 200+ existing tests pass, plus ≥100 new tests covering Phases 1–7.**
2. **The runtime executes 1000 concurrent tasks on ≥4 OS threads with zero race conditions.**
3. **`once test` discovers, compiles, and runs tests with effect overrides.**
4. **`once build` with `goal` declarations produces correct, type-checked, effect-checked, example-verified code via LLM synthesis.**
5. **`once explain --regions` shows a correct, span-accurate region graph for any Once function.**
6. **`once.lock` hashes are validated on every build; mismatches fail the build.**
7. **Capability ceiling violations fail the build at dependency resolution time.**
8. **All FFI exports pass ASAN/Valgrind with zero errors.**
9. **LSP server supports hover, go-to-definition, completion, and diagnostics for all error/warning types.**
10. **The Once Book covers all 8 specification areas with compilable examples.**
