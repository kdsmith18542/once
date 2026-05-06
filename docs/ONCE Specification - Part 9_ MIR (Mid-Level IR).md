# **ONCE Specification – Part 9: MIR (Mid-Level IR)**

| Document ID | ONCE-009 |
| :---- | :---- |
| **Title** | MIR (Mid-Level IR) |
| **Version** | 1.0 |
| **Date** | 2026-05-03 |
| **Status** | Draft |
| **Supersedes** | N/A |
| **Related Docs** | ONCE-002, ONCE-003, ONCE-004 |

## **1. Introduction**

This document defines the Mid-Level Intermediate Representation (MIR) for the Once compiler. MIR is a lowered, control-flow-graph-based IR that sits between the typed HIR and the codegen backend. It makes all operations explicit: moves, drops, region allocations/frees, bounds checks, and concurrency primitives.

MIR is the last IR before machine code generation. Every semantic guarantee about resource safety, memory management, and concurrency is reflected in explicit MIR operations that can be verified and lowered mechanically.

## **2. MIR Structure**

### 2.1. Program Structure

A MIR program consists of a set of function definitions and a global region identifier.

```
MirProgram ::= { MirFunction } GlobalRegion
GlobalRegion ::= Region
```

### 2.2. Function Structure

Each MIR function is a flat list of statements forming a single basic block per function (pre-SSA flattening). Control flow is expressed via explicit Label, Jump, and Branch operations.

```
MirFunction ::= name: String
                params: [(String, Type)]
                return_type: Type
                body: MirBlock
                local_count: Nat
                temp_count: Nat
```

### 2.3. Block Structure

```
MirBlock ::= { MirStmt }
MirStmt  ::= op: MirOp  span: Span  region: Option<Region>
```

### 2.4. Locations

Values in MIR are stored at locations:

```
MirLocation ::= Local(Nat)           -- Named local variable
              | Param(Nat)           -- Function parameter
              | Return               -- Return value slot
              | Temp(Nat)            -- Compiler-generated temporary
              | Field { base: MirLocation, field: String }
              | Index { base: MirLocation, index: MirLocation }
```

## **3. Complete MirOp Catalog**

### 3.1. Value Movement and Lifetime

#### Move
Semantics: Transfers ownership of a value from `from` to `to`. If the source type is linear, `from` becomes inaccessible after this operation. If the source type is ordinary (copyable), this may emit a bitwise copy.

```
MirOp::Move { from: MirLocation, to: MirLocation }
```

**Linearity rule**: After `Move { from: x, to: y }`, `x` is consumed (removed from the linear environment Δ).

#### Drop
Semantics: Explicitly consumes a linear value at `location` without transferring it elsewhere. Required when a linear value goes out of scope without being moved or returned. Maps to `Resource::consume()` call at runtime if the type implements `Resource`.

```
MirOp::Drop { location: MirLocation }
```

**Linearity rule**: Consumes the linear binding at `location`.

#### Copy
Semantics: Explicitly duplicates a value of a `Copy` type. Only valid when the type at `from` implements the `Copy` trait.

```
MirOp::Move { from, to } where typeof(from) : Copy
```

Note: No separate Copy op exists; copy semantics are expressed via Move on Copy types.

### 3.2. Region Management

#### Allocate
Semantics: Allocates `size` bytes in the given `region` and stores the resulting pointer in `dest`. The allocation is bulk-freed when the region is freed.

```
MirOp::Allocate { region: Region, size: usize, dest: MirLocation }
```

**Region invariant**: `Allocate { region: ρ, ... }` must be dominated by the function entry and post-dominated by `FreeRegion { region: ρ }`.

#### FreeRegion
Semantics: Bulk-deallocates all memory allocated in `region`. After this operation, all pointers derived from allocations in this region become invalid.

```
MirOp::FreeRegion { region: Region }
```

**Region invariant**: Must post-dominate all `Allocate` ops for the same region and must dominate all exits from the function.

### 3.3. Computation

#### LoadLiteral
Semantics: Loads a compile-time constant value into `dest`.

```
MirOp::LoadLiteral { value: MirValue, dest: MirLocation }
MirValue ::= Int(i64) | Float(f64) | Bool(bool) | String(String) | Unit
```

