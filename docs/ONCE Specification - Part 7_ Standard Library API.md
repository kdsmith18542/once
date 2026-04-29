# **ONCE Specification \- Part 7: Standard Library API**

| Document ID | ONCE-007 |
| :---- | :---- |
| **Title** | Standard Library API |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-003, ONCE-004 |

## **1\. Introduction**

This document provides an overview of the Once Standard Library (std). The library's design adheres to the core principles of the language: safety, clarity, and performance. It provides a minimal but powerful set of tools for common programming tasks, with a strong emphasis on verifiable resource management and explicit side effects.

All functions and types mentioned here reside under the std namespace (e.g., std::io::File). A small prelude automatically imports the most essential types like Option, Result, Vec, Str, and Int.

## **2\. Design Principles**

* **Minimalism and Composability:** The library provides essential, orthogonal building blocks rather than a sprawling, all-encompassing framework.  
* **Safety by Default:** APIs are designed to be misuse-resistant. Error handling is explicit via the Result type, and resource management is guaranteed through linear types.  
* **Transparent Effects:** All functions that interact with the outside world have explicit effect signatures, as defined in ONCE-003.  
* **Performance:** APIs are designed to be zero-cost abstractions where possible, giving developers predictable performance.

## **3\. Core Modules**

### **3.1. std::core \- Core Primitives and Traits**

This module contains the most fundamental types and traits.

* **Types:**  
  * Option\<T\>: Represents an optional value (Some(T) or None).  
  * Result\<T, E\>: Represents a value that can be a success (Ok(T)) or a failure (Err(E)).  
* **Traits:**  
  * Show: For converting a type to a human-readable string representation (fn show(self) \-\> Str).  
  * Eq: For testing equality between two values (fn eq(self, other: Self) \-\> Bool).  
  * Ord: For defining a total ordering between two values.  
  * Resource: The trait for all linear, managed resources (see ONCE-004).

### **3.2. std::collections \- Data Structures**

This module provides a small set of efficient, general-purpose collection types.

* Vec\<T\>: A growable, heap-allocated vector.  
* Map\<K, V\>: A hash map.  
* Set\<T\>: A hash set.  
* Array\<T, N\>: A fixed-size array, typically stack-allocated. While a language primitive, its methods are defined here.

### **3.3. std::io \- Input/Output**

The io module provides tools for interacting with the filesystem and other I/O streams. All I/O types that represent an operating system resource are **linear**.

* **File (linear)**: Represents an open file handle.  
  * File::open(path: Str) \-\> Result\<lin File, Error\> \!io  
  * fn read\_to\_string(self: lin Self) \-\> Result\<Str, Error\> \!io  
  * fn write(self, bytes: Bytes) \-\> Result\<(), Error\> \!io  
* **Traits:**  
  * Reader: An abstraction for types that can be read from.  
  * Writer: An abstraction for types that can be written to.

*Example:*

import std::io::File;

fn log(message: Str) \-\> Result\<(), Error\> \!io {  
  // \`f\` is a linear resource.  
  using f \= File::create("log.txt")? {  
    f.write(message)?;  
  } // \`f\` is automatically closed here.  
  Ok(())  
}

### **3.4. std::net \- Networking**

The net module provides primitives for network communication. Like io, all socket types are **linear**.

* TcpListener (linear): A TCP socket server.  
  * fn bind(addr: Str) \-\> Result\<lin TcpListener, Error\> \!net  
  * fn accept(self) \-\> Result\<(lin TcpStream, Str), Error\> \!net  
* TcpStream (linear): A TCP stream between a local and a remote socket.

### **3.5. std::concurrency \- Concurrency Primitives**

This module provides the core tools for concurrent programming.

* Chan\<T\>: A channel for message passing between processes.  
  * Chan::new() \-\> Chan\<T\>  
  * fn send(self, value: T) \-\> Result\<(), Error\> \!time  
  * fn recv(self) \-\> Result\<T, Error\> \!time  
* spawn: The keyword used to create a new lightweight process. Its use introduces the \!spawn effect.  
* group: A block for structured concurrency, ensuring all spawned processes within it are completed before exiting the scope.  
* Actor: A helper for building stateful, concurrent entities that communicate via messages.

*Example:*

import std::concurrency::{Chan, spawn};

fn main() \!spawn {  
  let ch \= Chan::new();

  spawn {  
    // This runs in a new process.  
    ch.send("Hello from a process\!")?;  
  };

  let message \= ch.recv()?;  
  // Prints "Hello from a process\!"  
  print(message);  
}

### **3.6. std::time \- Time and Durations**

Provides types for measuring time. Accessing system time is an effect.

* Instant: A monotonic, opaque point in time.  
* Duration: The elapsed time between two Instants.  
* SystemTime: Represents calendar time.  
  * SystemTime::now() \-\> SystemTime \!time