# Once: Minimal Language Spec & Compiler Plan

> Goal: a simple-to-learn systems language that *removes lifetimes from user code*, guarantees memory/resource safety and race freedom by construction, and ships with hermetic tooling.

---

## 0) Design Tenets

1. **No lifetimes in user code.** Region-based memory management (RMM) with *static lifetime inference*; compiler auto-inserts alloc/free.
2. **Linear resources; ordinary values.** Anything that can leak/break (files, sockets, txns, GPU buffers) is *linear/affine*.
3. **Immutable by default; explicit local mutation.** `let` binds immutable; `var` opens a local mutation scope.
4. **Communication-first concurrency.** Channels & actors; "**mutable XOR shared**" enforced by the type system.
5. **Effects are visible but inferred.** Functions are pure by default; I/O, spawn, time, FFI appear in signatures and metadata.
6. **High-value invariants on by default.** Non-nullable types; size-aware arrays with lightweight bounds proofs.
7. **Hermetic, reproducible builds; safe FFI by default.** Non–Turing-complete build files; Wasm Component boundary + PCC-lite.

---

## 1) Core Model

### 1.1 Values & Mutability
- **Value categories**: scalars (`Int`, `Bool`, `Float`), `Str` (UTF-8), records, enums (ADTs), tuples, arrays/vecs, functions, channels/actors, handles.
- **Mutability**: `let` (immutable), `var` (scoped mutable binding). Mutation is lexical and cannot be aliased through immutable references.

### 1.2 Linear & Affine Types
- **Linear**: must be consumed exactly once. Denoted `lin T` in typings; elided in source and inferred from constructors.
- **Affine**: at-most-once use (compiler may allow drop). Denoted `aff T` internally; exposed only in diagnostics.
- **Moves** on assignment/call/return; copies require an explicit `clone` (if implemented) that consumes a linear capability and yields two *independent* linear handles when safe (e.g., split GPU buffer views with disjoint ranges).

### 1.3 Regions (RMM)
- Each function body has an implicit **primary region** `R_fn` plus optional *subregions* created by the compiler.
- Allocations default to the innermost region; at return, *escaping* values are moved to the caller's region; all non-escaped allocations are bulk-freed when their region ends.
- Fallbacks: `box T` (owned heap box with deterministic drop) and `rc T` (intrusive reference counting) when inference cannot prove region containment or uniqueness economically.

### 1.4 Effects
- Effects form a **row** `ε = [io, net, time, spawn, ffi[lib], ...]`.
- Effects are **inferred** and **printed** in IDE/signature views; adding an effect to a public function is a breaking change.
- Capability configuration in build metadata limits the set of effects a package may use.

---

## 2) Surface Syntax (excerpt)

Once uses a small, expression-first syntax with ML-style ADTs and Go-like import ergonomics.

```ebnf
Program   ::= { Item }
Item      ::= FnDecl | TypeDecl | TraitDecl | ImplDecl | LetDecl

FnDecl    ::= "fn" Ident ParamList ReturnAnn OptEffects Block
ParamList ::= "(" [ Param { "," Param } ] ")"
Param     ::= Ident ":" Type | Ident
ReturnAnn ::= [ "->" Type ]
OptEffects::= [ "!" EffectRow ]  // printed by tools; optional in source
EffectRow ::= "[" [ Effect { "," Effect } ] "]"
Effect    ::= Ident | Ident "[" Ident "]"

TypeDecl  ::= "type" Ident "=" TypeAlt { "|" TypeAlt }
TypeAlt   ::= Ident ["(" [ Type { "," Type } ] ")"] | Record
Record    ::= "{" [ Field { "," Field } ] "}"
Field     ::= Ident ":" Type

Type      ::= Ident ["<" Type { "," Type } ">"]
           |  "(" Type { "," Type } ")"
           |  "[" Type ";" Nat "]"        // sized array
           |  "Vec<" Type ">"             // growable vector
           |  "Option<" Type ">"

Block     ::= "{" { Stmt } "}"
Stmt      ::= LetStmt | VarStmt | ExprStmt | ReturnStmt | MatchStmt | ForStmt | IfStmt
LetStmt   ::= "let" Ident [":" Type] "=" Expr
VarStmt   ::= "var" Ident [":" Type] "=" Expr
ReturnStmt::= "return" Expr
Expr      ::= primary { op primary } | Call | FieldAccess | Index | Lambda
```

### Examples
```once
// algebraic data types
type Result<T, E> = Ok(T) | Err(E)

type User = { id: Int, name: Str }

fn sum(xs: Vec<Int>) -> Int {
  var acc = 0
  for x in xs { acc = acc + x }
  acc
}

// beginner-friendly resource handling
fn write_log(path: Str, line: Str) -> Unit !io {
  using f = File.open(path) {   // auto-consumes at block end
    f.write(line)
  }
}

fn map_lines(in: Chan<Str>, out: Chan<Str>) !spawn {
  for line in in { out.send(transform(line)) }
}
```

---

## 3) Static Semantics (selected rules)

### 3.1 Judgement Forms
- **Typing**: `Γ ⊢ e : T ! ε`  (in environment `Γ`, expression `e` has type `T` and effects `ε`)
- **Linearity**: `Δ` tracks linear bindings; consumption removes a binding.
- **Regions**: constraints `alloc(e) ∈ R`, `escapes(v, R_src → R_dst)` generated during typing.

### 3.2 Linearity (moves & consumption)
- **Var use**: if `x : lin T ∈ Δ`, then using `x` yields `T` and **removes** `x` from `Δ` (moved). Rebinding requires a new name.
- **Function call**: passing a linear arg consumes it in the caller; callee may return it (ownership transfer) or consume it.
- **Return**: returning a linear value transfers it to the caller's region; function must not retain it.

### 3.3 Effects (row polymorphism)
- **Purity**: primitives are pure unless marked; effectful ops contribute to `ε` and unify via row union.
- **Visibility**: `fn f -> T ! [io, spawn]` is equivalent to eliding `!` in source; tools always materialize it.