#### BinOp
Semantics: Performs a binary operation on `left` and `right`, storing the result in `dest`.

```
MirOp::BinOp { op: MirBinOp, left: MirLocation, right: MirLocation, dest: MirLocation }
MirBinOp ::= Add | Sub | Mul | Div
           | Eq | Ne | Lt | Le | Gt | Ge
           | And | Or
           | Move                     -- Assignment (=)
```

**Operator semantics**:

| Op | Signature | Description |
|:---|:----------|:------------|
| Add | `Int × Int → Int`, `Float × Float → Float` | Addition |
| Sub | `Int × Int → Int`, `Float × Float → Float` | Subtraction |
| Mul | `Int × Int → Int`, `Float × Float → Float` | Multiplication |
| Div | `Int × Int → Int`, `Float × Float → Float` | Division (panics on div-by-zero) |
| Eq | `T × T → Bool` where `T : Eq` | Equality |
| Ne | `T × T → Bool` where `T : Eq` | Inequality |
| Lt | `T × T → Bool` where `T : Ord` | Less than |
| Le | `T × T → Bool` where `T : Ord` | Less or equal |
| Gt | `T × T → Bool` where `T : Ord` | Greater than |
| Ge | `T × T → Bool` where `T : Ord` | Greater or equal |
| And | `Bool × Bool → Bool` | Logical AND |
| Or | `Bool × Bool → Bool` | Logical OR |
| Move | `T → T` | Assignment (source consumed, dest assigned) |

### 3.4. Bounds Checking

#### BoundsCheck
Semantics: Verifies that `index` is within bounds of `bound`. If `proven` is true, the check is erased at codegen time (the compiler has proven it statically). If `proven` is false, a runtime check is emitted.

```
MirOp::BoundsCheck { index: MirLocation, bound: MirLocation, proven: bool }
```

**Codegen contract**: When `proven = true`, emit no runtime instruction. When `proven = false`, emit a bounds comparison and trap on failure.

### 3.5. Function Calls

#### Call
Semantics: Calls the function `function` with arguments `args`, storing the return value in `result`.

```
MirOp::Call { function: String, args: Vec<MirLocation>, result: MirLocation }
```

**Effect rule**: The caller's effect set must include all effects in the callee's effect signature.

**Linearity rule**: Arguments of linear type are consumed by the call. If the callee returns a linear value, it is materialized in `result`.

#### Return
Semantics: Returns from the current function. If `value` is `Some(loc)`, the value at `loc` is returned to the caller. If `None`, the function returns `()`.

```
MirOp::Return { value: Option<MirLocation> }
```

**Linearity rule**: Any remaining unconsumed linear bindings at the point of Return cause a compile error.

**Region rule**: `FreeRegion` ops for the function's regions must precede `Return` in the MIR block.

### 3.6. Concurrency Primitives

#### SpawnTask
Semantics: Creates a new lightweight task executing `function(args)`. The task handle (of type `Task<T>`) is stored in `result`.

```
MirOp::SpawnTask { function: String, args: Vec<MirLocation>, result: MirLocation }
```

**Effect**: Contributes `spawn` to the effect row.

**Linearity**: The `result` task handle is linear and must be consumed by `AwaitTask`, joined, or cancelled.

#### AwaitTask
Semantics: Blocks the current task until `task` completes, then stores the result value in `result`. Consumes the task handle.

```
MirOp::AwaitTask { task: MirLocation, result: MirLocation }
```

**Effect**: Contributes `time` (and possibly `spawn`) to the effect row.

**Linearity**: Consumes the task handle at `task`.

#### CreateGroup
Semantics: Creates a new task group for structured concurrency. The group handle is stored in `result`.

```
MirOp::CreateGroup { result: MirLocation }
```

#### SpawnInGroup
Semantics: Spawns a task as a child of the given group. The task handle is stored in `result`.

```
MirOp::SpawnInGroup { group: MirLocation, function: String, args: Vec<MirLocation>, result: MirLocation }
```

**Structured concurrency**: The group cannot exit until all children have completed (or been cancelled).

