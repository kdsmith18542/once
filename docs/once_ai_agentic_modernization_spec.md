# Once Add-On Spec: AI, Agentic Tooling, Components, and Secure Supply Chain

**Version:** v0.1 Draft  
**Date:** 2026-05-05  
**Companion documents:** Once Language Spec, Once Compiler Blueprint  
**Audience:** compiler engineers, tooling engineers, runtime engineers, security engineers, AI/agent integration developers

---

## 0. Purpose

This document defines a focused modernization add-on for the **Once** programming language project.

It does **not** change the core identity of Once.

Once remains:

> A safe, deterministic, general-purpose systems language with inferred regions, linear resources, visible effects, and secure tooling.

This add-on improves Once for:

- agentic coding workflows
- LLM-driven code review/refactoring
- secure plugin systems
- Wasm Component deployment
- heterogeneous CPU/GPU/AI workloads
- software supply-chain assurance

The goal is to make Once feel modern for the 2026-era developer environment without bloating the core language.

---

## 1. Current External Signals

### 1.1 MCP: Agent-tool integration is becoming a standard workflow

The Model Context Protocol defines an open protocol for connecting LLM applications to external tools and data sources. This aligns directly with Once's planned machine-readable compiler APIs.

**Design implication:**

- Once should ship an official **MCP server**.
- Compiler explainers, fix-its, tests, effect checks, and security checks should be invokable by coding agents.

### 1.2 Wasm Component Model and WASI 0.2 are stable enough for serious interop

The WebAssembly Component Model and WASI 0.2 provide a standardized interface model using WIT definitions. This aligns with Once's preferred safe FFI boundary.

**Design implication:**

- Wasm Components should become an official Once build target.
- Once should export and consume WIT.
- Safe plugin systems should become a flagship use case.

### 1.3 MLIR remains the strongest long-term compiler path for heterogeneous compute

MLIR is reusable compiler infrastructure for heterogeneous hardware and domain-specific compilers. It remains highly relevant for AI, GPUs, tensor workloads, and domain-specific optimization.

**Design implication:**

- Cranelift remains the bootstrap backend.
- MLIR becomes the strategic backend path for GPU/tensor/AI workloads.
- Once should eventually define dedicated MLIR dialects for regions, effects, linearity, tensors, and GPU kernels.

### 1.4 Supply-chain security is now a language/toolchain concern

SLSA, SBOM generation, provenance, signing, reproducible builds, and dependency verification are now expected in mature software ecosystems.

**Design implication:**

- Once should treat supply-chain metadata as first-class.
- Security profiles should be built into `once.toml`.
- `once build` should produce provenance and SBOM artifacts in release mode.

---

## 2. Strategic Positioning

Once should **not** pivot into being an AI-only language.

Instead, position Once as:

> A safe, deterministic systems language for human + agent developer teams, secure plugins, reproducible infrastructure, and modern heterogeneous compute.

This gives Once a distinct identity from:

- Rust: strong safety, but high ownership/lifetime friction.
- Go: simple concurrency, but weaker type/resource guarantees.
- C#: strong DX, but GC/runtime-heavy for systems use.
- F#: expressive functional style, but not a systems/performance-first language.
- Mojo: AI/performance focused, but less centered on general secure systems tooling.

Once's wedge:

```text
simple syntax + inferred regions + linear resources + visible effects + agent-native compiler APIs
```

---

## 3. New Official Component: `once-mcp`

### 3.1 Purpose

`once-mcp` exposes Once compiler and project tooling to LLM agents through MCP.

It should be safe-by-default, read-only by default, and capable of returning structured results.

### 3.2 Package

```text
crates/once-mcp/
```

### 3.3 CLI

```text
once mcp serve
once mcp serve --readonly
once mcp serve --workspace .
once mcp list-tools
```

### 3.4 MCP Tools

Initial tool set:

```text
once.analyze
once.diagnostics
once.fix_plan
once.apply_fix
once.test
once.explain_regions
once.explain_effects
once.explain_linearity
once.explain_imports
once.effect_diff
once.abi_diff
once.security_audit
once.agent_check
```

### 3.5 Tool Definitions

#### `once.analyze`

Inputs:

```json
{
  "workspace": ".",
  "target": "all",
  "format": "summary | hir | mir | effects | regions"
}
```

Outputs:

```json
{
  "status": "ok",
  "modules": [],
  "public_api_hash": "...",
  "effect_hash": "...",
  "diagnostics": []
}
```

#### `once.fix_plan`

Generates a patch plan but does not mutate files.

```json
{
  "diagnostic_codes": ["E-LIN-002"],
  "safety": "mechanical | semantic | risky"
}
```

Output:

