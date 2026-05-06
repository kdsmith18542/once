# Once Compiler Blueprint

**Version:** v0.1 Draft  
**Audience:** compiler engineers, runtime engineers, tooling/LSP engineers  
**Goal:** implementation blueprint for the Once programming language compiler, runtime, and developer tooling.

---

## 0. Compiler Mission

Once is a general-purpose systems language built around:

- inferred region-based memory management,
- linear/affine resource safety,
- inferred but visible effects,
- communication-first concurrency,
- deterministic builds,
- strong import/type ergonomics,
- agentic/LLM-friendly compiler APIs.

The compiler must make the language feel simple while internally handling advanced safety machinery.

Priority order:

1. Correctness and soundness
2. Excellent diagnostics
3. Deterministic compilation
4. Incremental and tool-friendly architecture
5. Good generated performance
6. Backend flexibility

---

## 1. High-Level Compiler Architecture

```text
Source .onc
  |
  v
Lexer
  |
  v
Parser
  |
  v
AST
  |
  v
Name Resolution + Import Resolution
  |
  v
HIR
  |
  v
Type Inference
  |
  v
Effect Inference
  |
  v
Linearity / Ownership Checking
  |
  v
Closure Capture Analysis
  |
  v
Region Constraint Generation
  |
  v
Region Inference Solver
  |
  v
TIR / RIR
  |
  v
MIR Lowering
  |
  v
MIR Verification
  |
  v
Optimization Passes
  |
  v
Codegen IR
  |
  +--> Cranelift backend
  +--> LLVM backend later
  +--> Wasm Component backend later
  |
  v
Object / binary / component artifact
```

Primary compiler executable: `oncec`  
Primary build/tool executable: `once`  
Recommended bootstrap implementation language: **Rust**.

---

## 2. Repository Layout

```text
once/
├─ Cargo.toml
├─ README.md
├─ RFCs/
├─ docs/
│  ├─ language/
│  ├─ compiler/
│  ├─ runtime/
│  └─ devtools/
├─ crates/
│  ├─ once-span/              # source spans, file map, diagnostics primitives
│  ├─ once-lexer/             # tokenization
│  ├─ once-parser/            # parser -> AST
│  ├─ once-ast/               # AST definitions
│  ├─ once-resolve/           # modules/imports/name resolution
│  ├─ once-hir/               # high-level IR
│  ├─ once-types/             # type representation, unification
│  ├─ once-effects/           # effect row representation + unification
│  ├─ once-linear/            # linearity and move checking
│  ├─ once-closure/           # capture analysis
│  ├─ once-region/            # region constraints + solver
│  ├─ once-mir/               # MIR representation + verifier
│  ├─ once-opt/               # MIR optimization passes
│  ├─ once-codegen/           # backend abstraction
│  ├─ once-codegen-cranelift/
│  ├─ once-runtime/           # runtime library
│  ├─ once-std/               # standard library source
│  ├─ once-build/             # once.toml, lockfile, DAG, cache
│  ├─ once-lsp/               # LSP server
│  ├─ once-analyze/           # JSON compiler analysis output
│  ├─ once-fmt/               # formatter
│  ├─ once-cli/               # once CLI
│  └─ oncec/                  # compiler binary
├─ examples/
│  ├─ hello/
│  ├─ csv-sum/
│  ├─ http-echo/
│  ├─ actors/
│  └─ ffi-component/
└─ tests/
   ├─ parser/
   ├─ typecheck/
   ├─ linear/
   ├─ effects/
   ├─ regions/
   ├─ mir/
   ├─ codegen/
   └─ diagnostics/
```

---

## 3. Source File Model

Recommended extensions:

```text
.onc      Once source file
.sig      Once signature/interface file
.onceo    Once object/module summary artifact
once.toml package manifest
once.lock lockfile
```

Rules:

- Source files are UTF-8.
- No top-level effects.
- Top-level declarations may define imports, types, traits, impls, functions, and constants.
- Module initialization must be explicit:

