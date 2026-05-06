# **ONCE Specification – Part 3: Type System & Effects**

| Document ID | ONCE-003 |
| :---- | :---- |
| **Title** | Type System & Effects |
| **Version** | 1.1 |
| **Date** | 2026-05-03 |
| **Status** | Draft |
| **Supersedes** | ONCE-003 v1.0 |
| **Related Docs** | ONCE-002, ONCE-004, ONCE-009 |

## **1. Introduction**

This document specifies the semantic rules of the Once static type system. A primary goal of Once is to provide strong, verifiable safety guarantees at compile time. This is achieved through a combination of a powerful Hindley-Milner-style type system, a strict linearity discipline for resource management, and a novel, transparent effect system for managing side effects.

This specification assumes familiarity with the syntax defined in ONCE-002.

## **2. Core Type System**

### **2.1. Primitive Types**

Once provides a set of built-in primitive types:

- **Int**: A signed, machine-width integer (i64 on 64-bit systems, i32 on 32-bit).
- **Float**: A 64-bit floating-point number (IEEE 754).
- **Bool**: A boolean value, either `true` or `false`.
- **Str**: An immutable, UTF-8 encoded string.
- **()**: The "unit" type, which has a single value `()` and is used to represent the absence of a meaningful value.

### **2.2. Composite Types**

- **Records:** Product types defined with curly braces. Collections of named fields.
  ```
  type Point = { x: Float, y: Float };
  ```

- **Enums:** Sum types (ADTs) that define a type with a set of possible variants.
  ```
  type Option<T> = Some(T) | None;
  ```

- **Tuples:** Anonymous product types. `(T1, T2, ...)`
- **Arrays:** Fixed-size, stack-allocated collections. `[T; N]` where `N` is a compile-time constant.
- **Vectors:** Growable, heap-allocated collections. `Vec<T>`.

### **2.3. Type Inference**

Once employs full type inference for local bindings within function bodies. Developers are not required to annotate the types of `let` or `var` bindings. However, top-level function signatures (parameters and return types) **must** be explicitly annotated for clarity and to guarantee stable public APIs.

- **Type Holes (`_`):** A developer can use `_` in place of an expression to ask the compiler what type is expected at that position. The compiler will report the inferred type as an error, guiding development.

## **3. Generics & Traits**

### **3.1. Generics (Parametric Polymorphism)**

Types and functions can be parameterized by one or more type variables, which are specified in angle brackets.

```
fn wrap_in_some<T>(value: T) -> Option<T> {
  Some(value)
}

type Result<T, E> = Ok(T) | Err(E);
```

### **3.2. Traits (Ad-hoc Polymorphism)**

Traits define a set of methods that a type must implement. They are Once's mechanism for abstracting over behavior.

```
trait_decl ::= [ "export" ] "trait" ident [ type_params ] "{" { fn_signature } "}"
fn_signature ::= "fn" ident "(" [ param_list ] ")" [ "->" type_expr ] ";"

impl_decl ::= "impl" [ type_params ] ident [ "<" ... ">" ] "for" ident [ "<" ... ">" ] "{" { fn_decl } "}"
```

*Example:*
```
trait Show {
  fn show(self) -> Str;
}

impl Show for Int {
  fn show(self) -> Str {
    // ... implementation for converting Int to Str
  }
}
```

## **4. Formal Typing Rules**

The typing judgement has the form:

```
Γ; Δ; Σ ⊢ e : τ ! ε
```

Where:
- **Γ** (gamma): ordinary typing environment, mapping variable names to type schemes `∀ᾱ. τ` for non-linear bindings
- **Δ** (delta): linear typing environment, tracking linear bindings and their availability
- **Σ** (sigma): effect row variable environment
- **e**: expression being typed
- **τ**: the type of `e`
- **ε**: the effect row of `e`

### **4.1. T-Var — Variable Reference**

```
x : σ ∈ Γ     instantiate(σ) = τ
─────────────────────────────────
Γ; Δ; Σ ⊢ x : τ ! ∅
```

### **4.2. T-Lit — Literal**

```
lit ∈ { Int, Float, Bool, Str, Unit }
──────────────────────────────────────
Γ; Δ; Σ ⊢ lit : typeof(lit) ! ∅
```

Where `typeof(n: Int) = Int`, `typeof(true) = Bool`, etc.

### **4.3. T-Lam — Lambda Abstraction**

```
Γ, x:τ₁; Δ; Σ ⊢ e : τ₂ ! ε     ε ⊆ {spawn}   -- closures may spawn
────────────────────────────────────────────────────
Γ; Δ; Σ ⊢ |x| => e : fn(τ₁) -> τ₂ ! ε
```