### 3.4 Arrays & Size Types
- Arrays/vecs carry a latent length `ℓ`. Simple arithmetic/guards refine constraints.
- Constraint domain: naturals with linear (Presburger) constraints only (e.g., `i < ℓ`, `k = i + 1`).
- When constraints prove safety, the bounds check is erased; otherwise a single runtime check is emitted.

---

## 4) Region Inference (compiler sketch)

**Goal:** insert `alloc`/`free(region)` for non-stack data, with zero user annotations.

### 4.1 IR Stages
1. **HIR** (desugared AST): names resolved; implicit effects/regions.
2. **TIR** (typed IR): types/effects inferred; linear usage constraints recorded.
3. **RIR** (region IR): region variables `ρ` introduced; constraints generated and solved.
4. **MIR** (lowered): explicit regions, moves, and drops; bounds checks annotated with proof status.

### 4.2 Constraints
- `alloc(e) ∈ ρ` for each allocation site.
- `escapes(v, ρ_src → ρ_dst)` when `v` crosses scope (return, channel send, actor post, closure capture).
- **Liveness**: region end ≥ last use of contained allocations.
- **Merging**: coalesce regions when beneficial; split hot paths via heuristics (based on allocation density/size profiles).

### 4.3 Solving
- Build a region DAG per function.
- Topologically place `free(ρ)` at the earliest point that post-dominates all uses and not before any escape.
- If unsatisfiable without heap: box/rc fallback sites are suggested; compiler can auto-insert with a warning or respect `@no_rc` hints.

---

## 5) Concurrency Semantics

### 5.1 Processes, Channels, Actors
- **Process**: lightweight runtime thread; scheduler is work-stealing by default; deterministic testing scheduler available.
- **Channel<T>**: SPSC or MPMC variants; `send` moves `T` if `T` is linear, copies otherwise; receiving process becomes owner.
- **Actor**: mailbox + loop; internal `state: var S` is *not shareable*; external interaction via messages only.

### 5.2 Safety Invariants
- **Mutable XOR Shared**: a value of a mutable type cannot be shared; a shared value must be immutable.
- **Send**: A type is `Send` iff either (a) it is immutable, or (b) it is linear and the send moves ownership.

### 5.3 Effects & Versioning
- Spawning contributes `spawn` to `ε`; blocking ops contribute `time`.
- Adding effects to public APIs increments the major version per tool enforcement.

---

## 6) FFI & Interop

### 6.1 Preferred path: Wasm Component boundary
- Interface defined in a WIT-like IDL; host enforces memory isolation; linear handles are represented as *capability indices*.
- Compiler emits a **PCC-lite** bundle containing: memory layout hashes, aliasing guarantees, bounds summaries for exported funcs.
- Loader validates bundle before instantiation; failure aborts load.

### 6.2 Unsafe in-proc FFI
- `unsafe ffi` blocks allow C/Rust calls inside the same address space.
- Required: explicit marshalling signatures, `!ffi[lib]` effect, and a generated fuzz harness.
- Linear handles cannot cross unless the callee contract proves single-ownership semantics.

---

## 7) Build System (Hermetic & Declarative)

- **Format**: `once.toml` (no loops, no conditionals; feature flags are declarative).
- **DAG**: computed from imports; the file pins versions, capabilities, and build profiles.
- **Repro**: content-addressed cache; timestamps/paths scrubbed; lockfile includes effect/capability set to guard supply-chain drift.

```toml
[package]
name = "acme.web"
version = "0.1.0"

[deps]
http = "2.1.0"
sql  = "0.9.3"

[capabilities]
io   = true
net  = true
spawn= true

[profile.release]
opt-level = 3
lto = true
```

---

## 8) Standard Library (MVP)

- `core`: `Option`, `Result`, tuples, math, ordering, formatting.
- `collections`: `Vec`, `Array[n]`, `Map`, `Set`, `Deque`.
- `io`: `File` (linear), `Reader`, `Writer`, buffered I/O.
- `net`: `TcpListener` (linear), `TcpStream` (linear), DNS.
- `concurrency`: `Chan<T>`, `Actor`, `spawn`, timers.
- `time`: `Instant`, `Duration`, `Deadline` (linear cancel token).
- `ffi`: Wasm components, unsafe bindings, marshalling helpers.

---

## 9) Diagnostics & Learnability

- **Explain modes**: `--explain=regions` shows annotated region graph; `--explain=effects` shows derivation; `--explain=linearity` highlights move/consume sites.
- **Fix-its**: Insert missing `close/commit`, suggest local `var` blocks, propose `box`/`rc` when inference fails.
- **Beginner profile**: Level 1 docs teach `using`, `spawn/join`, `Option/Result` with `?`; channels/actors and refinements are Level 2.

---

## 10) Backend & Compiler Architecture

### 10.1 Pipeline
1. **Frontend**: parser → name resolution → HIR.
2. **Type & Effect Inference**: Hindley–Milner with row-polymorphic effects; linear usage checking; generate constraints.
3. **Region Inference**: generate region variables and (escape, liveness) constraints → solver → RIR.
4. **Lowering to MIR**: explicit moves, drops, region frees; bounds-check annotations; actor/channel ops.
5. **Codegen** (choose one first; keep abstraction boundary):
   - **Cranelift** for fast dev/test; or
   - **LLVM** for maximum perf; or
   - **MLIR** if planning custom dialects (regions/effects) long-term.
6. **Runtime**: tiny core (scheduler, channels, time); platform shims; no GC, no stop-the-world.

### 10.2 Artifacts
- `.onceo` object modules include type/effect/region summaries for link-time checks.
- Linker enforces capability ceilings and deduplicates compatible versions via namespacing.

---

## 11) Roadmap (6–12 months)

**M0–M2: Frontend & Checker**
- Grammar, parser, HIR.
- HM type inference + effect rows; linear usage checker (move/consume).
- Minimal std (`Option`, `Result`, `Vec`, `File` as linear with stub runtime).

**M3–M4: Region Inference & MIR**
- Constraint generator; solver; MIR with explicit regions/drops.
- Bounds reasoning (linear constraints); check erasure when proven.
- Debug views: region/effect explainers.

