# **ONCE Specification \- Part 3: Type System & Effects**

| Document ID | ONCE-003 |
| :---- | :---- |
| **Title** | Type System & Effects |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-001, ONCE-002 |

## **1\. Introduction**

This document specifies the semantic rules of the Once static type system. A primary goal of Once is to provide strong, verifiable safety guarantees at compile time. This is achieved through a combination of a powerful Hindley-Milner-style type system, a strict linearity discipline for resource management, and a novel, transparent effect system for managing side effects.

This specification assumes familiarity with the syntax defined in ONCE-002.

## **2\. Core Type System**

### **2.1. Primitive Types**

Once provides a set of built-in primitive types:

* **Int**: A signed, machine-width integer (i64 on 64-bit systems, i32 on 32-bit).  
* **Float**: A 64-bit floating-point number (IEEE 754).  
* **Bool**: A boolean value, either true or false.  
* **Str**: An immutable, UTF-8 encoded string.  
* **()**: The "unit" type, which has a single value () and is used to represent the absence of a meaningful value (e.g., for functions that return no value).

### **2.2. Composite Types**

* **Records:** Product types defined with curly braces. They are collections of named fields.  
  type Point \= { x: Float, y: Float };

* **Enums:** Sum types (ADTs) that define a type with a set of possible variants.  
  type Option\<T\> \= Some(T) | None;

* **Tuples:** Anonymous product types. (T1, T2, ...)  
* **Arrays:** Fixed-size, stack-allocated collections. \[T; N\] where N is a compile-time constant.  
* **Vectors:** Growable, heap-allocated collections. Vec\<T\>.

### **2.3. Type Inference**

Once employs full type inference for local bindings within function bodies. Developers are not required to annotate the types of let or var bindings. However, top-level function signatures (parameters and return types) **must** be explicitly annotated for clarity and to guarantee stable public APIs.

* **Type Holes (\_):** A developer can use \_ in place of an expression to ask the compiler what type is expected at that position. The compiler will report the inferred type as an error, guiding development.

## **3\. Generics & Traits**

### **3.1. Generics (Parametric Polymorphism)**

Types and functions can be parameterized by one or more type variables, which are specified in angle brackets.

// A generic function  
fn wrap\_in\_some\<T\>(value: T) \-\> Option\<T\> {  
  Some(value)  
}

// A generic type  
type Result\<T, E\> \= Ok(T) | Err(E);

### **3.2. Traits (Ad-hoc Polymorphism)**

Traits define a set of methods that a type must implement. They are Once's mechanism for abstracting over behavior.

trait\_decl ::= \[ "export" \] "trait" ident \[ type\_params \] "{" { fn\_signature } "}"  
fn\_signature ::= "fn" ident "(" \[ param\_list \] ")" \[ "-\>" type\_expr \] ";"

impl\_decl ::= "impl" \[ type\_params \] ident \[ "\<" ... "\>" \] "for" ident \[ "\<" ... "\>" \] "{" { fn\_decl } "}"

*Example:*

trait Show {  
  fn show(self) \-\> Str;  
}

impl Show for Int {  
  fn show(self) \-\> Str {  
    // ... implementation for converting Int to Str  
  }  
}

## **4\. Linear & Affine Types**

To guarantee resource safety without a garbage collector or manual lifetime management, Once partitions types into two categories: **ordinary** and **linear**.

* **Ordinary Types:** The default. These types can be freely copied, moved, and shared. All primitive types are ordinary.  
* **Linear/Affine Types:** Represent unique, owned resources like file handles, network sockets, or database transactions. They are governed by strict usage rules enforced by the compiler.  
  * **Linear (lin):** A value of a linear type **must be consumed exactly once** before it goes out of scope.  
  * **Affine:** A relaxed form of linear where a value may be consumed **at most once** (i.e., it can be dropped without use). The lin keyword is used in source, and the compiler determines if an affine or strictly linear check is required.

### **4.1. Rules of Linearity**

1. **Consumption:** A linear value is "consumed" when it is passed by value to a function, returned from a function, or a method is called that takes self by value (e.g., file.close()).  
2. **No Implicit Copies:** Linear types cannot be copied. Assigning a linear value to another variable *moves* it, invalidating the original binding.  
3. **Scoped Use:** A linear value must be consumed by the end of its scope. The using block is the primary mechanism for guaranteeing this.

*Example:*

fn process\_file(path: Str) \-\> Result\<(), IoError\> {  
  // File.open returns a \`lin File\`  
  let f: lin File \= File.open(path)?;

  // 'using' guarantees that \`f\` is consumed (closed) at the end of the block.  
  using f {  
    // We can use \`f\` here.  
    f.write("hello")?;  
  }  
  // \`f\` is no longer valid here; it has been consumed.

  return Ok(());  
}

## **5\. The Effect System**

The effect system makes side effects a transparent, verifiable part of a function's signature. This allows developers and tools to reason precisely about what a function does.

### **5.1. Effect Signature**

A function's effects are denoted by a \! followed by a list of effect kinds.

fn\_decl ::= ... \[ "\!" "\[" \[ effect { "," effect } \] "\]" \] block\_expr  
effect ::= "io" | "net" | "spawn" | "time" | "ffi" | ...

A function with no \! annotation is **guaranteed to be pure**.

### **5.2. Effect Inference and Propagation**

* **Inference:** Within a function body, the compiler infers the set of effects used. Calling a function with effect E adds E to the caller's effect set.  
* **Propagation:** Effects propagate up the call stack. If main calls A, and A calls B, and B has the effect io, then both A and main must also be marked with the io effect.  
* **Public APIs:** For stable interfaces, all exported functions **must** have their effect signatures explicitly annotated in the source code. The compiler will verify that the implementation matches the annotation. For private functions, annotations are optional.

### **5.3. Core Effect Kinds**

* **io**: Filesystem access.  
* **net**: Network communication.  
* **spawn**: Creating new concurrent processes/actors.  
* **time**: Accessing system time or sleeping.  
* **ffi**: Calling external code via the Foreign Function Interface.  
* **nondet**: Accessing sources of non-determinism (e.g., random number generation).

The effect system is extensible, allowing for user-defined effects in the future.