Multi-parameter lambdas generalize to `|x₁, ..., xₙ| => e : fn(τ₁, ..., τₙ) -> τ`.

### **4.4. T-App — Application**

```
Γ; Δ; Σ ⊢ e₁ : fn(τ₁) -> τ₂ ! ε₁
Γ; Δ; Σ ⊢ e₂ : τ₁ ! ε₂
RowUnion(ε₁, ε₂) = ε
──────────────────────────────
Γ; Δ; Σ ⊢ e₁(e₂) : τ₂ ! ε
```

### **4.5. T-Let — Let Binding (Ordinary, Immutable)**

```
Γ; Δ; Σ ⊢ e₁ : τ₁ ! ε₁
Γ, x:gen(τ₁, ε₁); Δ; Σ ⊢ e₂ : τ₂ ! ε₂
RowUnion(ε₁, ε₂) = ε
──────────────────────────────────────
Γ; Δ; Σ ⊢ let x = e₁; e₂ : τ₂ ! ε
```

### **4.6. T-Let (Linear) — Let Binding for Linear Values**

```
Γ; Δ; Σ ⊢ e₁ : lin τ₁ ! ε₁
Γ; Δ, x:lin τ₁; Σ ⊢ e₂ : τ₂ ! ε₂     x ∉ dom(Δ)
RowUnion(ε₁, ε₂) = ε
──────────────────────────────────────────
Γ; Δ; Σ ⊢ let x = e₁; e₂ : τ₂ ! ε
```

### **4.7. T-If — Conditional**

```
Γ; Δ; Σ ⊢ e_cond : Bool ! ε_cond
Γ; Δ; Σ ⊢ e_then : τ ! ε_then
Γ; Δ; Σ ⊢ e_else : τ ! ε_else
RowUnion(ε_cond, ε_then, ε_else) = ε
──────────────────────────────────────
Γ; Δ; Σ ⊢ if e_cond { e_then } else { e_else } : τ ! ε
```

When no `else` branch is present, `τ` defaults to `()`.

### **4.8. T-Match — Pattern Match**

```
Γ; Δ; Σ ⊢ e_scrut : τ_scrut ! ε_scrut
For each arm i: Γ; Δ; Σ ⊢ e_body_i : τ_body_i ! ε_body_i
All τ_body_i unify to τ_result
RowUnion(ε_scrut, ε_body_1, ..., ε_body_n) = ε
──────────────────────────────────────────────────
Γ; Δ; Σ ⊢ match e_scrut { arms } : τ_result ! ε
```

### **4.9. T-Spawn — Task Spawning**

```
Γ; Δ; Σ ⊢ e : τ ! ε_body     (no linear vars escape into task)
──────────────────────────────────────────────────
Γ; Δ; Σ ⊢ spawn { e } : Task<τ> ! ε_body ∪ {spawn}
```

### **4.10. T-Await — Awaiting a Task**

```
Γ; Δ; Σ ⊢ e_task : Task<τ> ! ε_task
───────────────────────────────────────────
Γ; Δ; Σ ⊢ await e_task : τ ! ε_task ∪ {time}
```

### **4.11. T-Return — Return**

```
Γ; Δ; Σ ⊢ e : τ ! ε
──────────────────────
Γ; Δ; Σ ⊢ return e : !τ ! ε
```

## **5. Effect Row Unification**

### **5.1. Effect Row Representation**

An effect row ε is a set of effect kinds: `ε ⊆ {io, net, spawn, time, ffi, nondet, ...}`.

Effect rows are represented as row-polymorphic types. A row variable `ρ` can be unified with concrete effects.

### **5.2. RowUnion**

The `RowUnion` operation combines effect rows:

```
RowUnion(ε₁, ε₂) = ε₁ ∪ ε₂
```

Generalized to n-ary: `RowUnion(ε₁, ..., εₙ) = ⋃ᵢ εᵢ`

### **5.3. RowDiff**

For effect subtraction (used in `using` desugaring):

```
RowDiff(ε₁, ε₂) = ε₁ \ ε₂
```

### **5.4. RowSubset**

Capability checking: verify that a function's effects are a subset of the allowed capabilities:

```
RowSubset(ε_fn, ε_caps) ⇔ ε_fn ⊆ ε_caps
```

Build fails with `CapabilityError` if `RowSubset` is false.

### **5.5. Row Variable Unification Algorithm**

