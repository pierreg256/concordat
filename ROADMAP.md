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

## Step 2 — MV-Register ✅

**Agent**: Lattice
**Goal**: Multi-Value Register for JSON scalar values. Simplest CRDT type.

**Files**: `src/register.rs`

**Types**:
- `MvRegister<V>` — stores concurrent values tagged with dots

**Operations**:
- `MvRegister::new()`
- `MvRegister::set(value, dot, vv)`
- `MvRegister::merge(&mut self, other: &MvRegister)`
- `MvRegister::values() -> Vec<&V>` — all concurrent values
- `MvRegister::value() -> Option<&V>` — single value (if no conflict)

**Tests** (Sentinel): 10 tests
- [x] Commutativity: `merge(A, B) == merge(B, A)`
- [x] Associativity: `merge(merge(A, B), C) == merge(A, merge(B, C))`
- [x] Idempotence: `merge(A, A) == A`
- [x] Sequential writes: later write wins (causally)
- [x] Concurrent writes: both values preserved
- [x] Merge after concurrent writes resolves correctly

**Checkpoint**: ✅ `cargo test` — 27/27 tests pass (VV + Register).

---

## Step 3 — OR-Map ✅

**Agent**: Lattice
**Goal**: Observed-Remove Map for JSON objects.

**Files**: `src/ormap.rs`

**Types**:
- `OrMap<K, V>` — keys to CRDT values, with dot-per-value tracking

**Operations**:
- `OrMap::new()`
- `OrMap::put(key, value, dot)`
- `OrMap::remove(key, vv) -> bool`
- `OrMap::get(key) -> Option<&V>`
- `OrMap::merge(&mut self, other: &OrMap)`
- `OrMap::keys() -> impl Iterator`

**Tests** (Sentinel): 12 tests
- [x] Commutativity, associativity, idempotence
- [x] Add then remove: key disappears
- [x] Concurrent add/remove: add wins (OR-Map semantics)
- [x] Concurrent puts on same key: both present
- [x] Remove then re-add: key reappears with new value

**Checkpoint**: ✅ `cargo test` — 39/39 tests pass (VV + Register + OR-Map).

---

## Step 4 — RGA (Replicated Growable Array) ✅

**Agent**: Lattice
**Goal**: JSON arrays with insert/delete and tombstones.

**Files**: `src/rga.rs`

**Types**:
- `Rga<V>` — sequence with dot-identified nodes and tombstones
- `RgaNode { id: Dot, anchor: Option<Dot>, value: Option<V> }`

**Operations**:
- `Rga::new()`
- `Rga::insert(index, value, dot)`
- `Rga::delete(index) -> bool`
- `Rga::get(index) -> Option<&V>`
- `Rga::len() -> usize`
- `Rga::merge(&mut self, other: &Rga)`
- `Rga::iter() -> impl Iterator`

**Implementation note**: Uses DFS-based canonical reordering after each insert/merge to guarantee commutativity. Siblings with the same anchor are sorted by descending dot.

**Tests** (Sentinel): 13 tests
- [x] Commutativity, associativity, idempotence
- [x] Sequential insert/delete
- [x] **Concurrent insert at same index** — deterministic ordering (by Dot)
- [x] **Concurrent delete of same element** — no double-delete
- [x] **Insert after tombstone** — correct anchoring
- [x] Multiple inserts at different positions
- [x] Delete at head, middle, tail

**Checkpoint**: ✅ `cargo test` — 52/52 tests pass.

---

## Step 5 — CrdtValue & Nesting ✅

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

**Tests** (Sentinel): 8 tests
- [x] Nested Object → Scalar: set, merge, materialize
- [x] Nested Object → Array → Scalar: insert, merge, materialize
- [x] Nested Object → Object: recursive merge convergence
- [x] Type mismatch on concurrent set: Object wins over Array/Scalar

**Checkpoint**: ✅ `cargo test` — 60/60 tests pass.

---

## Step 6 — CrdtDoc & Public API ✅

**Agent**: Document
**Goal**: Top-level document with JsonPath resolution and ergonomic API.

**Files**: `src/doc.rs`

**Operations**:
- `CrdtDoc::new(replica_id: &str)`
- `CrdtDoc::set(path: &str, value: serde_json::Value)`
- `CrdtDoc::set_array(path: &str)`
- `CrdtDoc::remove(path: &str)`
- `CrdtDoc::array_insert(path: &str, index: usize, value: serde_json::Value)`
- `CrdtDoc::array_delete(path: &str, index: usize)`
- `CrdtDoc::materialize() -> serde_json::Value`
- `CrdtDoc::version_vector() -> &VersionVector`

**Tests** (Sentinel): 11 tests
- [x] `set` + `materialize` round-trip
- [x] `set` nested path creates intermediate objects
- [x] `remove` makes key disappear from materialized output
- [x] `array_insert` / `array_delete` on nested arrays
- [x] Multiple operations produce correct JSON output

**Checkpoint**: ✅ `cargo test` — 71/71 tests pass. Document API works end-to-end (single replica).

---

## Step 7 — Delta System ✅

**Agent**: Bridge + Document
**Goal**: Delta production and merge across replicas.

