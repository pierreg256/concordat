## Agent

Which agent scope does this PR belong to?

- [ ] **Lattice** – Core CRDT (OR-Map, RGA, Register, VersionVector)
- [ ] **Document** – Document & API (CrdtDoc, JsonPath, materialize)
- [ ] **Bridge** – Delta, Codec & WASM
- [ ] **Sentinel** – Unit Tests
- [ ] **Convergence** – Integration Tests
- [ ] **Interop** – TypeScript Interop Tests

## Description

<!-- What does this PR do? -->

## CRDT Invariants

- [ ] Commutativity preserved
- [ ] Associativity preserved
- [ ] Idempotence preserved
- [ ] No implicit Last-Writer-Wins
- [ ] No wall clocks used
- [ ] No network dependency introduced

## Checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt` applied
- [ ] New/updated tests for all behavioral changes
- [ ] WASM rebuilt and TS tests pass (if codec/WASM touched)
