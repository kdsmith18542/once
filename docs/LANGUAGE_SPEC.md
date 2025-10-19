# Once Language Specification

This document provides a comprehensive specification of the Once programming language.

## Table of Contents

1. [Lexical Structure](#lexical-structure)
2. [Syntax](#syntax)
3. [Type System](#type-system)
4. [Memory Management](#memory-management)
5. [Concurrency](#concurrency)
6. [Standard Library](#standard-library)
7. [Examples](#examples)

## Lexical Structure

### Keywords

```
async, await, break, case, catch, class, const, continue, default, do, else, 
enum, export, extends, finally, for, function, if, implements, import, in, 
instanceof, interface, let, new, package, private, protected, public, return, 
static, super, switch, this, throw, try, typeof, var, void, while, with, yield
```

### Identifiers

Identifiers follow the standard rules:
- Start with a letter or underscore
- Can contain letters, digits, and underscores
- Case-sensitive

### Literals

#### Integer Literals
```once
42          // Decimal
0x2A        // Hexadecimal
0o52        // Octal
0b101010    // Binary
```

#### Floating-Point Literals
```once
3.14        // Standard notation
1.23e-4     // Scientific notation
```

#### String Literals
```once
"Hello, World!"     // Double quotes
'Hello, World!'     // Single quotes
```

#### Boolean Literals
```once
true
false
```

## Syntax

### Function Definitions

```once
fn function_name(param1: Type1, param2: Type2) -> ReturnType {
    // Function body
}
```

### Variable Declarations

```once
let variable_name: Type = expression;
var mutable_variable: Type = expression;
```

### Control Flow

#### If Statements
```once
if condition {
    // Then branch
} else {
    // Else branch
}
```

#### Match Expressions
```once
match expression {
    pattern1 => result1,
    pattern2 => result2,
    _ => default_result
}
```

#### Loops
```once
for item in collection {
    // Loop body
}

while condition {
    // Loop body
}
```

### Type Definitions

#### Structs
```once
struct Point {
    x: Float,
    y: Float
}
```

#### Enums
```once
enum Option<T> {
    Some(T),
    None
}
```

## Type System

### Primitive Types

- `Int`: 64-bit signed integer
- `Float`: 64-bit floating-point number
- `Bool`: Boolean value
- `Str`: String (UTF-8 encoded)
- `Unit`: Unit type (similar to `void`)

### Linear Types

Linear types must be consumed exactly once:

```once
fn process_file(path: Str) -> Unit !io {
    using file = File::open(path) {
        let content = file.read_all();
        print(content)
    }
    // file is automatically closed here
}
```

### Effect Types

Effect types track computational effects:

```once
fn read_file(path: Str) -> Str !io {
    // I/O operations
}

fn spawn_task() -> Task<Unit> !spawn {
    // Spawn operations
}
```

### Generic Types

```once
fn map<A, B>(f: fn(A) -> B, xs: Vec<A>) -> Vec<B> {
    // Implementation
}
```

## Memory Management

### Region-Based Memory Management

Once uses region-based memory management, which provides:

1. **Automatic Memory Management**: No manual memory management required
2. **No Garbage Collection**: Zero runtime overhead
3. **Static Analysis**: Compile-time memory safety guarantees
4. **Escape Analysis**: Automatic detection of memory leaks

### Linear Types

Linear types ensure resource safety:

```once
fn consume_resource(resource: LinearResource) -> Unit {
    // Resource is consumed here
    // Cannot be used again
}
```

### Region Inference

The compiler automatically infers memory lifetimes:

```once
fn create_string() -> Str {
    let s = "Hello, World!";
    s  // Lifetime is inferred
}
```

## Concurrency

### Actors

Actors provide isolated processes with message-passing:

```once
fn main() -> Unit !spawn {
    let system = ActorSystem::new();
    let actor = system.spawn("worker", worker_behavior);
    actor.send("Hello, Actor!");
}

fn worker_behavior(msg: Str) -> Unit {
    print("Received: ".concat(msg))
}
```

### Channels

Channels provide type-safe communication:

```once
fn producer(output: Chan<Str>) -> Unit !spawn {
    output.send("Hello from producer")
}

fn consumer(input: Chan<Str>) -> Unit !io {
    let msg = input.recv();
    print(msg);
}
```

### Async/Await

Structured concurrency with async/await:

```once
async fn fetch_data(url: Str) -> Str !async {
    // Async operations
}

fn main() -> Unit !async {
    let task = async { fetch_data("http://example.com") };
    let result = await task;
    print(result);
}
```

## Standard Library

### Core Types

#### Linear Types
- `File`: File handle (must be closed)
- `TcpStream`: TCP connection (must be closed)
- `Channel<T>`: Communication channel
- `Task<T>`: Async task handle

#### Collections
- `Vec<T>`: Dynamic array
- `HashMap<K, V>`: Hash map
- `Set<T>`: Set data structure

### I/O Operations

```once
use once_std::io::File;

fn read_file(path: Str) -> Str !io {
    using file = File::open(path) {
        file.read_all()
    }
}
```

### Concurrency Primitives

```once
use once_runtime::channel::Chan;
use once_actors::{ActorSystem, ActorRef};

fn main() -> Unit !spawn {
    let system = ActorSystem::new();
    let (tx, rx) = Chan::new();
    
    spawn producer(tx);
    spawn consumer(rx);
}
```

## Examples

### Hello World

```once
fn main() -> Unit {
    print("Hello, World!")
}
```

### Linear Resources

```once
use once_std::io::File;

fn write_log(path: Str, line: Str) -> Unit !io {
    using f = File::create(path) {
        f.write(line)
    }
}

fn main() -> Unit !io {
    write_log("log.txt", "This is a log entry.")
}
```

### Concurrency

```once
use once_runtime::channel::Chan;

fn producer(output: Chan<Str>) -> Unit !spawn {
    output.send("Hello from producer")
}

fn consumer(input: Chan<Str>) -> Unit !io {
    let msg = input.recv();
    print(msg);
}

fn main() -> Unit !spawn, io {
    let (tx, rx) = Chan::new();
    
    spawn producer(tx);
    spawn consumer(rx);
}
```

### Pattern Matching

```once
fn factorial(n: Int) -> Int {
    match n {
        0 => 1,
        n => n * factorial(n - 1)
    }
}
```

### Generic Functions

```once
fn map<A, B>(f: fn(A) -> B, xs: Vec<A>) -> Vec<B> {
    var result = Vec::new();
    for x in xs {
        result.push(f(x));
    }
    result
}
```

### Error Handling

```once
fn divide(a: Int, b: Int) -> Result<Int, Str> {
    if b == 0 {
        Err("Division by zero")
    } else {
        Ok(a / b)
    }
}
```

## Advanced Features

### Effect Polymorphism

```once
fn read_file(path: Str) -> Str !io {
    // I/O operations
}

fn spawn_task() -> Task<Unit> !spawn {
    // Spawn operations
}
```

### Region Inference

```once
fn create_string() -> Str {
    let s = "Hello, World!";
    s  // Lifetime is inferred
}
```

### Linear Types

```once
fn consume_resource(resource: LinearResource) -> Unit {
    // Resource is consumed here
}
```

## Conclusion

Once provides a modern, safe, and efficient systems programming language with advanced features like linear types, region-based memory management, and structured concurrency. The language is designed to be both powerful and easy to learn, making it suitable for a wide range of applications.
