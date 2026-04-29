# **ONCE Specification \- Part 2: Core Language Specification**

| Document ID | ONCE-002 |
| :---- | :---- |
| **Title** | Core Language Specification |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Supersedes** | N/A |
| **Related Docs** | ONCE-001, ONCE-003 |

## **1\. Introduction**

This document provides a formal specification of the **Once Core Language**. It defines the lexical structure, syntax, and grammar that all conforming Once compilers must implement. This specification is concerned with the structure of valid Once programs; the deeper semantics of the type system, memory model, and effect system are detailed in subsequent documents (ONCE-003 and ONCE-004).

The grammar is presented in a notation similar to Extended Backus-Naur Form (EBNF).

## **2\. Source Files**

* **Encoding:** Once source files must be encoded in **UTF-8**.  
* **File Extension:** Once source files use the .onc extension.  
* **Module Mapping:** Each .onc file corresponds to a single module. The module's path is determined by its location within the source directory, as specified in ONCE-006.

## **3\. Lexical Structure**

### **3.1. Whitespace**

Whitespace characters (space, tab, newline, carriage return) are used to separate tokens. Other than acting as separators, whitespace has no semantic meaning.

### **3.2. Comments**

Once supports two forms of comments:

* **Line Comments:** Begin with // and extend to the end of the line.  
  // This is a line comment.  
  let x \= 10 // An inline comment.

* **Block Comments:** Begin with /\* and end with \*/. Block comments can be nested.  
  /\* This is a block comment.  
     It can span multiple lines.  
     /\* A nested comment. \*/  
  \*/

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
| where |  |  |  |  |

The following are reserved for future use:

| Reserved |  |  |  |  |
| :---- | :---- | :---- | :---- | :---- |
| class | defer | do | goto | interface |
| package | super | union | volatile | yield |

### **3.4. Identifiers**

Identifiers are used to name variables, types, functions, and other program constructs.

* **Rules:** Must begin with an alphabetic character or an underscore (\_), followed by any number of alphanumeric characters or underscores.  
* **Style Convention:** UpperCamelCase for types and snake\_case for all other bindings (variables, functions, module aliases).  
* **Grammar:** ident ::= ('a'..'z' | 'A'..'Z' | '\_') ('a'..'z' | 'A'..'Z' | '0'..'9' | '\_')\*

### **3.5. Literals**

* **Integers:** 123, 0x1A, 0b1011, \-42  
* **Floating-Point:** 3.14, \-0.5, 1.0e-5  
* **Booleans:** true, false  
* **Strings:** Enclosed in double quotes ("). "Hello, world\!". Escape sequences use a backslash (\\n, \\t, \\\\, \\").  
* **Unit:** The unit type has one literal value: ().

## **4\. Grammar**

### **4.1. Top-Level Declarations (Items)**

A Once program is a collection of items within a module.

module ::= { item }

item ::= fn\_decl | type\_decl | trait\_decl | impl\_decl | import\_decl

### **4.2. Imports**

Imports bring external modules or items into the current scope. All imports must be absolute paths from the project root or a named dependency.

import\_decl ::= "import" module\_path \[ "as" ident \] ";"  
              | "import" module\_path "::" "{" \[ ident { "," ident } \] "}" ";"

module\_path ::= ident { "::" ident }

*Example:*

import std::io;  
import std::net::http as web;  
import std::collections::{ Map, Set };

### **4.3. Function Declarations**

fn\_decl ::= \[ "export" \] "fn" ident \[ type\_params \] "(" \[ param\_list \] ")" \[ "-\>" type\_expr \] block\_expr

param\_list ::= param { "," param }  
param      ::= ident ":" type\_expr

*Example:*

fn identity(x: Int) \-\> Int {  
  return x  
}

export fn add(a: Int, b: Int) \-\> Int {  
  a \+ b  
}

### **4.4. Type Declarations**

Once supports algebraic data types (ADTs) in the form of enum (sum types) and records (product types, a special case of enums).

type\_decl ::= \[ "export" \] "type" ident \[ type\_params \] "=" type\_body ";"

type\_body ::= variant { "|" variant }  
variant   ::= ident \[ "(" type\_expr { "," type\_expr } ")" \]  
            | "{" \[ field\_decl { "," field\_decl } \] "}"

field\_decl ::= ident ":" type\_expr

*Example:*

type Option\<T\> \= Some(T) | None;

type User \= {  
  id: Int,  
  name: Str,  
  email: Option\<Str\>,  
};

### **4.5. Statements & Expressions**

Once is an expression-oriented language. Most constructs are expressions that evaluate to a value. Statements are constructs that do not evaluate to a value (their type is ()).

stmt ::= let\_stmt | expr\_stmt | return\_stmt

expr\_stmt ::= expr ";"

let\_stmt ::= ("let" | "var") ident \[ ":" type\_expr \] "=" expr ";"

return\_stmt ::= "return" \[ expr \] ";"

### **4.6. Expressions**

Expressions are evaluated to produce a value. The following list is ordered by decreasing precedence.

expr ::= unary\_expr  
       | binary\_expr  
       | primary\_expr

primary\_expr ::= literal  
               | ident  
               | "(" expr ")"  
               | block\_expr  
               | if\_expr  
               | match\_expr  
               | ...

block\_expr ::= "{" { stmt } \[ expr \] "}"

### **4.7. Control Flow**

* **if Expression:**  
  if\_expr ::= "if" expr block\_expr \[ "else" ( if\_expr | block\_expr ) \]

  An if without an else evaluates to (). If both branches have the same type T, the expression's type is T.  
* **match Expression:** Provides exhaustive pattern matching.  
  match\_expr ::= "match" expr "{" { match\_arm } "}"  
  match\_arm  ::= pattern "=\>" ( expr "," | block\_expr )  
  pattern    ::= literal | ident | ident "(" \[ pattern { "," pattern } \] ")" | "{" ... "}" | "\_"

* **for Loop:** Iterates over a collection.  
  for\_loop ::= "for" ident "in" expr block\_expr

### **4.8. Operators**

* **Arithmetic:** \+, \-, \*, /, %  
* **Comparison:** \==, \!=, \<, \<=, \>, \>=  
* **Logical:** &&, ||, \!  
* **Pipeline:** |\> (for left-to-right function composition)

### **4.9. Function Calls**

call\_expr ::= primary\_expr "(" \[ expr { "," expr } \] ")"

### **4.10. Field Access & Methods**

field\_access ::= primary\_expr "." ident  
method\_call  ::= primary\_expr "." ident "(" \[ expr { "," expr } \] ")"

## **5\. A Note on Advanced Topics**

This document specifies the core surface syntax. The following critical language features have syntactic components defined here but are specified semantically in other documents:

* **Traits and Implementations (trait/impl):** Defined in ONCE-003.  
* **Effect Signatures (\!\[\]):** The syntax is reserved; semantics are in ONCE-003.  
* **Linearity (lin):** The keyword is reserved; semantics are in ONCE-003.  
* **Concurrency (spawn, actor):** Keywords are reserved; semantics are in ONCE-004.  
* **Resource Management (using):** The keyword is reserved; semantics are in ONCE-004.