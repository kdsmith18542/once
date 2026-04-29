# **ONCE Specification \- Part 5: AI-Integration Layer & Goal-Oriented Syntax**

| Document ID | ONCE-005 |
| :---- | :---- |
| **Title** | AI-Integration Layer & Goal-Oriented Syntax |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-001, ONCE-002 |

## **1\. Introduction**

This document specifies the **Goal-Oriented Language Layer** of Once. As outlined in the project vision (ONCE-001), Once is designed with a dual interface: a verifiable **Core Language** and a high-level **Goal Language**. This document defines the latter.

The Goal Language is the primary interface for human and AI collaboration. It allows developers to specify functionality by describing *intent* ("what") rather than implementation ("how"). An AI-augmented compiler, hereafter referred to as the **Semantic Compiler**, is responsible for translating these high-level goals into verifiable Core Language code, which is then formally checked by the standard Once compiler.

## **2\. The goal Declaration**

The central feature of the Goal Language is the goal declaration. Syntactically, it resembles a function declaration but contains descriptive clauses instead of an implementation block.

### **2.1. Grammar**

goal\_decl ::= \[ "export" \] "goal" ident \[ type\_params \] "(" \[ param\_list \] ")" \[ "-\>" type\_expr \] "{" { goal\_clause } "}"

goal\_clause ::= "spec" ":" string\_literal ";"  
              | "constraints" ":" "\[" { constraint\_item } "\]" ";"  
              | "examples" ":" "\[" { example\_item } "\]" ";"

constraint\_item ::= string\_literal ","  
example\_item ::= "in" "(" { expr } ")" "-\>" "out" expr ","

### **2.2. Example**

export goal factorial(n: Int) \-\> Int {  
  spec: "Calculates the factorial of a non-negative integer \`n\`.";

  constraints: \[  
    "Must be a pure function with no side effects \!\[\]",  
    "Should return 1 if n is 0.",  
    "Should handle n up to 20 without overflow on a 64-bit Int.",  
    "Prefer a recursive implementation if it is clear and efficient.",  
  \];

  examples: \[  
    in(0) \-\> out 1,  
    in(1) \-\> out 1,  
    in(5) \-\> out 120,  
  \];  
}

## **3\. Semantic Components of a goal**

A goal declaration is composed of three key clauses that collectively define the specification for the code to be generated.

### **3.1. spec Clause**

* **Purpose:** Provides a high-level, natural language description of the goal's functionality.  
* **Cardinality:** Exactly one spec clause must be present.  
* **Role:** This is the primary input to the Semantic Compiler's underlying Large Language Model (LLM). It should be clear, concise, and unambiguous.

### **3.2. constraints Clause**

* **Purpose:** Provides a list of formal or semi-formal rules and requirements that the generated code must adhere to.  
* **Cardinality:** Optional.  
* **Role:** This clause is critical for guiding the AI and ensuring the generated code meets specific non-functional requirements. Constraints can include:  
  * **Effect Signatures:** "Must be pure \!\[\]" or "Allowed effects: \!\[io\]". The core compiler will formally verify this.  
  * **Performance:** "Prefer iteration over recursion", "Avoid heap allocations".  
  * **Algorithmic Choices:** "Use a quicksort algorithm".  
  * **Logical Invariants:** "The output vector must be sorted".

### **3.3. examples Clause**

* **Purpose:** Provides a list of concrete input-output pairs that serve as a simple, executable specification.  
* **Cardinality:** Optional, but highly recommended.  
* **Role:** Serves two functions:  
  1. **For the AI:** Clarifies the spec with concrete examples, especially for edge cases.  
  2. **For the Compiler:** The core compiler automatically uses these examples to generate a unit test suite for the AI-generated code. The build fails if the generated code does not pass these tests.

## **4\. The AI-Augmented Compilation Process**

The compilation of a goal block is a multi-stage process designed for safety and verifiability.

1. **Parsing & Analysis:** The once compiler front-end parses the goal block.  
2. **Prompt Construction:** The compiler constructs a structured prompt for the Semantic Compiler (LLM). This prompt includes the spec, constraints, examples, function signature, and relevant context from the surrounding code (e.g., definitions of types used in the signature).  
3. **Code Generation:** The Semantic Compiler receives the prompt and generates an implementation for the goal in the **Once Core Language**.  
4. **Formal Verification:** The generated Core Language code is passed to the standard oncec compiler pipeline.  
   * The **type checker** verifies its type correctness.  
   * The **linearity checker** verifies its resource safety.  
   * The **effect checker** verifies that its effects match the constraints.  
   * If any of these checks fail, the compilation fails with an error indicating that the AI-generated code was invalid, often with a suggestion to refine the goal specification.  
5. **Test Execution:** If verification succeeds, the compiler generates and runs a test function based on the examples clause. If any example fails, the compilation fails.  
6. **Codegen:** Only after passing both formal verification and testing does the compiler proceed to generate the final machine code.

## **5\. Tooling and Developer Experience**

### **5.1. Caching**

To ensure build determinism and speed, the output of the Semantic Compiler is cached. The cache key is a hash of the goal block's content and the relevant contextual type definitions. A regeneration only occurs if the goal itself is modified.

### **5.2. "Ejecting" to Core Code**

Developers must have the ability to take manual control. IDE tooling will provide a "Convert to Core" or "Eject" command. This action replaces the goal block with its latest successfully generated Core Language fn implementation. This generated code is then checked into source control and becomes the canonical source, maintained by the developer going forward. This is useful for performance tuning or handling complex logic that the AI cannot perfectly generate.