```once
export fn init() -> Unit ![io, net] {
  ...
}
```

---

## 4. Lexing

### 4.1 Token Classes

The lexer must emit:

- identifiers,
- keywords,
- integer literals,
- float literals,
- decimal literals,
- string literals,
- byte literals,
- punctuation,
- operators,
- comments,
- doc comments.

### 4.2 Initial Keywords

```text
fn let var type trait impl for while if else match return
import export as using async await spawn group const unsafe ffi
where in true false
```

Some keywords may remain reserved before they are fully implemented.

### 4.3 Comments

```once
// line comment
/* block comment */
/// doc comment
```

Nested block comments are recommended.

### 4.4 Lexer Output

```rust
struct Token {
    kind: TokenKind,
    span: Span,
}
```

`Span` points into a `SourceFileId` and byte offset range.

---

## 5. Parsing

### 5.1 Parser Requirements

The parser should be:

- lossless enough for formatting/LSP,
- error-recovering,
- deterministic,
- suitable for incremental parsing later.

Recommended strategy:

- hand-written recursive descent for items/statements,
- Pratt parser for expressions,
- diagnostics-first error recovery.

### 5.2 AST Principles

AST preserves:

- user syntax,
- spans,
- comments/doc comments association,
- explicit vs inferred elements.

AST does not resolve names or infer types.

### 5.3 Expression Precedence

Precedence from low to high:

1. pipeline `|>`
2. boolean OR
3. boolean AND
4. equality
5. comparison
6. additive
7. multiplicative
8. unary
9. call/index/member access

Pipeline is left associative.

Recommended rule:

```once
x |> f
```

means:

```once
f(x)
```

and:

```once
x |> f(a, b)
```

means:

```once
f(x, a, b)
```

---

## 6. Name and Module Resolution

### 6.1 Import Rules

Once avoids common import pain:

- absolute imports only,
- no wildcard imports,
- no environment search paths,
- no relative imports,
- lockfile-pinned dependency resolution,
- explicit re-exports.

Examples:

```once
import std::io::{File, Reader}
import http@3 as http3
export import http@3::client as http
```

### 6.2 Resolution Roots

Only two roots:

1. current package,
2. lockfile-pinned dependencies.

No ambient package search.

### 6.3 Namespace Model

Use one unified namespace for:

- types,
- values,
- traits,
- modules.

Ambiguity is resolved with explicit aliasing.

### 6.4 Visibility

Private by default.

```once
export fn run() -> Unit {
  ...
}

export type User = { id: Int, name: Text }
```

### 6.5 Cycles

No cross-package cycles.

Within a package:

- `.sig` files may declare interfaces,
- `.onc` files implement them.

This permits controlled mutual recursion without hidden initialization effects.

---

## 7. HIR: High-Level Intermediate Representation

HIR is the first semantically meaningful IR.

HIR contains:

- resolved names,
- canonical modules,
- desugared simple syntax,
- explicit scopes,
- unresolved type variables,
- unresolved effect variables,
- source spans retained.

### 7.1 HIR Example

Source:

```once
fn add(x: Int, y: Int) -> Int {
  x + y
}
```

HIR rough form:

```text
Fn {
  id: DefId,
  name: "add",
  params: [
    Param { name: x, ty: Int },
    Param { name: y, ty: Int }
  ],
  ret: Int,
  effects: InferredOrExplicit,
  body: Block([...])
}
```

### 7.2 Desugarings into HIR

Desugar:

- pipeline into function calls,
- `using` into protected resource-consumption form,
- `async` into task creation form,
- `await` into task-consumption form,
- `group` into structured concurrency scope.

Do not erase too much diagnostic context. Keep original syntax markers for fix-its.

---

## 8. Type System

### 8.1 Type Features

v1 type system includes:

- primitive types,
- records,
- ADTs,
- tuples,
- functions,
- generics,
- traits,
- `Option<T>`,
- `Result<T, E>`,
- `Task<T>`,
- `Chan<T>`,
- `Slice<T>`,
- `SliceMut<T>`,
- `Atomic<T>`,
- region-internal references where needed,
- linear/affine type qualifiers.

### 8.2 Type Representation

```rust
enum TyKind {
    Error,
    Unit,
    Bool,
    Int(IntKind),
    Float(FloatKind),
    Decimal,
    Text,
    Bytes,
    Tuple(Vec<Ty>),
    Record(RecordId),
    Adt(AdtId, Vec<Ty>),
    Fn(FnSig),
    Generic(Symbol),
    Infer(TyVarId),
    App(TypeCtorId, Vec<Ty>),
    Linear(Ty),
    Affine(Ty),
    Slice(Ty),
    SliceMut(Ty),
    Task(Ty),
    Chan(Ty),
    Atomic(Ty),
}
```

### 8.3 Inference Strategy

Use Hindley-Milner-inspired inference with constraints.

Important simplification:

- exported public APIs require explicit signatures,
- local inference is allowed everywhere.

This improves:

- compilation stability,
- diagnostics,
- package semver,
- agentic code editing.

### 8.4 Public API Rule

Exported functions must include:

- parameter types,
- return type,
- effect row.

Example:

```once
export fn fetch(url: Text) -> Result<Bytes, Err> ![net, io, time] {
  ...
}
```

Tooling may infer and insert:

```text
once fix --public-sigs
```

### 8.5 Trait System

Once uses traits/interfaces rather than inheritance.

```once
trait Show {
  fn show(self) -> Text
}
```

```once
impl Show for User {
  fn show(self) -> Text {
    self.name
  }
}
```

Recommended implementation:

- dictionary passing in MIR,
- monomorphization later for performance-critical paths,
- avoid specialization in v1.

### 8.6 Copy Trait

Implicit duplication is allowed only for `Copy`.

```once
derive Copy for Point
```

Non-`Copy` values move by default.

Linear values cannot be copied unless a safe explicit split/clone API exists.

---

## 9. Effect System

### 9.1 Effect Rows

Initial effects:

```text
io net time spawn ffi[name] gpu fs env random
```

Effects are inferred locally but visible in public APIs, hovers, and JSON analysis output.

Example:

```once
fn read(path: Text) -> Result<Text, Err> ![io, fs]
```

### 9.2 Representation

```rust
enum Effect {
    Io,
    Net,
    Time,
    Spawn,
    Fs,
    Env,
    Random,
    Gpu,
    Ffi(Symbol),
    Var(EffectVarId),
}

struct EffectRow {
    effects: BTreeSet<Effect>,
    tail: Option<EffectVarId>,
}
```

### 9.3 Effect Inference

Primitive operations contribute effects:

```text
File.open     -> fs, io
Tcp.connect   -> net, io, time
spawn         -> spawn
await         -> time
unsafe ffi    -> ffi[name]
gpu kernel    -> gpu
```

Function effects are union of body effects.

### 9.4 Capability Checking

Package manifest declares allowed effects/capabilities.

```toml
[capabilities]
io = true
spawn = true

[capabilities.net]
egress = ["*.example.com:443"]
```

The compiler/build tool rejects undeclared capability use.

---

## 10. Linearity and Ownership Checking

### 10.1 Core Rule

A linear value must be consumed exactly once.

A non-copy, non-linear value moves when passed, assigned, returned, or captured.

A `Copy` value may be duplicated.

### 10.2 Environments

Use two environments:

```text
Γ = ordinary values
Δ = linear values
```

When a linear value is used, remove it from `Δ`.

### 10.3 Consumption

A linear value is consumed by:

- `consume(self)`,
- `close(self)`,
- `commit(self)`,
- `await task`,
- `join task`,
- `cancel task`,
- moving into a returned value,
- moving into a channel send,
- moving into a closure.

### 10.4 Diagnostics

