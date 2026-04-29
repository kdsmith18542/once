# **ONCE Specification \- Part 4: Memory & Concurrency Model**

| Document ID | ONCE-004 |
| :---- | :---- |
| **Title** | Memory & Concurrency Model |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-001, ONCE-003 |

## **1\. Introduction**

This document specifies the memory and concurrency models of the Once language. These models are designed to work in concert with the type and effect systems (defined in ONCE-003) to provide compile-time guarantees against common errors such as memory leaks, use-after-free, null pointer dereferences, and data races.

The core tenets are:

1. **Automated Memory Management without a Garbage Collector:** Achieved via Region-Based Memory Management (RMM).  
2. **Guaranteed Resource Cleanup:** Built upon the linear type system.  
3. **Provably Race-Free Concurrency:** Enforced by the type system's "Mutable XOR Shared" invariant.

## **2\. Region-Based Memory Management (RMM)**

Once automates memory management by statically inferring lifetimes, a concept it calls **regions**. This approach avoids both the overhead of a tracing garbage collector and the cognitive complexity of manual lifetime annotations required by other systems languages.

### **2.1. Core Principles**

* **Compiler-Managed Lifetimes:** The developer never writes explicit lifetime annotations. The compiler is responsible for inferring the lifetime (region) of every value.  
* **Bulk Deallocation:** Memory allocated within a region is deallocated all at once when the region ends. This is highly efficient, turning many individual free calls into a single operation.

### **2.2. Regions and Scopes**

Every function body defines a **primary region**. The compiler may create additional, nested **subregions** to manage values with shorter lifetimes (e.g., inside loops or if blocks).

* **Allocation:** When a value is allocated (e.g., creating a Vec or a record), it is placed into the innermost currently active region.  
* **Region Exit:** When a region's scope ends, all memory associated with that region is immediately freed.

### **2.3. Escape Analysis**

Values can "escape" from one region to another, typically a parent region. This happens when a value is returned from a function or moved into a data structure that outlives the current region.

* The compiler performs an **escape analysis** to track these movements.  
* If a value escapes, its ownership (and its memory) is transferred to the destination region.  
* Any value that does not escape its region is guaranteed to be unreachable after the region ends and is safely deallocated.

### **2.4. Fallback Mechanisms**

In complex scenarios where the compiler's static analysis cannot prove a safe region for a value, Once provides two explicit, opt-in heap allocation strategies. Their use is discouraged and will trigger a compiler lint, as they signal a deviation from the preferred RMM model.

* **box T**: An owned, unique pointer to a heap-allocated value of type T. It has single ownership and is automatically deallocated when its owner goes out of scope.  
* **rc T**: A reference-counted pointer for shared, immutable ownership of a heap-allocated value.

## **3\. Structured Resource Management**

Building on the linear type system (ONCE-003), Once guarantees that resources like files and sockets are properly released.

### **3.1. The Resource Trait**

A standard library trait defines the contract for any type that represents a managed resource.

trait Resource {  
  // Consumes the resource, performing the final action (e.g., close, release, commit).  
  fn consume(self: lin Self) \-\> Result\<(), Error\>;  
}

All standard I/O and resource types implement this trait.

### **3.2. The using Construct**

The using keyword provides an ergonomic way to ensure a linear resource is consumed at the end of a scope.

// f is a \`lin File\` which implements \`Resource\`  
using f \= File.open("data.txt")? {  
  // f can be used here.  
  f.write("...")?;  
} // f.consume() is implicitly called here, even with early returns.

This construct desugars into a block that guarantees the consume method is called on the resource, preventing leaks by construction.

## **4\. Concurrency Model**

Once's concurrency model is designed for safety and clarity, based on the principles of Communicating Sequential Processes (CSP) and the Actor model.

### **4.1. Concurrency Primitives**

* **Process:** A lightweight, cooperatively scheduled task, similar to a goroutine or virtual thread. Created with the spawn keyword.  
* **Channel (Chan\<T\>):** A typed conduit for sending messages between processes. Channels are the primary means of communication.  
* **Actor:** An encapsulated entity with its own state that communicates exclusively through messages sent to its channel-based mailbox.

### **4.2. The "Mutable XOR Shared" Invariant**

This is the cornerstone of Once's concurrency safety. The type system enforces this rule at compile time:

A value may be **mutable** and have a single, unique owner, **OR** it may be **immutable** and be shared by many. It can **never** be both mutable and shared simultaneously.

This invariant fundamentally eliminates data races. Since shared data cannot be mutated, there is no possibility of one process interfering with another's read/write operations.

### **4.3. Ownership Transfer via Channels**

Sending a value over a channel is an ownership-transfer operation.

* If T is an **ordinary** (copyable) type, a copy of the value is sent.  
* If T is a **linear** or **non-copyable** type (like Vec\<T\> or a mutable var), the value itself is **moved** into the channel. The sending process relinquishes ownership and can no longer access the value.

This ensures that only one process can own (and therefore mutate) a given piece of data at any time.

### **4.4. Structured Concurrency**

To prevent orphaned or "leaked" processes, Once encourages the use of structured concurrency blocks.

group {  
  let handle1 \= spawn { do\_work\_1() };  
  let handle2 \= spawn { do\_work\_2() };

  await handle1?; // Await completion  
  await handle2?;  
} // The program is guaranteed not to exit this block until handle1 and handle2 have completed.

This ensures that the lifetime of concurrent operations is tied to a specific lexical scope, making programs easier to reason about.