**Files**: `src/delta.rs`, updated `src/doc.rs`

**Types**:
- `Delta` — full state fragment (root OrMap + VersionVector)

**Operations**:
- `CrdtDoc::delta_since(vv: &VersionVector) -> Delta`
- `CrdtDoc::merge_delta(delta: &Delta)`

**Key design**: Delta is a full state snapshot. Merge is idempotent — sending more than needed is safe. OR-Map merge now recursively merges nested `CrdtValue`s when the same key exists on both sides with shared dots (`ValueMerge` trait). Removed dots tracked explicitly for correct add-wins semantics across multi-hop sync.

**Tests** (Sentinel + Convergence): 9 tests
- [x] Single mutation produces a delta, merge reproduces the state
- [x] `delta_since(empty_vv)` returns full state
- [x] `delta_since(current_vv)` — merging is idempotent
- [x] **2-replica convergence**: disjoint keys + same key
- [x] **3-replica convergence**: cascaded delta exchange
- [x] Delta merge is commutative, associative, idempotent
- [x] 2 replicas: concurrent array inserts → convergence

**Checkpoint**: ✅ `cargo test` — multi-replica convergence verified.

---

## Step 8 — Codec (Serialization) ✅

**Agent**: Bridge
**Goal**: Binary serialization of deltas for transport.

**Files**: `src/codec.rs`

**Operations**:
- `encode(delta: &Delta) -> Vec<u8>`
- `decode(bytes: &[u8]) -> Result<Delta, CodecError>`

**Implementation**: Uses `serde_json` for now (portable, debuggable). A binary format (CBOR, MessagePack) can be swapped without API change.

**Tests** (Sentinel): 7 tests
- [x] Round-trip: `decode(encode(delta))` reproduces state
- [x] Corrupt bytes return error (not panic)
- [x] Empty bytes return error
- [x] Integration: serialize → transport → deserialize → merge → convergence
- [x] Idempotent round-trip decode+merge

**Checkpoint**: ✅ `cargo test` — deltas survive serialization round-trip.

---

## Step 9 — Full Integration Tests ✅

**Agent**: Convergence
**Goal**: Comprehensive multi-replica scenarios.

**Files**: `tests/integration/convergence.rs`

**Scenarios**: 12 tests
- [x] 2 replicas: concurrent edits on disjoint keys → merge → convergence
- [x] 2 replicas: concurrent edits on same key → merge → convergence
- [x] 2 replicas: concurrent array inserts → convergence
- [x] 3 replicas: chain sync (A→B→C) → all converge
- [x] 3 replicas: star sync (A→B, A→C, B→C) → all converge
- [x] 5 replicas: complex partition/reconnect scenario
- [x] Simulated partition: local mutations → no sync → reconnect → full sync → convergence
- [x] Duplicate delta delivery: no side effects (idempotence)
- [x] Out-of-order delta delivery: same final state (commutativity)
- [x] Deeply nested JSON: Object → Array → Object convergence
- [x] Convergence via serialized bytes (full transport simulation)
- [x] Concurrent set + remove: add-wins semantics

**Checkpoint**: ✅ `cargo test` — 99 tests pass (87 unit + 12 integration). **Library is feature-complete for Rust.**

---

## Step 10 — WASM Bindings ✅

**Agent**: Bridge
**Goal**: Expose CrdtDoc to JavaScript/TypeScript via WebAssembly.

**Files**: `src/wasm.rs`

**Type**: `WasmCrdtDoc` — `#[wasm_bindgen]` wrapper around `CrdtDoc`

**Bindings**:
- `new(replica_id)` → constructor
- `set(path, value)` / `setArray(path)` / `remove(path)`
- `arrayInsert(path, index, value)` / `arrayDelete(path, index)`
- `materialize()` → plain JS object (via JSON.parse)
- `deltaSince(sinceBytes?)` → `Uint8Array`
- `mergeDelta(bytes)` → void
- `versionVector()` → `Uint8Array`
- `replicaId()` → string

**Dependencies**: `wasm-bindgen`, `serde-wasm-bindgen`, `js-sys`

**Tests**:
- [x] `wasm-pack build --target nodejs` compiles
- [x] All 99 Rust tests still pass

**Checkpoint**: ✅ WASM package builds at `pkg/`.

---

## Step 11 — TypeScript Interop Tests ✅

**Agent**: Interop
**Goal**: Prove cross-language convergence via WASM.

**Files**: `tests-ts/interop.test.mjs`, `tests-ts/package.json`

**Scenarios**: 14 tests across 6 suites
- [x] Scenario 1: Rust → TS — simple values, nested objects, arrays (3 tests)
- [x] Scenario 2: TS → Rust — mutations + remove operations (2 tests)
- [x] Scenario 3: Cross concurrency — same key, disjoint keys, commutativity, arrays (4 tests)
- [x] Idempotent replay — same delta applied 5+ times, no change (1 test)
- [x] Binary round-trip — Uint8Array survives encode→decode→merge (2 tests)
- [x] Complex JSON — nested objects/arrays, mixed types across WASM boundary (2 tests)

**Checkpoint**: ✅ `npm run test:interop` — 14/14 pass. **V1 is complete.**

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