Linearity errors must include the **linearity chain**:

1. origin,
2. transfers,
3. consumption,
4. illegal reuse.

Example diagnostic:

```text
error[E-LIN-002]: linear value `f` used after it was consumed

  src/main.onc:4:10
    let f = File.open(path)
        - origin: `f` created here

  src/main.onc:5:3
    f.close()
    --------- consumed here

  src/main.onc:6:3
    f.write("again")
    ^ illegal reuse here

help: remove the second use, or move `f.close()` after the final write
```

---

## 11. Resource System

### 11.1 Resource Trait

```once
trait Resource {
  fn consume(self) -> Unit !ε
}
```

All standard linear resources implement it:

- `File`,
- `TcpStream`,
- `Txn`,
- `Task<T>`,
- `Deadline`,
- `GpuBuffer<T>`.

### 11.2 using Desugaring

Source:

```once
using f = File.open(path)? {
  f.write("hello")?
}
```

HIR-level protected form:

```text
Using {
  binding: f,
  init: File.open(path)?,
  body: ...,
  consume: Resource::consume(f)
}
```

MIR lowering must guarantee consumption on:

- normal exit,
- early return,
- error propagation,
- panic/unwind if Once supports unwinding.

Recommendation:

- no general exceptions/unwinding in v1,
- use `Result` + `?`,
- define panic as abort or controlled failure profile.

---

## 12. Closure Capture Analysis

### 12.1 Capture Modes

Capture by:

- copy for `Copy`,
- immutable borrow/view for non-escaping immutable values,
- move for non-`Copy`,
- move for all linear values.

### 12.2 Linear Closure Rule

If a closure captures a linear value, the closure itself is linear and implements `FnOnce`.

It can be:

- called once,
- sent to another task if captured values are `Send`,
- stored as a linear value.

### 12.3 Region Rule

Captured moved values escape:

```text
escapes(v, region_enclosing -> region_closure)
```

Async closures extend closure region to task completion.

---

## 13. Region Inference

### 13.1 Goal

Infer memory regions without user lifetime annotations.

The compiler inserts:

- region creation,
- region allocation,
- region free,
- moves to outer/caller regions,
- fallback boxes/reference counting where necessary.

### 13.2 Constraint Kinds

```rust
enum RegionConstraint {
    AllocIn { value: ValueId, region: RegionVar },
    Outlives { longer: RegionVar, shorter: RegionVar },
    Escapes { value: ValueId, from: RegionVar, to: RegionVar },
    LastUse { value: ValueId, point: ProgramPoint },
    FreeAfter { region: RegionVar, point: ProgramPoint },
    NoRc { span: Span },
}
```

### 13.3 Generation Rules

Allocation:

```text
let x = new T
=> AllocIn(x, fresh_region)
```

Return:

```text
return x
=> Escapes(x, current_region, caller_region)
```

Closure capture:

```text
capture x
=> Escapes(x, current_region, closure_region)
```

Channel send:

```text
chan.send(x)
=> Escapes(x, current_region, receiver_region_or_task_region)
```

### 13.4 Solver Algorithm

Simplified v1 solver:

1. Build control-flow graph.
2. Compute liveness for values.
3. Generate region variables by lexical scope.
4. Add escape constraints.
5. Merge compatible regions.
6. Place free at earliest post-dominator after last use.
7. Validate no value is used after region free.
8. If unsatisfied:
   - suggest `box<T>`,
   - suggest `rc<T>`,
   - or fail under `@no_rc`.

### 13.5 Soundness Requirements

The solver must prove:

- no pointer/reference outlives its region,
- no linear resource is freed/consumed twice,
- captured values outlive closure invocation,
- async task data outlives task completion,
- channel-sent moved values are no longer available to sender.

### 13.6 Explain Output

```text
once explain --regions src/main.onc
```

Should render:

- region graph,
- allocation sites,
- free points,
- escapes,
- fallback boxes/rc,
- estimated memory waste.

