# Concordat – Copilot Instructions

## Project

Concordat is a delta-state CRDT JSON library in Rust with TypeScript/WASM interop.

## Architecture

- **OR-Map** → JSON objects
- **RGA** → JSON arrays (insert/delete with tombstones)
- **MV-Register** → JSON scalars
- Synchronization via **delta-state** with `VersionVector` / logical dots
- No network dependency — deltas are opaque bytes transported by the driver

## Agent Roles

| Agent | Scope | Files |
|-------|-------|-------|
| **Lattice** | Core CRDT (OR-Map, RGA, Register, VersionVector) | `ormap.rs`, `rga.rs`, `register.rs`, `vv.rs` |
| **Document** | Document API, JsonPath, materialize | `doc.rs`, `value.rs` |
| **Bridge** | Delta, Codec & WASM bindings | `delta.rs`, `codec.rs`, `wasm/` |
| **Sentinel** | Unit Tests (CRDT property proofs) | `tests/unit/*.rs` |
| **Convergence** | Integration Tests (multi-replica scenarios) | `tests/integration/*.rs` |
| **Interop** | TypeScript interop tests via WASM | `tests-ts/*.ts` |

Agent definitions: `.github/agents/*.agent.md`

## Hard Rules

- No implicit Last-Writer-Wins
- No wall clocks — only VersionVector / logical dots
- No conflict resolution outside of Rust
- No network dependency
- No `unsafe` in core CRDT code
- `pub` only for external API, `pub(crate)` otherwise
- Every CRDT type must satisfy: commutativity, associativity, idempotence

## Conventions

- Format with `cargo fmt`
- Lint with `cargo clippy -- -D warnings`
- Serialize deltas with `serde` + `bincode` or `postcard`
- WASM build: `wasm-pack build --target nodejs`
- TS tests: `cd tests-ts && npm run test:interop`
