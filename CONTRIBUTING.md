# Contributing to concordat

Thank you for your interest in contributing to `concordat`. This document describes the project organization, conventions, and rules you must follow.

## Architecture Overview

The project is structured around **six specialized agents**. Each agent has a clear scope. Contributions must stay within the boundaries of the relevant agent.

| Agent | Scope | Key files |
|---|---|---|
| **Lattice** – Core CRDT | OR-Map, RGA, Register, VersionVector | `ormap.rs`, `rga.rs`, `register.rs`, `vv.rs` |
| **Document** – API | `CrdtDoc`, JsonPath, JSON Patch → CRDT ops, `materialize()` | `doc.rs`, `value.rs` |
| **Bridge** – Delta, Codec & WASM | Delta types, serialization, WASM bindings | `delta.rs`, `codec.rs`, `wasm/` |
| **Sentinel** – Unit Tests | CRDT property tests, convergence, RGA edge cases, JSON conflicts | `tests/unit/*.rs` |
| **Convergence** – Integration Tests | Multi-replica scenarios, partitions, delta-state correctness | `tests/integration/*.rs` |
| **Interop** – TS Interop Tests | Cross-language Rust ↔ TypeScript tests via WASM | `tests-ts/*.ts` |

Agent definitions live in `.github/agents/*.agent.md`.

## Non-Negotiable Rules

These rules are **absolute**. Any contribution violating them will be rejected.

### Forbidden

- **No implicit Last-Writer-Wins.** Conflict resolution must come from CRDT semantics, not timestamps.
- **No wall clocks.** Use `VersionVector` and logical dots only.
- **No conflict resolution in the driver.** All convergence logic lives inside the Rust library.
- **No network dependency.** The library must never import or assume any network transport.
- **No `unsafe` Rust code** in the core CRDT module.

### Required

- **Total determinism.** Given the same set of deltas, every replica must produce the same state.
- **Stable serialization.** The binary format of deltas must not silently change between versions.
- **Reproducible tests.** No randomness without fixed seeds, no time-dependent assertions.
- **Minimal public API.** Use `pub` only for the external surface. Everything else is `pub(crate)` or private.

## CRDT Invariants

Every CRDT type **must** satisfy these properties. Tests proving them are mandatory.

```
merge(A, B) == merge(B, A)             // Commutativity
merge(merge(A, B), C) == merge(A, merge(B, C))  // Associativity
merge(A, A) == A                       // Idempotence
```

## Development Workflow

### Prerequisites

- Rust stable (latest)
- `wasm-pack` (for WASM builds)
- Node.js ≥ 18 (for TypeScript interop tests)
- npm or yarn

### Building

```bash
cargo build
```

### Running Tests

```bash
# All Rust tests
cargo test

# Unit tests only
cargo test --test unit

# Integration tests only
cargo test --test integration

# TypeScript interop tests
cd tests-ts && npm install && npm run test:interop
```

### WASM Build

```bash
wasm-pack build --target nodejs
```

## Submitting Changes

### Before You Submit

1. **Run the full test suite** — `cargo test` must pass with zero failures.
2. **Run clippy** — `cargo clippy -- -D warnings` must produce no warnings.
3. **Format your code** — `cargo fmt` must report no changes.
4. **If you touch WASM or codec** — rebuild WASM and run TypeScript tests.
5. **If you add a new CRDT type or operation** — add commutativity, associativity, and idempotence tests.

### Commit Messages

Use clear, imperative-mood commit messages:

```
feat(rga): add concurrent insert resolution at same index
fix(ormap): correct tombstone handling on re-add
test(sentinel): add associativity tests for MV-Register
test(convergence): add 5-replica partition scenario
```

### Pull Requests

- One logical change per PR.
- Reference the agent your change belongs to (Lattice, Document, Bridge, Sentinel, Convergence, Interop).
- Include or update tests for every behavioral change.
- Describe what invariants your change preserves or introduces.

## Test Requirements

### Unit Tests (Sentinel)

Every CRDT type must have tests for:

- Commutativity, associativity, idempotence.
- Multi-replica convergence with varied delivery orders.
- Duplicated and delayed deltas.
- RGA: concurrent insert at same index, concurrent delete, insert after tombstone.
- JSON: key conflicts, concurrent array edits, nested structures.

### Integration Tests (Convergence)

- Scenarios with 2–5 replicas.
- Simulated network partitions: local mutations → partial delta exchange → reconnection → convergence.
- `delta_since(vv)` must return exactly the right deltas (no missing, no superfluous).
- Final verification: all replicas produce identical output from `materialize()`.

### TypeScript Interop Tests (Interop)

- Rust → TS: mutation in Rust, delta sent as bytes, applied in TS, convergence verified.
- TS → Rust: mutation in TS, delta sent as bytes, applied in Rust, convergence verified.
- Cross concurrency: concurrent mutations in both Rust and TS, all delivery orders, final convergence.
- Deltas replayed multiple times must have no additional effect (idempotence).

See [README_INTEROP_TS.md](README_INTEROP_TS.md) for the full TypeScript testing guide.

## Code Style

- Follow standard `rustfmt` defaults.
- Prefer explicit types over inference when the type is not obvious.
- Document invariants with `///` doc comments on public items.
- No unnecessary abstractions — keep it simple and direct.

## Feature Flags

Optional features behind flags:

| Flag | Description |
|---|---|
| `wasm` | Enable WASM bindings |
| `array_move` | Enable `array_move` operation (experimental) |

## Questions?

Open an issue describing what you want to change and which agent it falls under (Lattice, Document, Bridge, Sentinel, Convergence, Interop). We'll discuss scope and approach before you write code.