Machine-readable:

```text
once analyze --regions --json
```

---

## 14. Bounds and Size Types

### 14.1 Goal

Prevent array/slice bounds errors while erasing checks when proof is simple.

### 14.2 Types

```once
Array<T, N>
Slice<T>
SliceMut<T>
Vec<T>
```

Each carries a length fact.

### 14.3 Constraint Domain

Use lightweight linear arithmetic:

- equalities,
- inequalities,
- addition/subtraction with constants,
- range facts from loops and guards.

Avoid full SMT in v1.

### 14.4 Example

```once
if i < xs.len {
  xs[i] // check erased
}
```

Constraint:

```text
0 <= i && i < len(xs)
```

### 14.5 Fallback

If proof fails:

- emit runtime check,
- diagnostic in optimization report,
- no unsafety.

---

## 15. Async and Structured Concurrency

### 15.1 Task Type

```once
Task<T>
```

`Task<T>` is linear.

Must be consumed by:

- `await`,
- `join`,
- `cancel`.

### 15.2 async Desugaring

Source:

```once
let t = async { compute() }
```

HIR:

```text
TaskCreate {
  body: compute(),
  captures: ...
}
```

### 15.3 group Desugaring

Source:

```once
group(policy=FailFast) {
  let a = spawn { fa() }
  let b = spawn { fb() }
  await a?
  await b?
}
```

Lower to structured runtime nursery:

```text
nursery_begin(policy)
spawn_child(...)
spawn_child(...)
await_child(...)
nursery_end_join_or_cancel_all()
```

### 15.4 No Orphan Tasks

Compiler/runtime must ensure tasks created in a group are resolved before exit.

Standalone tasks are still linear, so caller must consume them.

---

## 16. Channels and Actors

### 16.1 Channels

Explicit backpressure:

```once
let c = Chan::new(cap=1024, policy=Backpressure::Block)
```

Policies:

- `Block`,
- `DropOldest`,
- `DropNewest`,
- `Error`.

### 16.2 Channel Type Semantics

Sending a linear value moves ownership.

Sending immutable `Copy` or shareable values duplicates/sends safely.

### 16.3 Actor Model

Actors own internal mutable state.

External code interacts by messages.

Actor state is not directly shareable.

### 16.4 Debug Runtime

Maintain wait-for graph:

- task waits for task,
- task waits for channel send,
- task waits for channel receive,
- actor waits for mailbox.

Detect cycles in deterministic mode.

---

## 17. MIR: Middle Intermediate Representation

### 17.1 MIR Purpose

MIR is the primary compiler workhorse after semantic analysis.

It should make explicit:

- control flow,
- moves,
- drops,
- resource consumption,
- region allocation/free,
- effects,
- bounds checks,
- task/channel operations.

### 17.2 MIR Structure

```rust
struct MirFunction {
    id: DefId,
    sig: MirSignature,
    locals: Vec<LocalDecl>,
    blocks: Vec<BasicBlock>,
}

struct BasicBlock {
    id: BlockId,
    statements: Vec<Statement>,
    terminator: Terminator,
}
```

### 17.3 Statements

```rust
enum Statement {
    Assign(Place, Rvalue),
    Move(Place, Place),
    Consume(Place, ConsumeKind),
    RegionAlloc { place: Place, region: RegionId, layout: Layout },
    RegionFree { region: RegionId },
    BoundsAssert { index: Operand, len: Operand, proven: bool },
    EffectMark(Effect),
    StorageLive(LocalId),
    StorageDead(LocalId),
}
```

### 17.4 Terminators

```rust
enum Terminator {
    Return,
    Goto(BlockId),
    SwitchInt { discr: Operand, targets: Vec<(Const, BlockId)>, otherwise: BlockId },
    Call { func: Operand, args: Vec<Operand>, destination: Place, target: BlockId },
    TailCall { func: Operand, args: Vec<Operand> },
    Await { task: Operand, destination: Place, target: BlockId },
    Spawn { closure: Operand, destination: Place, target: BlockId },
    Panic { message: Operand },
    Unreachable,
}
```