```json
{
  "plan_id": "fix-123",
  "edits": [],
  "safety": "mechanical",
  "requires_tests": true,
  "affected_public_api": false,
  "affected_effects": []
}
```

#### `once.apply_fix`

Applies a previously generated plan.

Requirements:

- plan id
- workspace hash
- optional human approval gate

```json
{
  "plan_id": "fix-123",
  "workspace_hash": "..."
}
```

### 3.6 Permission Model

Default mode:

```text
readonly
```

Write mode requires:

```text
once mcp serve --allow-edits
```

Risky edits require explicit approval unless project policy allows them.

### 3.7 Agent Safety Rules

An agent may not:

- add undeclared capabilities
- add new dependencies
- add unsafe FFI
- widen public effect rows
- change public ABI
- disable tests
- weaken security profiles

unless allowed by `agent_policy` in `once.toml`.

---

## 4. Agent Policy in `once.toml`

### 4.1 Manifest Section

```toml
[agent_policy]
allow_edits = true
allow_new_deps = false
allow_new_effects = false
allow_unsafe_ffi = false
require_tests_for_public_api_change = true
require_human_approval_for_risky_fixes = true
max_changed_files = 20
```

### 4.2 Effect Budgets

Strict form:

```toml
[agent_policy.effect_budget]
allowed_new_effects = []
allowed_new_capabilities = []
```

Relaxed form for prototypes:

```toml
[agent_policy.effect_budget]
allowed_new_effects = ["io"]
allowed_new_capabilities = ["fs"]
```

### 4.3 Dependency Budgets

```toml
[agent_policy.dependency_budget]
allow_new = false
allow_major_upgrade = false
allow_minor_upgrade = true
```

### 4.4 Command

```text
once agent-check
```

Checks:

- public ABI changes
- public effect changes
- dependency changes
- capability changes
- unsafe FFI changes
- test coverage for affected modules
- security profile weakening

---

## 5. LLM-Safe Refactoring Contracts

### 5.1 Rationale

Coding agents should not patch source text blindly.

Once should support refactoring plans as structured operations with compiler validation.

### 5.2 Commands

```text
once refactor --plan rename-symbol --symbol old --to new
once refactor --plan extract-function --range src/main.onc:10:1-25:1
once refactor --plan add-using --diagnostic E-LIN-001
once refactor --apply plan.json
```

### 5.3 Plan Format

```json
{
  "plan_id": "refactor-001",
  "kind": "rename-symbol",
  "safety": "mechanical",
  "affected_files": ["src/main.onc"],
  "affected_public_api": false,
  "affected_effects": [],
  "requires_tests": true,
  "preconditions": {
    "workspace_hash": "...",
    "compiler_version": "oncec 0.1.0"
  },
  "edits": [
    {
      "file": "src/main.onc",
      "range": {
        "start": 120,
        "end": 126
      },
      "replacement": "newName"
    }
  ]
}
```

### 5.4 Safety Levels

```text
mechanical  - syntax-preserving or compiler-proven
semantic    - changes behavior but compiler can validate types/effects
risky       - touches unsafe, FFI, concurrency policy, public ABI, security profile
```

Risky plans require explicit approval.

---

## 6. Machine-Readable Compiler Interfaces

### 6.1 Commands

```text
once analyze --json
once analyze --ast --json
once analyze --hir --json
once analyze --mir --json
once analyze --effects --json
once analyze --regions --json
once analyze --imports --json
once analyze --public-api --json
```

### 6.2 JSON Stability

JSON output should be versioned.

```json
{
  "schema": "once.analyze.v1",
  "compiler": "oncec 0.1.0",
  "workspace_hash": "...",
  "payload": {}
}
```

### 6.3 Span Format

All spans use byte offsets and file IDs.

```json
{
  "file": "src/main.onc",
  "start": 120,
  "end": 142,
  "line_start": 8,
  "col_start": 5,
  "line_end": 8,
  "col_end": 27
}
```

### 6.4 IR Export Policy

- AST/HIR can be exported freely.
- MIR export can include unstable compiler internals but must be schema-versioned.
- Region/effect summaries should be stable enough for agents and CI tools.

---

## 7. Wasm Component Model as an Official Target

### 7.1 Targets

Add official targets:

```text
once build --target native
once build --target wasm-component
once build --target wasi-cli
once build --target wasi-http
```

### 7.2 Commands

```text
once wit export
once wit import ./wit
once component verify
once component run
once component sign
```

### 7.3 Package Layout

```text
project/
├─ once.toml
├─ src/
├─ wit/
│  └─ package.wit
└─ components/
```

### 7.4 WIT Export Example

Once:

```once
export fn classify(input: Bytes) -> Result<Text, Err> ![io]
```

Generated WIT:

```wit
package acme:classifier;

interface api {
  classify: func(input: list<u8>) -> result<string, string>;
}

world classifier {
  export api;
}
```

