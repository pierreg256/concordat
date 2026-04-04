# Concordat – Development Roadmap

Each step produces a **stable, compiling, fully tested** checkpoint.
No step depends on code that hasn't been tested in a previous step.

---

## Step 0 — Project Scaffolding ✅

**Agent**: —
**Goal**: Cargo project structure, module declarations, dependencies.

**Tasks**:
- [x] `cargo init --lib`
- [x] `Cargo.toml` with dependencies: `serde`, `serde_json`, `bincode` or `postcard`
- [x] Module skeleton: `lib.rs` → `vv`, `register`, `ormap`, `rga`, `value`, `doc`, `delta`, `codec`
- [x] Empty `tests/` structure
- [x] Compiles with `cargo check`

**Checkpoint**: ✅ `cargo check` passes.

---

## Step 1 — VersionVector & Dot ✅

**Agent**: Lattice
**Goal**: Causal context tracking — the foundation everything else builds on.

**Files**: `src/vv.rs`

**Types**:
- `ReplicaId` (type alias or newtype)
- `Dot { replica: ReplicaId, counter: u64 }`
- `VersionVector` (map of ReplicaId → counter)

**Operations**:
- `VersionVector::new()`
- `VersionVector::inc(replica) -> Dot` — increment and return the new dot
- `VersionVector::contains(dot) -> bool`
- `VersionVector::merge(&mut self, other: &VersionVector)`
- `VersionVector::delta_since(other: &VersionVector) -> VersionVector`

**Tests** (Sentinel): 17 tests
- [x] `merge` is commutative
- [x] `merge` is associative
- [x] `merge` is idempotent
- [x] `inc` produces sequential dots
- [x] `contains` returns true for seen dots, false for unseen
- [x] `delta_since` returns only the diff

**Checkpoint**: ✅ `cargo test` — 17/17 VV tests pass.

---

## Step 2 — MV-Register

**Agent**: Lattice
**Goal**: Multi-Value Register for JSON scalar values. Simplest CRDT type.

**Files**: `src/register.rs`

**Types**:
- `MvRegister<V>` — stores concurrent values tagged with dots

**Operations**:
- `MvRegister::new()`
- `MvRegister::set(value, dot, vv) -> Delta`
- `MvRegister::merge(&mut self, other: &MvRegister)`
- `MvRegister::values() -> &[V]` — all concurrent values
- `MvRegister::value() -> Option<&V>` — single value (if no conflict)

**Tests** (Sentinel):
- [ ] Commutativity: `merge(A, B) == merge(B, A)`
- [ ] Associativity: `merge(merge(A, B), C) == merge(A, merge(B, C))`
- [ ] Idempotence: `merge(A, A) == A`
- [ ] Sequential writes: later write wins (causally)
- [ ] Concurrent writes: both values preserved
- [ ] Merge after concurrent writes resolves correctly

**Checkpoint**: `cargo test` — VV + Register tests pass.

---

## Step 3 — OR-Map

**Agent**: Lattice
**Goal**: Observed-Remove Map for JSON objects.

**Files**: `src/ormap.rs`

**Types**:
- `OrMap<K, V>` — keys to CRDT values, with causal context for add/remove

**Operations**:
- `OrMap::new()`
- `OrMap::put(key, value, dot) -> Delta`
- `OrMap::remove(key, vv) -> Delta`
- `OrMap::get(key) -> Option<&V>`
- `OrMap::merge(&mut self, other: &OrMap)`
- `OrMap::keys() -> impl Iterator`

**Tests** (Sentinel):
- [ ] Commutativity, associativity, idempotence
- [ ] Add then remove: key disappears
- [ ] Concurrent add/remove: add wins (OR-Map semantics)
- [ ] Concurrent puts on same key: values coexist until resolved
- [ ] Remove then re-add: key reappears with new value

**Checkpoint**: `cargo test` — VV + Register + OR-Map tests pass.

---

## Step 4 — RGA (Replicated Growable Array)

**Agent**: Lattice
**Goal**: JSON arrays with insert/delete and tombstones.

**Files**: `src/rga.rs`

**Types**:
- `Rga<V>` — sequence of elements with unique IDs and tombstones
- `RgaEntry { id: Dot, value: Option<V>, tombstone: bool }`