### 17.5 MIR Verifier

Must verify:

- all linear locals consumed exactly once,
- no use after move,
- no region free before last use,
- every block terminates,
- every effect is present in function signature,
- every bounds assertion either proven or emits runtime check,
- no undeclared capability use.

---

## 18. Optimization Passes

Initial MIR optimizations:

1. constant folding,
2. copy propagation for `Copy` values,
3. dead code elimination,
4. dead store elimination,
5. bounds check elimination,
6. region free sinking/hoisting when safe,
7. function inlining,
8. monomorphization,
9. simple loop invariant code motion,
10. async state machine simplification.

Avoid advanced optimizations until MIR verifier is mature.

---

## 19. Code Generation

### 19.1 Backend Abstraction

```rust
trait Backend {
    fn compile_module(&mut self, module: MirModule) -> Result<ObjectArtifact>;
}
```

### 19.2 Cranelift Backend v1

Use Cranelift first for:

- faster compiler iteration,
- simpler integration,
- good enough performance.

LLVM can be added later for:

- more mature optimization,
- platform support,
- LTO.

### 19.3 Runtime ABI

Define stable internal ABI for:

- region allocation/free,
- task spawn/await/cancel,
- channel send/recv,
- panic/fail,
- component calls.

Example runtime calls:

```text
once_region_new(size_hint) -> RegionHandle
once_region_alloc(region, layout) -> Ptr
once_region_free(region)

once_task_spawn(fn_ptr, env_ptr) -> TaskHandle
once_task_await(task) -> ResultValue
once_task_cancel(task)

once_chan_send(chan, value_ptr)
once_chan_recv(chan) -> value
```

---

## 20. Runtime Design

### 20.1 Runtime Responsibilities

- scheduler,
- tasks,
- nurseries/groups,
- channels,
- timers,
- cancellation,
- debug wait-for graph,
- region allocator support,
- panic/failure handling,
- capability enforcement hooks.

### 20.2 Scheduler

v1:

- cooperative work-stealing scheduler,
- deterministic mode for tests,
- task-local arenas,
- cancellation checkpoints.

### 20.3 Deterministic Mode

Used for:

```text
once test --deterministic
```

Must control:

- task scheduling,
- timers,
- pseudo-randomness when using deterministic RNG,
- IO stubs/mocks where possible.

### 20.4 Deadlock Detection

Debug runtime tracks wait-for graph.

If cycle:

```text
Deadlock:
  task A awaiting recv on channel C
  task B awaiting join task A
  task A waits for B
```

Output source trace if debug info exists.

---

## 21. FFI and Components

### 21.1 Preferred FFI

Wasm Component Model.

Benefits:

- memory isolation,
- capability boundary,
- safer cross-language integration.

### 21.2 In-Process Unsafe FFI

Syntax:

```once
unsafe ffi "libsqlite3" {
  ...
}
```

Requirements:

- `![ffi[libsqlite3]]` effect,
- fuzz harness,
- `[profile.security].ffi_safe = true`,
- explicit marshalling.

Build fails without these.

### 21.3 PCC-Lite Metadata

For component/integration boundaries, emit:

- memory layout hashes,
- aliasing contracts,
- bounds summaries,
- effect/capability summary,
- ABI version.

---

## 22. Build System

### 22.1 once.toml

```toml
[package]
name = "app"
version = "0.1.0"

[deps]
http = "3.0.0"

[capabilities]
io = true
spawn = true

[capabilities.net]
egress = ["*.example.com:443"]

[profile.release]
opt-level = 3
deterministic-float = false

[profile.security]
ffi_safe = false
```

### 22.2 Build DAG

Computed from:

- imports,
- manifest deps,
- lockfile,
- feature flags,
- capabilities.

No Turing-complete build scripts in v1.