```
function unify_rows(ρ₁, ρ₂):
    if ρ₁ is a row variable:
        substitute(ρ₁, ρ₂)
    elif ρ₂ is a row variable:
        substitute(ρ₂, ρ₁)
    else:
        for each effect e in ρ₁:
            if e ∉ ρ₂: raise RowMismatchError
```

## **6. Generalization & Instantiation**

### **6.1. Generalization — gen(τ, ε)**

```
gen(τ, ε) = ∀ᾱ β̄. τ ! ε
```

Where:
- `ᾱ` = free type variables in τ not appearing in Γ
- `β̄` = free effect row variables in ε not appearing in the environment

### **6.2. Instantiation — inst(σ)**

```
inst(∀ᾱ β̄. τ ! ε) = τ[ᾱ→ᾱ'] ! ε[β̄→β̄']
```

Where ᾱ' and β̄' are fresh type and row variables respectively.

### **6.3. Effect Quantification Rules**

Effects visible in a function signature must be quantified over:
- Public API: all effects are explicit (no quantification)
- Private functions: effects may be generalized

```
// Private: effects generalized
fn helper(x: Int) -> Int {
  read_file(x)  // inferred: !io, but may be generalized if helper is only used in pure contexts
}

// Public: effects MUST be explicit
export fn process() -> Int !io { ... }
```

## **7. Linear Type Rules**

### **7.1. Judgement Form**

Linearity is tracked via the judgement:

```
Δ ⊢ e : Δ'
```

Where Δ and Δ' are the linear environments before and after evaluating `e`. A binding removed from Δ is "consumed."

### **7.2. L-Var-Use — Using a Linear Variable**

```
x : lin τ ∈ Δ
────────────────────
Δ ⊢ x : Δ \ {x}
```

Using a linear variable consumes it (removes it from Δ).

### **7.3. L-Var-Copy — Using an Ordinary Variable**

```
x : τ ∈ Γ     τ : Copy
─────────────────────
Δ ⊢ x : Δ    (Δ unchanged)
```

Ordinary variables with `Copy` trait leave the linear environment unchanged.

### **7.4. L-Let-Linear — Binding a Linear Value**

```
Δ ⊢ e₁ : Δ'      τ is linear
Δ', x:lin τ ⊢ e₂ : Δ''
───────────────────────────
Δ ⊢ let x : lin τ = e₁; e₂ : Δ''
```

### **7.5. L-App — Function Call with Linear Arguments**

```
For each arg aᵢ of lin type: aᵢ ∈ Δ, removed from Δ
Callee returns a lin value v: v added to Δ
───────────────────────────────────────────────────
Δ ⊢ f(args) : Δ'  (Δ' = (Δ \ consumed_linear) ∪ returned_linear)
```

### **7.6. L-Return — Returning a Linear Value**

```
x : lin τ ∈ Δ
────────────────────────
Δ ⊢ return x : Δ \ {x}
```

### **7.7. L-If/Match — Linear Flow Across Branches**

```
Δ ⊢ e_then : Δ₁
Δ ⊢ e_else : Δ₂
Δ₁ = Δ₂  (both branches consume the same linear bindings)
────────────────────────────────────────────
Δ ⊢ if cond { e_then } else { e_else } : Δ₁
```

### **7.8. L-Spawn — Task Cannot Capture Linear Values**

```
Any linear binding in Δ when evaluating spawn { e } causes an error
──────────────────────────────────────────────────────────
Δ ⊢ spawn { e } : Δ    (only if no linear vars are in scope)
```

### **7.9. L-Using — Resource Consumption**

```
Δ ⊢ e_init : Δ', x:lin τ
Δ', x:lin τ ⊢ e_body : Δ'', x:lin τ    (x NOT consumed in body)
Δ'', x:lin τ ⊢ consume(x) : Δ''
─────────────────────────────────────────────────────
Δ ⊢ using x = e_init { e_body } : Δ''
```

### **7.10. Linear Consumption Diagnostics**

When a linear value `x` is not consumed before the end of its scope:
```
Error: Linear value 'x' of type 'File' is not consumed.
  Created at main.onc:10:14
  Hint: Add 'using', return it, or call a consuming method.
```

When a linear value `x` is used after being consumed:
```
Error: Linear value 'f' used after move.
  Value moved at main.onc:15:10.
  Second use at main.onc:15:22.
```

## **8. Sendability**

### **8.1. Send(T) Judgement**

A type `T` is `Send` if a value of type `T` can be safely transferred between tasks:

```
Send(T) ⇔ T is immutable ∧ T : Copy
         ∨ T is linear (ownership is transferred, no sharing)
```