#### AwaitGroup
Semantics: Blocks until all children of `group` have completed. Consumes the group handle. Stores the aggregated result in `result`.

```
MirOp::AwaitGroup { group: MirLocation, result: MirLocation }
```

**Effect**: Contributes `time` to the effect row.

### 3.7. Channel Operations

#### ChannelSend
Semantics: Sends `value` over `channel`. If `value` has linear type, ownership is transferred to the receiver. If `value` has ordinary type, a copy is sent.

```
MirOp::ChannelSend { channel: MirLocation, value: MirLocation }
```

**Backpressure**: Behavior depends on the channel's `Backpressure` policy:
- `Block`: Blocks when buffer is full.
- `DropOldest`: Drops the oldest message, enqueues new message.
- `DropNewest`: Drops the new message, returns `Dropped`.
- `Error`: Returns `Err(Full)` immediately.

**Linearity**: If `value` type is linear, the send consumes it in the sender.

#### ChannelRecv
Semantics: Receives a value from `channel`, storing it in `result`.

```
MirOp::ChannelRecv { channel: MirLocation, result: MirLocation }
```

**Backpressure**: Blocks when buffer is empty under `Block` policy. Returns `Err(Empty)` under `Error` policy.

**Linearity**: The received value is a fresh linear/ordinary binding depending on type.

### 3.8. Control Flow

#### Label
Semantics: Marks a position in the MIR instruction stream as a jump target.

```
MirOp::Label { id: usize }
```

**Invariant**: Every `id` used as a jump target must have exactly one corresponding `Label` op in the same function. Labels must be unique within a function.

#### Jump
Semantics: Unconditional jump to the Label with the given `target` id.

```
MirOp::Jump { target: usize }
```

#### Branch
Semantics: Evaluates `condition`. If true, jumps to `true_target`; if false, jumps to `false_target`.

```
MirOp::Branch { condition: MirLocation, true_target: usize, false_target: usize }
```

**Type rule**: `condition` must have type `Bool`.

### 3.9. Error Handling

#### TryBlock
Semantics: Instruments an expression for error context capture. If the expression evaluates to an `Err` variant, the context is captured for diagnostics before propagating the error.

```
MirOp::TryBlock { result: MirLocation }
```

## **4. MIR Invariants**

### 4.1. SSA-Like Properties

- Each `Temp` is assigned exactly once (single static assignment for temporaries).
- `Local` slots may be reassigned (via `Move` into the same `Local`), reflecting mutable `var` bindings.
- After a `Move` from a linear-typed location, that location is "dead" and may not be read again.

### 4.2. Dead-After-Move Invariant

For any `Move { from: x, to: y }` where `typeof(x)` is linear:
- `x` must not appear as `from` or as any operand in any subsequent MIR statement.
- Violation indicates a double-use of a linear value and is a compiler error.

### 4.3. Free-Dominates-Alloc Invariant

For every `Allocate { region: ρ, ... }` in a function:
- There must exist a `FreeRegion { region: ρ }` that post-dominates the allocate.
- No `Return` may appear between `Allocate` and its corresponding `FreeRegion` unless the allocation has escaped to the caller's region.

### 4.4. Flat CFG

- MIR functions contain a flat list of statements with explicit labels, jumps, and branches.
- Basic block structure is implicit: a basic block spans from one `Label` to the next `Label`/`Jump`/`Branch`/`Return`.
- This flat representation simplifies analysis and transformation passes.

## **5. Backend Contract**

### 5.1. Exported Symbols

Functions marked `export` in the source language emit MIR functions with the `export` flag. The codegen backend must:
- Emit the function with a mangled name following the pattern: `_ONCE_<module>_<name>_<type_hash>`.
- Use the System V AMD64 ABI calling convention (platform-appropriate for non-x86_64 targets).

### 5.2. Calling Convention

- **System V AMD64 ABI** for x86_64 targets.
- Integer/pointer arguments in registers: RDI, RSI, RDX, RCX, R8, R9.
- Floating-point arguments in XMM0–XMM7.
- Return value in RAX (integer/pointer) or XMM0 (float).
- Stack alignment: 16 bytes at call site.
- Callee-saved registers: RBX, RBP, R12–R15.