### 22.3 Reproducibility

Build must scrub:

- timestamps,
- absolute paths,
- host-specific nondeterminism.

Artifacts include:

- compiler version,
- lockfile hash,
- source hash,
- capability hash,
- public API/effect hash.

---

## 23. Diagnostics System

### 23.1 Diagnostic Requirements

Each diagnostic has:

- stable error code,
- primary span,
- secondary spans,
- explanation,
- machine-applicable fix if possible,
- JSON form.

### 23.2 Example JSON Diagnostic

```json
{
  "code": "E-LIN-002",
  "severity": "error",
  "message": "linear value used after consumption",
  "primary_span": { "file": "src/main.onc", "start": 120, "end": 121 },
  "notes": [
    { "message": "created here", "span": { "start": 80, "end": 90 } },
    { "message": "consumed here", "span": { "start": 100, "end": 110 } }
  ],
  "fixes": [
    { "title": "move close after final use", "edits": [] }
  ]
}
```

---

## 24. LSP and Agentic Tooling

### 24.1 LSP Required Features

- go to definition,
- hover resolved signature,
- effect row display,
- linearity display,
- imports quick-fix,
- public signature insertion,
- rename,
- format,
- organize imports,
- explain regions/effects/linearity inline.

### 24.2 Agent-Friendly Compiler APIs

Commands:

```text
once analyze --json
once analyze --hir --json
once analyze --mir --json
once analyze --effects --json
once analyze --regions --json
once fix --json
```

### 24.3 Stable Edit Protocol

The compiler/LSP should emit patch sets:

```json
{
  "edits": [
    {
      "file": "src/main.onc",
      "range": { "start": 100, "end": 100 },
      "replacement": "import std::io::File\n"
    }
  ]
}
```

This helps coding agents apply precise fixes safely.

---

## 25. Formatter

Formatter rules:

- deterministic,
- no configuration in v1 or minimal config only,
- stable import ordering,
- stable public signature formatting,
- consistent match layout.

Command:

```text
once fmt
```

Formatter should be idempotent.

---

## 26. Testing Strategy

### 26.1 Compiler Tests

- lexer golden tests,
- parser golden tests,
- AST roundtrip tests,
- type inference tests,
- effect inference tests,
- linearity tests,
- closure capture tests,
- region solver property tests,
- MIR verifier tests,
- codegen execution tests,
- diagnostics snapshot tests.

### 26.2 Runtime Tests

- task scheduling,
- cancellation,
- group policies,
- channel backpressure,
- deadlock detection,
- deterministic scheduler replay.

### 26.3 Property Tests

Region solver property:

- no free before last use,
- no leaked linear value,
- no double consumption.

Linearity property:

- every generated valid program consumes linear values once.

### 26.4 End-to-End Examples

Required examples:

- hello world,
- file read/write,
- CSV sum,
- HTTP echo,
- actor counter,
- channel pipeline,
- unsafe FFI wrapper,
- Wasm component import,
- ndarray example.

---

## 27. Incremental Compilation

Not required in initial prototype, but architecture should prepare for it.

Persistent keys:

- `DefId`,
- `ModuleId`,
- `TypeId`,
- `EffectId`,
- `RegionSummaryId`.

Cache:

- parsed AST,
- HIR,
- type summaries,
- effect summaries,
- region summaries,
- object artifacts.

Invalidation:

- source hash,
- dependency public API hash,
- effect hash,
- capability hash.

---

## 28. Milestone Plan

### M0: Design Lock

Deliver:

- grammar draft,
- AST definitions,
- HIR definitions,
- diagnostic format,
- once.toml schema.

### M1: Lexer + Parser

Deliver:

- tokenization,
- parser,
- AST golden tests,
- formatter skeleton.

### M2: Name Resolution + HIR

Deliver:

- modules/imports,
- DefId system,
- HIR lowering,
- no wildcard imports,
- explicit exports.

### M3: Type Inference