**Operations**:
- `Rga::new()`
- `Rga::insert(index, value, dot) -> Delta`
- `Rga::delete(index, vv) -> Delta`
- `Rga::get(index) -> Option<&V>` — skips tombstones
- `Rga::len() -> usize` — visible (non-tombstone) count
- `Rga::merge(&mut self, other: &Rga)`
- `Rga::iter() -> impl Iterator` — visible elements only

**Tests** (Sentinel):
- [ ] Commutativity, associativity, idempotence
- [ ] Sequential insert/delete
- [ ] **Concurrent insert at same index** — deterministic ordering (by Dot)
- [ ] **Concurrent delete of same element** — no double-delete
- [ ] **Insert after tombstone** — correct anchoring
- [ ] Multiple inserts at different positions
- [ ] Delete at head, middle, tail

**Checkpoint**: `cargo test` — all core CRDT tests pass. This is the hardest step.

---

## Step 5 — CrdtValue & Nesting

**Agent**: Lattice + Document
**Goal**: Recursive value type that ties all CRDT types together.

**Files**: `src/value.rs`

**Types**:
```rust
enum CrdtValue {
    Scalar(MvRegister<serde_json::Value>),
    Object(OrMap<String, CrdtValue>),
    Array(Rga<CrdtValue>),
}
```

**Operations**:
- `CrdtValue::merge(&mut self, other: &CrdtValue)`
- `CrdtValue::materialize() -> serde_json::Value`

**Tests** (Sentinel):
- [ ] Nested Object → Scalar: set, merge, materialize
- [ ] Nested Object → Array → Scalar: insert, merge, materialize
- [ ] Nested Object → Object: recursive merge convergence
- [ ] Type mismatch on concurrent set: defined behavior

**Checkpoint**: `cargo test` — core CRDTs + nested values tested.

---

## Step 6 — CrdtDoc & Public API

**Agent**: Document
**Goal**: Top-level document with JsonPath resolution and ergonomic API.

**Files**: `src/doc.rs`

**Operations**:
- `CrdtDoc::new(replica_id: &str)`
- `CrdtDoc::set(path: &str, value: serde_json::Value)`
- `CrdtDoc::remove(path: &str)`
- `CrdtDoc::array_insert(path: &str, index: usize, value: serde_json::Value)`
- `CrdtDoc::array_delete(path: &str, index: usize)`
- `CrdtDoc::materialize() -> serde_json::Value`
- `CrdtDoc::version_vector() -> &VersionVector`

**Tests** (Sentinel):
- [ ] `set` + `materialize` round-trip
- [ ] `set` nested path creates intermediate objects
- [ ] `remove` makes key disappear from materialized output
- [ ] `array_insert` / `array_delete` on nested arrays
- [ ] Multiple operations produce correct JSON output

**Checkpoint**: `cargo test` — document API works end-to-end locally (single replica).

---

## Step 7 — Delta System

**Agent**: Bridge + Document
**Goal**: Delta production and merge across replicas.

**Files**: `src/delta.rs`, update `src/doc.rs`

**Types**:
- `Delta` — captures a set of CRDT mutations
- `DeltaPayload` — serializable wrapper

**Operations**:
- `CrdtDoc::delta_since(vv: &VersionVector) -> Delta`
- `CrdtDoc::merge_delta(delta: Delta)`
- Each mutation (`set`, `remove`, `array_insert`, `array_delete`) returns a `Delta`

**Tests** (Sentinel + Convergence):
- [ ] Single mutation produces a delta, merge reproduces the state
- [ ] `delta_since(empty_vv)` returns full state
- [ ] `delta_since(current_vv)` returns nothing
- [ ] **2-replica convergence**: mutations + delta exchange → equal `materialize()`
- [ ] **3-replica convergence**: cascaded delta exchange
- [ ] Delta merge is commutative, associative, idempotent

**First integration tests** (Convergence):
- [ ] 2 replicas: concurrent `set` on same key → convergence
- [ ] 2 replicas: concurrent array inserts → convergence
- [ ] Delta applied twice has no effect

**Checkpoint**: `cargo test` — multi-replica convergence verified. **Major milestone.**