### 7.5 Component Capability Mapping

Once effects map into component permissions.

```text
![io]       -> wasi:filesystem or wasi:io usage
![net]      -> wasi:sockets / wasi:http capability
![time]     -> wasi:clocks
![random]   -> wasi:random
```

### 7.6 Component Verification

`once component verify` checks:

- WIT compatibility
- capability declarations
- memory limits
- effect/capability match
- signature hash
- provenance/signature if present

---

## 8. Safe Plugin Host as Flagship Example

### 8.1 Example Path

```text
examples/safe-plugin-host/
```

### 8.2 Demonstrates

- host app in Once
- plugins as Wasm Components
- WIT interface
- capability-limited plugin execution
- signed component artifacts
- memory limits
- optional MCP control surface
- effect budget enforcement

### 8.3 Plugin Manifest

```toml
[plugin]
name = "image-filter"
version = "0.1.0"

[plugin.capabilities]
io = false
net = false
time = true

[plugin.limits]
memory_mb = 64
fuel = 10000000
```

### 8.4 Host Policy

```toml
[plugin_policy]
require_signature = true
allow_network = false
max_memory_mb = 128
```

---

## 9. MLIR Roadmap

### 9.1 Backend Strategy

Bootstrap path:

```text
HIR -> TIR -> RIR -> MIR -> Cranelift
```

Strategic path:

```text
HIR -> TIR -> RIR -> MIR -> Once MLIR Dialects -> LLVM/GPU/Wasm
```

Cranelift remains the fastest implementation path.

MLIR becomes the long-term route for:

- GPU
- tensor operations
- SIMD
- AI/ML kernels
- hardware-specific optimization

### 9.2 Planned Once Dialects

```text
once.region
once.linear
once.effect
once.task
once.tensor
once.gpu
```

### 9.3 Dialect Responsibilities

#### `once.region`

Represents:

- region allocation
- region free
- region ownership
- escape metadata

#### `once.linear`

Represents:

- moves
- consumes
- resource terminal operations

#### `once.effect`

Represents:

- effect rows
- capability metadata
- effect barriers

#### `once.task`

Represents:

- async task creation
- await/join/cancel
- group/nursery semantics

#### `once.tensor`

Represents:

- shape-aware tensors
- slices/views
- bounds facts

#### `once.gpu`

Represents:

- device buffers
- kernel launches
- host/device transfer
- GPU capability effects

### 9.4 Non-Goal

Do not make MLIR mandatory in the first compiler prototype.

---

## 10. AI / Compute Library Roadmap

### 10.1 Keep AI Out of Core Syntax

No AI-specific keywords in the core language.

AI support should live in first-party libraries.

### 10.2 First-Party Libraries

```text
once-nd
once-gpu
once-onnx
once-tokenizers
once-embed
once-dataflow
```

### 10.3 `once-nd`

Features:

- `NdArray<T, D>`
- shape facts via size types
- safe slicing/views
- deterministic RNG
- debug NaN/Inf guards
- CPU kernels

Example:

```once
let x: NdArray<Float, [Batch, Features]> = load_batch(...)
let y = model.forward(x)?
```

### 10.4 `once-gpu`

Features:

- `GpuBuffer<T>` as linear resource
- `gpu` effect
- WGPU backend first
- CUDA/PTX path later through MLIR
- safe host/device transfers

Example:

```once
fn run_kernel(buf: GpuBuffer<Float>) -> GpuBuffer<Float> ![gpu] {
  ...
}
```

### 10.5 `once-onnx`

Features:

- ONNX runtime via Wasm Component or native component
- explicit memory limits
- capability-scoped model loading
- reproducible inference config

### 10.6 `once-dataflow`

Features:

- streaming pipelines
- backpressure-aware channels
- deterministic replay
- useful for ETL, embeddings, and batch inference

---

## 11. Supply Chain and Provenance

### 11.1 Commands

```text
once sbom
once attest
once verify
once vendor
once audit
once sign
```

### 11.2 Security Profiles

```toml
[profile.security.dev]
sbom = true
provenance = "local"
sign = false
reproducible = true

[profile.security.release]
sbom = true
provenance = "slsa"
sign = true
reproducible = true
require_locked_deps = true
```

### 11.3 SBOM

Default format:

```text
SPDX JSON
```

Optional:

```text
CycloneDX
```

SBOM includes:

- package name/version
- source hashes
- dependency graph
- capability graph
- build profile
- compiler version
- generated artifacts

### 11.4 Provenance

Attestation includes:

- builder identity
- source digest
- lockfile digest
- dependency digests
- build command
- target triple
- artifact digest
- public API hash
- effect hash
- capability hash

### 11.5 Registry Signing