Deliver:

- primitive types,
- records,
- ADTs,
- functions,
- generics,
- basic traits,
- public signature enforcement.

### M4: Effect Inference

Deliver:

- effect rows,
- primitive effects,
- capability checking,
- public effect signatures.

### M5: Linearity Checker

Deliver:

- move/consume checking,
- `Copy`,
- `Resource`,
- `using`,
- `Task<T>` linearity,
- diagnostics chain.

### M6: Closure Capture + Async

Deliver:

- closure capture modes,
- `Fn`/`FnOnce`,
- async lowering,
- group/nursery lowering.

### M7: Region Inference

Deliver:

- region constraints,
- solver,
- MIR explicit region alloc/free,
- region explain output.

### M8: MIR + Verifier

Deliver:

- MIR representation,
- verifier,
- bounds checks,
- size facts.

### M9: Runtime Prototype

Deliver:

- task scheduler,
- channels,
- timers,
- cancellation,
- deterministic scheduler.

### M10: Cranelift Codegen

Deliver:

- executable binaries,
- stdlib minimum,
- examples running.

### M11: Build Tool + Lockfile

Deliver:

- `once build`,
- manifest parser,
- dependency DAG,
- reproducibility hash.

### M12: LSP + Agent Tooling

Deliver:

- hover,
- diagnostics,
- quick fixes,
- `once analyze --json`.

---

## 29. Initial Implementation Priorities

Do not try to implement everything at once.

Recommended prototype slice:

1. `fn`, `let`, `var`, `Int`, `Bool`, `Text`
2. records and ADTs
3. basic type inference
4. `Result` and `?`
5. linear `File`
6. `using`
7. simple region allocation/free
8. Cranelift hello-world
9. CSV sum example

Only after this:

- async,
- actors,
- channels,
- FFI,
- SIMD,
- GPU,
- AI libs.

---

## 30. Critical Soundness Questions to Resolve

Before production:

1. Are `using` blocks guaranteed to consume on all control-flow exits?
2. Can linear values be hidden inside non-linear containers?
3. Can closures accidentally duplicate linear captures?
4. Can async tasks outlive captured regions?
5. Can channel sends leak mutable aliases?
6. Can `rc<T>` undermine mutable XOR shared?
7. Are region free points valid under all branches?
8. Are effect rows complete after desugaring?
9. Does unsafe FFI quarantine prevent unsound imports?
10. Can public API inference drift between compiler versions?

These should become formal compiler test suites.

---

## 31. Recommended Engineering Standards

- All compiler passes must be deterministic.
- All diagnostics must have stable codes.
- All IRs must support JSON dump.
- MIR verifier runs after every MIR-transform pass in debug compiler builds.
- Fuzz the parser and MIR verifier.
- Snapshot test diagnostics.
- Keep v1 syntax small.
- Avoid macros in v1.
- Avoid subtyping.
- Prefer compiler fix-its over clever syntax.

---

## 32. Appendix: Example Once Program

```once
import std::io::File

export fn sum_file(path: Text) -> Result<Int, Err> ![io, fs] {
  using f = File.open(path)? {
    var total = 0

    for line in f.lines() {
      total = total + parse_int(line)?
    }

    Ok(total)
  }
}
```

Expected compiler behavior:

- `File.open` creates linear `File`.
- `using` guarantees `Resource::consume`.
- `f.lines()` contributes `io`.
- `parse_int` returns `Result`.
- `?` propagates errors.
- effect row is `[io, fs]`.
- local variables are inferred.
- public signature is explicit.
- no GC allocation required for loop body except region-local temporary text.

---

## 33. Final Notes

The Once compiler should not merely compile code. It should explain code.

The differentiator is not only memory safety or performance; it is the combination of:

- invisible complexity for humans,
- visible semantics for tools,
- deterministic artifacts for teams,
- strong boundaries for security,
- machine-readable compiler intelligence for coding agents.

That is the core compiler vision.
