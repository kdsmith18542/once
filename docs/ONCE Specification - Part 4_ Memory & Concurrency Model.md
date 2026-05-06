# **ONCE Specification – Part 4: Memory & Concurrency Model**

| Document ID | ONCE-004 |
| :---- | :---- |
| **Title** | Memory & Concurrency Model |
| **Version** | 1.1 |
| **Date** | 2026-05-03 |
| **Status** | Draft |
| **Supersedes** | ONCE-004 v1.0 |
| **Related Docs** | ONCE-001, ONCE-003, ONCE-009 |

## **1. Introduction**

This document specifies the memory and concurrency models of the Once language. These models are designed to work in concert with the type and effect systems (defined in ONCE-003) to provide compile-time guarantees against common errors such as memory leaks, use-after-free, null pointer dereferences, and data races.

The core tenets are:

1. **Automated Memory Management without a Garbage Collector:** Achieved via Region-Based Memory Management (RMM).
2. **Guaranteed Resource Cleanup:** Built upon the linear type system.
3. **Provably Race-Free Concurrency:** Enforced by the type system's "Mutable XOR Shared" invariant.

## **2. Region-Based Memory Management (RMM)**

### **2.1. Core Principles**

- **Compiler-Managed Lifetimes:** The developer never writes explicit lifetime annotations. The compiler infers the lifetime (region) of every value.
- **Bulk Deallocation:** Memory allocated within a region is deallocated all at once when the region ends. This is highly efficient, turning many individual free calls into a single operation.

### **2.2. Regions and Scopes**

Every function body defines a **primary region**. The compiler may create additional, nested **subregions** to manage values with shorter lifetimes (e.g., inside loops or `if` blocks).

- **Allocation:** When a value is allocated (e.g., creating a `Vec` or a record), it is placed into the innermost currently active region.
- **Region Exit:** When a region's scope ends, all memory associated with that region is immediately freed.

```
Region ::= { id: Nat, name: String, is_primary: Bool }
```

### **2.3. Escape Analysis**

Values can "escape" from one region to another, typically a parent region. This happens when a value is returned from a function or moved into a data structure that outlives the current region.

- The compiler performs an **escape analysis** to track these movements.
- If a value escapes, its ownership (and its memory) is transferred to the destination region.
- Any value that does not escape its region is guaranteed to be unreachable after the region ends and is safely deallocated.

### **2.4. Fallback Mechanisms**

In complex scenarios where the compiler's static analysis cannot prove a safe region for a value, Once provides two explicit, opt-in heap allocation strategies:

- **box T**: An owned, unique pointer to a heap-allocated value. Single ownership, automatically deallocated when owner goes out of scope.
- **rc T**: A reference-counted pointer for shared, immutable ownership of a heap-allocated value.

## **3. Region Constraint Language**

### **3.1. Constraint Types**

The region inference system generates four kinds of constraints:

```
Constraint ::= Alloc(e, ρ)          -- expression e allocates into region ρ
             | Escape(v, ρ_s, ρ_d) -- value v escapes from region ρ_s to ρ_d
             | Live(ρ, point)       -- region ρ must live until program point
             | Subregion(ρ₁, ρ₂)    -- ρ₁ is a subregion of ρ₂
```

### **3.2. Constraint Generation**

| Source Pattern | Generated Constraints |
|:---------------|:----------------------|
| `let x = alloc_expr` | `Alloc(alloc_expr, ρ_current)` |
| `return x` | `Escape(x, ρ_current, ρ_caller)` |
| `chan.send(x)` | `Escape(x, ρ_current, ρ_channel)` |
| `|...| => { ... x ... }` | `Escape(x, ρ_current, ρ_closure)` |
| `if ... { ... x ... }` | `Live(x, end_of_scope)` if x is local to the branch |
| Loop body allocation | Subregion between loop region and function region |

### **3.3. Liveness Analysis**

A region ρ must stay alive until the last use of any value allocated in ρ:

```
Live(ρ, last_use_point)    where last_use_point = max({ use_point(v) | Alloc(v, ρ) })
```