Registry packages should be signed.

Verification:

```text
once verify package.oncepkg
```

Checks:

- signature
- provenance
- SBOM
- lockfile consistency
- declared capabilities

---

## 12. Capability-Secure Build Profiles

### 12.1 Capability Hash

Every build artifact includes:

```text
capability_hash
```

Computed from:

- declared effects
- filesystem roots
- network egress policy
- FFI libraries
- GPU access
- plugin policy

### 12.2 Capability Diff

```text
once capability-diff old.onceo new.onceo
```

Fails CI if capability scope widens without explicit approval.

### 12.3 Example CI Rule

```yaml
- run: once effect-diff --fail-on-widening
- run: once capability-diff --fail-on-widening
- run: once agent-check
- run: once verify --release
```

---

## 13. Observability for Agent and Human Workflows

### 13.1 `once-trace`

First-party observability library.

Features:

- tracing spans
- metrics
- region allocation counters
- task/channel metrics
- effect-aware redaction

### 13.2 Effect-Aware Redaction

If a module lacks `![net]`, it cannot export telemetry externally.

If a module lacks declared PII capability, logs must redact tagged values.

Potential future types:

```once
type Secret<T>
type Pii<T>
```

### 13.3 Agent Debug Traces

Agents can request:

```text
once explain --trace task-id
once explain --regions --json
once explain --effects --json
```

---

## 14. Security Model for Coding Agents

### 14.1 Threat Model

Coding agents may accidentally or maliciously:

- add dependencies
- add network effects
- add filesystem access
- weaken checks
- introduce unsafe FFI
- change build profiles
- silence diagnostics
- widen public API/effect surface

### 14.2 Defense

Once should make these impossible without policy approval.

Defense layers:

1. explicit imports
2. lockfile resolution
3. capability gates
4. effect rows
5. ABI/effect diff
6. agent policy
7. structured patch plans
8. deterministic tests
9. security profile verification

### 14.3 Agent Commit Gate

Recommended command before accepting agent-generated patch:

```text
once agent-check --strict
once test --deterministic
once effect-diff --fail-on-widening
once capability-diff --fail-on-widening
once audit
```

---

## 15. Updates to Existing Compiler Blueprint

Add crates:

```text
once-mcp
once-wit
once-component
once-attest
once-sbom
once-agent
once-refactor
once-mlir
once-nd
once-gpu
once-onnx
once-dataflow
once-trace
```

Add compiler commands:

```text
once mcp serve
once agent-check
once refactor --plan ...
once refactor --apply ...
once wit export
once component verify
once sbom
once attest
once verify
once capability-diff
once analyze --json
```

Add MIR metadata:

```text
effect_hash
capability_hash
public_api_hash
region_summary_hash
```

Add artifact metadata:

```text
source_hash
lockfile_hash
compiler_hash
sbom_digest
provenance_digest
signature
```

---

## 16. Recommended Priority

### Immediate additions

1. `once analyze --json`
2. `once fix --json`
3. `agent_policy` in `once.toml`
4. `once agent-check`
5. `once-mcp`
6. `once wit export`
7. `once build --target wasm-component`

### After compiler prototype

8. `once sbom`
9. `once attest`
10. safe plugin host example
11. MLIR dialect prototype
12. `once-nd`
13. `once-gpu`

### Later

14. time-travel debugger
15. GPU kernel DSL
16. advanced MCP workflows
17. signed public package registry

---

## 17. Risk Assessment

### Risk: Overbuilding for AI too early

Mitigation:

- keep AI out of core syntax
- implement agent APIs around existing compiler data
- build MCP as a tooling layer

### Risk: MLIR delays first compiler

Mitigation:

- Cranelift first
- MLIR as separate roadmap crate

### Risk: Component Model adds complexity

Mitigation:

- start with WIT export/import only
- add runtime execution later

### Risk: Agent policies annoy developers

Mitigation:

- strict for CI/release
- relaxed for local prototypes

---

## 18. Final Recommendation

The best pivot is not to become an AI language.

The best pivot is to make Once:

> the safest and most deterministic language/toolchain for human + agent teams building systems, plugins, services, and compute workloads.

This requires:

- MCP integration
- machine-readable compiler APIs
- effect/capability budgets
- structured refactoring plans
- Wasm Component targets
- supply-chain provenance
- future MLIR/tensor/GPU path

These additions strengthen the original Once vision without making the language harder to learn.

---

## References

- Model Context Protocol specification: https://modelcontextprotocol.io/specification/2025-03-26
- WebAssembly Component Model introduction: https://component-model.bytecodealliance.org/
- WASI interfaces and WIT: https://wasi.dev/interfaces
- MLIR project: https://mlir.llvm.org/
- SLSA framework: https://slsa.dev/