### **8.2. Formal Send Rules**

**Send-Primitive**:
```
T ∈ {Int, Float, Bool, Str, ()}
────────────────────────────────
Send(T)
```

**Send-Linear**:
```
T is linear (lin prefix or inferred linear from constructor)
─────────────────────────────────────────
Send(T)
```

**Send-Record**:
```
For all fields fᵢ in record R: Send(typeof(fᵢ))
─────────────────────────────────────────────
Send(R)
```

**Send-NonSend**:
```
T contains a mutable reference or non-Send component
───────────────────────────────────────────────────
¬Send(T)
```

### **8.3. Channel Send Rule**

A value of type `T` can be sent over `Chan<T>` only if `Send(T)`:

```
Γ; Δ; Σ ⊢ e_chan : Chan<T> ! ε₁
Γ; Δ; Σ ⊢ e_val : T ! ε₂
Send(T)
RowUnion(ε₁, ε₂) = ε
────────────────────────────────────
Γ; Δ; Σ ⊢ e_chan.send(e_val) : () ! ε

If T is linear: Δ' = Δ \ {e_val}
If T is ordinary: Δ' = Δ
```

## **9. Size Type System (Bounded Arrays)**

### **9.1. Array Types with Size**

```
TypeExpr ::= '[' Type ';' Nat ']'    -- sized array
```

`Array<T, N>` where `N` is a compile-time constant natural number.

### **9.2. Constraint Language**

Bounds constraints are expressed in Presburger arithmetic on naturals:

```
Constraint ::= i < N | i ≤ N | i + k < N | i + k ≤ N | ...
```

Where `i` is an index variable and `N`, `k` are compile-time constants.

### **9.3. Proof Rules**

**BoundsCheck-Elim**:
```
i < N ∧ i' = i + 1 ∧ trusted(i' < N)
─────────────────────────────────────
bounds_check(i') → erased
```

When the compiler can prove that all accesses are within bounds, the runtime check is erased (MIR `BoundsCheck { proven: true }`).

**BoundsCheck-Fallback**:
```
¬proven(i < N) ∨ ¬proven(i + 1 < N)
────────────────────────────────────
bounds_check(i) → emit single runtime check
```

### **9.4. For-Loop Bounds Optimization**

```
for i in 0..arr.len {    // arr.len = N (compile-time known)
  // i < N is trivially true for all loop iterations
  arr[i]                 // bounds check erased
}
```

## **10. Async/Await Semantics**

### **10.1. Task Type**

`Task<T>` is a linear type representing an asynchronous computation that yields `T`.

### **10.2. Async Expression**

```
Γ; Δ; Σ ⊢ e : τ ! ε
───────────────────────────
Γ; Δ; Σ ⊢ async { e } : Task<τ> ! ε ∪ {spawn}
```

The resulting `Task<τ>` is **linear** and must be consumed.

### **10.3. Await Expression**

```
Γ; Δ; Σ ⊢ e_task : Task<τ> ! ε_task
Δ ⊢ e_task : Δ \ {e_task}
───────────────────────────────────
Γ; Δ; Σ ⊢ await e_task : τ ! ε_task ∪ {time}
```

Awaiting consumes the task handle (removes from Δ). If `ε_task` includes `io`, the await also carries `io`.

### **10.4. Task Consumption Requirements**

A `Task<T>` value in scope must be consumed by exactly one of:
- `await t` — block until completion, yield `T`
- `join_all(tasks)` — block until all complete, yield `Vec<T>`
- `cancel(t)` — cancel the task, yield `()`

Failing to consume a task is a linearity error.

### **10.5. Join All**

```
Γ; Δ; Σ ⊢ e_tasks : Vec<Task<T>> ! ε
──────────────────────────────────────
Γ; Δ; Σ ⊢ join_all(e_tasks) : Vec<T> ! ε ∪ {time}
```

## **11. `using` Desugaring & Resource Management**

### **11.1. Resource Trait**

```
trait Resource {
  fn consume(self) -> () !ε
}
```

All standard linear types (`File`, `TcpStream`, `Transaction`, `Deadline`, GPU buffers) implement `Resource`.

### **11.2. `using` Desugaring**

```
using x = E { B }

// desugars to:
let _tmp = E;           // evaluate initializer
let x = _tmp;           // move linear value into x
let _out = { B };        // execute body (with early-return awareness)
consume(x);              // call Resource::consume on x
_out                     // body result is the block result
```

### **11.3. Error-Propagating `using`**

If `consume` returns `Result<(), Err>`, the standard `using` appends `?` propagation:

```
using x = E { B }    // consume(x)?  — error propagates
using! x = E { B }   // consume(x)   — error is ignored (non-propagating)
```

### **11.4. Scope Guarantee in MIR**

The MIR lowering ensures `consume` runs even with early `return` from `B`. This is achieved by inserting the `Drop { x }` op (for `Resource` types, lowered to `Call { "Resource::consume", x }`) at every exit point from the body block.

```
// MIR for using x = File.open(path) { if cond { return; } ... }:
//   call File::open → temp
//   move temp → x
//   branch cond ? early_label : body_label
// early_label:
//   drop x        ← consume at early return
//   return
// body_label:
//   ... body ...
//   drop x        ← consume at normal exit
```

## **12. Derive System**

### **12.1. Syntax**

```
TypeDecl ::= ... [ 'derive' '(' ident { ',' ident } ')' ] '=' TypeBody ';'
```

*Example:*
```
type Point = { x: Float, y: Float } derive(Copy, Eq, Show);
```

### **12.2. Supported Derives**

| Derive | Applicable To | Generated Impl |
|:-------|:--------------|:---------------|
| `Copy` | Types where all fields implement `Copy` | Implicit duplication allowed |
| `Eq` | Types where all fields implement `Eq` | Structural equality |
| `Ord` | Types where all fields implement `Ord` | Lexicographic ordering |
| `Show` | Any type | Debug string representation |
| `Resource` | Types where all fields implement `Resource` | `consume()` calls consume on each field |

### **12.3. Derive Constraints**

- `derive(Copy)` is only valid for non-linear types. Linear types cannot derive `Copy`.
- `derive(Resource)` calls `consume()` on each field in declaration order, combining errors with `?`.
- Derive macros are not Turing-complete; they generate trait impls mechanically based on the type structure.

### **12.4. Generated Code Example**

```
type Config = { host: Str, port: Int } derive(Eq, Show);

// Compiler generates:
impl Eq for Config {
  fn eq(self, other: Config) -> Bool {
    self.host.eq(other.host) && self.port.eq(other.port)
  }
}

impl Show for Config {
  fn show(self) -> Str {
    "Config { host: " + self.host.show() + ", port: " + self.port.show() + " }"
  }
}
```

## **13. The Effect System**

### **13.1. Effect Signature**

A function's effects are denoted by a `!` followed by a list of effect kinds.

```
fn_decl ::= ... [ "!" "[" [ effect { "," effect } ] "]" ] block_expr
effect ::= "io" | "net" | "spawn" | "time" | "ffi" | ...
```

A function with no `!` annotation is **guaranteed to be pure**.

### **13.2. Effect Inference and Propagation**

- **Inference:** Within a function body, the compiler infers the set of effects used. Calling a function with effect `E` adds `E` to the caller's effect set.
- **Propagation:** Effects propagate up the call stack. If `main` calls `A`, and `A` calls `B`, and `B` has the effect `io`, then both `A` and `main` must also be marked with the `io` effect.
- **Public APIs:** For stable interfaces, all exported functions **must** have their effect signatures explicitly annotated in the source code. The compiler will verify that the implementation matches the annotation. For private functions, annotations are optional.

### **13.3. Core Effect Kinds**

- **io**: Filesystem access.
- **net**: Network communication.
- **spawn**: Creating new concurrent processes/actors.
- **time**: Accessing system time or sleeping.
- **ffi**: Calling external code via the Foreign Function Interface.
- **nondet**: Accessing sources of non-determinism (e.g., random number generation).

The effect system is extensible, allowing for user-defined effects in the future.

## **14. Closures and Captures**

### **14.1. Capture Rules**

- **Copy capture**: If a captured value has type `T` where `T : Copy`, the value is copied into the closure environment.
- **Move capture**: If a captured value has type `T` where `¬(T : Copy)`, the value is moved into the closure environment. The original binding is consumed.
- **Linear capture**: If any captured value is linear, the closure itself becomes linear and implements `FnOnce` (callable exactly once).

### **14.2. Region Interaction**

Captured values moved into a closure are treated as escaping: `escapes(v, ρ_enclosing → ρ_closure)`. The closure's environment region `ρ_closure` must outlive the last possible invocation.

### **14.3. Closure Trait Hierarchy**

| Traits | Properties | Callable |
|:-------|:-----------|:---------|
| `FnOnce` | Captures linear or non-Copy values | Once (consumes closure) |
| `FnMut` | Captures mutable `var` bindings | Multiple times (mutable) |
| `Fn` | Captures only `Copy` or immutable data | Any number of times |