## **4. Region DAG Construction**

### **4.1. Per-Function Root Region**

Each function has a primary region `R_fn`. Subregions are created for:

- Loop bodies (`R_loop_N`)
- `if`/`else` branches (`R_branch_N`)
- Closure environments (`R_closure_N`)
- `group` blocks (`R_group_N`)

### **4.2. DAG Construction Algorithm**

```
function build_region_dag(constraints, fn):
    dag = new RegionDag()
    dag.root = Region(fn.name, primary=true)

    for each alloc site:
        ρ = get_or_create_subregion(dag, alloc.site)
        dag.add_edge(Subregion(ρ, dag.root))

    for each escape constraint Escape(v, ρ_s, ρ_d):
        dag.add_edge(EscapeEdge(ρ_s, ρ_d))

    for each Subregion(ρ₁, ρ₂):
        dag.add_edge(DominanceEdge(ρ₁, ρ₂))

    return dag
```

### **4.3. Escape Edge DAG**

Escape edges form the backbone of the region DAG:

```
ρ_closure → ρ_enclosing
ρ_loop_body → ρ_fn_primary
ρ_callee → ρ_caller  (for returned values)
```

## **5. Solving Algorithm**

### **5.1. Pipeline**

```
function solve_regions(fn_hir):
    constraints = generate_constraints(fn_hir)
    dag = build_region_dag(constraints, fn_hir)
    free_points = compute_free_points(dag, constraints)
    return FreePointMap(free_points)
```

### **5.2. Constraint Collection**

Walk the HIR AST and generate all `Alloc`, `Escape`, `Live`, and `Subregion` constraints. This pass is in `once-rinf/src/lib.rs`.

### **5.3. Liveness Analysis**

For each region ρ, compute the set of program points where values in ρ are accessed. The maximum access point determines `Live(ρ, point)`.

### **5.4. Dominance Analysis**

For each region ρ, find the program point that post-dominates all allocation sites and all use sites, but does not precede any escape:

```
free_point(ρ) = earliest_point such that:
    point postdominates all alloc sites in ρ
    AND point postdominates all use sites in ρ
    AND point does not preceed any escape from ρ
```

### **5.5. Free Placement**

Once the safe free point is computed, a `FreeRegion { ρ }` MIR operation is inserted:

```
function place_frees(mir_fn, free_point_map):
    for (region, point) in free_point_map:
        insert FreeRegion { region } at point in mir_fn
```

### **5.6. Fallback (Unsatisfiable Constraints)**

When no safe free point can be found (e.g., a value escapes into multiple regions with incompatible lifetimes):

```
1. Emit warning: "Cannot statically prove region lifetime for 'x'"
2. Suggest fix: "Consider using box<T> or rc<T>"
3. If @no_rc annotation present: fail compilation
```

## **6. Heuristics**

### **6.1. Coalesce Threshold**

Merge regions when the total allocation size in both regions is below a threshold:

```
if sum_alloc_size(ρ₁) + sum_alloc_size(ρ₂) < COALESCE_THRESHOLD:
    merge ρ₁ and ρ₂
```

Reduces the number of region frees for small allocations.

### **6.2. Split Threshold**

Split a region into subregions when allocation density or size profiles indicate benefit:

```
if alloc_density(ρ) > SPLIT_THRESHOLD:
    split ρ at loop boundaries or hot/cold path boundaries
```

### **6.3. Hot Loop Splitting**

For loop bodies with heavy allocation, create a dedicated subregion to allow earlier free:

```
for i in 0..N {
    let temp = allocate_big_object()  // allocated in ρ_loop
}  // ρ_loop freed each iteration
```

## **7. Structured Resource Management**

### **7.1. The Resource Trait**

```
trait Resource {
  fn consume(self: lin Self) -> Result<(), Error>;
}
```

All standard I/O and resource types implement this trait.

### **7.2. The `using` Construct**

```
using f = File.open("data.txt")? {
  f.write("...")?;
} // f.consume() is implicitly called here, even with early returns.

// Desugars to (see ONCE-003 §11):
let _tmp = File.open("data.txt")?;
let f = _tmp;
let _out = { f.write("...")?; };
consume(f)?;
_out
```