**M5–M6: Concurrency Runtime**
- Work-stealing scheduler; channels (SPSC/MPMC); actors.
- Deterministic test scheduler; timeouts/cancellation tokens (linear).

**M7–M8: Codegen & Perf**
- Cranelift backend; escape hatches (`box`, `rc`) with lints.
- Benchmarks vs. C/Rust on parsing, HTTP echo, matrix ops.

**M9–M10: Tooling & Build**
- `once` build tool (declarative), content-addressed cache, lockfile.
- LSP (diagnostics, code actions for consumes/moves/close insertion).

**M11–M12: FFI & Hardening**
- Wasm Component interop + PCC-lite validator.
- Unsafe in-proc FFI scaffolding + fuzz harness generator.
- Capability-aware linker & effect-based semver enforcement.

---

## 12) Sample Programs

### 12.1 File processing (linear resource)
```once
fn run(path: Str) -> Result<Int, Err> {
  using f = File.open(path) {       // lin File auto-consumed
    var total = 0
    for line in f.lines() {
      total = total + parse_int(line)?
    }
    Ok(total)
  }
}

fn read_lines(f: lin File) -> Vec<Str> !io {
  var acc = Vec::new()
  var g = f
  for line in g.lines() { acc.push(line) }
  g.close()
  acc
}
```

### 12.2 Concurrency (CSP pipeline)
```once
fn pipeline(input: Chan<Str>, output: Chan<Int>) !spawn {
  let mid = Chan::new()
  spawn map_lines(input, mid)
  spawn parse_to_int(mid, output)
}
```

---

## 13) Open Risks & Mitigations

- **Region inference corner cases**: provide explicit `box`/`rc` escapes with clear diagnostics and perf hints; profiler-guided subregion splitting.
- **Internal fragmentation**: heuristics + `@packed` allocation hints; arena compaction for long-lived regions.
- **Effect creep**: effect ceilings in package metadata; public API diffs fail CI if effects widen.
- **FFI soundness**: default component boundary, mandatory validator; unsafe path quarantined under `feature = "unsafe_ffi"` and audited.

---

## 14) Minimal Spec Checklist (v1)

- [ ] Syntax & parser stable
- [ ] HM type inference w/row effects
- [ ] Linear usage checker (moves/consumes)
- [ ] Region inference + MIR drops/frees
- [ ] Bounds proofs + check erasure
- [ ] Channels, actors, scheduler
- [ ] Cranelift backend
- [ ] Declarative build tool + lockfile
- [ ] LSP w/ explain & fix-its
- [ ] Wasm Component FFI + PCC-lite

---

### Naming note
**Once** is the chosen name. Suggested file extension: `.onc` (sources), `.onceo` (objects). CLI: `once` (build), `oncec` (compile).

---

## 15) Developer Experience (DX) Additions

**Goal:** keep the language easy to learn and delightful to use from day 1.

### 15.1 Beginner-friendly surface
- **`using` by default** in docs and templates; fix-its insert it automatically.
- **Error propagation `?` operator** on `Result` (already shown in samples).
- **One-page mental model** shipped with the toolchain (`once help learn`).
- **Friendly errors** with actionable fix-its and links to short guides.

### 15.2 Tooling
- **REPL & playground** (`once repl`, `once play`) with region/effect visualizer.
- **Formatter & lints** (`once fmt`, `once lint`) with defaults that match all examples.
- **Test runner** (`once test`) with deterministic scheduler toggle (`--deterministic`).
- **Doctests**: code blocks in docs compile and run in CI.
- **Scaffolding** (`once new app|lib`) with Level‑1 templates (no channels/actors until opted in).

### 15.3 Observability
- **Region plan view**: `once explain --regions` renders a graph (alloc sites → frees). 
- **Effect map**: `once explain --effects` shows per‑fn effect rows and call‑graph diffs.
- **Perf hooks**: lightweight counters for allocations per region, moves/consumes, and channel traffic.

### 15.4 Interop ergonomics
- **Wasm Component first‑class**: `once component add <wit>` generates safe bindings.
- **Unsafe FFI guardrails**: generator produces fuzz harness + sanitizer flags; CI templates include them.

---

## 16) AI‑Friendly Extensions (optional, not in the core language)

**Position:** AI is a major workload class, but we keep the *language core* minimal. Provide first‑party libraries and capabilities so AI teams are productive without baking ML‑specific syntax into the language.

### 16.1 `once-nd` (Numerics/Tensors)
- **`NdArray<T, D>`** with shape tracked via size types; static proofs erase many bounds checks.
- **Views & slices** are linear or immutable depending on aliasing; kernel ops take linear buffers when mutating in‑place.
- **Deterministic RNG** (counter‑based) for reproducible training/eval.

### 16.2 `once-gpu` (Compute backends)
- **`gpu` effect** in the effect row for kernels; host/device memory as linear handles.
- **Backends**: WGPU (Vulkan/Metal/DX), CUDA (when available), and CPU fallback.
- **Kernel DSL**: a tiny embedded subset (no new syntax) compiled via MLIR to WGSL/PTX/LLVM.

### 16.3 `once-onnx` / `once-vllm` Interop
- **ONNX runtime component** behind the Wasm boundary; models load as capabilities with explicit memory limits.
- **Text/embedding pipelines** as examples, not core primitives.

### 16.4 Training-friendly runtime knobs
- **Pinned memory** and **zero‑copy views** as safe APIs.
- **Deterministic scheduler** mode for data‑loader pipelines.
- **Streaming IO** with backpressure via channels.

### 16.5 Debuggability for ML
- **NaN/Inf guards** optional in debug builds.
- **Shape mismatches** produce precise diagnostics with suggested fixes.

**Why optional:** keeps the language simple for general devs while giving AI users first‑class, safe building blocks.

---

## 17) Accepted Additions (from cross‑language pros)

These are **planned** and part of v1 design.

