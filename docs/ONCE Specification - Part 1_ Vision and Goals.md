# **ONCE Specification \- Part 1: Vision and Goals**

| Document ID | ONCE-001 |
| :---- | :---- |
| Title | Vision and Goals |
| Version | 1.0 |
| Date | 2025-10-20 |
| Status | Draft |

## **1\. Abstract**

This document outlines the vision, design philosophy, and overarching goals for the **Once** programming language. Once is a modern systems programming language designed to provide verifiable guarantees of memory safety, data-race freedom, and resource management *by construction*.

It introduces a novel dual-interface architecture:

1. A **Core Language** with a simple, explicit syntax, a powerful static type system, linear resource management, and a visible effect system. This core is designed to be unambiguously analyzable, making it an ideal, verifiable target for automated tools.  
2. A **Goal-Oriented Language Layer** designed for human and AI collaboration. This layer allows developers to specify program behavior through high-level descriptions, constraints, and examples, which are then compiled into the verifiable Core Language by an AI-augmented compiler.

The primary objective of Once is to dramatically increase software reliability and developer productivity by making correctness verifiable and harnessing the power of modern AI for code generation, without sacrificing performance or control.

## **2\. Design Philosophy**

The development of Once is guided by four fundamental principles that inform every aspect of its design, from syntax to semantics to tooling.

### **2.1. Provable Safety by Construction**

Safety should not be an afterthought or the result of complex, fallible analysis; it should be an intrinsic property of the language's structure.

* **No Lifetimes in User Code:** The cognitive overhead of manual lifetime management is a significant barrier. Once shifts this burden to the compiler via **Region-Based Memory Management (RMM)** with static inference.  
* **Explicit Resource Management:** Resources that can leak or be misused (files, sockets, transactions) are modeled as **linear types**, ensuring they are used exactly once. The using construct provides ergonomic, guaranteed consumption.  
* **Freedom from Data Races:** The type system enforces a "**Mutable XOR Shared**" invariant. Data can either be mutable and exclusively owned *or* immutable and shared, but never both simultaneously. Concurrency is primarily achieved through message passing, which moves ownership, eliminating entire classes of race conditions.

### **2.2. Clarity and Predictability over Cleverness**

The language must be easy to read, understand, and reason about, both for humans and for automated tools. Ambiguity is the enemy of correctness and maintainability.

* **Immutable by Default:** State is immutable unless explicitly declared within a tightly-scoped var block. This minimizes cognitive load by making state changes local and obvious.  
* **Visible Side Effects:** The effect system makes a function's interactions with the outside world (I/O, network, concurrency) an explicit part of its signature. This eliminates hidden surprises and makes program behavior transparent.  
* **Simple, Composable Syntax:** The syntax is minimal and expression-oriented, avoiding complex special cases in favor of a small set of orthogonal features that can be composed predictably.

### **2.3. AI as a First-Class Partner**

The language is designed with the assumption that a significant portion of code will be generated, analyzed, and refactored by AI assistants. The language must be structured to facilitate this partnership safely.

* **A Verifiable Target:** The Core Language's strictness, linearity, and explicit effects make it a perfect, verifiable compilation target for an AI. The Once compiler acts as a *formal verifier* for AI-generated code, rejecting any output that does not meet the language's safety guarantees.  
* **A High-Level Goal Language:** The Goal-Oriented Layer allows developers to work at a higher level of abstraction, focusing on *intent* (what) rather than implementation (how). This layer is the primary interface for AI-driven development.

### **2.4. Integrated, Hermetic Tooling**

A productive development experience requires powerful, consistent, and reproducible tooling that is part of the core project, not an afterthought.

* **Reproducible Builds:** The build system is declarative and non-Turing-complete, guaranteeing hermetic, content-addressed builds that are reproducible byte-for-byte.  
* **Capability-Based Security:** Packages must declare the effects they require (e.g., filesystem access, network egress). The build system enforces these capability ceilings, providing a robust defense against supply-chain attacks.  
* **Intelligent Diagnostics:** The compiler and its associated tools are designed to provide not just errors, but actionable diagnostics, fix-its, and rich "explain" modes that demystify complex topics like region inference and effect propagation.

## **3\. Target Audience & Use Cases**

Once is intended for a broad range of developers and applications where reliability, performance, and productivity are paramount.

* **Primary Audience:**  
  * **Systems & Cloud Developers:** Building high-performance, concurrent services (proxies, databases, message queues).  
  * **AI/ML Engineers:** Writing safe, high-performance data pipelines, training loops, and inference services.  
  * **Web & API Developers:** Creating robust and scalable backend applications.  
  * **Embedded Systems Developers:** Writing verifiable code for resource-constrained environments.  
* **Primary Use Cases:**  
  * Cloud-native infrastructure and services.  
  * High-performance data processing and ETL pipelines.  
  * Secure and reliable network services.  
  * AI model serving and orchestration.  
  * Safety-critical embedded systems.

## **4\. Specification Document Series**

This document is the first in a series that collectively defines the Once programming language.

* **ONCE-001: Vision and Goals** (This Document)  
* **ONCE-002: Core Language Specification** \- Defines the grammar, syntax, and semantics of the core verifiable language.  
* **ONCE-003: Type System & Effects** \- Details the static type system, traits, linearity, and row-polymorphic effect system.  
* **ONCE-004: Memory & Concurrency Model** \- Describes region-based memory management, the actor model, and channel semantics.  
* **ONCE-005: AI-Integration Layer & Goal-Oriented Syntax** \- Defines the high-level syntax for specifying program goals and the compilation process.  
* **ONCE-006: Build System & Tooling** \- Specifies the once.toml format, build process, and command-line interface.  
* **ONCE-007: Standard Library API** \- Outlines the modules and APIs available in the standard library.