---

## Step 8 — Codec (Serialization)

**Agent**: Bridge
**Goal**: Binary serialization of deltas for transport.

**Files**: `src/codec.rs`

**Operations**:
- `encode(delta: &Delta) -> Vec<u8>`
- `decode(bytes: &[u8]) -> Result<Delta>`

**Tests** (Sentinel + Convergence):
- [ ] Round-trip: `decode(encode(delta)) == delta`
- [ ] Corrupt bytes return error (not panic)
- [ ] Integration: serialize → transport → deserialize → merge → convergence

**Checkpoint**: `cargo test` — deltas survive serialization round-trip.

---

## Step 9 — Full Integration Tests

**Agent**: Convergence
**Goal**: Comprehensive multi-replica scenarios.

**Files**: `tests/integration/*.rs`

**Scenarios**:
- [ ] 2 replicas: concurrent edits on disjoint keys → merge → convergence
- [ ] 2 replicas: concurrent edits on same key → merge → convergence
- [ ] 3 replicas: chain sync (A→B→C) → all converge
- [ ] 3 replicas: star sync (A→B, A→C, B→C) → all converge
- [ ] 5 replicas: complex partition/reconnect scenario
- [ ] Simulated partition: local mutations → no sync → reconnect → full sync → convergence
- [ ] Duplicate delta delivery: no side effects
- [ ] Out-of-order delta delivery: same final state
- [ ] Deeply nested JSON: Object → Array → Object → Array → Scalar

**Checkpoint**: `cargo test` — all integration tests pass. **Library is feature-complete for Rust.**

---

## Step 10 — WASM Bindings

**Agent**: Bridge
**Goal**: Expose CrdtDoc to JavaScript/TypeScript via WebAssembly.

**Files**: `src/wasm.rs` or `wasm/`

**Bindings**:
- `CrdtDoc::new(replica_id: &str)` → `#[wasm_bindgen]`
- `set`, `remove`, `array_insert`, `array_delete` → `#[wasm_bindgen]`
- `materialize() -> JsValue`
- `delta_since(vv: &[u8]) -> Uint8Array`
- `merge_delta(bytes: &[u8])`
- `version_vector() -> Uint8Array`

**Tests**:
- [ ] `wasm-pack build --target nodejs` compiles
- [ ] `wasm-pack test --node` (basic smoke test)

**Checkpoint**: WASM package builds and basic smoke test passes.

---

## Step 11 — TypeScript Interop Tests

**Agent**: Interop
**Goal**: Prove cross-language convergence via WASM.

**Files**: `tests-ts/*.ts`, `tests-ts/package.json`

**Scenarios**:
- [ ] Scenario 1: Rust → TS — mutation in Rust, delta to TS, convergence
- [ ] Scenario 2: TS → Rust — mutation in TS, delta to Rust, convergence
- [ ] Scenario 3: Cross concurrency — concurrent mutations, both merge orders, convergence
- [ ] Idempotent replay — same delta applied twice, no change
- [ ] Binary round-trip — serialize → deserialize → merge → correct state
- [ ] Complex JSON — nested objects/arrays across the WASM boundary

**Checkpoint**: `npm run test:interop` passes. **V1 is complete.**

---

## Summary

| Step | Agent | What | Key Deliverable |
|------|-------|------|-----------------|
| 0 | — | Scaffolding | `cargo check` passes |
| 1 | Lattice | VersionVector & Dot | Causal context works |
| 2 | Lattice | MV-Register | Scalar CRDT with properties proven |
| 3 | Lattice | OR-Map | Object CRDT with add-wins semantics |
| 4 | Lattice | RGA | Array CRDT with tombstones |
| 5 | Lattice + Document | CrdtValue | Nested CRDT types |
| 6 | Document | CrdtDoc API | Ergonomic public API |
| 7 | Bridge + Document | Delta system | Multi-replica convergence |
| 8 | Bridge | Codec | Binary serialization |
| 9 | Convergence | Integration tests | Full scenario coverage |
| 10 | Bridge | WASM bindings | JS/TS interop ready |
| 11 | Interop | TS tests | Cross-language convergence proven |

**Each step is a commit (or PR). No step is merged without passing `cargo test`.**