### 17.1 Pipeline operator `|>` (F# ergonomics)
- **Purpose:** left‑to‑right composition for readability.
- **Desugaring:** `x |> f(a) |> g` ≡ `g(f(x, a))` when `f` is binary; otherwise `g(f(x))`.
- **Example:**
```once
let total = lines
  |> map(parse_int)
  |> filter(is_positive)
  |> sum()
```

### 17.2 `async/await` sugar (C# ergonomics, Go simplicity)
- **Model:** `async { expr } : Task<T>`; `await t` consumes a linear `Task<T>` and yields `T`.
- **Rules:** `await` is allowed only where effects permit blocking; `Task<T>` is linear (must be awaited, joined, or cancelled).
- **Example:**
```once
fn fetch_all(urls: Vec<Str>) -> Result<Vec<Str>, Err> {
  let tasks = map(|u| async { http_get(u) }, urls)
  // join all (consumes tasks)
  join_all(tasks)?
}
```

### 17.3 Slices/Spans (`Slice<T>`, `SliceMut<T>`) (C#/Rust ergonomics)
- **`Slice<T>`**: immutable, non‑owning view with length; freely shareable.
- **`SliceMut<T>`**: mutable, non‑owning view; **linear** and exclusively owned while in scope.
- **Safety:** bounds proven via size types when possible; else single check at creation.
- **Example:**
```once
fn normalize(win: SliceMut<Float>) {
  let m = mean(win as Slice<Float>)
  for i in 0..win.len { win[i] = win[i] - m }
} // win consumed (linear)
```

### 17.4 Units of Measure (library, F#‑inspired)
- **Library types:** `Measure<N, U>` where `N` is numeric, `U` is a unit tag.
- **Common aliases:** `Meters`, `Seconds`, `USD`, etc.
- **Checked ops:** only compatible units compose; mismatches are compile errors.
- **Example:**
```once
use once::uom::{Meters, Seconds}

fn speed(d: Meters, t: Seconds) -> MetersPerSecond {
  d / t
}
```

---

## 18) Kickoff Implementation Blueprint

### 18.1 Repo layout (bootstrap in Rust; small, modular crates)
```
once/
├─ LICENSES/ (MIT OR Apache-2.0)
├─ README.md
├─ RFCs/ (short proposals, numbered)
├─ crates/
│  ├─ once-lex/         # lexer (logos/chumsky-like, no deps commitment in spec)
│  ├─ once-parse/       # parser → AST (HIR-ready)
│  ├─ once-hir/         # name resolution, imports, basic desugaring
│  ├─ once-ty/          # HM types + traits + unification
│  ├─ once-effects/     # row-polymorphic effects inference
│  ├─ once-linear/      # move/consume checker
│  ├─ once-rinf/        # region inference solver (constraints → region DAG)
│  ├─ once-mir/         # lowered IR (explicit moves/drops/region frees)
│  ├─ once-codegen/     # Cranelift backend (LLVM pluggable later)
│  ├─ once-runtime/     # scheduler, channels, time, OS shims (no GC)
│  ├─ once-std/         # core, collections, io, net, concurrency (MVP)
│  ├─ once-build/       # build tool (parse once.toml, DAG, cache)
│  ├─ once-lsp/         # language server (diagnostics, fix-its, explainers)
│  └─ once-cli/         # `once` (build/test/fmt/lint/play) + `oncec` (compile)
├─ examples/
│  ├─ hello/
│  ├─ http-echo/
│  └─ csv-sum/
└─ docs/
   ├─ learn-once.md (Level 1)
   ├─ oop-in-once.md (traits/actors cookbook)
   └─ concurrency-deterministic-tests.md
```

### 18.2 Coding standards
- Rust 2021+, `clippy` clean; deny warnings in CI.
- Unit tests in each crate; integration tests under `once-cli/tests`.
- Snapshot tests for parser/diagnostics; property tests for region solver.

---

## 19) Minimal Grammar Slice (v1 scope)
- Expressions, `let/var`, `fn`, `type` (ADTs/records), `impl/trait`, `match`, `for`, `if`.
- Operators: arithmetic, comparison, boolean; pipeline `|>` (left-assoc, low precedence).
- Async sugar: `async { e }` and `await e` desugar to `Task` ops in MIR.
- Slices: `a[i..j]` produces `Slice<T>` / `SliceMut<T>` depending on binding.

---

## 20) Type & Effect Inference (algorithm sketch)
- **Types:** HM with monomorphic recursion (v1), parametric generics, traits via dictionary passing in MIR.
- **Effects:** row variables `ρ`, unify by row union; generalize effects for polymorphic functions where rows don't escape.
- **Signatures:** pretty-printer shows `fn f(...) -> T ![io, net]` but source omits `![...]`.

---

## 21) Linearity Checking
- Environment split: `Γ` for ordinary values; `Δ` for linear bindings.
- Rules: use = consume (remove from `Δ`); move on assign/call/return; copy only via type-provided `clone()` that semantically splits the resource.
- Diagnostics: "'{x}' is a linear value and must be consumed; add `using`, return it, or call a consuming method here."

---

## 22) Region Inference Solver
- Generate constraints: `alloc(e) ∈ ρ`, `escapes(v, ρs→ρd)`, liveness of `ρ` w.r.t. last use.
- Solve per function to a region DAG; place `free(ρ)` at earliest post-dominator safe point.
- Heuristics: subregion splitting on hot loops; coalesce tiny regions; 
- Fallback: insert `box`/`rc` with lint and fix-it; allow `@no_rc` attribute to force compilation failure instead.

---

## 23) Runtime (MVP)
- Work-stealing scheduler; cooperative cancellation via linear `Deadline` tokens.
- Channels: SPSC lock-free; MPMC using bounded ring buffers; backpressure APIs.
- Timers, monotonic `Instant`, `Duration` primitives.
- No global allocator requirement; arenas per region for large lifetimes.

---

## 24) Build Tool (`once`)
- `once new app|lib` → scaffolds Level‑1 template.
- `once build` → hermetic DAG, content-addressed cache, lockfile with capability/effect ceiling.
- `once test` (`--deterministic`), `once fmt`, `once lint`, `once play`, `once explain --regions|--effects`.

