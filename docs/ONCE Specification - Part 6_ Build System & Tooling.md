# **ONCE Specification \- Part 6: Build System & Tooling**

| Document ID | ONCE-006 |
| :---- | :---- |
| **Title** | Build System & Tooling |
| **Version** | 1.0 |
| **Date** | 2025-10-20 |
| **Status** | Draft |
| **Related Docs** | ONCE-001 |

## **1\. Introduction**

This document specifies the build system, package manifest format, and standard tooling for the Once language. The design of the tooling is a direct reflection of the language's core philosophies (ONCE-001): to provide a development experience that is reproducible, secure, and highly productive.

The once command-line tool is the single entry point for managing the entire lifecycle of a Once project, from creation and dependency management to compilation, testing, and diagnostics.

## **2\. The Package Manifest: once.toml**

Every Once project (a "package") is defined by a manifest file named once.toml at its root. This file is written in the TOML format and is strictly declarative; it contains no loops, conditionals, or script execution, ensuring that build configurations are static and analyzable.

### **2.1. Manifest Sections**

A once.toml file consists of the following primary sections:

* **\[package\]**: Defines metadata about the package.  
  * name: The name of the package (e.g., "my\_app").  
  * version: The version string, following Semantic Versioning (SemVer).  
  * authors: A list of authors.  
* **\[deps\]**: Declares the package's dependencies.  
  * Dependencies are listed with a version requirement (e.g., http \= "1.2.0").  
* **\[capabilities\]**: Declares the set of permissions the package requires to run. This is a critical security feature.  
  * io: true or false. Allows filesystem access.  
  * net: true or false. Allows network access.  
  * (See Section 4 for more details).  
* **\[profile.\*\]**: Defines different build profiles.  
  * \[profile.dev\]: Settings for development builds (e.g., low optimization, debug info).  
  * \[profile.release\]: Settings for production builds (e.g., high optimization, no debug info).

### **2.2. Example Manifest**

\[package\]  
name \= "web\_server"  
version \= "0.1.0"  
authors \= \["Ada Lovelace \<ada@example.com\>"\]

\[deps\]  
http \= "1.2.0"  
json \= "1.0.5"

\[capabilities\]  
net \= true  
io \= true  
spawn \= true

\[profile.release\]  
opt-level \= 3  
lto \= true

## **3\. Hermetic and Reproducible Builds**

Once guarantees that builds are fully reproducible. Given the same source code and once.lock file, the build tool will produce a byte-for-byte identical output, regardless of the machine, time of day, or network state.

### **3.1. The Lockfile: once.lock**

* When dependencies are first resolved, the build tool creates a once.lock file.  
* This file contains the exact, resolved versions and cryptographic hashes of every dependency in the transitive dependency graph.  
* Subsequent builds use the lockfile to ensure the exact same dependency versions are used, preventing "works on my machine" issues.  
* The lockfile is meant to be checked into version control.

### **3.2. Content-Addressed Cache**

All build artifacts, including compiled dependencies and intermediate files, are stored in a content-addressed cache. This means that if a source file or dependency has not changed, it is not recompiled. This provides extreme build performance, especially in CI environments.

## **4\. Capability-Based Security**

The \[capabilities\] section of the manifest is a cornerstone of Once's security model. It allows developers and operators to enforce the principle of least authority.

* **Declaration:** A package must declare the privileged effects it intends to use.  
* **Verification:** During compilation, the once tool checks the entire dependency graph. If any transitive dependency requires a capability that is not declared in the top-level package's once.toml, the build will fail.  
* **Example Failure:** If web\_server from the example above had io \= false in its manifest, the build would fail because its dependency http requires the io effect to read from sockets. The error message would be:Error: Capability 'io' is required by dependency 'http' but is not declared in web\_server's manifest.

This prevents supply-chain attacks where a dependency unexpectedly starts accessing the network or filesystem.

## **5\. The once Command-Line Interface (CLI)**

The once tool provides a consistent and powerful interface for all development tasks.

* **once new \<name\>**: Creates a new Once application or library skeleton.  
* **once build**: Compiles the current package.  
* **once run**: Compiles and runs the package.  
* **once test**: Compiles and runs the package's test suite.  
* **once fmt**: Formats all Once source files in the package according to a canonical style.  
* **once lint**: Runs a suite of static analysis checks to find potential bugs and style issues.  
* **once explain**: A powerful diagnostic tool.  
  * once explain \--regions \<file\>:\<line\>: Visualizes the inferred memory regions for a function.  
  * once explain \--effects \<file\>:\<line\>: Shows the derived effect signature for a function.

## **6\. Language Server Protocol (LSP)**

The Once toolchain includes a first-party implementation of the Language Server Protocol (once-lsp). This provides a rich, consistent, and deeply integrated experience in any compatible code editor, including:

* Live diagnostics and fix-its.  
* Type and effect information on hover.  
* Code completion.  
* Go-to-definition and find-references.  
* Seamless integration with the once explain and once fmt commands.