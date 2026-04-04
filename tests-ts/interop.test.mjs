import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { WasmCrdtDoc } from "concordat";

// ─── Helpers ────────────────────────────────────────────────

function fullSync(a, b) {
  const da = a.deltaSince(null);
  const db = b.deltaSince(null);
  a.mergeDelta(db);
  b.mergeDelta(da);
}

function assertConverged(docs) {
  const first = JSON.stringify(docs[0].materialize());
  for (let i = 1; i < docs.length; i++) {
    assert.equal(
      first,
      JSON.stringify(docs[i].materialize()),
      `doc[0] and doc[${i}] diverged`
    );
  }
}

// ═════════════════════════════════════════════════════════════
// Scenario 1 — Rust → TS (mutation in one, delta to another)
// ═════════════════════════════════════════════════════════════

describe("Scenario 1: Rust → TS convergence", () => {
  it("should converge after delta exchange", () => {
    const docA = new WasmCrdtDoc("rust-a");
    docA.set("/name", "Alice");
    docA.set("/age", 30);

    const delta = docA.deltaSince(null);

    const docB = new WasmCrdtDoc("ts-b");
    docB.mergeDelta(delta);

    assert.deepEqual(docA.materialize(), docB.materialize());
  });

  it("should converge with nested objects", () => {
    const docA = new WasmCrdtDoc("rust-a");
    docA.set("/user/name", "Bob");
    docA.set("/user/email", "bob@example.com");

    const delta = docA.deltaSince(null);

    const docB = new WasmCrdtDoc("ts-b");
    docB.mergeDelta(delta);

    const mat = docB.materialize();
    assert.equal(mat.user.name, "Bob");
    assert.equal(mat.user.email, "bob@example.com");
  });

  it("should converge with arrays", () => {
    const docA = new WasmCrdtDoc("rust-a");
    docA.setArray("/items");
    docA.arrayInsert("/items", 0, "first");
    docA.arrayInsert("/items", 1, "second");

    const delta = docA.deltaSince(null);

    const docB = new WasmCrdtDoc("ts-b");
    docB.mergeDelta(delta);

    const mat = docB.materialize();
    assert.deepEqual(mat.items, ["first", "second"]);
  });
});

// ═════════════════════════════════════════════════════════════
// Scenario 2 — TS → Rust (mutation in TS, delta to another)
// ═════════════════════════════════════════════════════════════

describe("Scenario 2: TS → Rust convergence", () => {
  it("should converge when TS mutates and sends delta", () => {
    const docTS = new WasmCrdtDoc("ts-1");
    docTS.set("/color", "blue");
    docTS.set("/count", 42);

    const delta = docTS.deltaSince(null);

    const docRust = new WasmCrdtDoc("rust-1");
    docRust.mergeDelta(delta);

    assert.deepEqual(docTS.materialize(), docRust.materialize());
  });

  it("should handle remove operations", () => {
    const docTS = new WasmCrdtDoc("ts-1");
    docTS.set("/x", 1);
    docTS.set("/y", 2);
    docTS.remove("/x");

    const delta = docTS.deltaSince(null);

    const docRust = new WasmCrdtDoc("rust-1");
    docRust.mergeDelta(delta);

    const mat = docRust.materialize();
    assert.equal(mat.y, 2);
    assert.equal(mat.x, undefined);
  });
});

// ═════════════════════════════════════════════════════════════
// Scenario 3 — Cross concurrency
// ═════════════════════════════════════════════════════════════

