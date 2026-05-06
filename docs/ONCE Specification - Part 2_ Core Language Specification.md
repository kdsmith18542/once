# **ONCE Specification – Part 2: Core Language Specification**

| Document ID | ONCE-002 |
| :---- | :---- |
| **Title** | Core Language Specification |
| **Version** | 1.1 |
| **Date** | 2026-05-03 |
| **Status** | Draft |
| **Supersedes** | ONCE-002 v1.0 |
| **Related Docs** | ONCE-001, ONCE-003, ONCE-004, ONCE-009 |

## **1. Introduction**

This document provides a formal specification of the **Once Core Language**. It defines the lexical structure, syntax, and grammar that all conforming Once compilers must implement. This specification is concerned with the structure of valid Once programs; the deeper semantics of the type system, memory model, and effect system are detailed in subsequent documents (ONCE-003 and ONCE-004).

The grammar is presented in Extended Backus-Naur Form (EBNF).

## **2. Source Files**

- **Encoding:** Once source files must be encoded in **UTF-8**.
- **File Extension:** Once source files use the `.onc` extension.
- **Module Mapping:** Each `.onc` file corresponds to a single module. The module's path is determined by its location within the source directory, as specified in ONCE-006.

## **3. Lexical Structure**

### **3.1. Whitespace**

Whitespace characters (space, tab, newline, carriage return) are used to separate tokens. Other than acting as separators, whitespace has no semantic meaning.

### **3.2. Comments**

Once supports two forms of comments:

- **Line Comments:** Begin with `//` and extend to the end of the line.
```
// This is a line comment.
let x = 10 // An inline comment.
```

- **Block Comments:** Begin with `/*` and end with `*/`. Block comments can be nested.
```
/* This is a block comment.
   It can span multiple lines.
   /* A nested comment. */
*/
```

### **3.3. Keywords**

The following are reserved keywords and may not be used as identifiers:

| Keywords |  |  |  |  |
| :---- | :---- | :---- | :---- | :---- |
| actor | as | async | await | break |
| const | continue | else | enum | export |
| fn | for | if | impl | import |
| in | let | lin | match | module |
| mut | pub | return | self | spawn |
| trait | type | unsafe | using | var |
| where | | | | |

The following are reserved for future use:

| Reserved |  |  |  |  |
| :---- | :---- | :---- | :---- | :---- |
| class | defer | do | goto | interface |
| package | super | union | volatile | yield |

### **3.4. Identifiers**

Identifiers are used to name variables, types, functions, and other program constructs.

- **Rules:** Must begin with an alphabetic character or an underscore (`_`), followed by any number of alphanumeric characters or underscores.
- **Style Convention:** `UpperCamelCase` for types and `snake_case` for all other bindings (variables, functions, module aliases).
- **Grammar:**
```
ident ::= ('a'..'z' | 'A'..'Z' | '_') ('a'..'z' | 'A'..'Z' | '0'..'9' | '_')*
```

### **3.5. Literals**

- **Integers:** `123`, `0x1A`, `0b1011`, `-42`
- **Floating-Point:** `3.14`, `-0.5`, `1.0e-5`
- **Booleans:** `true`, `false`
- **Strings:** Enclosed in double quotes (`"`). `"Hello, world!"`. Escape sequences use a backslash (`\n`, `\t`, `\\`, `\"`).
- **Unit:** The unit type has one literal value: `()`.

```
literal ::= int_lit | float_lit | bool_lit | string_lit | unit_lit
int_lit ::= dec_lit | hex_lit | bin_lit
dec_lit ::= [ '-' ] digit { digit }
hex_lit ::= '0' 'x' hex_digit { hex_digit }
bin_lit ::= '0' 'b' ('0' | '1') { '0' | '1' }
float_lit ::= [ '-' ] digit { digit } '.' digit { digit } [ ('e' | 'E') [ '-' ] digit { digit } ]
bool_lit ::= 'true' | 'false'
string_lit ::= '"' { char } '"'
unit_lit ::= '(' ')'
```

## **4. Complete Grammar**

### **4.1. Program Structure**

```
Program   ::= { Item }
Item      ::= FnDecl | TypeDecl | TraitDecl | ImplDecl | LetDecl | ImportDecl
```

### **4.2. Imports**

Imports bring external modules or items into the current scope. All imports must be absolute paths from the project root or a named dependency.

```
ImportDecl ::= 'import' ModulePath [ 'as' ident ] ';'
             | 'import' ModulePath '::' '{' [ ident { ',' ident } ] '}' ';'
ModulePath ::= ident { '::' ident }
```

*Example:*
```
import std::io;
import std::net::http as web;
import std::collections::{ Map, Set };
```