---

## 25) Test Strategy
- **Golden tests**: parser → AST/HIR, pretty-printer roundtrip.
- **Linear/region property tests**: generate small programs; assert no leaks/double‑free; region frees post-dominate uses.
- **Effect diffs**: ensure public API effect rows don't widen without semver bump.
- **Concurrency determinism**: fixed scheduler mode for reproducible traces.
- **Doctests**: all code in `docs/learn-once.md` runs in CI.

---

## 26) Showcase Apps (prove breadth)
1) **HTTP Echo** (no deps): `spawn` per-conn; linear `TcpStream`; demonstrate `using`.
2) **CSV Sum**: streaming IO + pipeline `|>`; bounds-safe slicing.
3) **Image Blur (cpu)**: `SliceMut<u8>` windows; proves safe in-place mutation.
4) **ONNX Inference (wasm component)**: load model with memory caps; deterministic RNG for pre/post.

---

## 27) AI Libraries (prototype plan)
- `once-nd`: `NdArray<T,D>`; BLAS-like ops; shape proofs; debug NaN guards.
- `once-gpu`: WGPU backend; `gpu` effect; host/device linear buffers; kernel DSL lowering via MLIR path (future-proofed for CUDA/PTX).
- `once-onnx`: component wrapper; examples: text embedding, image classify.

---