describe("Scenario 3: Cross concurrency", () => {
  it("concurrent set on same key converges", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    docA.set("/x", "from_a");
    docB.set("/x", "from_b");

    fullSync(docA, docB);
    assertConverged([docA, docB]);
  });

  it("concurrent set on different keys converges", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    docA.set("/x", 1);
    docB.set("/y", 2);

    fullSync(docA, docB);
    assertConverged([docA, docB]);

    const mat = docA.materialize();
    assert.equal(mat.x, 1);
    assert.equal(mat.y, 2);
  });

  it("merge order does not matter (commutativity)", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    docA.set("/val", "hello");
    docB.set("/val", "world");

    const da = docA.deltaSince(null);
    const db = docB.deltaSince(null);

    // Order 1: A then B
    const r1 = new WasmCrdtDoc("r1");
    r1.mergeDelta(da);
    r1.mergeDelta(db);

    // Order 2: B then A
    const r2 = new WasmCrdtDoc("r2");
    r2.mergeDelta(db);
    r2.mergeDelta(da);

    assertConverged([r1, r2]);
  });

  it("concurrent array inserts converge", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    // Both create arrays, sync
    docA.setArray("/items");
    docB.setArray("/items");
    fullSync(docA, docB);

    // Concurrent inserts
    docA.arrayInsert("/items", 0, "from_a");
    docB.arrayInsert("/items", 0, "from_b");

    fullSync(docA, docB);
    assertConverged([docA, docB]);

    const items = docA.materialize().items;
    assert.equal(items.length, 2);
    assert.ok(items.includes("from_a"));
    assert.ok(items.includes("from_b"));
  });
});

// ═════════════════════════════════════════════════════════════
// Idempotent replay
// ═════════════════════════════════════════════════════════════

describe("Idempotent delta replay", () => {
  it("applying same delta multiple times has no effect", () => {
    const doc = new WasmCrdtDoc("a");
    doc.set("/key", "value");
    doc.set("/num", 42);

    const delta = doc.deltaSince(null);

    const replica = new WasmCrdtDoc("b");
    replica.mergeDelta(delta);
    const afterFirst = JSON.stringify(replica.materialize());

    // Apply 5 more times
    for (let i = 0; i < 5; i++) {
      replica.mergeDelta(delta);
    }

    assert.equal(JSON.stringify(replica.materialize()), afterFirst);
  });
});

// ═════════════════════════════════════════════════════════════
// Binary round-trip
// ═════════════════════════════════════════════════════════════

describe("Binary round-trip integrity", () => {
  it("delta bytes survive round-trip", () => {
    const doc = new WasmCrdtDoc("a");
    doc.set("/msg", "hello");
    doc.setArray("/items");
    doc.arrayInsert("/items", 0, 1);
    doc.arrayInsert("/items", 1, 2);

    const bytes = doc.deltaSince(null);

    // bytes is a Uint8Array — verify it's non-empty
    assert.ok(bytes instanceof Uint8Array);
    assert.ok(bytes.length > 0);

    // Apply to a fresh doc
    const replica = new WasmCrdtDoc("b");
    replica.mergeDelta(bytes);

    assert.deepEqual(doc.materialize(), replica.materialize());
  });

  it("version vector is a non-empty Uint8Array after mutations", () => {
    const doc = new WasmCrdtDoc("a");
    doc.set("/x", 1);

    const vv = doc.versionVector();
    assert.ok(vv instanceof Uint8Array);
    assert.ok(vv.length > 0);
  });
});

// ═════════════════════════════════════════════════════════════
// Complex nested JSON across WASM boundary
// ═════════════════════════════════════════════════════════════

describe("Complex JSON across WASM boundary", () => {
  it("nested objects and arrays converge across replicas", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    // A builds user with scores array
    docA.set("/users/alice/name", "Alice");
    docA.setArray("/users/alice/scores");
    docA.arrayInsert("/users/alice/scores", 0, 95);

    // B builds a different user
    docB.set("/users/bob/name", "Bob");
    docB.setArray("/users/bob/scores");
    docB.arrayInsert("/users/bob/scores", 0, 88);

    fullSync(docA, docB);
    assertConverged([docA, docB]);

    const mat = docA.materialize();
    assert.equal(mat.users.alice.name, "Alice");
    assert.equal(mat.users.bob.name, "Bob");
  });

  it("mixed types in same document converge", () => {
    const docA = new WasmCrdtDoc("a");
    const docB = new WasmCrdtDoc("b");

    docA.set("/title", "My Document");
    docA.set("/version", 1);
    docA.set("/published", true);

    docB.set("/author", "Concordat");
    docB.setArray("/tags");
    docB.arrayInsert("/tags", 0, "crdt");
    docB.arrayInsert("/tags", 1, "distributed");

    fullSync(docA, docB);
    assertConverged([docA, docB]);

    const mat = docA.materialize();
    assert.equal(mat.title, "My Document");
    assert.equal(mat.author, "Concordat");
    assert.ok(Array.isArray(mat.tags));
    assert.equal(mat.tags.length, 2);
  });
});