### **7.3. Copy & Clone Contracts**

- **Marker trait `Copy`**: Only types implementing `Copy` may be implicitly duplicated.
- **`clone()`** for linear types is opt-in and must return two or more **independent** linear handles with documented disjointness guarantees.

## **8. Concurrency Model**

### **8.1. Concurrency Primitives**

- **Process:** A lightweight, cooperatively scheduled task (similar to a goroutine). Created with `spawn`.
- **Channel (`Chan<T>`):** A typed conduit for sending messages between processes. Primary means of communication.
- **Actor:** An encapsulated entity with its own state that communicates exclusively through messages sent to its channel-based mailbox.

### **8.2. The "Mutable XOR Shared" Invariant**

A value may be **mutable** and have a single, unique owner, **OR** it may be **immutable** and be shared by many. It can **never** be both mutable and shared simultaneously.

This invariant fundamentally eliminates data races.

### **8.3. Ownership Transfer via Channels**

- If `T` is an **ordinary** (copyable) type, a copy of the value is sent.
- If `T` is a **linear** or **non-copyable** type (like `Vec<T>` or a mutable `var`), the value is **moved** into the channel. The sending process relinquishes ownership.

### **8.4. Structured Concurrency**

```
group {
  let handle1 = spawn { do_work_1() };
  let handle2 = spawn { do_work_2() };
  await handle1?;
  await handle2?;
} // Program guaranteed not to exit until handle1 and handle2 have completed.
```

Group failure policies:
- **FailFast** (default): First error cancels all siblings.
- **All**: Wait for all to complete, collect errors.
- **Supervisor**: Parent handles child errors via a supervisor callback.

## **9. Deadlock Detection**

### **9.1. Wait-For Graph**

The runtime maintains a **wait-for graph** for tasks:

```
WaitForGraph ::= { nodes: Set<TaskId>, edges: Map<TaskId, Set<TaskId>> }
```

Edges are added when a task blocks on:
- `Chan::send` when channel is full (Block policy)
- `Chan::recv` when channel is empty (Block policy)
- `await task`
- `await group`
- Future: lock acquisition

### **9.2. Graph Population**

```
on_task_blocked(blocked_task, waiting_for_task):
    graph.add_edge(blocked_task, waiting_for_task)
    if has_cycle(graph):
        emit DeadlockError(cycle)

on_task_unblocked(task):
    graph.remove_edges_from(task)
```

### **9.3. Cycle Detection Algorithm (DFS)**

```
function has_cycle(graph):
    visited = Set()
    in_stack = Set()

    for node in graph.nodes:
        if node not in visited:
            if dfs(node, visited, in_stack):
                return true
    return false

function dfs(node, visited, in_stack):
    visited.add(node)
    in_stack.add(node)

    for neighbor in graph.edges[node]:
        if neighbor not in visited:
            if dfs(neighbor, visited, in_stack):
                cycle = extract_cycle(in_stack, neighbor)
                return true
        elif neighbor in in_stack:
            cycle = extract_cycle(in_stack, neighbor)
            return true

    in_stack.remove(node)
    return false
```

### **9.4. Deadlock Diagnostic Output**

When a cycle is detected, the runtime halts and outputs:

```
FATAL: Deadlock detected in deterministic scheduler mode.
Cycle trace (4 tasks):
  Task#3 (main.onc:22) waiting on Channel:recv from Task#7
  Task#7 (main.onc:30) waiting on Channel:recv from Task#12
  Task#12 (main.onc:38) waiting on Channel:recv from Task#7
  → Cycle involves tasks 7, 12, 3
```

### **9.5. Deterministic Scheduler Mode**

```
once test --deterministic --deadlock=fail
```

In deterministic mode, the scheduler explores a single execution path. Cycles are detected immediately, producing reproducible deadlock traces.

## **10. Channel Backpressure Semantics**

### **10.1. Backpressure Policies**

