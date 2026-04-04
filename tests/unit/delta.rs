use concordat::doc::CrdtDoc;
use concordat::vv::VersionVector;
use serde_json::json;

// ─── Single mutation delta round-trip ───────────────────────

#[test]
fn test_delta_single_mutation_roundtrip() {
    let mut doc_a = CrdtDoc::new("a");
    doc_a.set("/x", json!(42));

    let delta = doc_a.delta_since(&VersionVector::new());

    let mut doc_b = CrdtDoc::new("b");
    doc_b.merge_delta(&delta);

    assert_eq!(doc_a.materialize(), doc_b.materialize());
}

// ─── delta_since(empty) returns full state ──────────────────

#[test]
fn test_delta_since_empty_returns_full_state() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/name", json!("Alice"));
    doc.set("/age", json!(30));

    let delta = doc.delta_since(&VersionVector::new());
    assert!(!delta.is_empty());

    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&delta);
    assert_eq!(doc.materialize(), replica.materialize());
}

// ─── delta_since(current_vv) returns state (idempotent merge) ─

#[test]
fn test_delta_since_current_vv_is_safe() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/x", json!(1));

    let delta = doc.delta_since(doc.version_vector());

    // Merging the current state into itself should be a no-op
    let before = doc.materialize();
    doc.merge_delta(&delta);
    assert_eq!(doc.materialize(), before);
}

// ─── 2-replica convergence ──────────────────────────────────

#[test]
fn test_2_replica_convergence_disjoint_keys() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");

    doc_a.set("/x", json!(1));
    doc_b.set("/y", json!(2));

    let delta_a = doc_a.delta_since(&VersionVector::new());
    let delta_b = doc_b.delta_since(&VersionVector::new());

    doc_a.merge_delta(&delta_b);
    doc_b.merge_delta(&delta_a);

    assert_eq!(doc_a.materialize(), doc_b.materialize());
    assert_eq!(doc_a.materialize()["x"], json!(1));
    assert_eq!(doc_a.materialize()["y"], json!(2));
}

#[test]
fn test_2_replica_convergence_same_key() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");

    doc_a.set("/x", json!("from_a"));
    doc_b.set("/x", json!("from_b"));

    let delta_a = doc_a.delta_since(&VersionVector::new());
    let delta_b = doc_b.delta_since(&VersionVector::new());

    doc_a.merge_delta(&delta_b);
    doc_b.merge_delta(&delta_a);

    // Both must converge to the same value (deterministic winner)
    assert_eq!(doc_a.materialize(), doc_b.materialize());
}

// ─── 3-replica convergence ──────────────────────────────────

#[test]
fn test_3_replica_convergence_cascaded() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");
    let mut doc_c = CrdtDoc::new("c");

    doc_a.set("/x", json!(1));
    doc_b.set("/y", json!(2));
    doc_c.set("/z", json!(3));

    // A → B
    let delta_a = doc_a.delta_since(&VersionVector::new());
    doc_b.merge_delta(&delta_a);

    // B (now has A+B) → C
    let delta_b = doc_b.delta_since(&VersionVector::new());
    doc_c.merge_delta(&delta_b);

    // C (now has A+B+C) → A
    let delta_c = doc_c.delta_since(&VersionVector::new());
    doc_a.merge_delta(&delta_c);

    // A (now has all) → B
    let delta_a2 = doc_a.delta_since(&VersionVector::new());
    doc_b.merge_delta(&delta_a2);

    assert_eq!(doc_a.materialize(), doc_b.materialize());
    assert_eq!(doc_b.materialize(), doc_c.materialize());
}

// ─── Delta merge commutativity ──────────────────────────────

#[test]
fn test_delta_merge_commutativity() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");

    doc_a.set("/x", json!("hello"));
    doc_b.set("/y", json!("world"));

    let delta_a = doc_a.delta_since(&VersionVector::new());
    let delta_b = doc_b.delta_since(&VersionVector::new());

    // Order 1: merge A then B
    let mut replica1 = CrdtDoc::new("r1");
    replica1.merge_delta(&delta_a);
    replica1.merge_delta(&delta_b);

    // Order 2: merge B then A
    let mut replica2 = CrdtDoc::new("r2");
    replica2.merge_delta(&delta_b);
    replica2.merge_delta(&delta_a);

    assert_eq!(replica1.materialize(), replica2.materialize());
}

// ─── Delta merge idempotence ────────────────────────────────

#[test]
fn test_delta_merge_idempotence() {
    let mut doc = CrdtDoc::new("a");
    doc.set("/x", json!(42));
    doc.set("/y", json!("hello"));

    let delta = doc.delta_since(&VersionVector::new());

    let mut replica = CrdtDoc::new("b");
    replica.merge_delta(&delta);
    let after_first = replica.materialize();

    // Apply same delta again
    replica.merge_delta(&delta);
    assert_eq!(replica.materialize(), after_first);

    // And again
    replica.merge_delta(&delta);
    assert_eq!(replica.materialize(), after_first);
}

// ─── 2-replica concurrent array inserts ─────────────────────

#[test]
fn test_2_replica_concurrent_array_inserts() {
    let mut doc_a = CrdtDoc::new("a");
    let mut doc_b = CrdtDoc::new("b");

    // Both create the same array path
    doc_a.set_array("/items");
    doc_b.set_array("/items");

    // Sync the array creation first
    let init_a = doc_a.delta_since(&VersionVector::new());
    let init_b = doc_b.delta_since(&VersionVector::new());
    doc_a.merge_delta(&init_b);
    doc_b.merge_delta(&init_a);

    // Now concurrent inserts
    doc_a.array_insert("/items", 0, json!("from_a"));
    doc_b.array_insert("/items", 0, json!("from_b"));

    let delta_a = doc_a.delta_since(&VersionVector::new());
    let delta_b = doc_b.delta_since(&VersionVector::new());

    doc_a.merge_delta(&delta_b);
    doc_b.merge_delta(&delta_a);

    assert_eq!(doc_a.materialize(), doc_b.materialize());
}