### **4.3. Function Declarations**

```
FnDecl     ::= [ 'export' ] 'fn' ident [ TypeParams ] '(' [ ParamList ] ')' [ ReturnAnn ] [ OptEffects ] BlockExpr
ParamList  ::= Param { ',' Param }
Param      ::= ident ':' TypeExpr
ReturnAnn  ::= '->' TypeExpr
OptEffects ::= '!' '[' [ Effect { ',' Effect } ] ']'
Effect     ::= ident | ident '[' ident ']'
TypeParams ::= '<' ident { ',' ident } '>'
```

*Example:*
```
fn identity(x: Int) -> Int {
  return x
}

export fn add(a: Int, b: Int) -> Int {
  a + b
}
```

### **4.4. Type Declarations**

```
TypeDecl  ::= [ 'export' ] 'type' ident [ TypeParams ] '=' TypeBody ';'
TypeBody  ::= Variant { '|' Variant }
Variant   ::= ident [ '(' TypeExpr { ',' TypeExpr } ')' ]
            | '{' [ FieldDecl { ',' FieldDecl } ] '}'
FieldDecl ::= ident ':' TypeExpr

TypeExpr  ::= ident [ '<' TypeExpr { ',' TypeExpr } '>' ]
            | '(' TypeExpr { ',' TypeExpr } ')'
            | '[' TypeExpr ';' Nat ']'             -- sized array
            | 'Vec' '<' TypeExpr '>'               -- growable vector
            | 'Option' '<' TypeExpr '>'
            | 'lin' TypeExpr                        -- linear type annotation
            | 'Task' '<' TypeExpr '>'              -- task handle
            | 'Chan' '<' TypeExpr '>'              -- channel handle
            | 'fn' '(' [ TypeExpr { ',' TypeExpr } ] ')' [ '->' TypeExpr ]  -- function type
```

*Example:*
```
type Option<T> = Some(T) | None;

type User = {
  id: Int,
  name: Str,
  email: Option<Str>,
};

type Result<T, E> = Ok(T) | Err(E);
```

### **4.5. Trait and Implementation Declarations**

```
TraitDecl ::= [ 'export' ] 'trait' ident [ TypeParams ] '{' { FnSignature } '}'
FnSignature ::= 'fn' ident '(' [ ParamList ] ')' [ '->' TypeExpr ] ';'
ImplDecl  ::= 'impl' [ TypeParams ] ident [ '<' TypeExpr { ',' TypeExpr } '>' ] 'for' TypeExpr '{' { FnDecl } '}'
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

### **4.6. Statements & Expressions**

Once is an expression-oriented language. Most constructs are expressions that evaluate to a value. Statements are constructs that do not evaluate to a value (their type is `()`).

```
Stmt      ::= LetStmt | VarStmt | ExprStmt | ReturnStmt | UsingStmt
LetStmt   ::= 'let' ident [ ':' TypeExpr ] '=' Expr ';'
VarStmt   ::= 'var' ident [ ':' TypeExpr ] '=' Expr ';'
ExprStmt  ::= Expr ';'
ReturnStmt ::= 'return' [ Expr ] ';'

UsingStmt ::= [ 'using' | 'using!' ] ident '=' Expr BlockExpr
```

*Example:*
```
let x = 42;
var counter = 0;
counter = counter + 1;
return x;
```

### **4.7. Full Expression Grammar**

Expressions are ordered by decreasing precedence.

```
Expr ::= PipelineExpr

PipelineExpr ::= BinaryExpr { '|>' BinaryExpr }

BinaryExpr ::= UnaryExpr { BinOp UnaryExpr }
BinOp ::= '||' | '&&'
        | '==' | '!=' | '<' | '<=' | '>' | '>='
        | '+' | '-' | '*' | '/' | '%'

UnaryExpr ::= [ '-' | '!' ] PostfixExpr

PostfixExpr ::= PrimaryExpr { PostfixSuffix }
PostfixSuffix ::= '.' ident                                    -- field access
                | '.' ident '(' [ Expr { ',' Expr } ] ')'     -- method call
                | '(' [ Expr { ',' Expr } ] ')'               -- function call
                | '[' Expr ']'                                 -- indexing
                | '?'                                          -- error propagation

PrimaryExpr ::= literal
              | ident
              | '(' Expr ')'
              | BlockExpr
              | IfExpr
              | MatchExpr
              | ForLoop
              | WhileLoop
              | LambdaExpr
              | AsyncExpr
              | AwaitExpr
              | SpawnExpr
              | GroupExpr
              | ReturnExpr