```
enum Backpressure { Block, DropOldest, DropNewest, Error }
```

Channel constructors require both capacity and policy:

```
let c = Chan::new(cap=1024, policy=Backpressure::Block)
```

### **10.2. Policy Semantics**

#### Block (Default)

```
send(c, value) when c.len() == c.cap():
    block sender until space available
    contribute !time effect

recv(c) when c.len() == 0:
    block receiver until value available
    contribute !time effect
```

#### DropOldest

```
send(c, value) when c.len() == c.cap():
    drop the oldest queued value
    enqueue the new value
    return Ok(())
    // oldest value is silently lost
```

#### DropNewest

```
send(c, value) when c.len() == c.cap():
    return Err(Dropped)
    // new value is silently lost
```

#### Error

```
send(c, value) when c.len() == c.cap():
    return Err(Full)

recv(c) when c.len() == 0:
    return Err(Empty)
```

### **10.3. Formal Policy Definitions**

| Policy | send when full | recv when empty | Effect |
|:-------|:---------------|:----------------|:-------|
| `Block` | Block until space | Block until value | `!time` |
| `DropOldest` | Drop oldest, enqueue new | Block until value | `!time` (recv only) |
| `DropNewest` | Return `Err(Dropped)` | Block until value | `!time` (recv only) |
| `Error` | Return `Err(Full)` | Return `Err(Empty)` | Pure |

### **10.4. Visibility Tooling**

```
once explain --concurrency
```

Renders channel buffer sizes, policies, and hot sender/receiver analysis for all channels in the program.

## **11. Concurrency Safety Invariants**

### **11.1. Mutable XOR Shared**

A value of a mutable type cannot be shared. A shared value must be immutable. This is enforced at compile time by the type checker.

### **11.2. Send(T)**

A type `T` is `Send` iff:
- (a) `T` is immutable and implements `Copy`, OR
- (b) `T` is linear and the send moves ownership.

See ONCE-003 §8 for formal rules.

### **11.3. Effect Versioning**

- Spawning contributes `spawn` to ε.
- Blocking channel ops contribute `time` to ε.
- Adding effects to public APIs increments the major version per tool enforcement.

## **12. Runtime Architecture**

### **12.1. Work-Stealing Scheduler**

- Per-worker deques for cache-hot task injection.
- When a worker's deque is empty, it steals from other workers.
- `WorkerStats` tracks `tasks_stolen`, `tasks_executed`, `tasks_spawned`.

### **12.2. Actor System**

- Mailbox: `Chan<ActorMsg>` with blocking receive loop.
- State: encapsulated `var S` not shareable externally.
- Integration: actor threads are tracked in the scheduler's worker pool. On `Scheduler::stop()`, mailbox channels are closed to unblock actors.

### **12.3. Cooperative Cancellation**

- `Deadline` tokens: linear handles that cancel a task when dropped.
- `cancel(task_handle)`: explicit task cancellation.

## **13. Atomics & Memory Model**

### **13.1. Atomic Types**

- `Atomic<Int>`, `Atomic<Bool>`, `Atomic<Ptr<T>>`.
- Default ordering: `SeqCst`.
- Optional: `Acquire`, `Release`, `AcqRel`, `Relaxed`.

### **13.2. Usage Guidance**

Atomics are for interior mutability **inside actors**. Across tasks, prefer ownership moves (keeps *Mutable XOR Shared*).

```
fn tick(counter: &Atomic<Int>) {
  counter.fetch_add(1, Ordering::AcqRel)
}
```

## **14. Slices and Views**

### **14.1. Slice Types**

- **`Slice<T>`**: Immutable, non-owning view with length. Freely shareable.
- **`SliceMut<T>`**: Mutable, non-owning view. **Linear** and exclusively owned while in scope.

### **14.2. Safety**

Bounds proven via size types when possible; otherwise, a single runtime check at slice creation.

```
fn normalize(win: SliceMut<Float>) {
  let m = mean(win as Slice<Float>);
  for i in 0..win.len { win[i] = win[i] - m; }
} // win consumed (linear)
```