### 5.3. Type Layout

| Once Type | Size (bytes) | Alignment (bytes) | C/LLVM Equivalent |
|:----------|:-------------|:-------------------|:-------------------|
| `Int` | 8 | 8 | `i64` |
| `Float` | 8 | 8 | `double` |
| `Bool` | 1 | 1 | `i1` (zero-extended) |
| `Str` | 16 | 8 | `{ i8*, i64 }` (pointer + length) |
| `()` | 0 | 1 | Zero-sized |
| Enum (ADT) | tag_size + max_variant_size | max_field_alignment | `{ tag, union }` |
| Record | sum(field_sizes) | max(field_alignments) | Struct with fields in order |
| `Task<T>` | 8 | 8 | Opaque pointer |
| `Chan<T>` | 8 | 8 | Opaque pointer |
| `box T` | 8 | 8 | Pointer |
| `rc T` | 8 | 8 | Pointer |

### 5.4. Stack Map

For functions that allocate into regions, the codegen backend emits a **stack map** identifying:
- The location of each region pointer (live across calls for GC-free region tracking).
- The start and end PC offsets of each region's lifetime (for debugger integration).

### 5.5. Trap Table

The codegen emits a **trap table** mapping PC offsets to trap reasons:
- `BoundsCheckFailed`: Array index out of bounds.
- `DivByZero`: Division by zero.
- `NullPointer`: Dereference of null pointer (from `box`/`rc`).
- `DeadlockDetected`: Runtime deadlock cycle detection.
- `ChannelClosed`: Send on a closed channel.
- `CapabilityViolation`: Runtime capability check failure.

## **6. Lowering from HIR to MIR**

### 6.1. Expression Lowering

| HIR Construct | MIR Ops |
|:--------------|:--------|
| `Literal(n)` | `LoadLiteral { value, dest }` |
| `Ident(x)` | `Move { from: slot(x), to: dest }` |
| `Binary(left, op, right)` | Lower left → temp1, lower right → temp2, `BinOp { op, left: temp1, right: temp2, dest }` |
| `If(cond, then, else)` | Lower cond, `Branch`, then-block with `Label`/`Jump`, else-block with `Label`/`Jump` |
| `Match(scrutinee, arms)` | Lower scrutinee, for each arm: `Branch`/`Jump` to arm body, end `Label` |
| `f(args)` | Lower each arg to temp, `Call { f, args, dest }` |
| `spawn { e }` | `SpawnTask { function, args, result }` |
| `await t` | `AwaitTask { task: t, result }` |
| `for x in coll { body }` | Counter-based loop with `Branch` test, body `Label`, increment, `Jump` back |
| `while cond { body }` | `Branch` on cond to body `Label`, body block, `Jump` back to re-eval cond |
| `try expr` | `TryBlock`, `Branch` on Ok/Err, Err path: `Return { error }`, Ok path: continue |
| `using x = E { B }` | Lower E → temp, `Move` to x, lower B body, `Drop { x }` |

### 6.2. Region Integration

After expression lowering, the `add_region_frees` pass scans the region DAG and inserts `FreeRegion` ops at the appropriate points. The `add_drop_operations` pass inserts `Drop` ops for unconsumed linear values.

### 6.3. Bounds Check Integration

The `add_bounds_checks` pass scans for `Index` locations and ensures every one has a corresponding `BoundsCheck`. If the bounds checker (`once-bounds`) has proven an index is safe, the `proven` field is set to `true`.

## **7. MIR Pass Pipeline**

1. **Generation**: Lower HIR to flat MIR with labels/jumps.
2. **Region Free Insertion** (`add_region_frees`): Insert `FreeRegion` ops from region DAG.
3. **Drop Insertion** (`add_drop_operations`): Insert `Drop` ops for consumed linear values.
4. **Bounds Check Insertion** (`add_bounds_checks`): Insert `BoundsCheck` for array accesses.
5. **Optimization** (`once-opt`): Dead code elimination, move optimization, region coalescing.
6. **Codegen Lowering** (`once-codegen`): Lower MIR ops to Cranelift IR.