```

### **4.8. Block Expressions**

```
BlockExpr  ::= '{' { Stmt } [ Expr ] '}'
```

A block evaluates to the value of its final expression, or `()` if no final expression.

### **4.9. Control Flow**

```
IfExpr     ::= 'if' Expr BlockExpr [ 'else' ( IfExpr | BlockExpr ) ]
```

An `if` without an `else` evaluates to `()`. If both branches have the same type `T`, the expression's type is `T`.

```
MatchExpr  ::= 'match' Expr '{' { MatchArm } '}'
MatchArm   ::= Pattern [ Guard ] '=>' ( Expr ',' | BlockExpr )
Pattern    ::= literal
             | ident
             | ident '(' [ Pattern { ',' Pattern } ] ')'
             | '{' [ ident ':' Pattern { ',' ident ':' Pattern } ] '}'
             | '_'
Guard      ::= 'if' Expr
```

*Example:*
```
match value {
  Some(x) if x > 0 => process(x),
  Some(x) => handle_zero_or_neg(x),
  None => default_value,
}
```

```
ForLoop    ::= 'for' ident 'in' Expr BlockExpr
WhileLoop  ::= 'while' Expr BlockExpr
```

### **4.10. Closure Expressions**

```
LambdaExpr ::= '|' [ ident { ',' ident } ] '|' [ '->' TypeExpr ] BlockExpr
```

Closure capture rules (defined in ONCE-003 §Linear Type Rules):
- Capturing a `Copy` type copies the value.
- Capturing a non-`Copy` type moves the value into the closure.
- Capturing a linear type makes the closure itself linear (`FnOnce`).

### **4.11. Async/Await Expressions**

```
AsyncExpr  ::= 'async' BlockExpr
AwaitExpr  ::= 'await' Expr
```

`async { e }` produces a `Task<T>` handle (linear). `await t` consumes the task handle and yields `T`. Defined formally in ONCE-003 §Async Semantics.

### **4.12. Concurrency Expressions**

```
SpawnExpr  ::= 'spawn' BlockExpr
GroupExpr  ::= 'group' [ '(' 'policy' '=' GroupPolicy ')' ] BlockExpr
GroupPolicy ::= 'FailFast' | 'All' | 'Supervisor'
```

`spawn { e }` introduces the `!spawn` effect and produces a `Task<T>`. `group` defines a structured concurrency scope where all spawned children must complete before exit.

### **4.13. Error Propagation**

The postfix `?` operator propagates errors from `Result<T, E>` types:

```
// expr? desugars to:
match expr {
  Ok(v) => v,
  Err(e) => return Err(e),
}
```

## **5. Operator Precedence Table**

Operators are listed from highest to lowest precedence. Higher-precedence operators bind tighter.

| Precedence | Operators | Associativity | Description |
|:-----------|:----------|:--------------|:------------|
| 12 (highest) | `.` `()` `[]` `?` | Left | Postfix: field access, call, index, error propagation |
| 11 | `-` (unary) `!` | Right | Unary negation, logical not |
| 10 | `*` `/` `%` | Left | Multiplication, division, remainder |
| 9 | `+` `-` | Left | Addition, subtraction |
| 8 | `<` `<=` `>` `>=` | Left | Comparison |
| 7 | `==` `!=` | Left | Equality |
| 6 | `&&` | Left | Logical AND |
| 5 | `\|\|` | Left | Logical OR |
| 4 | `..` | Left | Range |
| 3 | `=` `+=` `-=` etc. | Right | Assignment |
| 2 | `\|>` | Left | Pipeline |
| 1 (lowest) | `;` | — | Statement separator |

## **6. Pipeline Operator `|>`**

### **6.1. Syntax**

```
PipelineExpr ::= BinaryExpr { '|>' BinaryExpr }
```

The pipeline operator `|>` is left-associative and has the lowest precedence of all expression operators (level 2).

### **6.2. Desugaring Rules**

**Unary pipeline**: `x |> f` desugars to `f(x)`.

**Binary pipeline**: `x |> f(a)` desugars to `f(x, a)`.

**Chained pipeline**: `x |> f(a) |> g` desugars to `g(f(x, a))`.

*Example:*
```
let total = lines
  |> map(parse_int)
  |> filter(is_positive)
  |> sum()