## 28) Issue Seeds (copy into tracker)
- Parser: pipeline precedence & associativity (#1)
- Type checker: trait dictionary passing in MIR (#2)
- Effects: row unification + pretty-print for public APIs (#3)
- Linear checker: move/consume across `match` arms (#4)
- Region solver: free placement + unsat fallback (#5)
- Slices: `SliceMut` linearity rules (#6)
- Async sugar: `await` must consume `Task` (#7)
- Build: content-addressed cache keys + capability ceilings (#8)
- LSP: fix-its for `using` insertion, effect explanation (#9)
- Runtime: deterministic scheduler mode (#10)

---

## 29) Closures, Captures, and Async Linearity (Refinement)

### 29.1 Closure capture rules
- **Copy vs Move:** Capturing a value of a type that implements `Copy` copies the value into the closure environment. Capturing a value of a **non‑Copy** type moves it.
- **Linear capture:** If a closure captures any **linear** value, the closure itself becomes **linear** (callable exactly once) and implements the trait `FnOnce`. Such a closure can:
  - be invoked **once**, consuming the closure; or
  - be **sent** to another process (if all captured values are `Send`) where it may be invoked once; or
  - be stored as a linear value and later consumed.
- **Nonlinear closures:** Closures that capture only `Copy` or immutable data may implement `Fn` (pure, re‑callable) or `FnMut` (requires a local `var` binding inside; not shareable across tasks unless linearized via actors).

### 29.2 Region inference interactions
- Captured values moved into a closure are treated as **escaping** from the enclosing region: `escapes(v, ρ_enclosing → ρ_closure)`.
- A closure's environment region `ρ_closure` must outlive the last possible invocation; for `async` closures this extends to task completion.

### 29.3 `async { e }` tasks are linear
- `async { e } : Task<T>` creates a **linear** task handle that must be consumed by exactly one of: `await`, `join`, or `cancel`.
- **Effects:** constructing an async task contributes `spawn` (and any effects from `e`); `await` may contribute `time`.
- **Diagnostics:** forgetting to consume a task is a compile error with a fix‑it suggesting `await`, `join_all`, or `cancel`.

---

## 30) Standard Linear Resource Interface & `using` Desugaring

### 30.1 Resource trait
```once
trait Resource {
  // perform the terminal action for this resource (close, commit, release)
  fn consume(self) -> Unit !ε
}
```
- All standard linear types (`File`, `TcpStream`, `Txn`, `Deadline`, GPU buffers) implement `Resource`.
- User types can opt in by implementing `Resource`.

### 30.2 `using` desugaring
```
using x = E { B }
// desugars to
let _tmp = E;
let x = _tmp;          // move linear value into x
let _out = (|| { B })();
consume(x);            // must type‑check against Resource::consume; may have effects !ε
_out
```
- If `consume` returns `Result<Unit, Err>`, the desugaring appends `?` unless `using!` (explicitly non‑propagating) is used.
- The desugaring guarantees `consume` runs even with early `return` from `B`.

### 30.3 Copy & Clone contracts
- **Marker trait `Copy`**: only types implementing `Copy` may be implicitly duplicated.
- **`clone()`** for linear types is opt‑in and must return two or more **independent** linear handles with documented disjointness guarantees; otherwise unavailable.

---

## 31) Concurrency Liveness Tooling & Backpressure Semantics

### 31.1 Deadlock detection (debug builds)
- The runtime maintains a **wait‑for graph** for tasks (edges: send/recv waits, joins, locks for future profiles).
- In deterministic scheduler mode, cycles are detected and produce an immediate **Deadlock** diagnostic with a minimal cycle trace and per‑edge source locations.
- CLI support: `once test --deterministic --deadlock=fail` (default in debug).

### 31.2 Channel backpressure policy (explicit)
- Channel constructors require a **capacity** and a **policy**:
```once
enum Backpressure { Block, DropOldest, DropNewest, Error }
let c = Chan::new(cap=1024, policy=Backpressure::Block)
```
- `send/recv` semantics:
  - **Block**: `send` blocks when full; `recv` blocks when empty.
  - **DropOldest/Newest**: `send` returns `Result<Unit, Dropped>`; no blocking.
  - **Error**: `send` returns `Err(Full)` immediately.
- `once explain --concurrency` renders channel buffer sizes, policies, and hot senders/receivers.

---

## 32) Unsafe FFI Hardening & Std as Components

### 32.1 Build‑time enforcement for `unsafe ffi`
- The build tool enforces:
  - Presence of a **fuzz harness** for each `unsafe ffi` binding.
  - Security profile flag enabled in `once.toml`:
```toml
[profile.security]
ffi_safe = true
```
  - Without both, `once build` fails with an actionable diagnostic.

### 32.2 Standard library as Wasm Components
- The standard library exposes a subset of modules as **Wasm Components** behind a stable IDL (WIT‑like), enabling cross‑language consumption and validating the component boundary at the heart of FFI.

---

## 33) Type System UX: Always‑Visible Signatures & Linearity Chain

### 33.1 Always‑visible signatures
- The LSP shows fully resolved types on hover including:
  - Effect row `![...]`
  - Linearity annotations for parameters/returns (e.g., `f: lin File`)
  - Sendability where relevant (`Send<T>`)

### 33.2 Linearity chain debugger
- On move/consume errors, diagnostics include a **chain**:
  - **Origin** (where the linear value was created)
  - **Ownership transfers** (calls, sends, returns)
  - **Consumption** site
  - The **second use** that violates linearity
- Quick‑fixes propose `using`, returning the handle, or `clone()` if available and safe.

---

## 34) Core Modernizations (v1 additions)

### 34.1 Atomics & Memory Model
- Types: `Atomic<Int>`, `Atomic<Bool>`, `Atomic<Ptr<T>>`.
- Default ordering: `SeqCst`; optional `Acquire`, `Release`, `AcqRel`, `Relaxed` via explicit APIs.
- Guidance: atomics are for interior mutability **inside actors**; across tasks prefer ownership move (keeps *mutable XOR shared*). 
- Example:
```once
fn tick(counter: &Atomic<Int>) { counter.fetch_add(1, Ordering::AcqRel) }
```

### 34.2 Structured Concurrency Nurseries
- Syntax: `group { let t = spawn ...; ... }` ensures all child tasks are **joined or cancelled** on scope exit.
- Failure policy: default **fail-fast** (first error cancels siblings); configurable: `All`, `FailFast`, `Supervisor`.
- Example:
```once
group(policy=FailFast) {
  let a = spawn { http_get(u1) }
  let b = spawn { http_get(u2) }
  let r1 = await a?; let r2 = await b?;
}
```

### 34.3 Deterministic Numerics Options
- CLI flag: `--deterministic-float` pins math (disables fused ops, sets rounding). 
- New numeric: `Decimal` (base‑10) for money; `FromStr`/`Display` in std.

### 34.4 Unicode‑Correct Text
- Distinguish `Bytes` vs `Text` (UTF‑8, grapheme aware). Indexing by code unit is disallowed; slicing by grapheme clusters via safe APIs.
- Formatting: locale‑aware in std; heavy i18n via components.

### 34.5 Portable SIMD & Slices
- Module `simd`: `Vec128<T>`, `Vec256<T>` with trait‑based ops; safe interop with `Slice<T>`/`SliceMut<T>`.
- Example:
```once
fn axpy(a: Float, x: Slice<Float>, y: SliceMut<Float>) {
  for i in 0..x.len { y[i] = a * x[i] + y[i] }
}
```

### 34.6 Lightweight Const‑Eval & Derives
- `const fn` for pure, terminating compile‑time computation (table gen, bounds, hashes).
- `derive` codegen (no macros) for `Copy`, `Eq`, `Ord`, `Show`, `Resource` where semantics are unambiguous.

---

## 35) Tooling & First‑Party Libraries Roadmap (post‑v1 epics)

### 35.1 Time‑Travel Debugger (TTD)
- `once test --ttd` record/replay; step backward across scheduler events and IO stubs.

### 35.2 Property‑Based + Concurrency Fuzz
- Library `once-check`: generators, shrinking; `--sched-fuzz` to explore interleavings.

### 35.3 Capability‑Aware Observability
- `once-trace` with OTLP exporters; signals gated by declared effects/capabilities to prevent accidental data leaks.

### 35.4 Policy‑Guarded FS/NET
- `once.toml` declares allowed roots/egress; runtime enforces least authority.
```toml
[capabilities.fs]
roots = ["/srv/app/data", "./tmp"]

[capabilities.net]
egress = ["*.example.com:443", "10.0.0.0/8:5432"]
```

### 35.5 ABI/Effect Diff & Semver Guard
- `once abi-diff` & `once effect-diff`: CI fails if public ABI or effect row widens without major bump.

### 35.6 Supply‑Chain Provenance
- Built‑in SBOM (SPDX) and SLSA attestations; registry signing; optional vendor mode with hash pinning.

### 35.7 Remote Cache & Cross‑Compile
- `once build --target <triple>` with content‑addressed **remote cache** to accelerate CI.

### 35.8 Dev Hot‑Reload via Components
- Watch mode compiles changed modules to **Wasm components** and swaps them behind stable trait shims; preserves state safely.

### 35.9 Schema‑Driven Data & Queries
- Generators for OpenAPI/SQL/WIT → typed clients & **row‑typed queries** with compile‑time nullability/size proofs.

---

## 36) Refinements Based on Expert Feedback

### 36.1 Refining Region Inference for Closures

**Blueprint Strength**: RMM fallbacks with `box T` and `rc T` provide essential escape hatches when static analysis is too complex or costly.

**Gaps / Recommendations**:

#### Explicit Closure Capture Rules
- **Rule**: Non-Copy variable capture in closures: Captured linear values must be moved into the closure. The closure must then be treated as a linear entity that can only be called once, or transferred to another process.
- **Rationale**: Prevents potential soundness gaps where a captured linear resource (like a file handle) could be closed by the outer region while the inner closure still holds a reference to it. This forces a clean transfer of ownership.

#### Escape Analysis Enhancement
- **Current**: RIR stage includes `escapes(v, ρ_src → ρ_dst)` constraints when a value crosses a scope (e.g., return, closure capture).
- **Enhancement**: More sophisticated escape analysis for complex closure interactions and async code patterns.

#### Async/Await Linearity Clarification
- **Rule**: The `Task<T>` object produced by `async { e }` is itself a linear resource that must be consumed by either `await`, `join`, or a dedicated `cancel` operation.
- **Rationale**: Enforces resource safety for the asynchronous token itself, ensuring tasks aren't accidentally leaked and preventing common C#/JS issues where tasks are forgotten.

### 36.2 Standardizing the Linear Resource Interface

**Blueprint Strength**: The `using` syntax provides a simple, beginner-friendly syntax for deterministic consumption.

**Gaps / Recommendations**:

#### Formal Resource Trait
- **Addition**: Introduce a standard library trait `Resource { fn consume(self) }` that all linear types (like `File`, `TcpStream`, `Deadline`, user-defined transactions) must implement.
- **Rationale**: Makes the linear safety model modular, allowing users to apply the high-value resource safety model to their custom types (e.g., a database `Transaction`, a GPU `Buffer` handle) without language-level changes.

#### Copy Trait Constraints
- **Addition**: For values that should be safe to copy (like scalars, immutable structs, or references to immutable data), an explicit `Copy` trait should exist.
- **Enforcement**: The linearity checker must enforce that only types implementing `Copy` can be implicitly duplicated.
- **Rationale**: Crucial defense mechanism inherited from safe language design, ensuring that non-copyable linear resources are never silently duplicated.

#### Cloning Operations
- **Current**: Explicit `clone` operations require consuming a linear capability to yield two independent linear handles.
- **Enhancement**: Ensure this is consistently enforced across all resource types.

### 36.3 Concurrency Debugging for Liveness Issues

**Blueprint Strength**: Deterministic scheduler for reproducible concurrency traces.

**Gaps / Recommendations**:

#### Deadlock Detector in Debug Runtime
- **Addition**: Implement a lightweight cycle detection algorithm on the wait-for graph during the execution of the deterministic scheduler.
- **Behavior**: If a cycle is detected (e.g., Process A waits for B, B waits for A), the runtime should immediately halt and output an explicit deadlock error with the trace.
- **Rationale**: Provides the same immediate feedback for liveness errors (deadlock) as the compiler provides for safety errors (linear/region checks), a massive DX win.

#### Backpressure Semantics/Visibility
- **Addition**: Clearly define the default backpressure semantics of channels (e.g., blocking, dropping, or erroring).
- **Tooling**: The `once explain --concurrency` tool should show the channel buffer size and the backpressure policy used by each channel instantiation.
- **Rationale**: Uncontrolled queue growth/blocking is a common point of failure. Making backpressure explicit (and perhaps linear for the buffer itself) is critical for high-load systems.

### 36.4 Hardening the Unsafe FFI Boundary

**Blueprint Strength**: PCC-lite bundle generation (hashes, aliasing summaries) is a pragmatic step towards verifiable interoperability.

**Gaps / Recommendations**:

#### Automated Fuzzing Enforcement
- **Addition**: Make the build tool (`once`) enforce the quarantine: an unsafe ffi block automatically triggers a failing lint check unless a corresponding fuzzing test is present and a high-security profile (`profile.ffi-safe = true`) is set in `once.toml`.
- **Rationale**: The cost of using unsafe should be high and mandatory. This ensures developers cannot ignore the security requirement when bypassing the safe component boundary.

#### Standard Library Component
- **Addition**: The standard library should itself be available as a Wasm Component interface.
- **Rationale**: Facilitates interoperation with other languages that are adopting the Component Model and ensures the FFI design can handle the core system APIs.

### 36.5 Simplifying the Type System Experience

**Blueprint Strength**: LSP and explainers with `--explain` modes for regions, effects, and linearity.

**Gaps / Recommendations**:

#### Interactive Signature Display
- **Addition**: The LSP must prioritize displaying the fully resolved, materialized type signature on hover, specifically including the Effect Row and the Linearity of all arguments/return values.
- **Rationale**: The complexity of effects and linearity is hidden by the syntax. Making them visible in the IDE at all times, without running a full `--explain`, is critical for a "visible but inferred" model.

#### "Linearity Chain" Debugger
- **Addition**: When a linearity error occurs (e.g., `x` used twice), the diagnostic should not just point to the second use, but provide a trace-back to the first use and the point of consumption/transfer that made the second use invalid.
- **Rationale**: Pinpointing the exact source of a linearity violation across a call stack dramatically reduces the frustration associated with ownership/move systems.

---

## 37) Implementation Status

### 37.1 Completed Components ✅
- **Lexer** (`once-lex`): Complete tokenization with async keywords
- **Parser** (`once-parse`): AST generation with spawn/await support
- **HIR** (`once-hir`): Name resolution and desugaring
- **Type System** (`once-ty`): HM inference with linear types and regions
- **Effects** (`once-effects`): Row-polymorphic effects with async support
- **Linearity** (`once-linear`): Move/consume analysis with resource safety
- **CLI** (`once-cli`): Complete command-line interface

### 37.2 In Progress 🔄
- **Region Inference** (`once-rinf`): Static lifetime inference solver
- **MIR** (`once-mir`): Lowered IR with explicit moves and drops
- **Code Generation** (`once-codegen`): Cranelift backend implementation
- **Runtime** (`once-runtime`): Scheduler, channels, and OS integration

### 37.3 Pending 📋
- **Standard Library** (`once-std`): Core types and functions
- **Build System** (`once-build`): Hermetic builds and dependency management
- **LSP** (`once-lsp`): Language server with diagnostics and fix-its

---

## 38) Next Implementation Priorities

### 38.1 Immediate (Next 2-4 weeks)
1. **Region Inference Solver**: Implement constraint generation and solving
2. **MIR Generation**: Lower HIR to explicit moves/drops
3. **Basic Runtime**: Work-stealing scheduler and channels

### 38.2 Short-term (1-2 months)
1. **Code Generation**: Cranelift backend integration
2. **Standard Library**: Core types and resource implementations
3. **Build System**: Hermetic build tool with dependency management

### 38.3 Medium-term (2-4 months)
1. **LSP Implementation**: Full language server with diagnostics
2. **FFI System**: Wasm Component integration
3. **Testing Framework**: Comprehensive test suite and examples

---

## 39) Feedback Integration Checklist

### 39.1 Closure Capture Rules
- [ ] Implement explicit closure capture analysis
- [ ] Add linearity constraints for captured variables
- [ ] Ensure closure linearity is properly tracked

### 39.2 Resource Trait System
- [ ] Implement formal `Resource` trait
- [ ] Add `Copy` trait constraints
- [ ] Ensure all built-in resources implement `Resource`

### 39.3 Concurrency Debugging
- [ ] Add deadlock detection to debug runtime
- [ ] Implement backpressure semantics visibility
- [ ] Add concurrency explanation tools

### 39.4 FFI Security
- [ ] Implement automated fuzzing enforcement
- [ ] Add security profile requirements
- [ ] Create standard library Wasm Component interface

### 39.5 Developer Experience
- [ ] Add interactive signature display to LSP
- [ ] Implement linearity chain debugging
- [ ] Enhance error messages with usage traces

---

## 40) Architecture Decisions Record (ADR)

### 40.1 ADR-001: Resource Trait Design
**Status**: Accepted  
**Decision**: Implement formal `Resource` trait with `consume(self)` method  
**Rationale**: Provides modular resource safety model for user-defined types  
**Implementation**: Add to `once-std` crate with derive macro support

### 40.2 ADR-002: Closure Linearity Rules
**Status**: Accepted  
**Decision**: Captured linear values must be moved into closure, making closure linear  
**Rationale**: Prevents soundness gaps in resource management  
**Implementation**: Extend linearity checker in `once-linear` crate

### 40.3 ADR-003: Deadlock Detection
**Status**: Accepted  
**Decision**: Implement cycle detection in debug runtime with immediate halt  
**Rationale**: Provides immediate feedback for liveness errors  
**Implementation**: Add to `once-runtime` crate with deterministic scheduler

### 40.4 ADR-004: FFI Security Model
**Status**: Accepted  
**Decision**: Require fuzzing tests and security profiles for unsafe FFI  
**Rationale**: Ensures security requirements cannot be bypassed  
**Implementation**: Add to `once-build` crate with lint enforcement

### 40.5 ADR-005: LSP Signature Display
**Status**: Accepted  
**Decision**: Show full type signatures with effects and linearity in IDE  
**Rationale**: Makes complex type information visible without explicit commands  
**Implementation**: Extend `once-lsp` crate with enhanced hover information

---

## 41) Testing Strategy Updates

### 41.1 Property-Based Testing
- **Linear Resource Tests**: Generate programs with various resource usage patterns
- **Region Inference Tests**: Verify no memory leaks in generated region plans
- **Effect Propagation Tests**: Ensure effects are correctly inferred and propagated

### 41.2 Concurrency Testing
- **Deterministic Scheduler Tests**: Verify reproducible execution traces
- **Deadlock Detection Tests**: Ensure cycle detection works correctly
- **Backpressure Tests**: Verify channel behavior under load

### 41.3 FFI Testing
- **Security Profile Tests**: Verify unsafe FFI requirements are enforced
- **Fuzzing Integration Tests**: Ensure fuzz harnesses are generated and run
- **Component Boundary Tests**: Verify Wasm Component isolation

### 41.4 User Experience Testing
- **Error Message Quality**: Verify error messages are actionable and helpful
- **LSP Responsiveness**: Ensure IDE features work smoothly
- **Build System Reliability**: Verify hermetic builds are reproducible

---

## 42) Documentation Plan

### 42.1 User Documentation
- **Learning Guide**: Step-by-step introduction to Once concepts
- **Resource Management**: Comprehensive guide to linear types and `using`
- **Concurrency Guide**: Patterns for channels, actors, and async programming
- **FFI Guide**: Safe interop with other languages

### 42.2 Developer Documentation
- **Compiler Internals**: Architecture and implementation details
- **Contributing Guide**: How to add new language features
- **Testing Guide**: How to write effective tests for the compiler

### 42.3 API Documentation
- **Standard Library**: Complete API reference with examples
- **Runtime APIs**: Scheduler, channels, and system integration
- **Build System**: Configuration and deployment options

---

## 43) Success Metrics

### 43.1 Technical Metrics
- **Compilation Speed**: Target < 1s for typical programs
- **Memory Safety**: Zero memory safety violations in test suite
- **Resource Safety**: Zero resource leaks in test suite
- **Concurrency Safety**: Zero data races in test suite

### 43.2 User Experience Metrics
- **Error Message Quality**: Users can fix 90% of errors without documentation
- **Learning Curve**: New users productive within 1 hour
- **IDE Responsiveness**: LSP responds within 100ms
- **Build Reliability**: 99.9% reproducible builds

### 43.3 Adoption Metrics
- **Community Growth**: Active contributors and users
- **Ecosystem Health**: Third-party libraries and tools
- **Industry Adoption**: Production usage in real projects
- **Academic Interest**: Research papers and citations

---

## 44) Risk Mitigation

### 44.1 Technical Risks
- **Region Inference Complexity**: Provide clear fallback paths and diagnostics
- **Performance Overhead**: Continuous benchmarking and optimization
- **Compatibility Issues**: Comprehensive test suite and migration tools

### 44.2 Adoption Risks
- **Learning Curve**: Excellent documentation and tooling
- **Ecosystem Maturity**: Focus on core use cases first
- **Competition**: Clear differentiation and value proposition

### 44.3 Project Risks
- **Scope Creep**: Strict adherence to minimal spec
- **Resource Constraints**: Prioritize high-impact features
- **Timeline Pressure**: Realistic milestones and buffer time

---

## 45) Conclusion

The Once language project represents a significant advancement in systems programming language design, combining the best ideas from modern languages while addressing their limitations. The feedback from experts has been invaluable in identifying areas for improvement and ensuring the language meets real-world needs.

The implementation is progressing well, with core components already functional and the remaining work clearly defined. The focus on user experience, safety, and performance positions Once as a compelling alternative to existing systems programming languages.

Success will be measured not just by technical achievements, but by the positive impact on developers' productivity and the safety of the systems they build. The Once language has the potential to become the go-to choice for systems programming in the 21st century.

---

*This document will be updated as the project evolves and new insights are gained from implementation and user feedback.*
