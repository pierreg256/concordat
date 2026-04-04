# concordat

```
        ◉──────◉
       ╱ ╲    ╱
      ╱   ╲  ╱
     ◉─────◉
      ╲   ╱
       ╲ ╱
        ◉
   concordat
```

A **delta-state CRDT JSON library** written in Rust, providing Strong Eventual Consistency for distributed JSON documents — with no network dependency.

## Overview

`concordat` implements a full JSON CRDT document model:

| JSON construct | CRDT type | Description |
|---|---|---|
| Object | **OR-Map** | Observed-Remove Map for JSON objects |
| Array | **RGA** | Replicated Growable Array with tombstones |
| Scalar | **MV-Register** | Multi-Value Register for leaf values |

Every mutation produces a **delta** that can be shipped to any other replica. Merging deltas is:

- **Commutative** — `merge(A, B) == merge(B, A)`
- **Associative** — `merge(merge(A, B), C) == merge(A, merge(B, C))`
- **Idempotent** — `merge(A, A) == A`

This guarantees **deterministic convergence** regardless of delivery order, duplication, or network topology.

## Key Properties

- **Strong Eventual Consistency** — replicas that have seen the same set of updates converge to the same state.
- **Delta-state synchronization** — only changed portions are transmitted, not the full state.
- **No network dependency** — the library handles convergence; the transport layer is entirely yours.
- **No wall clocks** — causality is tracked via `VersionVector` / logical dots only.
- **No implicit Last-Writer-Wins** — conflicts are resolved structurally by the CRDT semantics.
- **TypeScript / Node.js interop via WASM** — use the same CRDT engine from JavaScript.

## Architecture

```
┌─────────────────────────────────────────┐
│              CrdtDoc                    │
│  ┌──────────┐  ┌──────┐  ┌──────────┐  │
│  │  OR-Map   │  │ RGA  │  │ Register │  │
│  └──────────┘  └──────┘  └──────────┘  │
│         ▲          ▲          ▲         │
│         └──────────┴──────────┘         │
│              CrdtValue                  │
├─────────────────────────────────────────┤
│         VersionVector / Dot             │
├─────────────────────────────────────────┤
│        Delta / DeltaPayload             │
├─────────────────────────────────────────┤
│      Codec (serde + bincode/postcard)   │
├─────────────────────────────────────────┤
│          WASM binding (optional)        │
└─────────────────────────────────────────┘
```

### Separation of Concerns

The library enforces a strict boundary:

- **Inside the library** — all CRDT logic, conflict resolution, and convergence.
- **Outside the library** — transport, persistence, and any networking. Deltas are opaque byte buffers the driver simply relays.

## Modules

| Module | File | Description |
|---|---|---|
| OR-Map | `ormap.rs` | Observed-Remove Map (JSON objects) |
| RGA | `rga.rs` | Replicated Growable Array (JSON arrays) |
| Register | `register.rs` | Multi-Value Register (JSON scalars) |
| VersionVector | `vv.rs` | Causal context tracking (dots & version vectors) |
| Document | `doc.rs` | `CrdtDoc` — top-level JSON document API |
| Value | `value.rs` | `CrdtValue` enum (`Object`, `Array`, `Scalar`) |
| Delta | `delta.rs` | Delta types: `Delta`, `SyncState`, `DeltaPayload` |
| Codec | `codec.rs` | Serialization/deserialization of deltas |
| WASM | `wasm/` | WebAssembly bindings for Node.js / browser |

## Public API

```rust
use concordat::CrdtDoc;

// Create a new document with a unique replica ID
let mut doc = CrdtDoc::new("replica-1");

// Set a value at a JSON path
doc.set("/name", json!("Alice"));
doc.set("/scores", json!([]));

// Array operations
doc.array_insert("/scores", 0, json!(100));
doc.array_delete("/scores", 0);

// Remove a key
doc.remove("/name");

// Produce a delta since a known version
let delta = doc.delta_since(&remote_version_vector);

// Merge a remote delta
doc.merge(remote_delta);

// Materialize the current state as plain JSON
let snapshot: serde_json::Value = doc.materialize();
```

## Synchronization Protocol

```
Replica A                          Replica B
    │                                  │
    │── mutation ──►                   │
    │   produces Delta                 │
    │                                  │
    │── serialize(delta) ────────────► │
    │                                  │── merge(delta)
    │                                  │   state converges
    │                                  │
    │ ◄──────────── serialize(delta) ──│
    │── merge(delta)                   │
    │   state converges                │
    │                                  │
    ▼  materialize() == materialize()  ▼
```

1. A replica performs a local mutation, producing a `Delta`.
2. The delta is serialized to bytes via the codec.
3. The driver transports the bytes (TCP, WebSocket, file, pigeon — doesn't matter).
4. The receiving replica deserializes and calls `merge()`.
5. After all deltas are exchanged, `materialize()` returns identical JSON on every replica.

## Building

```bash
# Build the Rust library
cargo build

# Run all Rust tests (unit + integration)
cargo test

# Build the WASM package
wasm-pack build --target nodejs
```

## Testing

The test suite is extensive and structured in layers:

### Unit Tests (`tests/unit/`)

- **CRDT property tests** — commutativity, associativity, idempotence for every CRDT type.
- **Convergence tests** — multiple replicas, varied delivery orders, duplicated/delayed deltas.
- **RGA-specific tests** — concurrent insert at same index, concurrent delete, insert after tombstone, deterministic final order.
- **JSON tests** — key conflicts on objects, concurrent array modifications, nested Object → Array → Object.

### Integration Tests (`tests/integration/`)

- **Multi-replica scenarios** (2–5 replicas).
- **Simulated network partitions** — local mutations, partial delta exchange, reconnection, final convergence check.
- **Delta-state correctness** — `delta_since(vv)` returns exactly the right set of operations, no missing, no superfluous.

### TypeScript Interop Tests (`tests-ts/`)

See [README_INTEROP_TS.md](README_INTEROP_TS.md) for details.

## WASM / TypeScript

The library compiles to WebAssembly and can be consumed from Node.js or the browser. TypeScript can:

- Create and mutate CRDT documents.
- Produce and receive deltas as `Uint8Array`.
- Converge deterministically — the CRDT logic remains in Rust/WASM.

A TypeScript developer can use the library **without understanding CRDTs**. See the [TypeScript interop guide](README_INTEROP_TS.md).

## Roadmap (post-V1)

- Array move operations (advanced JSON CRDT)
- Undo / Redo
- Snapshots
- Advanced tombstone compaction
- Pluggable storage backend

## License

TBD