// Desugars to: sum(filter(map(lines, parse_int), is_positive))
```

## **7. Modules & Imports**

### **7.1. Module Hierarchy**

Module paths follow a namespace hierarchy separated by `::`. The root of the hierarchy is the package source directory.

```
ModulePath ::= ident { '::' ident }
```

- **Absolute imports only**: All imports are relative to the project root or a named dependency root.
- **No wildcard imports**: All imported names must be explicitly listed or aliased.
- **Version-pinned**: Dependency versions are resolved from the lockfile (`once.lock`).

*Example:*
```
import std::io::File
import std::collections::{ Map, Set }
import http@3 as http3
```

### **7.2. Visibility**

- **`pub`**: Exported from the module, visible to importing modules.
- **Default (private)**: Visible only within the current module.
- **`export`**: Top-level items (functions, types, traits) marked `export` are part of the public API and subject to semver compatibility checking.

### **7.3. Effect Visibility**

Public functions must declare their effect signatures explicitly. The compiler verifies that the declared effects match the inferred effects. Adding an effect to a public function is a breaking change (major version bump).

```
// Public: must declare effects
export fn read_config(path: Str) -> Config !io { ... }

// Private: effects can be inferred (optional annotation)
fn helper() -> Str !io { ... }
```

## **8. Function Parameters (Named & Default)**

### **8.1. Named Parameters**

Function calls support named arguments for clarity:

```
fn connect(host: Str, port: Int = 443) -> TcpStream !net { ... }

// Positional call
let stream = connect("localhost", 8080)

// Named call (order-independent)
let stream = connect(port=8080, host="localhost")

// Partial named + default
let stream = connect("localhost")  // uses default port=443
```

### **8.2. Default Parameter Values**

Default values must be compile-time constants:
- Literals: `42`, `true`, `"localhost"`
- `()` (unit)
- Enum variants without fields: `None`
- `const` expressions

*Example:*
```
fn create_window(title: Str, width: Int = 800, height: Int = 600) -> Window { ... }
```

## **9. Patterns**

### **9.1. Pattern Grammar**

```
Pattern ::= literal                              -- matches exact literal
          | ident                                 -- binding pattern
          | ident '(' [ Pattern { ',' Pattern } ] ')'  -- enum variant
          | '{' [ ident ':' Pattern { ',' ident ':' Pattern } ] '}'  -- record destructure
          | '_'                                   -- wildcard
```

### **9.2. Pattern Guards**

Pattern arms in `match` expressions may include an optional guard clause:

```
MatchArm ::= Pattern [ Guard ] '=>' ( Expr ',' | BlockExpr )
Guard ::= 'if' Expr
```

**Semantics**: The guard expression is evaluated after the pattern matches. If the guard evaluates to `true`, the arm is taken. If the guard evaluates to `false`, the match falls through to the next arm as if the pattern had not matched.

*Example:*
```
match value {
  Some(x) if x > 0 => process_positive(x),
  Some(x) if x < 0 => process_negative(x),
  Some(x) => process_zero(x),
  None => default_value,
}
```

**Desugaring (MIR)**: The guard condition is emitted as a `Branch` after the pattern-match label. If the guard fails, control jumps to the next arm's label.

## **10. Source Spans**

### **10.1. Span Definition**

```
Span ::= { file: String, start_line: Nat, start_col: Nat, end_line: Nat, end_col: Nat }
```

### **10.2. Span Coverage**

Every AST node carries an optional source span for end-to-end traceability:

| Construct | Span Target |
|:----------|:------------|
| Top-level items (`fn`, `type`, `trait`, `impl`, `import`) | Full declaration |
| Statements (`let`, `var`, `return`, `using`, expression statements) | Full statement |
| Expressions (all forms) | Full expression with sub-expressions |
| Patterns | Full pattern with sub-patterns |
| HIR nodes | Carried through from AST |
| MIR operations | Carried through from HIR |

### **10.3. Diagnostic Integration**

Every type error, linearity violation, effect mismatch, and region error carries the source span pointing to the exact expression involved. This enables precise error messages at any nesting level:

```
Error at main.onc:15:22:
  Linear value 'f' used after move.
  Value moved at main.onc:15:10.
  Second use at main.onc:15:22.
```

## **11. A Note on Advanced Topics**

This document specifies the core surface syntax. The following critical language features have syntactic components defined here but are specified semantically in other documents:

- **Traits and Implementations (`trait`/`impl`):** Defined in ONCE-003.
- **Effect Signatures (`![]`):** The syntax is reserved; semantics are in ONCE-003.
- **Linearity (`lin`):** The keyword is reserved; semantics are in ONCE-003.
- **Concurrency (`spawn`, `actor`, `group`):** Keywords are reserved; semantics are in ONCE-004.
- **Resource Management (`using`):** The keyword is reserved; semantics are in ONCE-004.
- **MIR Operations:** Lowering of all expression forms to MIR ops is defined in ONCE-009.
- **Pipeline Desugaring:** Formal rules in this document §6; MIR lowering in ONCE-009.
- **Async/Await:** Syntax defined here; type and linearity semantics in ONCE-003.
