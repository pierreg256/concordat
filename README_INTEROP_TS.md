# TypeScript Interoperability Guide

This document explains how to use `concordat` from TypeScript / Node.js via WebAssembly, and describes the interop test suite.

## Principle

**Rust is the single source of truth for all CRDT logic.** TypeScript never resolves conflicts — it calls into the Rust/WASM module for every mutation and merge. The transport layer (bytes in, bytes out) is the only boundary.

```
┌──────────────┐         Uint8Array         ┌──────────────┐
│  Rust/WASM   │ ◄──────────────────────── │  TypeScript   │
│  (CrdtDoc)   │ ────────────────────────► │  (CrdtDoc)    │
│              │       delta bytes          │              │
│  Source of   │                            │  Thin wrapper │
│  truth       │                            │  over WASM    │
└──────────────┘                            └──────────────┘
```

## Setup

### Prerequisites

- Rust stable (latest)
- `wasm-pack` — install with `cargo install wasm-pack`
- Node.js ≥ 18
- npm

### Build the WASM Package

```bash
# From the project root
wasm-pack build --target nodejs
```

This produces a `pkg/` directory containing:

- `concordat_bg.wasm` — the compiled WASM module
- `concordat.js` — JavaScript glue
- `concordat.d.ts` — TypeScript type definitions

### Install Test Dependencies

```bash
cd tests-ts
npm install
```

The `tests-ts/package.json` should reference the local WASM package:

```json
{
  "dependencies": {
    "concordat": "file:../pkg"
  },
  "scripts": {
    "test:interop": "npx ts-node --esm *.test.ts"
  }
}
```

## TypeScript API

```typescript
import { CrdtDoc } from "concordat";

// Create a document with a unique replica ID
const doc = new CrdtDoc("replica-ts-1");

// Set values
doc.set("/username", "Bob");
doc.set("/items", []);

// Array operations
doc.arrayInsert("/items", 0, "first");
doc.arrayDelete("/items", 0);

// Remove a key
doc.remove("/username");

// Get the current state as a plain JS object
const snapshot: object = doc.materialize();

// Get a delta since a known version (returns Uint8Array)
const delta: Uint8Array = doc.deltaSince(remoteVersionVector);

// Merge a remote delta (accepts Uint8Array)
doc.merge(remoteDelta);
```

### Key Points

- All deltas are `Uint8Array` — opaque binary buffers.
- `materialize()` returns a plain JavaScript object (JSON-compatible).
- The TypeScript wrapper does **not** implement any CRDT logic — it delegates entirely to the WASM module.
- A TypeScript developer does **not** need to understand CRDTs to use the library.

## Interop Test Scenarios

All tests live in `tests-ts/` and are run with `npm run test:interop`.

### Scenario 1 — Rust → TypeScript

Validates that a mutation performed in a Rust-created document can be received and applied in TypeScript.

```
1. Create doc_rust (in Rust/WASM)
2. doc_rust.set("/key", "value")
3. delta = doc_rust.deltaSince(empty_vv)     → Uint8Array
4. Create doc_ts (in TS/WASM)
5. doc_ts.merge(delta)
6. assert doc_ts.materialize() == doc_rust.materialize()
```

### Scenario 2 — TypeScript → Rust

Validates the reverse direction: a mutation in TypeScript converges correctly when applied to a Rust replica.

```
1. Create doc_ts
2. doc_ts.set("/key", "value")
3. delta = doc_ts.deltaSince(empty_vv)       → Uint8Array
4. Create doc_rust
5. doc_rust.merge(delta)
6. assert doc_rust.materialize() == doc_ts.materialize()
```

### Scenario 3 — Cross Concurrency

Validates convergence when both Rust and TypeScript replicas mutate concurrently.

```
1. Create doc_a (replica A) and doc_b (replica B)
2. doc_a.set("/x", "from_a")    — concurrent
   doc_b.set("/x", "from_b")    — concurrent
3. delta_a = doc_a.deltaSince(...)
   delta_b = doc_b.deltaSince(...)
4. Apply in both orders:
   - doc_a.merge(delta_b),  doc_b.merge(delta_a)
5. assert doc_a.materialize() == doc_b.materialize()
```

**Additional cross-concurrency checks:**

- Reverse the merge order — result must be the same (commutativity).
- Replay deltas multiple times — result must not change (idempotence).
- Use different data types (objects, arrays, scalars) in the same concurrent scenario.

## Test Assertions

Every interop test must verify:

| Assertion | Description |
|---|---|
| **Strict JSON equality** | `JSON.stringify(a.materialize()) === JSON.stringify(b.materialize())` |
| **No assumed delivery order** | Tests must work regardless of which delta is applied first |
| **Idempotent replay** | Applying the same delta twice must not alter the state |
| **Binary round-trip** | `serialize → deserialize → merge` must be lossless |

## Running the Tests

```bash
# Full build + test pipeline
wasm-pack build --target nodejs
cd tests-ts
npm install
npm run test:interop
```

### Expected Output

```
✓ Scenario 1: Rust → TS convergence
✓ Scenario 2: TS → Rust convergence
✓ Scenario 3: Cross concurrency convergence
✓ Idempotent delta replay
✓ Binary round-trip integrity
```

## Troubleshooting

| Problem | Solution |
|---|---|
| `Module not found: concordat` | Run `wasm-pack build --target nodejs` first |
| Type errors in `.d.ts` | Rebuild WASM — types are auto-generated |
| Delta merge produces different state | Check that both replicas use distinct replica IDs |
| `Uint8Array` is empty | Verify `deltaSince()` is called with the correct version vector |

## Design Decisions

### Why WASM and not a native Node addon?

- **Portability** — WASM runs in Node.js, Deno, Bun, and browsers without recompilation.
- **Single source of truth** — the exact same Rust code that passes the Rust test suite runs in TypeScript.
- **No native build toolchain required** — consumers don't need a C compiler or Rust installed.

### Why are deltas opaque bytes?

- The transport layer must not interpret, modify, or depend on the delta format.
- This enforces the strict separation between CRDT logic (Rust) and networking (driver).
- Binary format changes are internal to the library and don't break the transport API.

### Why no conflict resolution in TypeScript?

- Conflict resolution in two places leads to divergence.
- Rust is mathematically verified (via tests) to satisfy CRDT properties.
- TypeScript is a thin wrapper — it calls WASM, nothing more.
